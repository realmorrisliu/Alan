use alan_protocol::Event;
use anyhow::Result;
use serde_json::json;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    llm::{build_generation_request, convert_session_messages},
    prompts,
};

use super::agent_loop::{
    RuntimeLoopState, generate_with_retry_with_cancel, maybe_compact_context_with_cancel,
};
use super::tool_orchestrator::{
    ToolBatchOrchestratorOutcome, ToolOrchestratorInputs, ToolTurnOrchestrator,
};
use super::turn_support::{
    check_turn_cancelled, detect_provider, emit_streaming_chunks, emit_task_completed_success,
    emit_thinking_chunks, normalize_tool_calls,
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

/// Run a single agent turn
pub(super) async fn run_turn_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
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

        let mut request = build_generation_request(
            Some(system_prompt.clone()),
            llm_messages,
            llm_tools,
            Some(state.runtime_config.temperature),
            Some(state.runtime_config.max_tokens as i32),
        );
        request.thinking_budget_tokens = state.runtime_config.thinking_budget_tokens;

        let request_start = Instant::now();
        info!(
            messages = messages.len(),
            estimated_prompt_tokens,
            context_revision,
            tools = tools.len(),
            provider,
            "LLM request"
        );

        let use_streaming = request.thinking_budget_tokens.is_some();

        let response = if use_streaming {
            // Streaming path: emit thinking/text deltas in real time
            match state.llm_client.generate_stream(request).await {
                Ok(mut rx) => {
                    let mut accumulated_thinking = String::new();
                    let mut accumulated_content = String::new();
                    let mut accumulated_tool_calls: Vec<crate::llm::ToolCall> = Vec::new();
                    // Track tool call assembly from deltas
                    let mut tool_call_buffers: std::collections::HashMap<
                        usize,
                        (Option<String>, Option<String>, String),
                    > = std::collections::HashMap::new();
                    let mut thinking_finalized = false;

                    while let Some(chunk) = rx.recv().await {
                        if cancel.is_cancelled() {
                            break;
                        }

                        // Handle thinking delta
                        if let Some(ref thinking) = chunk.thinking
                            && !thinking.is_empty()
                        {
                            accumulated_thinking.push_str(thinking);
                            emit(Event::ThinkingDelta {
                                chunk: thinking.clone(),
                                is_final: false,
                            })
                            .await;
                        }

                        // Handle text delta — finalize thinking first
                        if let Some(ref text) = chunk.text {
                            if !thinking_finalized && !accumulated_thinking.is_empty() {
                                emit(Event::ThinkingDelta {
                                    chunk: String::new(),
                                    is_final: true,
                                })
                                .await;
                                thinking_finalized = true;
                            }
                            if !text.is_empty() {
                                accumulated_content.push_str(text);
                                emit(Event::TextDelta {
                                    chunk: text.clone(),
                                    is_final: false,
                                })
                                .await;
                            }
                        }

                        // Handle tool call deltas
                        if let Some(ref delta) = chunk.tool_call_delta {
                            let entry = tool_call_buffers
                                .entry(delta.index)
                                .or_insert_with(|| (None, None, String::new()));
                            if let Some(ref id) = delta.id {
                                entry.0 = Some(id.clone());
                            }
                            if let Some(ref name) = delta.name {
                                entry.1 = Some(name.clone());
                            }
                            if let Some(ref args) = delta.arguments_delta {
                                entry.2.push_str(args);
                            }
                        }

                        if chunk.is_finished {
                            break;
                        }
                    }

                    if cancel.is_cancelled() && check_turn_cancelled(state, emit, cancel).await? {
                        return Ok(TurnExecutionOutcome::Finished);
                    }

                    // Finalize thinking if not yet done
                    if !thinking_finalized && !accumulated_thinking.is_empty() {
                        emit(Event::ThinkingDelta {
                            chunk: String::new(),
                            is_final: true,
                        })
                        .await;
                    }

                    // Finalize text
                    if !accumulated_content.is_empty() {
                        emit(Event::TextDelta {
                            chunk: String::new(),
                            is_final: true,
                        })
                        .await;
                    }

                    // Assemble tool calls from buffers
                    let mut indices: Vec<usize> = tool_call_buffers.keys().copied().collect();
                    indices.sort();
                    for idx in indices {
                        if let Some((id, Some(name), args_json)) = tool_call_buffers.remove(&idx) {
                            let arguments =
                                serde_json::from_str(&args_json).unwrap_or(serde_json::Value::Null);
                            accumulated_tool_calls.push(crate::llm::ToolCall {
                                id,
                                name,
                                arguments,
                            });
                        }
                    }

                    crate::llm::GenerationResponse {
                        content: accumulated_content,
                        thinking: if accumulated_thinking.is_empty() {
                            None
                        } else {
                            Some(accumulated_thinking)
                        },
                        tool_calls: accumulated_tool_calls,
                        usage: None,
                    }
                }
                Err(error) => {
                    if cancel.is_cancelled() && check_turn_cancelled(state, emit, cancel).await? {
                        return Ok(TurnExecutionOutcome::Finished);
                    }
                    error!(elapsed_ms = request_start.elapsed().as_millis(), error = %error, "LLM stream failed");
                    emit(Event::Error {
                        message: format!("LLM request failed: {}", error),
                        recoverable: true,
                    })
                    .await;
                    return Ok(TurnExecutionOutcome::Finished);
                }
            }
        } else {
            // Non-streaming path (existing behavior)
            match generate_with_retry_with_cancel(
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
            }
        };

        let tool_calls = normalize_tool_calls(response.tool_calls);

        if !use_streaming {
            // Emit thinking if present (non-streaming path)
            if let Some(ref thinking) = response.thinking
                && !thinking.is_empty()
            {
                emit_thinking_chunks(emit, thinking).await;
            }

            if !response.content.is_empty() {
                emit_streaming_chunks(emit, &response.content).await;
            }
        }

        if !tool_calls.is_empty() {
            let session_tool_calls: Vec<crate::tape::ToolRequest> = tool_calls
                .iter()
                .map(|tc| crate::tape::ToolRequest {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                })
                .collect();
            state.session.add_assistant_message_with_tool_calls(
                &response.content,
                session_tool_calls,
                response.thinking.as_deref(),
            );
        } else if !response.content.is_empty() {
            state
                .session
                .add_assistant_message(&response.content, response.thinking.as_deref());
        }

        if !tool_calls.is_empty() {
            match tool_orchestrator
                .orchestrate_tool_batch(state, &tool_calls, ToolOrchestratorInputs { cancel }, emit)
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
            emit(Event::TextDelta {
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
    use alan_llm::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk, ToolCall};
    use async_trait::async_trait;

    // Mock provider that returns content without tool calls
    struct ContentMockProvider {
        content: String,
    }

    impl ContentMockProvider {
        fn new(content: impl Into<String>) -> Self {
            Self {
                content: content.into(),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for ContentMockProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            Ok(GenerationResponse {
                content: self.content.clone(),
                thinking: None,
                tool_calls: vec![],
                usage: None,
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok(self.content.clone())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(StreamChunk {
                    text: Some(self.content.clone()),
                    thinking: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stop".to_string()),
                })
                .await;
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "content_mock"
        }
    }

    // Mock provider that returns tool calls
    struct ToolCallMockProvider {
        tool_calls: Vec<ToolCall>,
        content: String,
    }

    impl ToolCallMockProvider {
        fn new(tool_calls: Vec<ToolCall>, content: impl Into<String>) -> Self {
            Self {
                tool_calls,
                content: content.into(),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for ToolCallMockProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            Ok(GenerationResponse {
                content: self.content.clone(),
                thinking: None,
                tool_calls: self.tool_calls.clone(),
                usage: None,
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok(format!("mock: {}", self.content))
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(StreamChunk {
                    text: Some(self.content.clone()),
                    thinking: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stop".to_string()),
                })
                .await;
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "tool_mock"
        }
    }

    fn create_test_state_with_provider<P: LlmProvider + 'static>(provider: P) -> RuntimeLoopState {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        let runtime_config = RuntimeConfig::default();

        RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(provider),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        }
    }

    #[tokio::test]
    async fn test_run_turn_with_content_response() {
        let mut state = create_test_state_with_provider(ContentMockProvider::new("Hello, world!"));
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some("Test input".to_string()),
            &mut emit,
            &cancel,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));

        // Check events
        let has_turn_started = events.iter().any(|e| matches!(e, Event::TurnStarted {}));
        let has_task_completed = events
            .iter()
            .any(|e| matches!(e, Event::TaskCompleted { .. }));
        let has_turn_completed = events.iter().any(|e| matches!(e, Event::TurnCompleted {}));

        assert!(has_turn_started, "Expected TurnStarted event");
        assert!(has_task_completed, "Expected TaskCompleted event");
        assert!(has_turn_completed, "Expected TurnCompleted event");
    }

    #[tokio::test]
    async fn test_run_turn_empty_response_fallback() {
        // Provider returns empty content
        let mut state = create_test_state_with_provider(ContentMockProvider::new(""));
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some("Test input".to_string()),
            &mut emit,
            &cancel,
        )
        .await;

        assert!(result.is_ok());

        // Check for empty response fallback
        let has_fallback = events.iter().any(|e| {
            matches!(e, Event::TaskCompleted { results, .. } if results.get("fallback") == Some(&json!("empty_response")))
        });
        assert!(has_fallback, "Expected empty response fallback");
    }

    #[tokio::test]
    async fn test_run_turn_resume_turn() {
        let mut state = create_test_state_with_provider(ContentMockProvider::new("Response"));
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::ResumeTurn, // Resume, not new turn
            None,                    // No new user input
            &mut emit,
            &cancel,
        )
        .await;

        assert!(result.is_ok());

        // Resume turn should not emit TurnStarted
        let turn_started_count = events
            .iter()
            .filter(|e| matches!(e, Event::TurnStarted {}))
            .count();
        assert_eq!(
            turn_started_count, 0,
            "Resume turn should not emit TurnStarted"
        );
    }

    #[tokio::test]
    async fn test_run_turn_with_cancel() {
        let mut state = create_test_state_with_provider(ContentMockProvider::new("Response"));
        let cancel = CancellationToken::new();
        cancel.cancel(); // Cancel immediately

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some("Test input".to_string()),
            &mut emit,
            &cancel,
        )
        .await;

        assert!(result.is_ok());
        // Should finish early due to cancellation
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));
    }

    #[tokio::test]
    async fn test_run_turn_with_update_plan_tool() {
        let mut state = create_test_state_with_provider(ToolCallMockProvider::new(
            vec![ToolCall {
                id: Some("call_1".to_string()),
                name: "update_plan".to_string(),
                arguments: json!({
                    "explanation": "Test plan",
                    "items": [{"id": "1", "content": "Step 1", "status": "in_progress"}]
                }),
            }],
            "", // No content, just tool call
        ));
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some("Test input".to_string()),
            &mut emit,
            &cancel,
        )
        .await;

        assert!(result.is_ok());

        // Should have PlanUpdated event from the tool
        let has_plan_updated = events
            .iter()
            .any(|e| matches!(e, Event::PlanUpdated { .. }));
        assert!(has_plan_updated, "Expected PlanUpdated event");
    }

    #[tokio::test]
    async fn test_run_turn_with_confirmation_tool() {
        let mut state = create_test_state_with_provider(ToolCallMockProvider::new(
            vec![ToolCall {
                id: Some("call_1".to_string()),
                name: "request_confirmation".to_string(),
                arguments: json!({
                    "checkpoint_id": "chk_123",
                    "checkpoint_type": "test",
                    "summary": "Test confirmation"
                }),
            }],
            "",
        ));
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some("Test input".to_string()),
            &mut emit,
            &cancel,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Paused));

        // Should have Yield Confirmation event
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
    async fn test_run_turn_llm_error() {
        // Use error provider
        struct ErrorMockProvider;

        #[async_trait]
        impl LlmProvider for ErrorMockProvider {
            async fn generate(
                &mut self,
                _request: GenerationRequest,
            ) -> anyhow::Result<GenerationResponse> {
                Err(anyhow::anyhow!("LLM error"))
            }

            async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
                Err(anyhow::anyhow!("LLM error"))
            }

            async fn generate_stream(
                &mut self,
                _request: GenerationRequest,
            ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
                Err(anyhow::anyhow!("LLM error"))
            }

            fn provider_name(&self) -> &'static str {
                "error_mock"
            }
        }

        let mut state = create_test_state_with_provider(ErrorMockProvider);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some("Test input".to_string()),
            &mut emit,
            &cancel,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));

        // Should have error event
        let has_error = events.iter().any(
            |e| matches!(e, Event::Error { message, .. } if message.contains("LLM request failed")),
        );
        assert!(has_error, "Expected Error event for LLM failure");
    }
}
