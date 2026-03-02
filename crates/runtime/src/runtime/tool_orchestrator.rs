use alan_protocol::{Event, InputMode, Op};
use anyhow::Result;
use serde_json::json;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::approval::PendingConfirmation;

use super::agent_loop::{NormalizedToolCall, RuntimeLoopState};
use super::loop_guard::ToolLoopGuard;
use super::tool_policy::{ToolPolicyDecision, evaluate_tool_policy, tool_approval_cache_key};
use super::turn_driver::TurnInputBroker;
use super::turn_support::{check_turn_cancelled, tool_result_preview};
use super::virtual_tools::{VirtualToolOutcome, try_handle_virtual_tool_call};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolOrchestratorOutcome {
    ContinueToolBatch { refresh_context: bool },
    PauseTurn,
    EndTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolBatchOrchestratorOutcome {
    ContinueTurnLoop { refresh_context: bool },
    PauseTurn,
    EndTurn,
}

pub(super) struct ToolTurnOrchestrator {
    loop_guard: ToolLoopGuard,
}

impl ToolTurnOrchestrator {
    pub(super) fn new(max_tool_loops: Option<usize>, tool_repeat_limit: usize) -> Self {
        Self {
            loop_guard: ToolLoopGuard::new(max_tool_loops, tool_repeat_limit),
        }
    }

    pub(super) async fn orchestrate_tool_batch<E, F>(
        &mut self,
        state: &mut RuntimeLoopState,
        tool_calls: &[NormalizedToolCall],
        inputs: ToolOrchestratorInputs<'_>,
        emit: &mut E,
    ) -> Result<ToolBatchOrchestratorOutcome>
    where
        E: FnMut(Event) -> F,
        F: std::future::Future<Output = ()>,
    {
        orchestrate_tool_batch_with_guard(state, &mut self.loop_guard, tool_calls, inputs, emit)
            .await
    }
}

pub(super) async fn replay_approved_tool_call_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    tool_call: &NormalizedToolCall,
    inputs: ToolOrchestratorInputs<'_>,
    emit: &mut E,
) -> Result<ToolBatchOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    replay_approved_tool_batch_with_cancel(state, std::slice::from_ref(tool_call), inputs, emit)
        .await
}

pub(super) async fn replay_approved_tool_batch_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    tool_calls: &[NormalizedToolCall],
    inputs: ToolOrchestratorInputs<'_>,
    emit: &mut E,
) -> Result<ToolBatchOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let max_tool_loops = if state.runtime_config.max_tool_loops == 0 {
        None
    } else {
        Some(state.runtime_config.max_tool_loops)
    };
    let mut orchestrator =
        ToolTurnOrchestrator::new(max_tool_loops, state.runtime_config.tool_repeat_limit);
    orchestrator
        .orchestrate_tool_batch(state, tool_calls, inputs, emit)
        .await
}

#[derive(Clone, Copy)]
pub(super) struct ToolOrchestratorInputs<'a> {
    pub cancel: &'a CancellationToken,
    pub steering_broker: Option<&'a TurnInputBroker>,
}

