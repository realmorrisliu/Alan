use alan_protocol::{Event, Op};
use anyhow::Result;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::approval::{ToolApprovalCacheKey, ToolApprovalDecision};
use crate::tape::ContentPart;

use super::agent_loop::{NormalizedToolCall, RuntimeLoopState, maybe_compact_context};
use super::turn_executor::TurnRunKind;
use super::turn_state::PendingYield;
use super::turn_support::cancel_current_task;

#[derive(Debug, Clone)]
pub(super) enum RuntimeOpAction {
    NoTurn,
    RunTurn {
        turn_kind: TurnRunKind,
        user_input: Option<Vec<ContentPart>>,
        activate_task: bool,
    },
    ReplayApprovedToolCall {
        tool_call: NormalizedToolCall,
    },
    ReplayApprovedToolBatch {
        tool_calls: Vec<NormalizedToolCall>,
    },
}

pub(super) async fn handle_runtime_op_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    op: Op,
    emit: &mut E,
    _cancel: &CancellationToken,
) -> Result<RuntimeOpAction>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    match op {
        Op::RegisterDynamicTools { tools } => {
            let mut invalidated_tool_names: std::collections::BTreeSet<String> =
                state.session.dynamic_tools.keys().cloned().collect();
            invalidated_tool_names.extend(tools.iter().map(|tool| tool.name.clone()));
            state.session.revoke_dynamic_tool_approvals_for_tool_names(
                invalidated_tool_names.iter().map(String::as_str),
            );
            state.session.dynamic_tools = tools
                .iter()
                .cloned()
                .map(|tool| (tool.name.clone(), tool))
                .collect();
            emit(Event::DynamicToolsRegistered {
                tool_names: state.session.dynamic_tools.keys().cloned().collect(),
            })
            .await;
        }
        Op::Compact => {
            maybe_compact_context(state, emit).await?;
        }
        Op::Rollback { turns } => {
            if turns == 0 {
                emit(Event::Error {
                    message: "turns must be >= 1".to_string(),
                    recoverable: true,
                })
                .await;
                return Ok(RuntimeOpAction::NoTurn);
            }
            let removed_messages = state.session.rollback_last_turns(turns);
            emit(Event::SessionRolledBack {
                num_turns: turns,
                removed_messages,
            })
            .await;
        }
        Op::Interrupt => {
            cancel_current_task(state, emit).await?;
        }

        // ====================================================================
        // New unified operations (Phase 2)
        // ====================================================================
        Op::Turn { parts, context } => {
            let workspace_id = context.as_ref().and_then(|c| c.workspace_id.clone());

            if let Some(requested_workspace_id) = workspace_id.as_deref()
                && requested_workspace_id != state.workspace_id
            {
                emit(Event::Error {
                    message: format!(
                        "Turn requested workspace '{}' but this runtime is '{}'. Route the request to the matching workspace runtime.",
                        requested_workspace_id, state.workspace_id
                    ),
                    recoverable: true,
                })
                .await;
                return Ok(RuntimeOpAction::NoTurn);
            }

            state.turn_state.clear();

            return Ok(RuntimeOpAction::RunTurn {
                turn_kind: TurnRunKind::NewTurn,
                user_input: Some(parts),
                activate_task: true,
            });
        }

        Op::Input { parts } => {
            let turn_kind = if state.turn_state.is_turn_active()
                || state.turn_state.has_pending_interaction()
            {
                TurnRunKind::ResumeTurn
            } else {
                TurnRunKind::NewTurn
            };

            return Ok(RuntimeOpAction::RunTurn {
                turn_kind,
                user_input: Some(parts),
                activate_task: matches!(turn_kind, TurnRunKind::NewTurn),
            });
        }

        Op::Resume {
            request_id,
            content,
        } => {
            let result = resume_content_to_value(&content);
            match state.turn_state.take_pending(&request_id) {
                Some(PendingYield::Confirmation(pending)) => {
                    let choice = result
                        .get("choice")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string)
                        .or_else(|| first_resume_text(&content));
                    let choice_str = choice.as_deref().unwrap_or("approve");
                    let modifications = result
                        .get("modifications")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    return handle_confirmation_resolution(
                        state,
                        pending,
                        choice_str,
                        modifications,
                    );
                }
                Some(PendingYield::StructuredInput(pending)) => {
                    state.session.add_tool_message(
                        &pending.request_id,
                        "request_user_input",
                        result,
                    );
                    return Ok(RuntimeOpAction::RunTurn {
                        turn_kind: TurnRunKind::ResumeTurn,
                        user_input: None,
                        activate_task: false,
                    });
                }
                Some(PendingYield::DynamicToolCall(pending)) => {
                    let success = result
                        .get("success")
                        .and_then(|value| value.as_bool())
                        .unwrap_or_else(|| result.get("error").is_none());
                    emit(Event::ToolCallCompleted {
                        call_id: pending.call_id.clone(),
                        tool_name: pending.tool_name.clone(),
                        result: result.clone(),
                        success,
                    })
                    .await;
                    state
                        .session
                        .add_tool_message(&pending.call_id, &pending.tool_name, result);
                    return Ok(RuntimeOpAction::RunTurn {
                        turn_kind: TurnRunKind::ResumeTurn,
                        user_input: None,
                        activate_task: false,
                    });
                }
                None => {
                    emit(Event::Error {
                        message: format!(
                            "Resume request_id '{}' does not match any pending yield.",
                            request_id
                        ),
                        recoverable: true,
                    })
                    .await;
                    return Ok(RuntimeOpAction::NoTurn);
                }
            }
        }
    }
    Ok(RuntimeOpAction::NoTurn)
}

