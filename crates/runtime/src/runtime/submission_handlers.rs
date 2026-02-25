use anyhow::Result;
use alan_protocol::{Event, Op};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::approval::{ToolApprovalCacheKey, ToolApprovalDecision};

use super::agent_loop::{
    RuntimeLoopState, NormalizedToolCall, build_task_prompt, maybe_compact_context,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        llm::LlmClient,
        runtime::{RuntimeConfig, TurnState},
        session::Session,
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
            agent_id: "test-agent".to_string(),
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

        let op = Op::StartTask {
            agent_id: Some("wrong-agent".to_string()),
            domain: None,
            input: "test input".to_string(),
            attachments: vec![],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                // Check that error event was emitted
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { .. })
                });
                assert!(has_error, "Expected Error event for wrong agent");
            }
            _ => panic!("Expected NoTurn for wrong agent"),
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

        let op = Op::StartTask {
            agent_id: Some("test-agent".to_string()),
            domain: None,
            input: "test input".to_string(),
            attachments: vec![],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::RunTurn { user_input, activate_task, .. } => {
                assert!(activate_task);
                assert!(user_input.is_some());
                assert!(user_input.unwrap().contains("test input"));
                // Session should be cleared
                assert!(state.session.tape.messages().is_empty());
            }
            _ => panic!("Expected RunTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_start_task_no_agent_id() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::StartTask {
            agent_id: None,
            domain: None,
            input: "test input".to_string(),
            attachments: vec!["doc1.pdf".to_string(), "doc2.pdf".to_string()],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::RunTurn { user_input, .. } => {
                let input = user_input.unwrap();
                assert!(input.contains("test input"));
                assert!(input.contains("doc1.pdf"));
                assert!(input.contains("doc2.pdf"));
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

        let op = Op::Confirm {
            checkpoint_id: "chk_123".to_string(),
            choice: alan_protocol::ConfirmChoice::Approve,
            modifications: None,
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { message, .. } if message.contains("No pending"))
                });
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_confirm_wrong_checkpoint() {
        let mut state = create_test_state();
        state.turn_state.set_confirmation(crate::approval::PendingConfirmation {
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

        let op = Op::Confirm {
            checkpoint_id: "chk_123".to_string(),
            choice: alan_protocol::ConfirmChoice::Approve,
            modifications: None,
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
        state.turn_state.set_confirmation(crate::approval::PendingConfirmation {
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

        let op = Op::Confirm {
            checkpoint_id: "chk_123".to_string(),
            choice: alan_protocol::ConfirmChoice::Approve,
            modifications: None,
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        // Tool message should be recorded
        let messages = state.session.tape.messages();
        assert!(!messages.is_empty());
        assert!(messages[0].content.contains("approve"));
    }

    #[tokio::test]
    async fn test_handle_confirm_with_modifications() {
        let mut state = create_test_state();
        state.turn_state.set_confirmation(crate::approval::PendingConfirmation {
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

        let op = Op::Confirm {
            checkpoint_id: "chk_123".to_string(),
            choice: alan_protocol::ConfirmChoice::Modify,
            modifications: Some("Changed something".to_string()),
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        // Tool message should contain modifications
        let messages = state.session.tape.messages();
        assert!(!messages.is_empty());
        assert!(messages[0].content.contains("modify"));
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

        let op = Op::UserInput {
            content: "Hello world".to_string(),
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::RunTurn { user_input, activate_task, .. } => {
                assert!(activate_task);
                assert_eq!(user_input, Some("Hello world".to_string()));
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

        let op = Op::StructuredUserInput {
            request_id: "req_123".to_string(),
            answers: vec![],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { message, .. } if message.contains("No pending"))
                });
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_structured_user_input_wrong_id() {
        let mut state = create_test_state();
        state.turn_state.set_structured_input(crate::approval::PendingStructuredInputRequest {
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

        let op = Op::StructuredUserInput {
            request_id: "req_123".to_string(),
            answers: vec![],
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
        state.turn_state.set_structured_input(crate::approval::PendingStructuredInputRequest {
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

        let op = Op::StructuredUserInput {
            request_id: "req_123".to_string(),
            answers: vec![alan_protocol::StructuredInputAnswer {
                question_id: "q1".to_string(),
                value: "answer1".to_string(),
            }],
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::RunTurn { user_input, activate_task, turn_kind } => {
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

        let op = Op::DynamicToolResult {
            call_id: "call_123".to_string(),
            success: true,
            result: json!({"data": "value"}),
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { message, .. } if message.contains("No pending"))
                });
                assert!(has_error);
            }
            _ => panic!("Expected NoTurn"),
        }
    }

    #[tokio::test]
    async fn test_handle_dynamic_tool_result_success() {
        let mut state = create_test_state();
        state.turn_state.set_dynamic_tool_call(crate::approval::PendingDynamicToolCall {
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

        let op = Op::DynamicToolResult {
            call_id: "call_123".to_string(),
            success: true,
            result: json!({"data": "result"}),
        };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::RunTurn { user_input, activate_task, turn_kind } => {
                assert!(!activate_task);
                assert!(user_input.is_none());
                assert!(matches!(turn_kind, TurnRunKind::ResumeTurn));
            }
            _ => panic!("Expected RunTurn"),
        }
        
        // Verify ToolCallCompleted event was emitted
        let has_event = events.iter().any(|e| {
            matches!(e, Event::ToolCallCompleted { call_id, .. } if call_id == "call_123")
        });
        assert!(has_event);
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

        let op = Op::Rollback { num_turns: 0 };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                let has_error = events.iter().any(|e| {
                    matches!(e, Event::Error { message, .. } if message.contains("num_turns must be >= 1"))
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
        state.session.add_assistant_message("a1");
        state.session.add_user_message("u2");
        state.session.add_assistant_message("a2");
        
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let op = Op::Rollback { num_turns: 1 };

        let result = handle_runtime_op_with_cancel(&mut state, op, &mut emit, &cancel).await;
        assert!(result.is_ok());
        
        match result.unwrap() {
            RuntimeOpAction::NoTurn => {
                // Verify SessionRolledBack event was emitted
                let has_event = events.iter().any(|e| {
                    matches!(e, Event::SessionRolledBack { num_turns, .. } if *num_turns == 1)
                });
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

        let op = Op::Cancel;

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
}