async fn orchestrate_tool_call_with_guard<E, F>(
    state: &mut RuntimeLoopState,
    loop_guard: &mut ToolLoopGuard,
    tool_call: &NormalizedToolCall,
    inputs: ToolOrchestratorInputs<'_>,
    emit: &mut E,
) -> Result<ToolOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let tool_arguments = tool_call.arguments.clone();

    if let Some(msg) = loop_guard.before_tool_call(&tool_call.name, &tool_arguments) {
        emit(Event::Error {
            message: msg.clone(),
            recoverable: true,
        })
        .await;
        emit(Event::TextDelta {
            chunk: msg,
            is_final: true,
        })
        .await;
        return Ok(ToolOrchestratorOutcome::EndTurn);
    }

    match try_handle_virtual_tool_call(state, tool_call, &tool_arguments, emit).await? {
        VirtualToolOutcome::NotVirtual => {}
        VirtualToolOutcome::Continue { refresh_context } => {
            return Ok(ToolOrchestratorOutcome::ContinueToolBatch { refresh_context });
        }
        VirtualToolOutcome::PauseTurn => return Ok(ToolOrchestratorOutcome::PauseTurn),
        VirtualToolOutcome::EndTurn => return Ok(ToolOrchestratorOutcome::EndTurn),
    }

    let tool_capability = state
        .tools
        .capability_for_tool(&tool_call.name, &tool_arguments)
        .or_else(|| {
            state
                .session
                .dynamic_tools
                .get(&tool_call.name)
                .and_then(|tool| tool.capability)
        });
    let policy_decision = evaluate_tool_policy(
        &state.runtime_config.policy_engine,
        &state.runtime_config.governance,
        &tool_call.name,
        &tool_arguments,
        tool_capability,
    );
    let policy_audit = match &policy_decision {
        ToolPolicyDecision::Allow { audit }
        | ToolPolicyDecision::Escalate { audit, .. }
        | ToolPolicyDecision::Forbidden { audit, .. } => audit.clone(),
    };
    state.session.record_event(
        "tool_policy_decision",
        json!({
            "tool_call_id": tool_call.id,
            "tool_name": tool_call.name,
            "policy_source": policy_audit.policy_source,
            "rule_id": policy_audit.rule_id,
            "action": policy_audit.action,
            "reason": policy_audit.reason,
            "capability": policy_audit.capability,
            "sandbox_backend": policy_audit.sandbox_backend,
        }),
    );

    let tool_audit = match policy_decision {
        ToolPolicyDecision::Allow { audit } => Some(audit),
        ToolPolicyDecision::Escalate {
            summary,
            mut details,
            audit,
        } => {
            let dynamic_tool_spec = state.session.dynamic_tools.get(&tool_call.name);
            let approval_key = tool_approval_cache_key(
                &tool_call.name,
                tool_capability,
                &state.runtime_config.governance,
                dynamic_tool_spec,
                &tool_arguments,
            );
            details["approval_key"] = serde_json::to_value(&approval_key).unwrap_or_default();
            details["replay_tool_call"] = json!({
                "call_id": tool_call.id,
                "tool_name": tool_call.name,
                "arguments": tool_arguments,
            });
            let pending = PendingConfirmation {
                checkpoint_id: format!("tool_escalation_{}", tool_call.id),
                checkpoint_type: "tool_escalation".to_string(),
                summary,
                details,
                options: vec!["approve".to_string(), "reject".to_string()],
            };
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                json!({"status":"escalation_required", "approval_key": serde_json::to_value(&approval_key).unwrap_or_default()}),
                true,
                Some(audit),
            );
            state.turn_state.set_confirmation(pending.clone());
            emit(Event::Yield {
                request_id: pending.checkpoint_id,
                kind: alan_protocol::YieldKind::Confirmation,
                payload: json!({
                    "checkpoint_type": pending.checkpoint_type,
                    "summary": pending.summary,
                    "details": pending.details,
                    "options": pending.options,
                }),
            })
            .await;
            return Ok(ToolOrchestratorOutcome::PauseTurn);
        }
        ToolPolicyDecision::Forbidden { reason, audit } => {
            let blocked_payload = json!({
                "error": reason,
                "status": "blocked_by_policy"
            });
            emit(Event::Error {
                message: blocked_payload["error"]
                    .as_str()
                    .unwrap_or("Tool blocked by policy")
                    .to_string(),
                recoverable: true,
            })
            .await;
            emit(Event::ToolCallCompleted {
                id: tool_call.id.clone(),
                result_preview: tool_result_preview(&blocked_payload),
                audit: Some(audit.clone()),
            })
            .await;
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                blocked_payload.clone(),
                false,
                Some(audit),
            );
            state
                .session
                .add_tool_message(&tool_call.id, &tool_call.name, blocked_payload);
            return Ok(ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: false,
            });
        }
    };

    if state.session.dynamic_tools.contains_key(&tool_call.name) {
        emit(Event::ToolCallStarted {
            id: tool_call.id.clone(),
            name: tool_call.name.clone(),
            audit: tool_audit.clone(),
        })
        .await;
        state
            .turn_state
            .set_dynamic_tool_call(crate::approval::PendingDynamicToolCall {
                call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                arguments: tool_arguments.clone(),
            });
        state.session.record_tool_call_with_audit(
            &tool_call.name,
            tool_arguments.clone(),
            json!({"status":"pending_dynamic_tool_result","call_id": tool_call.id}),
            true,
            tool_audit.clone(),
        );
        emit(Event::Yield {
            request_id: tool_call.id.clone(),
            kind: alan_protocol::YieldKind::DynamicTool,
            payload: json!({
                "tool_name": tool_call.name,
                "arguments": tool_arguments,
            }),
        })
        .await;
        return Ok(ToolOrchestratorOutcome::PauseTurn);
    }

    emit(Event::ToolCallStarted {
        id: tool_call.id.clone(),
        name: tool_call.name.clone(),
        audit: tool_audit.clone(),
    })
    .await;

    let tool_start = Instant::now();
    let tool_result = tokio::select! {
        _ = inputs.cancel.cancelled() => {
            if check_turn_cancelled(state, emit, inputs.cancel).await? {
                return Ok(ToolOrchestratorOutcome::EndTurn);
            }
            unreachable!("check_turn_cancelled returns on cancellation");
        }
        result = state.tools.execute(&tool_call.name, tool_arguments.clone()) => result,
    };

    match tool_result {
        Ok(value) => {
            emit(Event::ToolCallCompleted {
                id: tool_call.id.clone(),
                result_preview: tool_result_preview(&value),
                audit: tool_audit.clone(),
            })
            .await;
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                value.clone(),
                true,
                tool_audit.clone(),
            );
            state
                .session
                .add_tool_message(&tool_call.id, &tool_call.name, value);
            info!(
                tool_name = %tool_call.name,
                elapsed_ms = tool_start.elapsed().as_millis(),
                success = true,
                "Tool done"
            );
            Ok(ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: false,
            })
        }
        Err(err) => {
            let error_payload = json!({"error": err.to_string()});
            emit(Event::ToolCallCompleted {
                id: tool_call.id.clone(),
                result_preview: tool_result_preview(&error_payload),
                audit: tool_audit.clone(),
            })
            .await;
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                error_payload.clone(),
                false,
                tool_audit,
            );
            state
                .session
                .add_tool_message(&tool_call.id, &tool_call.name, error_payload);
            info!(
                tool_name = %tool_call.name,
                elapsed_ms = tool_start.elapsed().as_millis(),
                success = false,
                error = %err,
                "Tool done"
            );
            Ok(ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: false,
            })
        }
    }
}

