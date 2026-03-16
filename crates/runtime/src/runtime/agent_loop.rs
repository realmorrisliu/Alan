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
    rollout::{CompactedItem, CompactionReason, CompactionResult, CompactionTrigger},
    runtime::RuntimeConfig,
    session::Session,
    tools::ToolRegistry,
};

use super::submission_handlers::{RuntimeOpAction, handle_runtime_op_with_cancel};
use super::tool_orchestrator::{
    ToolBatchOrchestratorOutcome, ToolOrchestratorInputs, replay_approved_tool_batch_with_cancel,
    replay_approved_tool_call_with_cancel,
};
use super::turn_driver::TurnInputBroker;
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

#[derive(Debug, Clone)]
pub(super) struct CompactionRequest {
    trigger: CompactionTrigger,
    reason: CompactionReason,
    focus: Option<String>,
}

impl CompactionRequest {
    pub(super) fn manual(focus: Option<String>) -> Self {
        Self {
            trigger: CompactionTrigger::Manual,
            reason: CompactionReason::ExplicitRequest,
            focus: normalize_compaction_focus(focus),
        }
    }

    pub(super) fn automatic_pre_turn() -> Self {
        Self {
            trigger: CompactionTrigger::Auto,
            reason: CompactionReason::WindowPressure,
            focus: None,
        }
    }
}

