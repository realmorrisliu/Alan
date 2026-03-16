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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompactionMode {
    Manual,
    AutoPreTurn,
    AutoMidTurn,
}

impl CompactionMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::AutoPreTurn => "auto_pre_turn",
            Self::AutoMidTurn => "auto_mid_turn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompactionExecution {
    Skipped,
    Applied {
        input_prompt_tokens: usize,
        output_prompt_tokens: usize,
        result: CompactionResult,
    },
}

#[derive(Debug, Clone)]
pub(super) struct CompactionRequest {
    mode: CompactionMode,
    trigger: CompactionTrigger,
    reason: CompactionReason,
    focus: Option<String>,
}

impl CompactionRequest {
    pub(super) fn manual(focus: Option<String>) -> Self {
        Self {
            mode: CompactionMode::Manual,
            trigger: CompactionTrigger::Manual,
            reason: CompactionReason::ExplicitRequest,
            focus: normalize_compaction_focus(focus),
        }
    }

    pub(super) fn automatic_pre_turn() -> Self {
        Self {
            mode: CompactionMode::AutoPreTurn,
            trigger: CompactionTrigger::Auto,
            reason: CompactionReason::WindowPressure,
            focus: None,
        }
    }

    pub(super) fn automatic_mid_turn() -> Self {
        Self {
            mode: CompactionMode::AutoMidTurn,
            trigger: CompactionTrigger::Auto,
            reason: CompactionReason::ContinuationPressure,
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

const COMPACTION_TOOL_OUTPUT_CHAR_LIMIT: usize = 4_000;
const COMPACTION_TOOL_OUTPUT_HEAD_LINES: usize = 12;
const COMPACTION_TOOL_OUTPUT_TAIL_LINES: usize = 12;
const COMPACTION_TOOL_OUTPUT_IDENTIFIER_LINES: usize = 24;
const COMPACTION_TOOL_OUTPUT_INLINE_LINE_LIMIT: usize = 80;
const DEGRADED_COMPACTION_SNIPPET_CHARS: usize = 240;
const DEGRADED_COMPACTION_SUMMARY_MESSAGES: usize = 6;

fn sanitize_messages_for_compaction(
    messages: &[crate::tape::Message],
) -> Vec<crate::tape::Message> {
    messages
        .iter()
        .map(sanitize_message_for_compaction)
        .collect()
}

fn sanitize_message_for_compaction(message: &crate::tape::Message) -> crate::tape::Message {
    match message {
        crate::tape::Message::Tool { responses } => crate::tape::Message::tool_multi(
            responses
                .iter()
                .map(sanitize_tool_response_for_compaction)
                .collect(),
        ),
        _ => message.clone(),
    }
}

fn sanitize_tool_response_for_compaction(
    response: &crate::tape::ToolResponse,
) -> crate::tape::ToolResponse {
    let text = response.text_content();
    if text.chars().count() <= COMPACTION_TOOL_OUTPUT_CHAR_LIMIT
        && text.lines().count() <= COMPACTION_TOOL_OUTPUT_INLINE_LINE_LIMIT
    {
        return response.clone();
    }

    crate::tape::ToolResponse::text(
        response.id.clone(),
        sanitize_tool_text_for_compaction(&text),
    )
}

fn sanitize_tool_text_for_compaction(text: &str) -> String {
    let line_count = text.lines().count();
    let char_count = text.chars().count();
    if char_count <= COMPACTION_TOOL_OUTPUT_CHAR_LIMIT
        && line_count <= COMPACTION_TOOL_OUTPUT_INLINE_LINE_LIMIT
    {
        return text.to_string();
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut keep = std::collections::BTreeSet::new();

    for idx in 0..lines.len().min(COMPACTION_TOOL_OUTPUT_HEAD_LINES) {
        keep.insert(idx);
    }
    for idx in lines
        .len()
        .saturating_sub(COMPACTION_TOOL_OUTPUT_TAIL_LINES)..lines.len()
    {
        keep.insert(idx);
    }

    let mut identifier_lines = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || line_looks_like_compaction_noise(trimmed) {
            continue;
        }
        if line_contains_critical_identifier(trimmed) {
            keep.insert(idx);
            identifier_lines += 1;
            if identifier_lines >= COMPACTION_TOOL_OUTPUT_IDENTIFIER_LINES {
                break;
            }
        }
    }

    let mut output = vec![format!(
        "[tool output trimmed for compaction; original {line_count} lines / {char_count} chars]"
    )];
    let mut previous = None;
    for idx in keep {
        if let Some(prev) = previous
            && idx > prev + 1
        {
            output.push(format!("[... {} lines omitted ...]", idx - prev - 1));
        }
        output.push(lines[idx].to_string());
        previous = Some(idx);
    }
    if let Some(prev) = previous
        && prev + 1 < lines.len()
    {
        output.push(format!(
            "[... {} lines omitted ...]",
            lines.len() - prev - 1
        ));
    }

    let mut sanitized = output.join("\n");
    if sanitized.chars().count() > COMPACTION_TOOL_OUTPUT_CHAR_LIMIT {
        sanitized = sanitized
            .chars()
            .take(COMPACTION_TOOL_OUTPUT_CHAR_LIMIT)
            .collect::<String>();
        sanitized.push_str("\n[truncated for compaction]");
    }
    sanitized
}

fn line_contains_critical_identifier(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.contains('/')
        || line.contains('\\')
        || lower.contains("call_")
        || lower.contains("tool_call")
        || lower.contains("id=")
        || lower.contains("id:")
        || lower.contains("uuid")
        || lower.contains("sha256:")
        || lower.contains("sha1:")
        || lower.contains("path:")
        || lower.contains("command:")
        || looks_like_shell_command(&lower)
}

fn looks_like_shell_command(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("$ ")
        || [
            "cargo ", "git ", "just ", "bash ", "sh ", "npm ", "pnpm ", "bun ", "make ",
        ]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn line_looks_like_compaction_noise(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("debug")
        || lower.starts_with("[debug]")
        || lower.starts_with("trace")
        || lower.starts_with("[trace]")
        || lower.contains(" debug ")
        || lower.contains(" trace ")
}

fn build_degraded_compaction_summary(
    messages: &[crate::tape::Message],
    existing_summary: Option<&str>,
) -> Option<String> {
    let mut sections = Vec::new();
    if let Some(summary) = existing_summary.filter(|summary| !summary.trim().is_empty()) {
        sections.push("Previous summary:".to_string());
        sections.push(summary.trim().to_string());
    }

    let snippets: Vec<String> = messages
        .iter()
        .filter_map(degraded_compaction_snippet)
        .rev()
        .take(DEGRADED_COMPACTION_SUMMARY_MESSAGES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if snippets.is_empty() {
        return existing_summary
            .filter(|summary| !summary.trim().is_empty())
            .map(ToString::to_string);
    }

    sections.push("Deterministic fallback summary after compaction failure:".to_string());
    sections.push("Recent preserved context:".to_string());
    sections.extend(snippets.into_iter().map(|snippet| format!("- {snippet}")));
    Some(sections.join("\n"))
}

fn degraded_compaction_snippet(message: &crate::tape::Message) -> Option<String> {
    match message {
        crate::tape::Message::User { .. } => {
            let text = message.text_content();
            if text.trim().is_empty() {
                None
            } else {
                Some(format!(
                    "user: {}",
                    truncate_compaction_text(&text, DEGRADED_COMPACTION_SNIPPET_CHARS)
                ))
            }
        }
        crate::tape::Message::Assistant { .. } => {
            let text = message.non_thinking_text_content();
            if text.trim().is_empty() {
                None
            } else {
                Some(format!(
                    "assistant: {}",
                    truncate_compaction_text(&text, DEGRADED_COMPACTION_SNIPPET_CHARS)
                ))
            }
        }
        crate::tape::Message::Tool { responses } => {
            let tool_summaries: Vec<String> = responses
                .iter()
                .filter_map(|response| {
                    let text = sanitize_tool_text_for_compaction(&response.text_content());
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(format!(
                            "tool[{}]: {}",
                            response.id,
                            truncate_compaction_text(trimmed, DEGRADED_COMPACTION_SNIPPET_CHARS)
                        ))
                    }
                })
                .collect();
            if tool_summaries.is_empty() {
                None
            } else {
                Some(tool_summaries.join(" | "))
            }
        }
        crate::tape::Message::System { .. } | crate::tape::Message::Context { .. } => None,
    }
}

fn truncate_compaction_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut truncated = trimmed.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn compaction_warning_message(
    result: CompactionResult,
    error: &str,
    retry_count: u32,
    failure_streak: u32,
) -> String {
    let mut message = match result {
        CompactionResult::Degraded => format!(
            "Context compaction degraded after {retry_count} retry attempt(s): {error}. Used deterministic fallback summary."
        ),
        CompactionResult::Failure => format!(
            "Context compaction failed after {retry_count} retry attempt(s): {error}. Preserving existing context."
        ),
        _ => format!("Context compaction result {result:?}: {error}"),
    };

    if failure_streak >= 2 {
        message.push_str(
            " Repeated compaction degradation/failure detected; consider starting a new session.",
        );
    }

    message
}

async fn handle_compaction_generation_failure<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    request: &CompactionRequest,
    sanitized_to_summarize: &[crate::tape::Message],
    keep_last: usize,
    input_prompt_tokens: usize,
    retry_count: u32,
    error_message: String,
    started_at: std::time::Instant,
) -> Result<CompactionExecution>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let reference_context_revision = state.session.tape.context_revision();

    if let Some(summary) =
        build_degraded_compaction_summary(sanitized_to_summarize, state.session.tape.summary())
    {
        let failure_streak = state.session.note_compaction_failure();
        let warning_message = compaction_warning_message(
            CompactionResult::Degraded,
            &error_message,
            retry_count,
            failure_streak,
        );
        emit(Event::Warning {
            message: warning_message.clone(),
        })
        .await;
        state.session.record_event(
            "compaction_attempt",
            serde_json::json!({
                "mode": request.mode.as_str(),
                "trigger": request.trigger,
                "reason": request.reason,
                "focus": request.focus,
                "retry_count": retry_count,
                "result": "degraded",
                "error": error_message,
                "failure_streak": failure_streak,
                "reference_context_revision": reference_context_revision,
            }),
        );

        state.session.tape.compact(summary.clone(), keep_last);
        let output_prompt_tokens = state.session.tape.estimated_prompt_tokens();
        state.session.record_compaction(CompactedItem {
            message: summary,
            trigger: Some(request.trigger),
            reason: Some(request.reason),
            focus: request.focus.clone(),
            input_messages: Some(sanitized_to_summarize.len()),
            output_messages: Some(state.session.tape.len()),
            input_tokens: Some(input_prompt_tokens),
            output_tokens: Some(output_prompt_tokens),
            duration_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
            retry_count: Some(retry_count),
            result: Some(CompactionResult::Degraded),
            reference_context_revision: Some(reference_context_revision),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        return Ok(CompactionExecution::Applied {
            input_prompt_tokens,
            output_prompt_tokens,
            result: CompactionResult::Degraded,
        });
    }

    let failure_streak = state.session.note_compaction_failure();
    let warning_message = compaction_warning_message(
        CompactionResult::Failure,
        &error_message,
        retry_count,
        failure_streak,
    );
    emit(Event::Warning {
        message: warning_message.clone(),
    })
    .await;
    state.session.record_event(
        "compaction_attempt",
        serde_json::json!({
            "mode": request.mode.as_str(),
            "trigger": request.trigger,
            "reason": request.reason,
            "focus": request.focus,
            "retry_count": retry_count,
            "result": "failure",
            "error": error_message,
            "failure_streak": failure_streak,
            "reference_context_revision": reference_context_revision,
        }),
    );

    Ok(CompactionExecution::Skipped)
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
) -> Result<CompactionExecution>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let cancel = CancellationToken::new();
    maybe_compact_context_with_cancel(state, emit, &request, &cancel).await
}

pub(super) async fn maybe_compact_context_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    request: &CompactionRequest,
    cancel: &CancellationToken,
) -> Result<CompactionExecution>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let trigger_threshold = state.runtime_config.compaction_trigger_messages;
    let keep_last = state.runtime_config.compaction_keep_last;

    let message_count = state.session.tape.len();
    let estimated_prompt_tokens = state.session.tape.estimated_prompt_tokens();
    let context_window_tokens = state.runtime_config.context_window_tokens as usize;
    let emergency_mid_turn_compaction = matches!(request.mode, CompactionMode::AutoMidTurn)
        && super::turn_state::is_auto_mid_turn_compaction_emergency(
            estimated_prompt_tokens,
            context_window_tokens,
        );
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

    if !emergency_mid_turn_compaction && !over_message_threshold && !over_token_threshold {
        return Ok(CompactionExecution::Skipped);
    }

    let messages = state.session.tape.messages().to_vec();
    let retention_start = state.session.tape.compaction_retention_start(keep_last);
    let to_summarize = messages[..retention_start].to_vec();

    if to_summarize.is_empty() {
        return Ok(CompactionExecution::Skipped);
    }

    let compaction_count = state.session.tape.compaction_count();

    info!(
        total_messages = message_count,
        estimated_prompt_tokens,
        context_window_tokens,
        context_window_utilization,
        compaction_trigger_ratio = trigger_ratio,
        token_trigger_threshold,
        emergency_mid_turn_compaction,
        summarize = to_summarize.len(),
        keep_last,
        compaction_count,
        compaction_mode = ?request.mode,
        "Compacting conversation history"
    );

    // Build the messages to send to the compaction LLM.
    // If a previous compaction summary exists, include it as the first message
    // so the LLM can integrate prior context into the new summary.
    let started_at = std::time::Instant::now();
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

    let sanitized_to_summarize = sanitize_messages_for_compaction(&to_summarize);
    llm_messages.extend(state.llm_client.project_messages(&sanitized_to_summarize));

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
                    return Ok(CompactionExecution::Skipped);
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
                return handle_compaction_generation_failure(
                    state,
                    emit,
                    request,
                    &sanitized_to_summarize,
                    keep_last,
                    estimated_prompt_tokens,
                    trimmed_count as u32,
                    err.to_string(),
                    started_at,
                )
                .await;
            }
        }
    };

    if summary.is_empty() {
        return handle_compaction_generation_failure(
            state,
            emit,
            request,
            &sanitized_to_summarize,
            keep_last,
            estimated_prompt_tokens,
            trimmed_count as u32,
            "compaction summary was empty".to_string(),
            started_at,
        )
        .await;
    }

    // Apply compaction
    let input_prompt_tokens = estimated_prompt_tokens;
    state.session.tape.compact(summary.clone(), keep_last);
    let output_prompt_tokens = state.session.tape.estimated_prompt_tokens();
    state.session.reset_compaction_failure_streak();
    state.session.record_compaction(CompactedItem {
        message: summary,
        trigger: Some(request.trigger),
        reason: Some(request.reason),
        focus: request.focus.clone(),
        input_messages: Some(to_summarize.len()),
        output_messages: Some(state.session.tape.len()),
        input_tokens: Some(input_prompt_tokens),
        output_tokens: Some(output_prompt_tokens),
        duration_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
        retry_count: Some(trimmed_count as u32),
        result: Some(CompactionResult::Success),
        reference_context_revision: Some(state.session.tape.context_revision()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    Ok(CompactionExecution::Applied {
        input_prompt_tokens,
        output_prompt_tokens,
        result: CompactionResult::Success,
    })
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

    #[test]
    fn test_sanitize_tool_text_for_compaction_preserves_identifiers_and_trims_noise() {
        let mut tool_output = String::new();
        tool_output.push_str("DEBUG starting noisy stream\n");
        tool_output.push_str("command: cargo test -p alan-runtime compact\n");
        tool_output.push_str("path: crates/runtime/src/tape.rs\n");
        tool_output.push_str("tool_call_id: call_123\n");
        for idx in 0..200 {
            tool_output.push_str(&format!("DEBUG noisy line {idx}\n"));
        }
        tool_output.push_str("final status: ok\n");

        let sanitized = sanitize_tool_text_for_compaction(&tool_output);
        assert!(sanitized.contains("cargo test -p alan-runtime compact"));
        assert!(sanitized.contains("crates/runtime/src/tape.rs"));
        assert!(sanitized.contains("call_123"));
        assert!(sanitized.contains("lines omitted"));
        assert!(sanitized.chars().count() < tool_output.chars().count());
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
        assert_eq!(
            state.session.tape.messages()[0].text_content(),
            "y".repeat(1200)
        );
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
        assert_eq!(
            state.session.tape.messages()[0].text_content(),
            "y".repeat(1200)
        );
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
    #[allow(clippy::field_reassign_with_default)]
    async fn test_maybe_compact_context_allows_mid_turn_emergency_near_hard_limit() {
        let config = Config::default();
        let mut session = Session::new();
        session.add_user_message(&"x".repeat(1200));
        session.add_assistant_message(&"y".repeat(1200), None);
        let estimated_prompt_tokens = session.tape.estimated_prompt_tokens();

        let tools = ToolRegistry::new();
        let mut runtime_config = super::RuntimeConfig::default();
        runtime_config.compaction_trigger_messages = 100;
        runtime_config.compaction_keep_last = 1;
        runtime_config.context_window_tokens = (estimated_prompt_tokens + 10) as u32;
        runtime_config.compaction_trigger_ratio = 1.0;

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(DelayedMockProvider::new(
                tokio::time::Duration::from_millis(0),
                "Summary from emergency mid-turn compaction",
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
            CompactionRequest::automatic_mid_turn(),
        )
        .await;

        assert!(matches!(result, Ok(CompactionExecution::Applied { .. })));
        assert_eq!(
            state.session.tape.summary(),
            Some("Summary from emergency mid-turn compaction")
        );
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
    async fn test_compaction_generation_failure_uses_degraded_fallback_and_audits_it() {
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
            llm_client: LlmClient::new(ErrorMockProvider::new("synthetic compaction failure")),
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

        let outcome = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::manual(Some("preserve open todos".to_string())),
        )
        .await
        .unwrap();

        match outcome {
            CompactionExecution::Applied { result, .. } => {
                assert_eq!(result, CompactionResult::Degraded);
            }
            _ => panic!("expected degraded compaction to apply"),
        }
        assert!(
            state
                .session
                .tape
                .summary()
                .is_some_and(|summary| summary.contains("Deterministic fallback summary"))
        );
        assert!(events.iter().any(|event| {
            matches!(event, Event::Warning { message } if message.contains("deterministic fallback summary"))
        }));

        state.session.flush().await;
        let items = RolloutRecorder::load_history(&rollout_path).await.unwrap();
        let compacted = items.iter().find_map(|item| match item {
            RolloutItem::Compacted(compacted) => Some(compacted),
            _ => None,
        });
        let compacted = compacted.expect("expected compacted rollout item");
        assert_eq!(compacted.result, Some(CompactionResult::Degraded));

        let attempt_event = items.iter().find_map(|item| match item {
            RolloutItem::Event(event) if event.event_type == "compaction_attempt" => Some(event),
            _ => None,
        });
        let attempt_event = attempt_event.expect("expected compaction attempt event");
        assert_eq!(
            attempt_event.payload["result"],
            serde_json::json!("degraded")
        );
    }

    #[tokio::test]
    async fn test_compaction_failure_without_fallback_escalates_warning_and_preserves_tape() {
        let temp_dir = TempDir::new_in(std::env::temp_dir()).unwrap();
        let config = Config::default();
        let mut session = Session::new_with_recorder_in_dir("gemini-2.0-flash", temp_dir.path())
            .await
            .unwrap();
        for _ in 0..65 {
            session.tape.push(crate::tape::Message::assistant(""));
        }

        let original_messages = stateful_messages_snapshot(&session);
        let rollout_path = session.rollout_path().unwrap().clone();
        let tools = ToolRegistry::new();
        let runtime_config = super::RuntimeConfig::default();

        let mut state = RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(ErrorMockProvider::new("synthetic compaction failure")),
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

        let first = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::manual(None),
        )
        .await
        .unwrap();
        let second = maybe_compact_context_for_request(
            &mut state,
            &mut emit,
            CompactionRequest::manual(None),
        )
        .await
        .unwrap();

        assert!(matches!(first, CompactionExecution::Skipped));
        assert!(matches!(second, CompactionExecution::Skipped));
        assert_eq!(
            stateful_messages_snapshot(&state.session),
            original_messages
        );
        assert!(state.session.tape.summary().is_none());

        let warning_messages: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                Event::Warning { message } => Some(message.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(warning_messages.len(), 2);
        assert!(warning_messages[1].contains("consider starting a new session"));

        state.session.flush().await;
        let items = RolloutRecorder::load_history(&rollout_path).await.unwrap();
        let failure_events: Vec<_> = items
            .iter()
            .filter_map(|item| match item {
                RolloutItem::Event(event) if event.event_type == "compaction_attempt" => {
                    Some(event)
                }
                _ => None,
            })
            .collect();
        assert_eq!(failure_events.len(), 2);
        assert!(
            failure_events
                .iter()
                .all(|event| event.payload["result"] == serde_json::json!("failure"))
        );
    }

    fn stateful_messages_snapshot(session: &Session) -> Vec<String> {
        session
            .tape
            .messages()
            .iter()
            .map(crate::tape::Message::text_content)
            .collect()
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