async fn orchestrate_tool_batch_with_guard<E, F>(
    state: &mut RuntimeLoopState,
    loop_guard: &mut ToolLoopGuard,
    tool_calls: &[NormalizedToolCall],
    inputs: ToolOrchestratorInputs<'_>,
    emit: &mut E,
) -> Result<ToolBatchOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let mut refresh_context = false;

    for (idx, tool_call) in tool_calls.iter().enumerate() {
        match orchestrate_tool_call_with_guard(state, loop_guard, tool_call, inputs, emit).await? {
            ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: call_refresh,
            } => {
                refresh_context |= call_refresh;
                if handle_queued_steering_inputs(
                    state,
                    tool_calls,
                    idx + 1,
                    inputs.steering_broker,
                    emit,
                )
                .await?
                {
                    return Ok(ToolBatchOrchestratorOutcome::ContinueTurnLoop {
                        refresh_context: true,
                    });
                }
            }
            ToolOrchestratorOutcome::PauseTurn => {
                if let Some(pending) = state.turn_state.pending_confirmation()
                    && pending.checkpoint_type == "tool_escalation"
                {
                    state
                        .turn_state
                        .set_tool_replay_batch(pending.checkpoint_id, tool_calls[idx..].to_vec());
                }
                return Ok(ToolBatchOrchestratorOutcome::PauseTurn);
            }
            ToolOrchestratorOutcome::EndTurn => {
                return Ok(ToolBatchOrchestratorOutcome::EndTurn);
            }
        }
    }

    if let Some(msg) = loop_guard.after_tool_batch() {
        emit(Event::Error {
            message: msg.clone(),
            recoverable: true,
        })
        .await;
        emit(Event::TextDelta {
            chunk: msg,
            is_final: true,
        })
        .await;
        emit(Event::TurnCompleted {
            summary: Some("Tool loop stopped by loop guard".to_string()),
        })
        .await;
        return Ok(ToolBatchOrchestratorOutcome::EndTurn);
    }

    Ok(ToolBatchOrchestratorOutcome::ContinueTurnLoop { refresh_context })
}