fn resume_content_to_value(content: &[ContentPart]) -> serde_json::Value {
    match content {
        [] => serde_json::Value::Null,
        [single] => match single {
            ContentPart::Structured { data } => data.clone(),
            ContentPart::Text { text } | ContentPart::Thinking { text, .. } => {
                serde_json::Value::String(text.clone())
            }
            other => serde_json::to_value(other).unwrap_or(serde_json::Value::Null),
        },
        _ => serde_json::Value::Array(
            content
                .iter()
                .map(|part| match part {
                    ContentPart::Structured { data } => data.clone(),
                    ContentPart::Text { text } | ContentPart::Thinking { text, .. } => {
                        serde_json::Value::String(text.clone())
                    }
                    other => serde_json::to_value(other).unwrap_or(serde_json::Value::Null),
                })
                .collect(),
        ),
    }
}

fn first_resume_text(content: &[ContentPart]) -> Option<String> {
    content.iter().find_map(|part| match part {
        ContentPart::Text { text } | ContentPart::Thinking { text, .. } => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        _ => None,
    })
}

fn handle_confirmation_resolution(
    state: &mut RuntimeLoopState,
    pending: crate::approval::PendingConfirmation,
    choice_str: &str,
    modifications: Option<String>,
) -> Result<RuntimeOpAction> {
    let replay_tool_batch = if pending.checkpoint_type == "tool_approval" {
        state
            .turn_state
            .take_tool_replay_batch(&pending.checkpoint_id)
    } else {
        None
    };

    if pending.checkpoint_type == "tool_approval"
        && choice_str == "approve"
        && let Some(approval_key_value) = pending.details.get("approval_key")
        && let Ok(approval_key) =
            serde_json::from_value::<ToolApprovalCacheKey>(approval_key_value.clone())
    {
        state
            .session
            .record_tool_approval_decision(approval_key, ToolApprovalDecision::ApprovedForSession);
    }

    let mut payload = json!({
        "checkpoint_id": pending.checkpoint_id,
        "checkpoint_type": pending.checkpoint_type.clone(),
        "choice": choice_str,
    });

    if let Some(modifications) = modifications {
        payload["modifications"] = serde_json::Value::String(modifications);
    }

    state
        .session
        .add_tool_message(&pending.checkpoint_id, "request_confirmation", payload);

    if pending.checkpoint_type == "tool_approval"
        && choice_str == "approve"
        && let Some(tool_calls) = replay_tool_batch
    {
        return Ok(RuntimeOpAction::ReplayApprovedToolBatch { tool_calls });
    }
    if pending.checkpoint_type == "tool_approval"
        && choice_str == "approve"
        && let Some(tool_call) = parse_replay_tool_call_from_confirmation_details(&pending.details)
    {
        return Ok(RuntimeOpAction::ReplayApprovedToolCall { tool_call });
    }
    Ok(RuntimeOpAction::RunTurn {
        turn_kind: TurnRunKind::ResumeTurn,
        user_input: None,
        activate_task: false,
    })
}

