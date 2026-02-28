//! Core agent loop implementation.
//!
//! This module contains the main agent execution logic.

use alan_protocol::{Event, Submission};
use anyhow::Result;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    config::Config,
    llm::{LlmClient, build_generation_request},
    prompts, retry,
    runtime::RuntimeConfig,
    session::Session,
    tools::ToolRegistry,
};

use super::submission_handlers::{RuntimeOpAction, handle_runtime_op_with_cancel};
use super::tool_orchestrator::{
    ToolBatchOrchestratorOutcome, ToolOrchestratorInputs, replay_approved_tool_batch_with_cancel,
    replay_approved_tool_call_with_cancel,
};
pub(super) use super::turn_executor::run_turn_with_cancel;
use super::turn_executor::{TurnExecutionOutcome, TurnRunKind};
use super::turn_state::{TurnActivityState, TurnState};
#[allow(unused_imports)]
use super::turn_support::{
    cancel_current_task, detect_provider, emit_streaming_chunks, normalize_tool_calls,
    split_text_for_typing,
};

/// Normalized tool call with guaranteed ID
#[derive(Debug, Clone)]
pub struct NormalizedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Agent state for the execution loop
pub struct RuntimeLoopState {
    pub workspace_id: String,
    pub session: Session,
    pub llm_client: LlmClient,
    pub core_config: Config,
    pub runtime_config: RuntimeConfig,
    pub tools: ToolRegistry,
    pub turn_state: TurnState,
}

/// Handle a single submission
#[cfg_attr(not(test), allow(dead_code))]
pub async fn handle_submission<E, F>(
    state: &mut RuntimeLoopState,
    submission: Submission,
    emit: &mut E,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let cancel = CancellationToken::new();
    handle_submission_with_cancel(state, submission, emit, &cancel).await
}

pub(crate) async fn handle_submission_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    submission: Submission,
    emit: &mut E,
    cancel: &CancellationToken,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let op = submission.op;

    match handle_runtime_op_with_cancel(state, op, emit, cancel).await? {
        RuntimeOpAction::NoTurn => Ok(()),
        RuntimeOpAction::RunTurn {
            turn_kind,
            user_input,
            activate_task,
        } => {
            state
                .turn_state
                .set_turn_activity(TurnActivityState::Running);
            let turn_outcome =
                match run_turn_with_cancel(state, turn_kind, user_input, emit, cancel).await {
                    Ok(outcome) => outcome,
                    Err(err) => {
                        state.turn_state.set_turn_activity(TurnActivityState::Idle);
                        return Err(err);
                    }
                };
            state.turn_state.set_turn_activity(
                if matches!(turn_outcome, TurnExecutionOutcome::Paused) {
                    TurnActivityState::Paused
                } else {
                    TurnActivityState::Idle
                },
            );
            if activate_task {
                state.session.has_active_task = true;
            }
            Ok(())
        }
        RuntimeOpAction::ReplayApprovedToolCall { tool_call } => {
            state
                .turn_state
                .set_turn_activity(TurnActivityState::Running);
            match replay_approved_tool_call_with_cancel(
                state,
                &tool_call,
                ToolOrchestratorInputs { cancel },
                emit,
            )
            .await
            {
                Ok(outcome) => match outcome {
                    ToolBatchOrchestratorOutcome::ContinueTurnLoop { .. } => {
                        let turn_outcome = match run_turn_with_cancel(
                            state,
                            TurnRunKind::ResumeTurn,
                            None,
                            emit,
                            cancel,
                        )
                        .await
                        {
                            Ok(outcome) => outcome,
                            Err(err) => {
                                state.turn_state.set_turn_activity(TurnActivityState::Idle);
                                return Err(err);
                            }
                        };
                        state.turn_state.set_turn_activity(
                            if matches!(turn_outcome, TurnExecutionOutcome::Paused) {
                                TurnActivityState::Paused
                            } else {
                                TurnActivityState::Idle
                            },
                        );
                    }
                    ToolBatchOrchestratorOutcome::PauseTurn => {
                        state
                            .turn_state
                            .set_turn_activity(TurnActivityState::Paused);
                    }
                    ToolBatchOrchestratorOutcome::EndTurn => {
                        state.turn_state.set_turn_activity(TurnActivityState::Idle);
                    }
                },
                Err(err) => {
                    state.turn_state.set_turn_activity(TurnActivityState::Idle);
                    return Err(err);
                }
            };
            Ok(())
        }
        RuntimeOpAction::ReplayApprovedToolBatch { tool_calls } => {
            state
                .turn_state
                .set_turn_activity(TurnActivityState::Running);
            match replay_approved_tool_batch_with_cancel(
                state,
                &tool_calls,
                ToolOrchestratorInputs { cancel },
                emit,
            )
            .await
            {
                Ok(outcome) => match outcome {
                    ToolBatchOrchestratorOutcome::ContinueTurnLoop { .. } => {
                        let turn_outcome = match run_turn_with_cancel(
                            state,
                            TurnRunKind::ResumeTurn,
                            None,
                            emit,
                            cancel,
                        )
                        .await
                        {
                            Ok(outcome) => outcome,
                            Err(err) => {
                                state.turn_state.set_turn_activity(TurnActivityState::Idle);
                                return Err(err);
                            }
                        };
                        state.turn_state.set_turn_activity(
                            if matches!(turn_outcome, TurnExecutionOutcome::Paused) {
                                TurnActivityState::Paused
                            } else {
                                TurnActivityState::Idle
                            },
                        );
                    }
                    ToolBatchOrchestratorOutcome::PauseTurn => {
                        state
                            .turn_state
                            .set_turn_activity(TurnActivityState::Paused);
                    }
                    ToolBatchOrchestratorOutcome::EndTurn => {
                        state.turn_state.set_turn_activity(TurnActivityState::Idle);
                    }
                },
                Err(err) => {
                    state.turn_state.set_turn_activity(TurnActivityState::Idle);
                    return Err(err);
                }
            };
            Ok(())
        }
    }
}

