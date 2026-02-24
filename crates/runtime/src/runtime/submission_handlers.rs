use anyhow::Result;
use alan_protocol::{Event, Op};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::approval::{ToolApprovalCacheKey, ToolApprovalDecision};

use super::agent_loop::{
    AgentLoopState, NormalizedToolCall, build_task_prompt, maybe_compact_context,
};
use super::turn_executor::TurnRunKind;
use super::turn_support::cancel_current_task;

#[derive(Debug, Clone)]
pub(super) enum RuntimeOpAction {
    NoTurn,
    RunTurn {
        turn_kind: TurnRunKind,
        user_input: Option<String>,
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
    state: &mut AgentLoopState,
    op: Op,
    emit: &mut E,
    _cancel: &CancellationToken,
) -> Result<RuntimeOpAction>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    match op {
        Op::StartTask {
            agent_id,
            domain: _,
            input,
            attachments,
        } => {
            if let Some(requested_agent_id) = agent_id.as_deref()
                && requested_agent_id != state.agent_id
            {
                emit(Event::Error {
                    message: format!(
                        "Task requested agent '{}' but this runtime is '{}'. Route the request to the matching agent runtime.",
                        requested_agent_id, state.agent_id
                    ),
                    recoverable: true,
                })
                .await;
                return Ok(RuntimeOpAction::NoTurn);
            }

            // Only reset the turn/session after routing validation succeeds.
            // Otherwise a misrouted StartTask would destructively clear history.
            state.turn_state.clear();
            state.session.clear();

            let effective_prompt = build_task_prompt(input, attachments, None);
            return Ok(RuntimeOpAction::RunTurn {
                turn_kind: TurnRunKind::NewTurn,
                user_input: Some(effective_prompt),
                activate_task: true,
            });
        }
        Op::Confirm {
            checkpoint_id,
            choice,
            modifications,
        } => {
            let pending = match state.turn_state.take_confirmation(&checkpoint_id) {
                Some(pending) => pending,
                None if state.turn_state.pending_confirmation().is_some() => {
                    emit(Event::Error {
                        message: "Checkpoint ID does not match any pending confirmation."
                            .to_string(),
                        recoverable: true,
                    })
                    .await;
                    return Ok(RuntimeOpAction::NoTurn);
                }
                None => {
                    emit(Event::Error {
                        message: "No pending confirmations.".to_string(),
                        recoverable: true,
                    })
                    .await;
                    return Ok(RuntimeOpAction::NoTurn);
                }
            };

            let choice_str = match choice {
                alan_protocol::ConfirmChoice::Approve => "approve",
                alan_protocol::ConfirmChoice::Modify => "modify",
                alan_protocol::ConfirmChoice::Reject => "reject",
            };

            let replay_tool_batch = if pending.checkpoint_type == "tool_approval" {
                state
                    .turn_state
                    .take_tool_replay_batch(&pending.checkpoint_id)
            } else {
                None
            };

            if pending.checkpoint_type == "tool_approval"
                && matches!(choice, alan_protocol::ConfirmChoice::Approve)
                && let Some(approval_key_value) = pending.details.get("approval_key")
                && let Ok(approval_key) = serde_json::from_value::<ToolApprovalCacheKey>(approval_key_value.clone())
            {
                state.session.record_tool_approval_decision(
                    approval_key,
                    ToolApprovalDecision::ApprovedForSession,
                );
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
                && matches!(choice, alan_protocol::ConfirmChoice::Approve)
                && let Some(tool_calls) = replay_tool_batch
            {
                return Ok(RuntimeOpAction::ReplayApprovedToolBatch { tool_calls });
            }
            if pending.checkpoint_type == "tool_approval"
                && matches!(choice, alan_protocol::ConfirmChoice::Approve)
                && let Some(tool_call) =
                    parse_replay_tool_call_from_confirmation_details(&pending.details)
            {
                return Ok(RuntimeOpAction::ReplayApprovedToolCall { tool_call });
            }
            return Ok(RuntimeOpAction::RunTurn {
                turn_kind: TurnRunKind::ResumeTurn,
                user_input: None,
                activate_task: false,
            });
        }
        Op::UserInput { content } => {
            return Ok(RuntimeOpAction::RunTurn {
                turn_kind: TurnRunKind::NewTurn,
                user_input: Some(content),
                activate_task: true,
            });
        }
        Op::StructuredUserInput {
            request_id,
            answers,
        } => {
            let pending = match state.turn_state.take_structured_input(&request_id) {
                Some(pending) => pending,
                None if state.turn_state.pending_structured_input().is_some() => {
                    emit(Event::Error {
                        message: "Structured input request_id does not match any pending request."
                            .to_string(),
                        recoverable: true,
                    })
                    .await;
                    return Ok(RuntimeOpAction::NoTurn);
                }
                None => {
                    emit(Event::Error {
                        message: "No pending structured input request.".to_string(),
                        recoverable: true,
                    })
                    .await;
                    return Ok(RuntimeOpAction::NoTurn);
                }
            };

            let payload = json!({
                "request_id": request_id,
                "answers": answers,
            });
            state
                .session
                .add_tool_message(&pending.request_id, "request_user_input", payload);
            return Ok(RuntimeOpAction::RunTurn {
                turn_kind: TurnRunKind::ResumeTurn,
                user_input: None,
                activate_task: false,
            });
        }
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
        Op::DynamicToolResult {
            call_id,
            success,
            result,
        } => {
            let pending = match state.turn_state.take_dynamic_tool_call(&call_id) {
                Some(pending) => pending,
                None if state.turn_state.pending_dynamic_tool_call().is_some() => {
                    emit(Event::Error {
                        message: "Dynamic tool call_id does not match any pending call."
                            .to_string(),
                        recoverable: true,
                    })
                    .await;
                    return Ok(RuntimeOpAction::NoTurn);
                }
                None => {
                    emit(Event::Error {
                        message: "No pending dynamic tool call.".to_string(),
                        recoverable: true,
                    })
                    .await;
                    return Ok(RuntimeOpAction::NoTurn);
                }
            };

            emit(Event::ToolCallCompleted {
                call_id: pending.call_id.clone(),
                tool_name: pending.tool_name.clone(),
                result: result.clone(),
                success,
            })
            .await;
            state.session.record_tool_call(
                &pending.tool_name,
                pending.arguments.clone(),
                result.clone(),
                success,
            );
            state
                .session
                .add_tool_message(&pending.call_id, &pending.tool_name, result);
            return Ok(RuntimeOpAction::RunTurn {
                turn_kind: TurnRunKind::ResumeTurn,
                user_input: None,
                activate_task: false,
            });
        }
        Op::Compact => {
            maybe_compact_context(state, emit).await?;
        }
        Op::Rollback { num_turns } => {
            if num_turns == 0 {
                emit(Event::Error {
                    message: "num_turns must be >= 1".to_string(),
                    recoverable: true,
                })
                .await;
                return Ok(RuntimeOpAction::NoTurn);
            }
            let removed_messages = state.session.rollback_last_turns(num_turns);
            emit(Event::SessionRolledBack {
                num_turns,
                removed_messages,
            })
            .await;
        }
        Op::Cancel => {
            cancel_current_task(state, emit).await?;
        }
    }
    Ok(RuntimeOpAction::NoTurn)
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