fn parse_replay_tool_call_from_confirmation_details(
    details: &serde_json::Value,
) -> Option<NormalizedToolCall> {
    let replay = details.get("replay_tool_call")?;
    let call_id = replay.get("call_id")?.as_str()?.trim();
    let tool_name = replay.get("tool_name")?.as_str()?.trim();
    let arguments = replay.get("arguments")?.clone();

    if call_id.is_empty() || tool_name.is_empty() {
        return None;
    }

    Some(NormalizedToolCall {
        id: call_id.to_string(),
        name: tool_name.to_string(),
        arguments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        llm::LlmClient,
        runtime::{RuntimeConfig, TurnState},
        session::Session,
        tape::ContentPart,
        tools::ToolRegistry,
    };
    use alan_llm::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk};
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
        let runtime_config = RuntimeConfig::default();

        RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(SimpleMockProvider),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        }
    }

    #[tokio::test]
    async fn test_handle_start_task_wrong_agent() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Turn {
            parts: vec![ContentPart::text("test input")],
            context: Some(alan_protocol::TurnContext {
                workspace_id: Some("wrong-workspace".to_string()),
                domain: None,
            }),
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                // Check that error event was emitted
                let has_error = events.iter().any(|e| matches!(e, Event::Error { .. }));
                assert!(has_error, "Expected Error event for wrong workspace");
            }
            _ => panic!("Expected NoTurn for wrong workspace"),
        }
    }

    #[tokio::test]
    async fn test_handle_start_task_correct_agent() {
        let mut state = create_test_state();
        state.session.add_user_message("existing message");
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Turn {
            parts: vec![ContentPart::text("test input")],
            context: Some(alan_protocol::TurnContext {
                workspace_id: Some("test-workspace".to_string()),
                domain: None,
            }),
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn {
                user_input,
                activate_task,
                ..
            } => {
                assert!(activate_task);
                assert!(user_input.is_some());
                let text = alan_protocol::parts_to_text(&user_input.unwrap());
                assert!(text.contains("test input"));
                // Turn should preserve existing conversation history.
                assert_eq!(state.session.tape.messages().len(), 1);
                assert_eq!(
                    state.session.tape.messages()[0].text_content(),
                    "existing message"
                );
            }
            _ => panic!("Expected RunTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_start_task_no_workspace_id() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Turn {
            parts: vec![
                ContentPart::text("test input"),
                ContentPart::Attachment {
                    hash: "doc1.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    metadata: serde_json::Value::Null,
                },
                ContentPart::Attachment {
                    hash: "doc2.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    metadata: serde_json::Value::Null,
                },
            ],
            context: Some(alan_protocol::TurnContext {
                workspace_id: None,
                domain: None,
            }),
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn { user_input, .. } => {
                let parts = user_input.unwrap();
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0].as_text(), Some("test input"));
                assert!(matches!(parts[1], ContentPart::Attachment { .. }));
                assert!(matches!(parts[2], ContentPart::Attachment { .. }));
            }
            _ => panic!("Expected RunTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_confirm_no_pending() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "chk_123".to_string(),
            content: vec![ContentPart::structured(json!({"choice": "approve"}))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(
                    |e| matches!(e, Event::Error { message, .. } if message.contains("does not match")),
                );
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_confirm_wrong_checkpoint() {
        let mut state = create_test_state();
        state
            .turn_state
            .set_confirmation(crate::approval::PendingConfirmation {
                checkpoint_id: "other_checkpoint".to_string(),
                checkpoint_type: "test".to_string(),
                summary: "Test".to_string(),
                details: json!({}),
                options: vec!["approve".to_string()],
            });
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "chk_123".to_string(),
            content: vec![ContentPart::structured(json!({"choice": "approve"}))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { message, .. } if message.contains("does not match"))
                });
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_confirm_approve() {
        let mut state = create_test_state();
        state
            .turn_state
            .set_confirmation(crate::approval::PendingConfirmation {
                checkpoint_id: "chk_123".to_string(),
                checkpoint_type: "test".to_string(),
                summary: "Test".to_string(),
                details: json!({
                    "replay_tool_call": {
                        "call_id": "call_1",
                        "tool_name": "read_file",
                        "arguments": {"path": "test.txt"}
                    }
                }),
                options: vec!["approve".to_string()],
            });
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "chk_123".to_string(),
            content: vec![ContentPart::structured(json!({"choice": "approve"}))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        // Tool message should be recorded
        let messages = state.session.tape.messages();
        assert!(!messages.is_empty());
        assert!(messages[0].text_content().contains("approve"));
    }

    #[tokio::test]
    async fn test_handle_confirm_with_modifications() {
        let mut state = create_test_state();
        state
            .turn_state
            .set_confirmation(crate::approval::PendingConfirmation {
                checkpoint_id: "chk_123".to_string(),
                checkpoint_type: "test".to_string(),
                summary: "Test".to_string(),
                details: json!({}),
                options: vec!["approve".to_string(), "modify".to_string()],
            });
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "chk_123".to_string(),
            content: vec![ContentPart::structured(json!({
                "choice": "modify",
                "modifications": "Changed something"
            }))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        // Tool message should contain modifications
        let messages = state.session.tape.messages();
        assert!(!messages.is_empty());
        assert!(messages[0].text_content().contains("modify"));
    }

    #[tokio::test]
    async fn test_handle_user_input() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Input {
            parts: vec![ContentPart::text("Hello world")],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn {
                user_input,
                activate_task,
                ..
            } => {
                assert!(activate_task);
                assert_eq!(user_input, Some(vec![ContentPart::text("Hello world")]));
            }
            _ => panic!("Expected RunTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_structured_user_input_no_pending() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "req_123".to_string(),
            content: vec![ContentPart::structured(json!({"answers": []}))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(
                    |e| matches!(e, Event::Error { message, .. } if message.contains("does not match")),
                );
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_structured_user_input_wrong_id() {
        let mut state = create_test_state();
        state
            .turn_state
            .set_structured_input(crate::approval::PendingStructuredInputRequest {
                request_id: "other_id".to_string(),
                title: "Test".to_string(),
                prompt: "Test".to_string(),
                questions: vec![],
            });
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "req_123".to_string(),
            content: vec![ContentPart::structured(json!({"answers": []}))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { message, .. } if message.contains("does not match"))
                });
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_structured_user_input_success() {
        let mut state = create_test_state();
        state
            .turn_state
            .set_structured_input(crate::approval::PendingStructuredInputRequest {
                request_id: "req_123".to_string(),
                title: "Test".to_string(),
                prompt: "Test".to_string(),
                questions: vec![],
            });
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "req_123".to_string(),
            content: vec![ContentPart::structured(json!({
                "answers": [{"question_id": "q1", "value": "answer1"}]
            }))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn {
                user_input,
                activate_task,
                turn_kind,
            } => {
                assert!(!activate_task);
                assert!(user_input.is_none());
                assert!(matches!(turn_kind, TurnRunKind::ResumeTurn));
            }
            _ => panic!("Expected RunTurn"),
        }

        // Verify tool message was recorded
        assert!(!state.session.tape.messages().is_empty());
    }

    #[tokio::test]
    async fn test_handle_register_dynamic_tools() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tools = vec![
            alan_protocol::DynamicToolSpec {
                name: "custom_tool1".to_string(),
                description: "Tool 1".to_string(),
                parameters: json!({}),
                capability: Some(alan_protocol::ToolCapability::Read),
            },
            alan_protocol::DynamicToolSpec {
                name: "custom_tool2".to_string(),
                description: "Tool 2".to_string(),
                parameters: json!({}),
                capability: None,
            },
        ];

        let op = Op::RegisterDynamicTools { tools };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                // Verify event was emitted
                let has_event = events.iter().any(|e| {
                    matches!(e, Event::DynamicToolsRegistered { tool_names } if tool_names.contains(&"custom_tool1".to_string()))
                });
                assert!(has_event);

                // Verify tools were registered
                assert!(state.session.dynamic_tools.contains_key("custom_tool1"));
                assert!(state.session.dynamic_tools.contains_key("custom_tool2"));
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_dynamic_tool_result_no_pending() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "call_123".to_string(),
            content: vec![ContentPart::structured(json!({
                "success": true,
                "result": {"data": "value"}
            }))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(
                    |e| matches!(e, Event::Error { message, .. } if message.contains("does not match")),
                );
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_dynamic_tool_result_success() {
        let mut state = create_test_state();
        state
            .turn_state
            .set_dynamic_tool_call(crate::approval::PendingDynamicToolCall {
                call_id: "call_123".to_string(),
                tool_name: "custom_tool".to_string(),
                arguments: json!({"arg": "value"}),
            });
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "call_123".to_string(),
            content: vec![ContentPart::structured(json!({
                "success": true,
                "result": {"data": "result"}
            }))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn {
                user_input,
                activate_task,
                turn_kind,
            } => {
                assert!(!activate_task);
                assert!(user_input.is_none());
                assert!(matches!(turn_kind, TurnRunKind::ResumeTurn));
            }
            _ => panic!("Expected RunTurn"),
        }

        let has_completed = events.iter().any(|event| {
            matches!(
                event,
                Event::ToolCallCompleted {
                    call_id,
                    tool_name,
                    success: true,
                    ..
                } if call_id == "call_123" && tool_name == "custom_tool"
            )
        });
        assert!(
            has_completed,
            "Expected ToolCallCompleted after dynamic tool resume"
        );

        // Verify tool message was recorded
        assert!(!state.session.tape.messages().is_empty());
    }

    #[tokio::test]
    async fn test_handle_compact() {
        let mut state = create_test_state();
        // Add some messages to make compaction meaningful
        for i in 0..10 {
            state.session.add_user_message(&format!("Message {}", i));
        }
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Compact;

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                // Compaction completed
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_rollback_invalid_zero() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Rollback { turns: 0 };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { message, .. } if message.contains("turns must be >= 1"))
                });
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_rollback_success() {
        let mut state = create_test_state();
        state.session.add_user_message("u1");
        state.session.add_assistant_message("a1", None);
        state.session.add_user_message("u2");
        state.session.add_assistant_message("a2", None);

        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Rollback { turns: 1 };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                // Verify SessionRolledBack event was emitted
                let has_event = events.iter().any(
                    |e| matches!(e, Event::SessionRolledBack { num_turns, .. } if *num_turns == 1),
                );
                assert!(has_event);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_cancel() {
        let mut state = create_test_state();
        state.session.has_active_task = true;
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Interrupt;

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                // Task should be cancelled
                assert!(!state.session.has_active_task);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    // Tests for parse_replay_tool_call_from_confirmation_details
    #[test]
    fn test_parse_replay_tool_call_valid() {
        let details = json!({
            "replay_tool_call": {
                "call_id": "call_123",
                "tool_name": "read_file",
                "arguments": {"path": "test.txt"}
            }
        });

        let result = parse_replay_tool_call_from_confirmation_details(&details);
        assert!(result.is_some());

        let call = result.unwrap();
        assert_eq!(call.id, "call_123");
        assert_eq!(call.name, "read_file");
        assert_eq!(call.arguments, json!({"path": "test.txt"}));
    }

    #[test]
    fn test_parse_replay_tool_call_missing_replay() {
        let details = json!({
            "other_field": "value"
        });

        assert!(parse_replay_tool_call_from_confirmation_details(&details).is_none());
    }

    #[test]
    fn test_parse_replay_tool_call_empty_call_id() {
        let details = json!({
            "replay_tool_call": {
                "call_id": "  ",
                "tool_name": "read_file",
                "arguments": {}
            }
        });

        assert!(parse_replay_tool_call_from_confirmation_details(&details).is_none());
    }

    #[test]
    fn test_parse_replay_tool_call_empty_tool_name() {
        let details = json!({
            "replay_tool_call": {
                "call_id": "call_123",
                "tool_name": "",
                "arguments": {}
            }
        });

        assert!(parse_replay_tool_call_from_confirmation_details(&details).is_none());
    }

    #[test]
    fn test_parse_replay_tool_call_missing_arguments() {
        let details = json!({
            "replay_tool_call": {
                "call_id": "call_123",
                "tool_name": "read_file"
            }
        });

        assert!(parse_replay_tool_call_from_confirmation_details(&details).is_none());
    }

    // ========================================================================
    // Tests for new Phase 2 Op variants
    // ========================================================================

    #[tokio::test]
    async fn test_handle_turn_op() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Turn {
            parts: vec![ContentPart::text("Hello from Turn")],
            context: None,
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn {
                turn_kind,
                user_input,
                activate_task,
            } => {
                assert!(matches!(turn_kind, TurnRunKind::NewTurn));
                let text = alan_protocol::parts_to_text(&user_input.unwrap());
                assert!(text.contains("Hello from Turn"));
                assert!(activate_task);
            }
            _ => panic!("Expected RunTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_input_op() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Input {
            parts: vec![ContentPart::text("follow up")],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn {
                user_input,
                activate_task,
                ..
            } => {
                assert_eq!(user_input, Some(vec![ContentPart::text("follow up")]));
                assert!(activate_task);
            }
            _ => panic!("Expected RunTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_input_op_during_active_turn_uses_resume_turn() {
        let mut state = create_test_state();
        state
            .turn_state
            .set_turn_activity(crate::runtime::turn_state::TurnActivityState::Running);
        state.session.has_active_task = true;
        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Input {
            parts: vec![ContentPart::text("steer current turn")],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn {
                turn_kind,
                user_input,
                activate_task,
            } => {
                assert!(matches!(turn_kind, TurnRunKind::ResumeTurn));
                assert_eq!(
                    user_input,
                    Some(vec![ContentPart::text("steer current turn")])
                );
                assert!(!activate_task);
            }
            _ => panic!("Expected RunTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_interrupt_op() {
        let mut state = create_test_state();
        state.session.has_active_task = true;
        state
            .turn_state
            .set_turn_activity(crate::runtime::turn_state::TurnActivityState::Running);
        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Interrupt;

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        assert!(!state.session.has_active_task);
    }

    #[tokio::test]
    async fn test_handle_resume_no_pending_yields_error() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "nonexistent".to_string(),
            content: vec![ContentPart::structured(
                serde_json::json!({"choice": "approve"}),
            )],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RuntimeOpAction::NoTurn));

        // Should have emitted an error event
        let has_error = events.iter().any(
            |e| matches!(e, Event::Error { message, .. } if message.contains("does not match")),
        );
        assert!(has_error);
    }

    #[tokio::test]
    async fn test_handle_resume_with_pending_confirmation() {
        use crate::approval::PendingConfirmation;

        let mut state = create_test_state();
        state.turn_state.set_confirmation(PendingConfirmation {
            checkpoint_id: "cp-1".to_string(),
            checkpoint_type: "review".to_string(),
            summary: "Review this".to_string(),
            details: json!({}),
            options: vec!["approve".to_string(), "reject".to_string()],
        });

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Resume {
            request_id: "cp-1".to_string(),
            content: vec![ContentPart::structured(json!({"choice": "approve"}))],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());

        match result.unwrap() {
            RuntimeOpAction::RunTurn { turn_kind, .. } => {
                assert!(matches!(turn_kind, TurnRunKind::ResumeTurn));
            }
            _ => panic!("Expected RunTurn with ResumeTurn"),
        }
    }
}
