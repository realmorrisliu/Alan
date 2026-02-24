use anyhow::Result;
use alan_protocol::Event;
use serde_json::json;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::approval::PendingConfirmation;

use super::agent_loop::{AgentLoopState, NormalizedToolCall};
use super::loop_guard::ToolLoopGuard;
use super::tool_policy::{
    ToolPolicyDecision, capability_label, evaluate_tool_policy, tool_approval_cache_key,
};
use super::turn_support::check_turn_cancelled;
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
        state: &mut AgentLoopState,
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
    state: &mut AgentLoopState,
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
    state: &mut AgentLoopState,
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
    #[allow(dead_code)]
    pub user_input: Option<&'a str>,
    pub cancel: &'a CancellationToken,
}

async fn orchestrate_tool_call_with_guard<E, F>(
    state: &mut AgentLoopState,
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
        emit(Event::MessageDeltaChunk {
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
    let dynamic_tool_spec = state.session.dynamic_tools.get(&tool_call.name);
    let approval_key = tool_approval_cache_key(
        &tool_call.name,
        tool_capability,
        state.runtime_config.sandbox_mode,
        dynamic_tool_spec,
        &tool_arguments,
    );
    let can_use_cached_approval = matches!(
        state.runtime_config.approval_policy,
        alan_protocol::ApprovalPolicy::OnRequest
    ) && state.session.has_tool_approval(&approval_key);

    match evaluate_tool_policy(
        state.runtime_config.approval_policy,
        state.runtime_config.sandbox_mode,
        &tool_call.name,
        &tool_arguments,
        tool_capability,
    ) {
        ToolPolicyDecision::Allow => {}
        ToolPolicyDecision::RequireApproval {
            summary,
            mut details,
        } => {
            if can_use_cached_approval {
                info!(
                    tool_name = %tool_call.name,
                    approval_key = %approval_key,
                    "Using cached tool approval"
                );
            } else {
                details["approval_key"] = serde_json::to_value(&approval_key).unwrap_or_default();
                details["replay_tool_call"] = json!({
                    "call_id": tool_call.id,
                    "tool_name": tool_call.name,
                    "arguments": tool_arguments,
                });
                let pending = PendingConfirmation {
                    checkpoint_id: format!("tool_approval_{}", tool_call.id),
                    checkpoint_type: "tool_approval".to_string(),
                    summary,
                    details,
                    options: vec!["approve".to_string(), "reject".to_string()],
                };
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    json!({"status":"approval_required", "approval_key": serde_json::to_value(&approval_key).unwrap_or_default()}),
                    true,
                );
                state.turn_state.set_confirmation(pending.clone());
                emit(Event::ConfirmationRequired {
                    checkpoint_id: pending.checkpoint_id,
                    checkpoint_type: pending.checkpoint_type,
                    summary: pending.summary,
                    details: pending.details,
                    options: pending.options,
                })
                .await;
                return Ok(ToolOrchestratorOutcome::PauseTurn);
            }
        }
        ToolPolicyDecision::Forbidden { reason } => {
            if can_use_cached_approval {
                info!(
                    tool_name = %tool_call.name,
                    approval_key = %approval_key,
                    "Bypassing sandbox policy with cached approval"
                );
            } else if matches!(
                state.runtime_config.approval_policy,
                alan_protocol::ApprovalPolicy::OnRequest
            ) {
                let pending = PendingConfirmation {
                    checkpoint_id: format!("tool_approval_{}", tool_call.id),
                    checkpoint_type: "tool_approval".to_string(),
                    summary: format!("Approve sandbox bypass for tool '{}'? ", tool_call.name)
                        .trim()
                        .to_string(),
                    details: json!({
                        "kind": "tool_approval",
                        "tool_name": tool_call.name,
                        "arguments": tool_arguments,
                        "capability": capability_label(tool_capability),
                        "approval_policy": state.runtime_config.approval_policy,
                        "sandbox_mode": state.runtime_config.sandbox_mode,
                        "blocked_by_sandbox_policy": true,
                        "blocked_reason": reason,
                        "approval_key": serde_json::to_value(&approval_key).unwrap_or_default(),
                        "replay_tool_call": {
                            "call_id": tool_call.id,
                            "tool_name": tool_call.name,
                            "arguments": tool_arguments
                        }
                    }),
                    options: vec!["approve".to_string(), "reject".to_string()],
                };
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    json!({"status":"approval_required", "reason": "sandbox_policy", "approval_key": serde_json::to_value(&approval_key).unwrap_or_default()}),
                    true,
                );
                state.turn_state.set_confirmation(pending.clone());
                emit(Event::ConfirmationRequired {
                    checkpoint_id: pending.checkpoint_id,
                    checkpoint_type: pending.checkpoint_type,
                    summary: pending.summary,
                    details: pending.details,
                    options: pending.options,
                })
                .await;
                return Ok(ToolOrchestratorOutcome::PauseTurn);
            } else {
                let blocked_payload = json!({
                    "error": reason,
                    "status": "blocked_by_sandbox_policy"
                });
                emit(Event::Error {
                    message: blocked_payload["error"]
                        .as_str()
                        .unwrap_or("Tool blocked by sandbox policy")
                        .to_string(),
                    recoverable: true,
                })
                .await;
                emit(Event::ToolCallCompleted {
                    call_id: tool_call.id.clone(),
                    tool_name: tool_call.name.clone(),
                    result: blocked_payload.clone(),
                    success: false,
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    blocked_payload.clone(),
                    false,
                );
                state
                    .session
                    .add_tool_message(&tool_call.id, &tool_call.name, blocked_payload);
                return Ok(ToolOrchestratorOutcome::ContinueToolBatch {
                    refresh_context: false,
                });
            }
        }
    }

    if state.session.dynamic_tools.contains_key(&tool_call.name) {
        emit(Event::ToolCallStarted {
            call_id: tool_call.id.clone(),
            tool_name: tool_call.name.clone(),
            arguments: tool_arguments.clone(),
        })
        .await;
        state
            .turn_state
            .set_dynamic_tool_call(crate::approval::PendingDynamicToolCall {
                call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                arguments: tool_arguments.clone(),
            });
        state.session.record_tool_call(
            &tool_call.name,
            tool_arguments.clone(),
            json!({"status":"pending_dynamic_tool_result","call_id": tool_call.id}),
            true,
        );
        emit(Event::DynamicToolCallRequested {
            call_id: tool_call.id.clone(),
            tool_name: tool_call.name.clone(),
            arguments: tool_arguments,
        })
        .await;
        return Ok(ToolOrchestratorOutcome::PauseTurn);
    }

    emit(Event::ToolCallStarted {
        call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        arguments: tool_arguments.clone(),
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
                call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                result: value.clone(),
                success: true,
            })
            .await;
            state.session.record_tool_call(
                &tool_call.name,
                tool_arguments.clone(),
                value.clone(),
                true,
            );
            let maybe_plan_update = if tool_call.name == "todo_list" {
                super::turn_support::plan_update_from_todo_result(&tool_arguments, &value)
            } else {
                None
            };
            state
                .session
                .add_tool_message(&tool_call.id, &tool_call.name, value);
            if let Some((explanation, items)) = maybe_plan_update {
                emit(Event::PlanUpdated { explanation, items }).await;
            }
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
                call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                result: error_payload.clone(),
                success: false,
            })
            .await;
            state.session.record_tool_call(
                &tool_call.name,
                tool_arguments.clone(),
                error_payload.clone(),
                false,
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
    state: &mut AgentLoopState,
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
            }
            ToolOrchestratorOutcome::PauseTurn => {
                if let Some(pending) = state.turn_state.pending_confirmation()
                    && pending.checkpoint_type == "tool_approval"
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
        emit(Event::MessageDeltaChunk {
            chunk: msg,
            is_final: true,
        })
        .await;
        emit(Event::TaskCompleted {
            summary: "Tool loop stopped by loop guard".to_string(),
            results: json!({"status":"stopped","reason":"tool_loop_guard"}),
        })
        .await;
        emit(Event::TurnCompleted {}).await;
        return Ok(ToolBatchOrchestratorOutcome::EndTurn);
    }

    Ok(ToolBatchOrchestratorOutcome::ContinueTurnLoop { refresh_context })
}
