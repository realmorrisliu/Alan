use anyhow::Result;
use alan_protocol::Event;
use serde_json::json;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    llm::{build_generation_request, convert_session_messages},
    prompts,
};

use super::agent_loop::{
    AgentLoopState, generate_with_retry_with_cancel, maybe_compact_context_with_cancel,
};
use super::tool_orchestrator::{
    ToolBatchOrchestratorOutcome, ToolOrchestratorInputs, ToolTurnOrchestrator,
};
use super::turn_support::{
    check_turn_cancelled, detect_provider, emit_streaming_chunks, emit_task_completed_success,
    normalize_tool_calls,
};
use super::virtual_tools::virtual_tool_definitions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TurnRunKind {
    NewTurn,
    ResumeTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TurnExecutionOutcome {
    Finished,
    Paused,
}

#[derive(Debug, Default, Clone, Copy)]
#[allow(dead_code)]
struct TurnEventTrackerState {
    turn_started: bool,
    turn_completed: bool,
}

/// Run a single agent turn
pub(super) async fn run_turn_with_cancel<E, F>(
    state: &mut AgentLoopState,
    turn_kind: TurnRunKind,
    user_input: Option<String>,
    emit: &mut E,
    cancel: &CancellationToken,
) -> Result<TurnExecutionOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    if matches!(turn_kind, TurnRunKind::NewTurn) {
        emit(Event::TurnStarted {}).await;
    }

    emit(Event::Thinking {
        message: "Working on your request...".to_string(),
    })
    .await;

    let compaction_timeout = tokio::time::Duration::from_secs(30);
    match tokio::time::timeout(
        compaction_timeout,
        maybe_compact_context_with_cancel(state, emit, cancel),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            warn!(error = %e, "Context compaction failed");
        }
        Err(_) => {
            warn!("Context compaction timeout - continuing without compaction");
        }
    }
    if check_turn_cancelled(state, emit, cancel).await? {
        return Ok(TurnExecutionOutcome::Finished);
    }

    if let Some(user_input) = user_input.as_ref() {
        state.session.add_user_message(user_input);
    }

    let system_prompt = prompts::build_agent_system_prompt(&state.core_config, "");
    let user_input_ref = user_input.as_deref();

    let mut tools = state.tools.get_tool_definitions();
    tools.extend(virtual_tool_definitions());
    tools.extend(
        state
            .session
            .dynamic_tools
            .values()
            .map(|tool| crate::llm::ToolDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: tool.parameters.clone(),
            }),
    );

    let max_tool_loops = if state.runtime_config.max_tool_loops == 0 {
        None
    } else {
        Some(state.runtime_config.max_tool_loops)
    };
    let mut tool_orchestrator =
        ToolTurnOrchestrator::new(max_tool_loops, state.runtime_config.tool_repeat_limit);

    loop {
        if check_turn_cancelled(state, emit, cancel).await? {
            return Ok(TurnExecutionOutcome::Finished);
        }
        let provider = detect_provider(&state.llm_client);

        let prompt_view = state.session.tape.prompt_view();
        let estimated_prompt_tokens = prompt_view.estimated_tokens;
        let context_revision = prompt_view.reference_context.revision;
        let messages = prompt_view.messages;
        let llm_messages = convert_session_messages(&messages);
        let llm_tools: Vec<crate::llm::ToolDefinition> = tools
            .iter()
            .map(|t| {
                crate::llm::ToolDefinition::new(&t.name, &t.description)
                    .with_parameters(t.parameters.clone())
            })
            .collect();

        let request = build_generation_request(
            Some(system_prompt.clone()),
            llm_messages,
            llm_tools,
            Some(state.runtime_config.temperature),
            Some(state.runtime_config.max_tokens as i32),
        );

        let request_start = Instant::now();
        info!(
            messages = messages.len(),
            estimated_prompt_tokens,
            context_revision,
            tools = tools.len(),
            provider,
            "LLM request"
        );

        let response = match generate_with_retry_with_cancel(
            &mut state.llm_client,
            request,
            state.runtime_config.llm_request_timeout_secs,
            cancel,
        )
        .await
        {
            Ok(response) => response,
            Err(error) => {
                if cancel.is_cancelled() && check_turn_cancelled(state, emit, cancel).await? {
                    return Ok(TurnExecutionOutcome::Finished);
                }
                error!(elapsed_ms = request_start.elapsed().as_millis(), error = %error, "LLM failed");
                emit(Event::Error {
                    message: format!("LLM request failed: {}", error),
                    recoverable: true,
                })
                .await;
                return Ok(TurnExecutionOutcome::Finished);
            }
        };

        let tool_calls = normalize_tool_calls(response.tool_calls);

        if !response.content.is_empty() {
            emit(Event::ThinkingComplete {}).await;
            emit_streaming_chunks(emit, &response.content).await;
        } else if !tool_calls.is_empty() {
            emit(Event::ThinkingComplete {}).await;
        }

        if !tool_calls.is_empty() {
            let session_tool_calls: Vec<crate::session::ToolCall> = tool_calls
                .iter()
                .map(|tc| crate::session::ToolCall {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                })
                .collect();
            state
                .session
                .add_assistant_message_with_tool_calls(&response.content, session_tool_calls);
        } else if !response.content.is_empty() {
            state.session.add_assistant_message(&response.content);
        }

        if !tool_calls.is_empty() {
            match tool_orchestrator
                .orchestrate_tool_batch(
                    state,
                    &tool_calls,
                    ToolOrchestratorInputs {
                        user_input: user_input_ref,
                        cancel,
                    },
                    emit,
                )
                .await?
            {
                ToolBatchOrchestratorOutcome::ContinueTurnLoop { .. } => {
                    // Continue the loop
                }
                ToolBatchOrchestratorOutcome::PauseTurn => return Ok(TurnExecutionOutcome::Paused),
                ToolBatchOrchestratorOutcome::EndTurn => {
                    return Ok(TurnExecutionOutcome::Finished);
                }
            }
            continue;
        }

        if response.content.is_empty() {
            emit(Event::MessageDeltaChunk {
                chunk: "I apologize, but I couldn't generate a response.".to_string(),
                is_final: true,
            })
            .await;
            emit(Event::TaskCompleted {
                summary: "Turn completed with empty response fallback".to_string(),
                results: json!({"status":"completed","fallback":"empty_response"}),
            })
            .await;
            emit(Event::TurnCompleted {}).await;
            return Ok(TurnExecutionOutcome::Finished);
        }

        emit_task_completed_success(emit, "Task completed").await;
        return Ok(TurnExecutionOutcome::Finished);
    }
}