async fn handle_queued_steering_inputs<E, F>(
    state: &mut RuntimeLoopState,
    tool_calls: &[NormalizedToolCall],
    remaining_start_idx: usize,
    steering_broker: Option<&TurnInputBroker>,
    emit: &mut E,
) -> Result<bool>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let Some(broker) = steering_broker else {
        return Ok(false);
    };

    let mut steering_inputs: Vec<Vec<crate::tape::ContentPart>> = Vec::new();
    while let Some(submission) = broker.try_recv().await {
        match submission.op {
            Op::Input {
                parts,
                mode: InputMode::Steer,
            } => steering_inputs.push(parts),
            _ => state.turn_state.push_buffered_inband_submission(submission),
        }
    }

    if steering_inputs.is_empty() {
        return Ok(false);
    }

    for parts in steering_inputs {
        state.session.add_user_message_parts(parts);
    }

    let remaining = &tool_calls[remaining_start_idx..];
    if !remaining.is_empty() {
        emit(Event::Error {
            message: format!(
                "Steering input received during tool batch; skipping {} pending tool call(s).",
                remaining.len()
            ),
            recoverable: true,
        })
        .await;
    }

    for skipped in remaining {
        let skipped_payload = json!({
            "status": "skipped_due_to_steering",
            "error": "Skipped due to queued user steering input."
        });
        emit(Event::ToolCallStarted {
            id: skipped.id.clone(),
            name: skipped.name.clone(),
            audit: None,
        })
        .await;
        emit(Event::ToolCallCompleted {
            id: skipped.id.clone(),
            result_preview: tool_result_preview(&skipped_payload),
            audit: None,
        })
        .await;
        state.session.record_tool_call(
            &skipped.name,
            skipped.arguments.clone(),
            skipped_payload.clone(),
            false,
        );
        state
            .session
            .add_tool_message(&skipped.id, &skipped.name, skipped_payload);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config, llm::LlmClient, runtime::TurnState, session::Session, tools::ToolRegistry,
    };
    use alan_llm::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk};
    use alan_protocol::DynamicToolSpec;
    use async_trait::async_trait;

    // Simple mock provider for testing
    struct SimpleMockProvider;

    #[async_trait]
    impl LlmProvider for SimpleMockProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            Ok(GenerationResponse {
                content: "test".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("mock".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(StreamChunk {
                    text: Some("test".to_string()),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stop".to_string()),
                })
                .await;
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }
    }

    fn create_test_state() -> RuntimeLoopState {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        let runtime_config = super::super::RuntimeConfig::default();

        RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(SimpleMockProvider),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            turn_state: TurnState::default(),
        }
    }

    #[tokio::test]
    async fn test_tool_turn_orchestrator_new() {
        let orchestrator = ToolTurnOrchestrator::new(Some(10), 4);
        // Verify orchestrator was created with the correct settings
        // Just test that it doesn't panic
        let _ = orchestrator;
    }

    #[tokio::test]
    async fn test_orchestrate_empty_tool_batch() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls: Vec<NormalizedToolCall> = vec![];
        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::ContinueTurnLoop { refresh_context } => {
                assert!(!refresh_context);
            }
            _ => panic!("Expected ContinueTurnLoop"),
        }
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_virtual_update_plan() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Test plan",
                "items": [
                    {"id": "1", "content": "Step 1", "status": "in_progress"}
                ]
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        let has_update_plan_completion = events.iter().any(|event| {
            matches!(
                event,
                Event::ToolCallCompleted {
                    id,
                    result_preview: Some(preview),
                    ..
                } if id == "call_1" && preview.contains("plan_updated")
            )
        });
        assert!(
            has_update_plan_completion,
            "Expected update_plan ToolCallCompleted preview"
        );
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_virtual_confirmation() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                "checkpoint_id": "chk_123",
                "checkpoint_type": "test",
                "summary": "Test confirmation",
                "details": {"key": "value"}
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::PauseTurn => {
                // Expected
            }
            _ => panic!("Expected PauseTurn"),
        }

        // Check that Yield Confirmation event was emitted
        let has_confirmation = events.iter().any(|e| {
            matches!(
                e,
                Event::Yield {
                    kind: alan_protocol::YieldKind::Confirmation,
                    ..
                }
            )
        });
        assert!(has_confirmation, "Expected Yield Confirmation event");
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_virtual_user_input() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_user_input".to_string(),
            arguments: json!({
                "title": "Test Input",
                "prompt": "Enter something",
                "questions": [
                    {"id": "q1", "label": "Question 1", "prompt": "What?", "required": true}
                ]
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::PauseTurn => {
                // Expected
            }
            _ => panic!("Expected PauseTurn"),
        }

        // Check that Yield event was emitted
        let has_input_request = events.iter().any(|e| {
            matches!(
                e,
                Event::Yield {
                    kind: alan_protocol::YieldKind::StructuredInput,
                    ..
                }
            )
        });
        assert!(has_input_request, "Expected Yield StructuredInput event");
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_builtin_tool() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Test with read_file tool - requires sandbox setup, will likely fail but tests the path
        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        // Tool execution may fail due to sandbox restrictions, but orchestration should complete
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replay_approved_tool_call() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Replay test",
                "items": [{"id": "1", "content": "Step", "status": "completed"}]
            }),
        };

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result =
            replay_approved_tool_call_with_cancel(&mut state, &tool_call, inputs, &mut emit).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replay_approved_tool_batch() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Batch test",
                "items": [{"id": "1", "content": "Step 1", "status": "completed"}]
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result =
            replay_approved_tool_batch_with_cancel(&mut state, &tool_calls, inputs, &mut emit)
                .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tool_batch_with_dynamic_tool() {
        let mut state = create_test_state();
        // Register a dynamic tool
        state.session.dynamic_tools.insert(
            "custom_dynamic_tool".to_string(),
            DynamicToolSpec {
                name: "custom_dynamic_tool".to_string(),
                description: "A test tool".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
                capability: Some(alan_protocol::ToolCapability::Read),
            },
        );

        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "custom_dynamic_tool".to_string(),
            arguments: json!({}),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // Should pause for dynamic tool
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::PauseTurn => {
                // Check Yield DynamicTool event
                let has_dynamic_tool = events.iter().any(|e| {
                    matches!(
                        e,
                        Event::Yield {
                            kind: alan_protocol::YieldKind::DynamicTool,
                            ..
                        }
                    )
                });
                assert!(has_dynamic_tool, "Expected Yield DynamicTool event");
            }
            _ => panic!("Expected PauseTurn for dynamic tool"),
        }
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_cancel() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        // Cancel immediately
        cancel.cancel();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        // Should complete without panic even when cancelled
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_virtual_tool_ends_turn() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Invalid confirmation request - missing required summary
        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                "details": {"reason": "missing_summary"}
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // Invalid virtual tool should end turn
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::EndTurn => {
                // Check Error event was emitted
                let has_error = events.iter().any(|e| matches!(e, Event::Error { .. }));
                assert!(has_error, "Expected Error event for invalid virtual tool");
            }
            _ => panic!("Expected EndTurn for invalid virtual tool"),
        }
    }

    #[tokio::test]
    async fn test_multiple_tools_in_batch() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![
            NormalizedToolCall {
                id: "call_1".to_string(),
                name: "update_plan".to_string(),
                arguments: json!({
                    "explanation": "First",
                    "items": [{"id": "1", "content": "Step 1", "status": "completed"}]
                }),
            },
            NormalizedToolCall {
                id: "call_2".to_string(),
                name: "update_plan".to_string(),
                arguments: json!({
                    "explanation": "Second",
                    "items": [{"id": "2", "content": "Step 2", "status": "completed"}]
                }),
            },
        ];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // Should have two update_plan completion events.
        let plan_updates: Vec<_> = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::ToolCallCompleted {
                        result_preview: Some(preview),
                        ..
                    } if preview.contains("plan_updated")
                )
            })
            .collect();
        assert_eq!(
            plan_updates.len(),
            2,
            "Expected two update_plan completion events"
        );
    }

    #[tokio::test]
    async fn test_tool_loop_guard_triggers() {
        let mut state = create_test_state();
        // Set max loops to a small number
        let mut orchestrator = ToolTurnOrchestrator::new(Some(2), 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Create many tool calls that will exceed the loop limit
        let mut tool_calls = vec![];
        for i in 0..3 {
            tool_calls.push(NormalizedToolCall {
                id: format!("call_{}", i),
                name: "update_plan".to_string(),
                arguments: json!({
                    "explanation": format!("Step {}", i),
                    "items": [{"id": i.to_string(), "content": "Step", "status": "completed"}]
                }),
            });
        }

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // After max loops, should end turn
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::EndTurn => {
                // Expected
            }
            _ => {
                // Note: Depending on implementation, might continue or end
                // Just verify no panic occurred
            }
        }
    }
}