fn normalize_compaction_focus(focus: Option<String>) -> Option<String> {
    focus.and_then(|focus| {
        let trimmed = focus.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn estimate_llm_text_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

fn estimate_llm_tool_call_tokens(tool_call: &crate::llm::ToolCall) -> usize {
    tool_call
        .id
        .as_deref()
        .map(estimate_llm_text_tokens)
        .unwrap_or_default()
        + estimate_llm_text_tokens(&tool_call.name)
        + estimate_llm_text_tokens(&tool_call.arguments.to_string())
        + 4
}

fn estimate_llm_message_tokens(message: &crate::llm::Message) -> usize {
    let thinking_tokens = message
        .thinking
        .as_deref()
        .map(estimate_llm_text_tokens)
        .unwrap_or_default();
    let signature_tokens = message
        .thinking_signature
        .as_deref()
        .map(estimate_llm_text_tokens)
        .unwrap_or_default();
    let redacted_tokens = message
        .redacted_thinking
        .as_ref()
        .map(|items| {
            items
                .iter()
                .map(|item| estimate_llm_text_tokens(item))
                .sum::<usize>()
        })
        .unwrap_or_default();
    let tool_calls_tokens = message
        .tool_calls
        .as_ref()
        .map(|calls| {
            calls
                .iter()
                .map(estimate_llm_tool_call_tokens)
                .sum::<usize>()
        })
        .unwrap_or_default();
    let tool_call_id_tokens = message
        .tool_call_id
        .as_deref()
        .map(estimate_llm_text_tokens)
        .unwrap_or_default();

    estimate_llm_text_tokens(&message.content)
        + thinking_tokens
        + signature_tokens
        + redacted_tokens
        + tool_calls_tokens
        + tool_call_id_tokens
        + 6
}

fn estimate_llm_messages_tokens(messages: &[crate::llm::Message]) -> usize {
    messages.iter().map(estimate_llm_message_tokens).sum()
}

fn record_compaction_attempt_event(
    session: &Session,
    request: &CompactionRequest,
    llm_messages: &[crate::llm::Message],
    retry_count: u32,
    result: CompactionResult,
    error: Option<&str>,
) {
    let mut payload = serde_json::json!({
        "trigger": request.trigger,
        "reason": request.reason,
        "focus": request.focus,
        "input_messages": llm_messages.len(),
        "input_tokens": estimate_llm_messages_tokens(llm_messages),
        "retry_count": retry_count,
        "result": result,
        "reference_context_revision": session.tape.context_revision(),
    });

    if let Some(error) = error {
        payload["error"] = serde_json::Value::String(error.to_string());
    }

    session.record_event("compaction_attempt", payload);
}
/// Agent state for the execution loop
pub struct RuntimeLoopState {
    pub workspace_id: String,
    pub session: Session,
    pub llm_client: LlmClient,
    pub core_config: Config,
    pub runtime_config: RuntimeConfig,
    pub workspace_persona_dir: Option<std::path::PathBuf>,
    pub tools: ToolRegistry,
    pub prompt_cache: super::prompt_cache::PromptAssemblyCache,
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
    handle_submission_with_cancel_and_steering(state, submission, emit, cancel, None).await
}

pub(crate) async fn handle_submission_with_cancel_and_steering<E, F>(
    state: &mut RuntimeLoopState,
    submission: Submission,
    emit: &mut E,
    cancel: &CancellationToken,
    steering_broker: Option<&TurnInputBroker>,
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
            let turn_outcome = match run_turn_with_cancel(
                state,
                turn_kind,
                user_input,
                emit,
                cancel,
                steering_broker,
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
            if activate_task {
                state.session.has_active_task = true;
            }
            Ok(())
        }
        RuntimeOpAction::ReplayApprovedToolCall {
            tool_call,
            approved_unknown_effect_call_id,
        } => {
            state
                .turn_state
                .set_turn_activity(TurnActivityState::Running);
            match replay_approved_tool_call_with_cancel(
                state,
                &tool_call,
                approved_unknown_effect_call_id.as_deref(),
                ToolOrchestratorInputs {
                    cancel,
                    steering_broker,
                },
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
                            steering_broker,
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
        RuntimeOpAction::ReplayApprovedToolBatch {
            tool_calls,
            approved_unknown_effect_call_id,
        } => {
            state
                .turn_state
                .set_turn_activity(TurnActivityState::Running);
            match replay_approved_tool_batch_with_cancel(
                state,
                &tool_calls,
                approved_unknown_effect_call_id.as_deref(),
                ToolOrchestratorInputs {
                    cancel,
                    steering_broker,
                },
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
                            steering_broker,
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

pub(super) async fn maybe_compact_context_for_request<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    request: CompactionRequest,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let cancel = CancellationToken::new();
    maybe_compact_context_with_cancel(state, emit, &request, &cancel).await
}

pub(super) async fn maybe_compact_context_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    _emit: &mut E,
    request: &CompactionRequest,
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
    let context_window_tokens = state.runtime_config.context_window_tokens as usize;
    let trigger_ratio = state
        .runtime_config
        .compaction_trigger_ratio
        .clamp(0.0, 1.0);
    let token_trigger_threshold = if context_window_tokens == 0 {
        0
    } else {
        ((context_window_tokens as f64) * (trigger_ratio as f64)).ceil() as usize
    };
    let context_window_utilization = if context_window_tokens == 0 {
        0.0
    } else {
        estimated_prompt_tokens as f64 / context_window_tokens as f64
    };
    let over_message_threshold = message_count > trigger_threshold;
    let over_token_threshold =
        context_window_tokens > 0 && estimated_prompt_tokens > token_trigger_threshold;

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
        context_window_tokens,
        context_window_utilization,
        compaction_trigger_ratio = trigger_ratio,
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
    let started_at = std::time::Instant::now();

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

    if let Some(focus) = request.focus.as_deref() {
        llm_messages.push(crate::llm::Message {
            role: crate::llm::MessageRole::Context,
            content: format!("[Compaction focus]\nPreserve and emphasize: {focus}"),
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
        let generation_request = build_generation_request(
            Some(prompts::COMPACT_PROMPT.to_string()),
            llm_messages.clone(),
            Vec::new(),
            Some(0.2),
            Some(2048),
        );

        match tokio::select! {
            _ = cancel.cancelled() => Err(anyhow::anyhow!("Compaction cancelled")),
            result = state.llm_client.generate(generation_request) => result,
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

                let error_message = err.to_string();
                warn!(error = %err, "Failed to generate compaction summary after retries");
                record_compaction_attempt_event(
                    &state.session,
                    request,
                    &llm_messages,
                    trimmed_count as u32,
                    CompactionResult::Failure,
                    Some(error_message.as_str()),
                );
                return Ok(());
            }
        }
    };

    if summary.is_empty() {
        record_compaction_attempt_event(
            &state.session,
            request,
            &llm_messages,
            trimmed_count as u32,
            CompactionResult::Failure,
            Some("empty_compaction_summary"),
        );
        return Ok(());
    }

    let input_messages = llm_messages.len();
    let input_tokens = estimate_llm_messages_tokens(&llm_messages);
    let compaction_result = if trimmed_count > 0 {
        CompactionResult::Retry
    } else {
        CompactionResult::Success
    };

    // Apply compaction
    state.session.tape.compact(summary.clone(), keep_last);
    state.session.record_compaction(CompactedItem {
        message: summary,
        trigger: Some(request.trigger),
        reason: Some(request.reason),
        focus: request.focus.clone(),
        input_messages: Some(input_messages),
        output_messages: Some(state.session.tape.prompt_view().messages.len()),
        input_tokens: Some(input_tokens),
        output_tokens: Some(state.session.tape.estimated_prompt_tokens()),
        duration_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
        retry_count: Some(trimmed_count as u32),
        result: Some(compaction_result),
        reference_context_revision: Some(state.session.tape.context_revision()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

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
    use crate::rollout::{
        CompactionReason, CompactionResult, CompactionTrigger, RolloutItem, RolloutRecorder,
    };
    use serde_json::json;
    use tempfile::TempDir;
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
                warnings: Vec::new(),
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

    struct FailOnceProvider {
        error_message: String,
        response_text: String,
        calls: usize,
    }

    impl FailOnceProvider {
        fn new(error_message: impl Into<String>, response_text: impl Into<String>) -> Self {
            Self {
                error_message: error_message.into(),
                response_text: response_text.into(),
                calls: 0,
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for FailOnceProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.calls += 1;
            if self.calls == 1 {
                Err(anyhow::anyhow!("{}", self.error_message))
            } else {
                Ok(GenerationResponse {
                    content: self.response_text.clone(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    usage: None,
                    warnings: Vec::new(),
                    tool_calls: Vec::new(),
                })
            }
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("mock".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "fail_once"
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
    fn test_manual_compaction_request_normalizes_focus() {
        let request =
            CompactionRequest::manual(Some(" preserve todos and constraints ".to_string()));
        assert_eq!(
            request.focus.as_deref(),
            Some("preserve todos and constraints")
        );

        let whitespace_only = CompactionRequest::manual(Some("   ".to_string()));
        assert_eq!(whitespace_only.focus, None);
    }

    #[test]
    fn test_estimate_llm_messages_tokens_scales_with_content() {
        let short = vec![crate::llm::Message {
            role: crate::llm::MessageRole::User,
            content: "short".to_string(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: None,
            tool_calls: None,
            tool_call_id: None,
        }];
        let long = vec![crate::llm::Message {
            role: crate::llm::MessageRole::User,
            content: "this message is substantially longer than the short one".to_string(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        assert!(estimate_llm_messages_tokens(&long) > estimate_llm_messages_tokens(&short));
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
        // - "google_gemini_generate_content" -> ProviderType::GoogleGeminiGenerateContent
        // - "openai_responses" -> ProviderType::OpenAiResponses
        // - "openai_chat_completions" -> ProviderType::OpenAiChatCompletions
        // - "openai_chat_completions_compatible" -> ProviderType::OpenAiChatCompletionsCompatible
        // - "anthropic_messages" -> ProviderType::AnthropicMessages
        // - others -> ProviderType::OpenAiChatCompletionsCompatible (default)
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

        let gemini_client = LlmClient::new(TestProvider {
            name: "google_gemini_generate_content",
        });
        assert_eq!(
            detect_provider(&gemini_client),
            "google_gemini_generate_content"
        );

        let anthropic_client = LlmClient::new(TestProvider {
            name: "anthropic_messages",
        });
        assert_eq!(detect_provider(&anthropic_client), "anthropic_messages");

        let openai_responses_client = LlmClient::new(TestProvider {
            name: "openai_responses",
        });
        assert_eq!(
            detect_provider(&openai_responses_client),
            "openai_responses"
        );

        let openai_chat_completions_client = LlmClient::new(TestProvider {
            name: "openai_chat_completions",
        });
        assert_eq!(
            detect_provider(&openai_chat_completions_client),
            "openai_chat_completions"
        );

        let openai_chat_completions_compatible_client = LlmClient::new(TestProvider {
            name: "openai_chat_completions_compatible",
        });
        assert_eq!(
            detect_provider(&openai_chat_completions_compatible_client),
            "openai_chat_completions_compatible"
        );

        // Unknown providers fall back to the chat-completions-compatible projection.
        let unknown_client = LlmClient::new(TestProvider { name: "custom" });
        assert_eq!(
            detect_provider(&unknown_client),
            "openai_chat_completions_compatible"
        );
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
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
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
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::TurnCompleted { summary } => {
                assert_eq!(summary.as_deref(), Some("Task cancelled by user"));
            }
            _ => panic!("Expected TurnCompleted event"),
        }
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
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
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
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Session is empty, no compaction needed
        let result = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::automatic_pre_turn(),
        )
        .await;

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
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::automatic_pre_turn(),
        )
        .await;

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
        runtime_config.context_window_tokens = 256;
        runtime_config.compaction_trigger_ratio = 0.8;

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
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut emit = |_event: Event| async {};
        let result = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::automatic_pre_turn(),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(state.session.tape.len(), 1);
        let prompt_messages = state.session.tape.messages_for_prompt();
        assert!(prompt_messages.iter().any(|m| {
            m.is_context()
                && m.text_content()
                    .contains("Summary from token-triggered compaction")
        }));
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn test_maybe_compact_context_triggers_immediately_when_ratio_is_zero() {
        let config = Config::default();
        let mut session = Session::new();
        session.add_user_message(&"x".repeat(1200));
        session.add_assistant_message(&"y".repeat(1200), None);

        let tools = ToolRegistry::new();
        let mut runtime_config = super::RuntimeConfig::default();
        runtime_config.compaction_trigger_messages = 100; // avoid message-count trigger
        runtime_config.compaction_keep_last = 1;
        runtime_config.context_window_tokens = 16_384;
        runtime_config.compaction_trigger_ratio = 0.0;

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "Summary from zero-ratio compaction",
            )),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut emit = |_event: Event| async {};
        let result = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::automatic_pre_turn(),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(state.session.tape.len(), 1);
        let prompt_messages = state.session.tape.messages_for_prompt();
        assert!(prompt_messages.iter().any(|m| {
            m.is_context()
                && m.text_content()
                    .contains("Summary from zero-ratio compaction")
        }));
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn test_maybe_compact_context_skips_when_context_window_budget_has_room() {
        let config = Config::default();
        let mut session = Session::new();
        session.add_user_message(&"x".repeat(1200));
        session.add_assistant_message(&"y".repeat(1200), None);

        let tools = ToolRegistry::new();
        let mut runtime_config = super::RuntimeConfig::default();
        runtime_config.compaction_trigger_messages = 100; // avoid message-count trigger
        runtime_config.compaction_keep_last = 1;
        runtime_config.context_window_tokens = 16_384;
        runtime_config.compaction_trigger_ratio = 0.8;

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "Should not compact",
            )),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let original_len = state.session.tape.len();
        let mut emit = |_event: Event| async {};
        let result = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::automatic_pre_turn(),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(state.session.tape.len(), original_len);
        assert!(state.session.tape.summary().is_none());
    }

    #[tokio::test]
    async fn test_manual_compaction_records_audit_fields() {
        let temp_dir = TempDir::new_in(std::env::temp_dir()).unwrap();
        let config = Config::default();
        let mut session = Session::new_with_recorder_in_dir("gemini-2.0-flash", temp_dir.path())
            .await
            .unwrap();
        for i in 0..65 {
            session.add_user_message(&format!("Message {}", i));
        }

        let rollout_path = session.rollout_path().unwrap().clone();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "Manual compaction summary",
            )),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut emit = |_event: Event| async {};
        maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::manual(Some("preserve todos and constraints".to_string())),
        )
        .await
        .unwrap();
        state.session.flush().await;

        let items = RolloutRecorder::load_history(&rollout_path).await.unwrap();
        let compacted = items.into_iter().find_map(|item| match item {
            RolloutItem::Compacted(compacted) => Some(compacted),
            _ => None,
        });

        let compacted = compacted.expect("expected compacted rollout item");
        assert_eq!(compacted.message, "Manual compaction summary");
        assert_eq!(compacted.trigger, Some(CompactionTrigger::Manual));
        assert_eq!(compacted.reason, Some(CompactionReason::ExplicitRequest));
        assert_eq!(
            compacted.focus.as_deref(),
            Some("preserve todos and constraints")
        );
        assert_eq!(compacted.result, Some(CompactionResult::Success));
        assert!(compacted.input_messages.is_some());
        assert!(compacted.output_messages.is_some());
        assert!(compacted.input_tokens.is_some());
        assert!(compacted.output_tokens.is_some());
        assert!(compacted.duration_ms.is_some());
        assert_eq!(compacted.reference_context_revision, Some(0));
    }

    #[tokio::test]
    async fn test_manual_compaction_records_retry_result_after_trimmed_retry() {
        let temp_dir = TempDir::new_in(std::env::temp_dir()).unwrap();
        let config = Config::default();
        let mut session = Session::new_with_recorder_in_dir("gemini-2.0-flash", temp_dir.path())
            .await
            .unwrap();
        for i in 0..65 {
            session.add_user_message(&format!("Message {}", i));
        }

        let rollout_path = session.rollout_path().unwrap().clone();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(FailOnceProvider::new(
                "context window exceeded",
                "Retry compaction summary",
            )),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut emit = |_event: Event| async {};
        maybe_compact_context_for_request(&mut state, &mut emit, CompactionRequest::manual(None))
            .await
            .unwrap();
        state.session.flush().await;

        let items = RolloutRecorder::load_history(&rollout_path).await.unwrap();
        let compacted = items.into_iter().find_map(|item| match item {
            RolloutItem::Compacted(compacted) => Some(compacted),
            _ => None,
        });

        let compacted = compacted.expect("expected compacted rollout item");
        assert_eq!(compacted.message, "Retry compaction summary");
        assert_eq!(compacted.retry_count, Some(1));
        assert_eq!(compacted.result, Some(CompactionResult::Retry));
        assert_eq!(compacted.input_messages, Some(44));
    }

    #[tokio::test]
    async fn test_manual_compaction_failure_records_attempt_event() {
        let temp_dir = TempDir::new_in(std::env::temp_dir()).unwrap();
        let config = Config::default();
        let mut session = Session::new_with_recorder_in_dir("gemini-2.0-flash", temp_dir.path())
            .await
            .unwrap();
        for i in 0..65 {
            session.add_user_message(&format!("Message {}", i));
        }

        let rollout_path = session.rollout_path().unwrap().clone();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(ErrorMockProvider::new("context window exceeded")),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut emit = |_event: Event| async {};
        maybe_compact_context_for_request(&mut state, &mut emit, CompactionRequest::manual(None))
            .await
            .unwrap();
        state.session.flush().await;

        let items = RolloutRecorder::load_history(&rollout_path).await.unwrap();
        let compacted = items.iter().find_map(|item| match item {
            RolloutItem::Compacted(compacted) => Some(compacted),
            _ => None,
        });
        assert!(compacted.is_none());

        let event = items.into_iter().find_map(|item| match item {
            RolloutItem::Event(event) if event.event_type == "compaction_attempt" => Some(event),
            _ => None,
        });

        let event = event.expect("expected compaction_attempt event");
        assert_eq!(event.payload["result"], "failure");
        assert_eq!(event.payload["retry_count"], 5);
        assert_eq!(event.payload["input_messages"], 40);
        assert_eq!(event.payload["error"], "context window exceeded");
    }

    #[tokio::test]
    async fn test_empty_compaction_summary_records_failure_attempt_event() {
        let temp_dir = TempDir::new_in(std::env::temp_dir()).unwrap();
        let config = Config::default();
        let mut session = Session::new_with_recorder_in_dir("gemini-2.0-flash", temp_dir.path())
            .await
            .unwrap();
        for i in 0..65 {
            session.add_user_message(&format!("Message {}", i));
        }

        let rollout_path = session.rollout_path().unwrap().clone();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "   ",
            )),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        };

        let mut emit = |_event: Event| async {};
        maybe_compact_context_for_request(&mut state, &mut emit, CompactionRequest::manual(None))
            .await
            .unwrap();
        state.session.flush().await;

        let items = RolloutRecorder::load_history(&rollout_path).await.unwrap();
        let compacted = items.iter().find_map(|item| match item {
            RolloutItem::Compacted(compacted) => Some(compacted),
            _ => None,
        });
        assert!(compacted.is_none());

        let event = items.into_iter().find_map(|item| match item {
            RolloutItem::Event(event) if event.event_type == "compaction_attempt" => Some(event),
            _ => None,
        });

        let event = event.expect("expected compaction_attempt event");
        assert_eq!(event.payload["result"], "failure");
        assert_eq!(event.payload["retry_count"], 0);
        assert_eq!(event.payload["error"], "empty_compaction_summary");
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
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
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
        assert_eq!(events.len(), 1);
        assert_eq!(state.session.tape.messages().len(), 1);
        assert_eq!(
            state.session.tape.messages()[0].text_content(),
            "existing history"
        );
        assert!(!state.session.has_active_task);
        match &events[0] {
            Event::TurnCompleted { summary } => {
                assert_eq!(summary.as_deref(), Some("Task cancelled by user"));
            }
            _ => panic!("Expected TurnCompleted event"),
        }
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
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
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
        assert_eq!(events.len(), 2);
        assert!(events.iter().any(|event| matches!(
            event,
            Event::TextDelta { chunk, is_final }
                if *is_final && chunk.contains("Rolled back 1 turn(s), removed 2 message(s).")
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::Warning { message }
                if message == crate::ROLLBACK_NON_DURABLE_WARNING
        )));
    }
}