/// Generate LLM response with retry logic
#[cfg_attr(not(test), allow(dead_code))]
async fn generate_with_retry(
    llm_client: &mut LlmClient,
    request: crate::llm::GenerationRequest,
    timeout_secs: u64,
) -> Result<crate::llm::GenerationResponse> {
    let cancel = CancellationToken::new();
    generate_with_retry_with_cancel(llm_client, request, timeout_secs, &cancel).await
}

pub(super) async fn generate_with_retry_with_cancel(
    llm_client: &mut LlmClient,
    request: crate::llm::GenerationRequest,
    timeout_secs: u64,
    cancel: &CancellationToken,
) -> Result<crate::llm::GenerationResponse> {
    let max_retries = retry::DEFAULT_MAX_RETRIES;
    let mut last_error = None;

    for attempt in 0..=max_retries {
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("LLM request cancelled"));
        }
        // timeout_secs == 0 means no timeout (wait indefinitely)
        let result = if timeout_secs == 0 {
            tokio::select! {
                _ = cancel.cancelled() => Err(anyhow::anyhow!("LLM request cancelled")),
                result = llm_client.generate(request.clone()) => result,
            }
        } else {
            let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);
            tokio::select! {
                _ = cancel.cancelled() => Err(anyhow::anyhow!("LLM request cancelled")),
                result = tokio::time::timeout(timeout_duration, llm_client.generate(request.clone())) => {
                    match result {
                        Ok(result) => result,
                        Err(_) => {
                            let timeout_error = anyhow::anyhow!("LLM request timed out");
                            if attempt >= max_retries {
                                return Err(timeout_error);
                            }
                            last_error = Some(timeout_error);
                            let delay = retry::backoff_delay(attempt + 1);
                            tokio::select! {
                                _ = cancel.cancelled() => return Err(anyhow::anyhow!("LLM request cancelled")),
                                _ = tokio::time::sleep(delay) => {}
                            }
                            continue;
                        }
                    }
                }
            }
        };

        match result {
            Ok(response) => return Ok(response),
            Err(error) => {
                if !retry::is_retryable(&error) || attempt >= max_retries {
                    return Err(error);
                }
                last_error = Some(error);
                let delay = retry::backoff_delay(attempt + 1);
                tokio::select! {
                    _ = cancel.cancelled() => return Err(anyhow::anyhow!("LLM request cancelled")),
                    _ = tokio::time::sleep(delay) => {}
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
}

/// Check if context compaction is needed and perform it
pub(super) async fn maybe_compact_context<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let cancel = CancellationToken::new();
    maybe_compact_context_with_cancel(state, emit, &cancel).await
}

pub(super) async fn maybe_compact_context_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    cancel: &CancellationToken,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let trigger_threshold = state.runtime_config.compaction_trigger_messages;
    let keep_last = state.runtime_config.compaction_keep_last;

    let message_count = state.session.tape.len();
    let estimated_prompt_tokens = state.session.tape.estimated_prompt_tokens();
    let token_trigger_threshold = (state.runtime_config.max_tokens as usize).saturating_mul(4);
    let over_message_threshold = message_count > trigger_threshold;
    let over_token_threshold =
        token_trigger_threshold > 0 && estimated_prompt_tokens > token_trigger_threshold;

    if !over_message_threshold && !over_token_threshold {
        return Ok(());
    }

    let messages = state.session.tape.messages().to_vec();
    let cutoff = messages.len().saturating_sub(keep_last);
    let to_summarize = messages[..cutoff].to_vec();

    if to_summarize.is_empty() {
        return Ok(());
    }

    let compaction_count = state.session.tape.compaction_count();

    info!(
        total_messages = message_count,
        estimated_prompt_tokens,
        token_trigger_threshold,
        summarize = to_summarize.len(),
        keep_last,
        compaction_count,
        "Compacting conversation history"
    );

    // Build the messages to send to the compaction LLM.
    // If a previous compaction summary exists, include it as the first message
    // so the LLM can integrate prior context into the new summary.
    let mut llm_messages = Vec::new();

    if let Some(existing_summary) = state.session.tape.summary() {
        llm_messages.push(crate::llm::Message {
            role: crate::llm::MessageRole::Context,
            content: format!(
                "[Previous compaction summary (compaction #{})]\n{}",
                compaction_count, existing_summary
            ),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    llm_messages.extend(state.llm_client.project_messages(&to_summarize));

    // Retry loop: if the compaction request is too large for the LLM context window,
    // progressively remove the oldest messages and retry (following Codex's pattern).
    let max_trim_retries = 5;
    let mut trimmed_count = 0usize;
    let summary = loop {
        let request = build_generation_request(
            Some(prompts::COMPACT_PROMPT.to_string()),
            llm_messages.clone(),
            Vec::new(),
            Some(0.2),
            Some(2048),
        );

        match tokio::select! {
            _ = cancel.cancelled() => Err(anyhow::anyhow!("Compaction cancelled")),
            result = state.llm_client.generate(request) => result,
        } {
            Ok(resp) => {
                let text = resp.content.trim().to_string();
                if trimmed_count > 0 {
                    info!(
                        trimmed_count,
                        "Trimmed oldest messages from compaction input to fit context window"
                    );
                }
                break text;
            }
            Err(err) => {
                if cancel.is_cancelled() {
                    return Ok(());
                }

                // If we still have messages to trim, remove the oldest and retry.
                // The first message might be the previous summary (Context role),
                // so we look for the first non-Context message to remove.
                let removable_count = llm_messages
                    .iter()
                    .filter(|m| !matches!(m.role, crate::llm::MessageRole::Context))
                    .count();

                if trimmed_count < max_trim_retries && removable_count > 1 {
                    // Find and remove the first non-Context message (oldest conversation message)
                    if let Some(idx) = llm_messages
                        .iter()
                        .position(|m| !matches!(m.role, crate::llm::MessageRole::Context))
                    {
                        llm_messages.remove(idx);
                        trimmed_count += 1;
                        warn!(
                            error = %err,
                            trimmed_count,
                            remaining = llm_messages.len(),
                            "Compaction failed, trimming oldest message and retrying"
                        );
                        continue;
                    }
                }

                warn!(error = %err, "Failed to generate compaction summary after retries");
                return Ok(());
            }
        }
    };

    if summary.is_empty() {
        return Ok(());
    }

    // Apply compaction
    state.session.tape.compact(summary.clone(), keep_last);
    state.session.record_summary(&summary);
    emit(Event::ContextCompacted {}).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::approval::PendingConfirmation;
    use crate::config::Config;
    use crate::llm::{
        GenerationRequest, GenerationResponse, LlmClient, LlmProvider, StreamChunk, ToolCall,
    };
    use serde_json::json;
    use tokio_util::sync::CancellationToken;

    struct DelayedMockProvider {
        delay: tokio::time::Duration,
        response_text: String,
    }

    impl DelayedMockProvider {
        fn new(delay: tokio::time::Duration, response_text: impl Into<String>) -> Self {
            Self {
                delay,
                response_text: response_text.into(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for DelayedMockProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            tokio::time::sleep(self.delay).await;
            Ok(GenerationResponse {
                content: self.response_text.clone(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: Vec::new(),
                usage: None,
            })
        }

        async fn chat(&mut self, _system: Option<&str>, user: &str) -> anyhow::Result<String> {
            Ok(format!("mock: {}", user))
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(StreamChunk {
                    text: Some(self.response_text.clone()),
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

    // Test provider that returns errors
    struct ErrorMockProvider {
        error_message: String,
    }

    impl ErrorMockProvider {
        fn new(error_message: impl Into<String>) -> Self {
            Self {
                error_message: error_message.into(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for ErrorMockProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            Err(anyhow::anyhow!("{}", self.error_message))
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Err(anyhow::anyhow!("{}", self.error_message))
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            Err(anyhow::anyhow!("{}", self.error_message))
        }

        fn provider_name(&self) -> &'static str {
            "error_mock"
        }
    }

    #[tokio::test]
    async fn test_generate_with_retry_timeout_zero_waits_for_response() {
        let provider =
            DelayedMockProvider::new(tokio::time::Duration::from_millis(50), "delayed response");
        let mut llm_client = LlmClient::new(provider);
        let request = GenerationRequest::new().with_user_message("hello");

        let started_at = std::time::Instant::now();
        let result = generate_with_retry(&mut llm_client, request, 0).await;

        assert!(
            result.is_ok(),
            "timeout=0 should not fail: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().content, "delayed response");
        assert!(
            started_at.elapsed() >= tokio::time::Duration::from_millis(40),
            "timeout=0 should wait for provider completion rather than timing out immediately"
        );
    }

    #[tokio::test]
    async fn test_generate_with_retry_timeout_triggers() {
        // Provider with long delay should timeout
        let provider = DelayedMockProvider::new(
            tokio::time::Duration::from_secs(10),
            "should not receive this",
        );
        let mut llm_client = LlmClient::new(provider);
        let request = GenerationRequest::new().with_user_message("hello");

        let result = generate_with_retry(&mut llm_client, request, 1).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("timed out") || err_msg.contains("Max retries"));
    }

    #[tokio::test]
    async fn test_generate_with_retry_can_be_cancelled() {
        let provider = DelayedMockProvider::new(
            tokio::time::Duration::from_secs(10),
            "should not receive this",
        );
        let mut llm_client = LlmClient::new(provider);
        let request = GenerationRequest::new().with_user_message("hello");
        let cancel = CancellationToken::new();
        let cancel_for_task = cancel.clone();

        let task = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            cancel_for_task.cancel();
        });

        let result = generate_with_retry_with_cancel(&mut llm_client, request, 0, &cancel).await;
        let _ = task.await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }

    #[tokio::test]
    async fn test_generate_with_retry_non_retryable_error() {
        let provider = ErrorMockProvider::new("non-retryable error");
        let mut llm_client = LlmClient::new(provider);
        let request = GenerationRequest::new().with_user_message("hello");

        let result = generate_with_retry(&mut llm_client, request, 5).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("non-retryable error")
        );
    }

    #[test]
    fn test_normalize_tool_calls_with_ids() {
        let tool_calls = vec![
            ToolCall {
                id: Some("call_1".to_string()),
                name: "search".to_string(),
                arguments: json!({"query": "test"}),
            },
            ToolCall {
                id: Some("call_2".to_string()),
                name: "memory_write".to_string(),
                arguments: json!({"content": "data"}),
            },
        ];

        let normalized = normalize_tool_calls(tool_calls);

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].id, "call_1");
        assert_eq!(normalized[0].name, "search");
        assert_eq!(normalized[1].id, "call_2");
        assert_eq!(normalized[1].name, "memory_write");
    }

    #[test]
    fn test_normalize_tool_calls_missing_ids() {
        let tool_calls = vec![
            ToolCall {
                id: None,
                name: "search".to_string(),
                arguments: json!({}),
            },
            ToolCall {
                id: Some("".to_string()),
                name: "write".to_string(),
                arguments: json!({}),
            },
            ToolCall {
                id: Some("  ".to_string()),
                name: "read".to_string(),
                arguments: json!({}),
            },
        ];

        let normalized = normalize_tool_calls(tool_calls);

        assert_eq!(normalized.len(), 3);
        // All should have generated IDs
        assert!(!normalized[0].id.is_empty());
        assert!(!normalized[1].id.is_empty());
        assert!(!normalized[2].id.is_empty());
        // IDs should be different
        assert_ne!(normalized[0].id, normalized[1].id);
    }

    #[test]
    fn test_normalize_tool_calls_empty() {
        let tool_calls: Vec<ToolCall> = vec![];
        let normalized = normalize_tool_calls(tool_calls);
        assert!(normalized.is_empty());
    }

    #[test]
    fn test_detect_provider_with_mock() {
        // Test that detect_provider returns the correct provider string
        // LlmClient::new maps provider_name() to ProviderType:
        // - "gemini" -> ProviderType::Gemini
        // - "openai" -> ProviderType::OpenAi
        // - "anthropic" -> ProviderType::Anthropic
        // - others -> ProviderType::OpenAi (default)
        struct TestProvider {
            name: &'static str,
        }
        #[async_trait::async_trait]
        impl LlmProvider for TestProvider {
            async fn generate(
                &mut self,
                _request: GenerationRequest,
            ) -> anyhow::Result<GenerationResponse> {
                unreachable!()
            }
            async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
                unreachable!()
            }
            async fn generate_stream(
                &mut self,
                _request: GenerationRequest,
            ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
                unreachable!()
            }
            fn provider_name(&self) -> &'static str {
                self.name
            }
        }

        // Test gemini detection
        let gemini_client = LlmClient::new(TestProvider { name: "gemini" });
        assert_eq!(detect_provider(&gemini_client), "gemini");

        // Test anthropic detection (note: must be exactly "anthropic", not "anthropic_compatible")
        let anthropic_client = LlmClient::new(TestProvider { name: "anthropic" });
        assert_eq!(detect_provider(&anthropic_client), "anthropic_compatible");

        // Test openai detection (note: must be exactly "openai", not "openai_compatible")
        let openai_client = LlmClient::new(TestProvider { name: "openai" });
        assert_eq!(detect_provider(&openai_client), "openai_compatible");

        // Test unknown provider falls back to openai_compatible (LlmClient default)
        let unknown_client = LlmClient::new(TestProvider { name: "custom" });
        assert_eq!(detect_provider(&unknown_client), "openai_compatible");
    }

    #[test]
    fn test_split_text_for_typing() {
        let text = "Hello";
        let chunks = split_text_for_typing(text);

        assert_eq!(chunks, vec!["Hello".to_string()]);
    }

    #[test]
    fn test_split_text_for_typing_empty() {
        let chunks = split_text_for_typing("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_split_text_for_typing_unicode() {
        let text = "你好";
        let chunks = split_text_for_typing(text);

        assert_eq!(chunks, vec!["你好".to_string()]);
    }

    #[test]
    fn test_split_text_for_typing_long_text_chunks_preserve_content() {
        let text = "This is a longer sentence that should be chunked near whitespace boundaries for streaming.";
        let chunks = split_text_for_typing(text);

        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| !c.is_empty()));
        assert_eq!(chunks.concat(), text);
    }

    #[tokio::test]
    async fn test_cancel_current_task() {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "",
            )),
            tools,
            core_config: config,
            runtime_config,
            turn_state: {
                let mut turn_state = TurnState::default();
                turn_state.set_confirmation(PendingConfirmation {
                    checkpoint_id: "cp_123".to_string(),
                    checkpoint_type: "test_checkpoint".to_string(),
                    summary: "Test".to_string(),
                    details: json!({}),
                    options: vec!["approve".to_string()],
                });
                turn_state
            },
        };
        state.session.add_user_message("existing history");
        state.session.has_active_task = true;

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = cancel_current_task(&mut state, &mut emit).await;

        assert!(result.is_ok());
        assert!(state.turn_state.pending_confirmation().is_none());
        assert!(!state.session.has_active_task);
        assert_eq!(state.session.tape.messages().len(), 1);
        assert_eq!(
            state.session.tape.messages()[0].text_content(),
            "existing history"
        );

        // Check events
        assert_eq!(events.len(), 2);
        match &events[0] {
            Event::TaskCompleted { summary, results } => {
                assert!(summary.contains("cancelled"));
                assert_eq!(results["status"], "cancelled");
            }
            _ => panic!("Expected TaskCompleted event"),
        }
        assert!(matches!(events[1], Event::TurnCompleted { .. }));
    }

    #[tokio::test]
    async fn test_emit_streaming_chunks() {
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        emit_streaming_chunks(&mut emit, "Hi").await;

        // Should have: TextDelta content chunk, TextDelta final
        assert_eq!(events.len(), 2);

        match &events[0] {
            Event::TextDelta { chunk, is_final } => {
                assert_eq!(chunk, "Hi");
                assert!(!is_final);
            }
            _ => panic!("Expected TextDelta"),
        }

        match &events[1] {
            Event::TextDelta { chunk, is_final } => {
                assert!(chunk.is_empty());
                assert!(*is_final);
            }
            _ => panic!("Expected final TextDelta"),
        }
    }

    #[test]
    fn test_agent_loop_state_creation() {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "",
            )),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        };

        assert!(state.turn_state.pending_confirmation().is_none());
    }

    #[test]
    fn test_pending_confirmation_clone() {
        let pending = PendingConfirmation {
            checkpoint_id: "cp_123".to_string(),
            checkpoint_type: "test_checkpoint".to_string(),
            summary: "Test summary".to_string(),
            details: json!({"key": "value"}),
            options: vec!["approve".to_string(), "reject".to_string()],
        };

        let cloned = pending.clone();
        assert_eq!(pending.checkpoint_id, cloned.checkpoint_id);
        assert_eq!(pending.checkpoint_type, cloned.checkpoint_type);
        assert_eq!(pending.summary, cloned.summary);
    }

    #[test]
    fn test_normalized_tool_call_creation() {
        let call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "search".to_string(),
            arguments: json!({"query": "test"}),
        };

        assert_eq!(call.id, "call_1");
        assert_eq!(call.name, "search");
    }

    // Tests for maybe_compact_context
    #[tokio::test]
    async fn test_maybe_compact_context_no_compaction_needed() {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "",
            )),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Session is empty, no compaction needed
        let result = maybe_compact_context(&mut state, &mut emit).await;

        assert!(result.is_ok());
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_maybe_compact_context_with_mock_llm() {
        let config = Config::default();
        let mut session = Session::new();

        // Add enough messages to trigger compaction
        for i in 0..65 {
            session.add_user_message(&format!("Message {}", i));
        }

        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "Summary",
            )),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = maybe_compact_context(&mut state, &mut emit).await;

        // Should succeed or fail gracefully
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn test_maybe_compact_context_triggers_on_estimated_token_budget() {
        let config = Config::default();
        let mut session = Session::new();
        session.add_user_message(&"x".repeat(1200));
        session.add_assistant_message(&"y".repeat(1200), None);

        let tools = ToolRegistry::new();
        let mut runtime_config = super::RuntimeConfig::default();
        runtime_config.compaction_trigger_messages = 100; // avoid message-count trigger
        runtime_config.compaction_keep_last = 1;
        runtime_config.max_tokens = 64; // token trigger threshold ~= 256

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "Summary from token-triggered compaction",
            )),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        };

        let mut emit = |_event: Event| async {};
        let result = maybe_compact_context(&mut state, &mut emit).await;

        assert!(result.is_ok());
        assert_eq!(state.session.tape.len(), 1);
        let prompt_messages = state.session.tape.messages_for_prompt();
        assert!(prompt_messages.iter().any(|m| {
            m.is_context()
                && m.text_content()
                    .contains("Summary from token-triggered compaction")
        }));
    }

    // Tests for handle_submission
    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn test_handle_submission_cancel() {
        let config = Config::default();
        let mut session = Session::new();
        session.add_user_message("existing history");
        session.has_active_task = true;
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "",
            )),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let submission = Submission::new(alan_protocol::Op::Interrupt);

        let result = handle_submission(&mut state, submission, &mut emit).await;

        assert!(result.is_ok());
        assert_eq!(events.len(), 2);
        assert_eq!(state.session.tape.messages().len(), 1);
        assert_eq!(
            state.session.tape.messages()[0].text_content(),
            "existing history"
        );
        assert!(!state.session.has_active_task);
        match &events[0] {
            Event::TaskCompleted { summary, results } => {
                assert!(summary.contains("cancelled"));
                assert_eq!(results["status"], "cancelled");
            }
            _ => panic!("Expected TaskCompleted event"),
        }
        assert!(matches!(events[1], Event::TurnCompleted { .. }));
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn test_handle_submission_rollback() {
        let config = Config::default();
        let mut session = Session::new();
        session.add_user_message("u1");
        session.add_assistant_message("a1", None);
        session.add_user_message("u2");
        session.add_assistant_message("a2", None);
        session.has_active_task = true;
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "",
            )),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let submission = Submission::new(alan_protocol::Op::Rollback { turns: 1 });

        let result = handle_submission(&mut state, submission, &mut emit).await;

        assert!(result.is_ok());
        assert_eq!(state.session.tape.messages().len(), 2);
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::SessionRolledBack {
                num_turns,
                removed_messages,
            } => {
                assert_eq!(*num_turns, 1);
                assert_eq!(*removed_messages, 2);
            }
            other => panic!("Expected SessionRolledBack, got {:?}", other),
        }
    }
}
