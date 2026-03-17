use alan_protocol::{
    AppliedCompactionOutcome, CompactionAttemptSnapshot, CompactionMode, CompactionOutcome,
    CompactionReason, CompactionRequestMetadata, CompactionResult, CompactionSkipReason,
    CompactionTrigger, Event, FailedCompactionOutcome, SkippedCompactionOutcome,
};
use anyhow::Result;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{llm::build_generation_request, prompts, rollout::CompactedItem};

use super::agent_loop::RuntimeLoopState;

#[derive(Debug, Clone)]
pub(crate) struct CompactionRequest {
    mode: CompactionMode,
    trigger: CompactionTrigger,
    reason: CompactionReason,
    focus: Option<String>,
}

impl CompactionRequest {
    pub(crate) fn manual(focus: Option<String>) -> Self {
        Self {
            mode: CompactionMode::Manual,
            trigger: CompactionTrigger::Manual,
            reason: CompactionReason::ExplicitRequest,
            focus: normalize_compaction_focus(focus),
        }
    }

    pub(crate) fn automatic_pre_turn() -> Self {
        Self {
            mode: CompactionMode::AutoPreTurn,
            trigger: CompactionTrigger::Auto,
            reason: CompactionReason::WindowPressure,
            focus: None,
        }
    }

    pub(crate) fn automatic_mid_turn() -> Self {
        Self {
            mode: CompactionMode::AutoMidTurn,
            trigger: CompactionTrigger::Auto,
            reason: CompactionReason::ContinuationPressure,
            focus: None,
        }
    }

    pub(crate) fn mode(&self) -> CompactionMode {
        self.mode
    }

    pub(crate) fn trigger(&self) -> CompactionTrigger {
        self.trigger
    }

    pub(crate) fn reason(&self) -> CompactionReason {
        self.reason
    }

    pub(crate) fn focus(&self) -> Option<&str> {
        self.focus.as_deref()
    }

    pub(crate) fn metadata(&self) -> CompactionRequestMetadata {
        CompactionRequestMetadata {
            mode: self.mode,
            trigger: self.trigger,
            reason: self.reason,
            focus: self.focus.clone(),
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

pub(crate) const COMPACTION_TOOL_OUTPUT_CHAR_LIMIT: usize = 4_000;
const COMPACTION_TOOL_OUTPUT_HEAD_LINES: usize = 12;
const COMPACTION_TOOL_OUTPUT_TAIL_LINES: usize = 12;
const COMPACTION_TOOL_OUTPUT_IDENTIFIER_LINES: usize = 24;
const COMPACTION_TOOL_OUTPUT_INLINE_LINE_LIMIT: usize = 80;
const COMPACTION_TOOL_OUTPUT_RENDER_LINE_MAX_CHARS: usize = 240;
const COMPACTION_TOOL_OUTPUT_RENDER_LINE_MIN_CHARS: usize = 32;
const DEGRADED_COMPACTION_SNIPPET_CHARS: usize = 240;
const DEGRADED_COMPACTION_SUMMARY_MESSAGES: usize = 6;
pub(crate) const DEGRADED_COMPACTION_PRIOR_SUMMARY_CHARS: usize = 800;
pub(crate) const DEGRADED_COMPACTION_SUMMARY_MAX_CHARS: usize = 2_400;

pub(crate) fn sanitize_messages_for_compaction(
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

pub(crate) fn sanitize_tool_text_for_compaction(text: &str) -> String {
    let line_count = text.lines().count();
    let char_count = text.chars().count();
    if char_count <= COMPACTION_TOOL_OUTPUT_CHAR_LIMIT
        && line_count <= COMPACTION_TOOL_OUTPUT_INLINE_LINE_LIMIT
    {
        return text.to_string();
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut keep = std::collections::BTreeSet::new();
    let mut critical_lines = std::collections::BTreeSet::new();
    let tail_start = lines
        .len()
        .saturating_sub(COMPACTION_TOOL_OUTPUT_TAIL_LINES);

    for idx in 0..lines.len().min(COMPACTION_TOOL_OUTPUT_HEAD_LINES) {
        keep.insert(idx);
    }
    for idx in tail_start..lines.len() {
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
            critical_lines.insert(idx);
            identifier_lines += 1;
            if identifier_lines >= COMPACTION_TOOL_OUTPUT_IDENTIFIER_LINES {
                break;
            }
        }
    }

    let header = format!(
        "[tool output trimmed for compaction; original {line_count} lines / {char_count} chars]"
    );
    let required: std::collections::BTreeSet<usize> = keep
        .iter()
        .copied()
        .filter(|idx| *idx >= tail_start || critical_lines.contains(idx))
        .collect();
    let optional: Vec<usize> = keep
        .iter()
        .copied()
        .filter(|idx| !required.contains(idx))
        .collect();

    render_tool_output_with_cap(&header, &lines, &required, &optional)
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

pub(crate) fn build_degraded_compaction_summary(
    messages: &[crate::tape::Message],
    existing_summary: Option<&str>,
) -> Option<String> {
    let bounded_existing_summary = existing_summary
        .filter(|summary| !summary.trim().is_empty())
        .map(|summary| truncate_compaction_text(summary, DEGRADED_COMPACTION_PRIOR_SUMMARY_CHARS));

    let mut sections = Vec::new();
    if let Some(summary) = bounded_existing_summary.as_deref() {
        sections.push("Prior summary excerpt:".to_string());
        sections.push(summary.to_string());
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
        return bounded_existing_summary;
    }

    sections.push("Deterministic fallback summary after compaction failure:".to_string());
    sections.push("Recent preserved context:".to_string());
    sections.extend(snippets.into_iter().map(|snippet| format!("- {snippet}")));
    Some(truncate_compaction_text(
        &sections.join("\n"),
        DEGRADED_COMPACTION_SUMMARY_MAX_CHARS,
    ))
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
    truncate_text_with_suffix(trimmed, max_chars, "...")
}

fn render_tool_output_with_cap(
    header: &str,
    lines: &[&str],
    required: &std::collections::BTreeSet<usize>,
    optional: &[usize],
) -> String {
    let mut line_limit = COMPACTION_TOOL_OUTPUT_RENDER_LINE_MAX_CHARS;
    let mut rendered = render_tool_output_selection(header, lines, required, line_limit);

    while rendered.chars().count() > COMPACTION_TOOL_OUTPUT_CHAR_LIMIT
        && line_limit > COMPACTION_TOOL_OUTPUT_RENDER_LINE_MIN_CHARS
    {
        line_limit = line_limit.saturating_sub(16);
        rendered = render_tool_output_selection(header, lines, required, line_limit);
    }

    let mut included = required.clone();
    for idx in optional {
        let mut candidate = included.clone();
        candidate.insert(*idx);
        let candidate_rendered =
            render_tool_output_selection(header, lines, &candidate, line_limit);
        if candidate_rendered.chars().count() <= COMPACTION_TOOL_OUTPUT_CHAR_LIMIT {
            included = candidate;
            rendered = candidate_rendered;
        }
    }

    rendered
}

fn render_tool_output_selection(
    header: &str,
    lines: &[&str],
    included: &std::collections::BTreeSet<usize>,
    line_limit: usize,
) -> String {
    let mut output = vec![header.to_string()];
    let mut previous = None;
    let mut truncated_line = false;

    for idx in included {
        if let Some(prev) = previous
            && *idx > prev + 1
        {
            output.push(format!("[... {} lines omitted ...]", idx - prev - 1));
        }

        let rendered_line = truncate_text_with_suffix(lines[*idx], line_limit, "...");
        truncated_line |= rendered_line.chars().count() < lines[*idx].chars().count();
        output.push(rendered_line);
        previous = Some(*idx);
    }

    if let Some(prev) = previous
        && prev + 1 < lines.len()
    {
        output.push(format!(
            "[... {} lines omitted ...]",
            lines.len() - prev - 1
        ));
    }

    if truncated_line {
        output.push("[truncated for compaction]".to_string());
    }

    output.join("\n")
}

fn truncate_text_with_suffix(text: &str, max_chars: usize, suffix: &str) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    if max_chars == 0 {
        return String::new();
    }

    let suffix_chars = suffix.chars().count();
    if suffix_chars >= max_chars {
        return suffix.chars().take(max_chars).collect();
    }

    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(suffix_chars))
        .collect::<String>();
    truncated.push_str(suffix);
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

fn compaction_success_result(trimmed_count: usize) -> CompactionResult {
    if trimmed_count > 0 {
        CompactionResult::Retry
    } else {
        CompactionResult::Success
    }
}

struct CompactionFailureContext<'a> {
    request: &'a CompactionRequest,
    sanitized_to_summarize: &'a [crate::tape::Message],
    keep_last: usize,
    input_prompt_tokens: usize,
    retry_count: u32,
    error_message: String,
    started_at: std::time::Instant,
}

fn skipped_outcome(
    request: &CompactionRequest,
    input_prompt_tokens: usize,
    reason: CompactionSkipReason,
) -> CompactionOutcome {
    CompactionOutcome::Skipped(SkippedCompactionOutcome {
        request: request.metadata(),
        input_prompt_tokens,
        reason,
    })
}

fn applied_outcome(
    request: &CompactionRequest,
    input_prompt_tokens: usize,
    output_prompt_tokens: usize,
    retry_count: u32,
    result: CompactionResult,
) -> CompactionOutcome {
    CompactionOutcome::Applied(AppliedCompactionOutcome {
        request: request.metadata(),
        input_prompt_tokens,
        output_prompt_tokens,
        retry_count,
        result,
    })
}

fn failed_outcome(
    request: &CompactionRequest,
    input_prompt_tokens: usize,
    retry_count: u32,
) -> CompactionOutcome {
    CompactionOutcome::Failed(FailedCompactionOutcome {
        request: request.metadata(),
        input_prompt_tokens,
        retry_count,
        result: CompactionResult::Failure,
    })
}

fn duration_ms_since(started_at: std::time::Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

struct CompactionAttemptDetails {
    result: CompactionResult,
    input_messages: Option<usize>,
    output_messages: Option<usize>,
    input_prompt_tokens: Option<usize>,
    output_prompt_tokens: Option<usize>,
    retry_count: u32,
    tape_mutated: bool,
    warning_message: Option<String>,
    error_message: Option<String>,
    failure_streak: Option<u32>,
    reference_context_revision_before: Option<u64>,
    reference_context_revision_after: Option<u64>,
    timestamp: String,
}

fn build_compaction_attempt_snapshot(
    attempt_id: String,
    request: &CompactionRequest,
    details: CompactionAttemptDetails,
) -> CompactionAttemptSnapshot {
    let CompactionAttemptDetails {
        result,
        input_messages,
        output_messages,
        input_prompt_tokens,
        output_prompt_tokens,
        retry_count,
        tape_mutated,
        warning_message,
        error_message,
        failure_streak,
        reference_context_revision_before,
        reference_context_revision_after,
        timestamp,
    } = details;

    CompactionAttemptSnapshot {
        attempt_id,
        submission_id: None,
        request: request.metadata(),
        result,
        input_messages,
        output_messages,
        input_prompt_tokens,
        output_prompt_tokens,
        retry_count,
        tape_mutated,
        warning_message,
        error_message,
        failure_streak,
        reference_context_revision_before,
        reference_context_revision_after,
        timestamp,
    }
}

async fn handle_compaction_generation_failure<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    failure: CompactionFailureContext<'_>,
) -> Result<CompactionOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let CompactionFailureContext {
        request,
        sanitized_to_summarize,
        keep_last,
        input_prompt_tokens,
        retry_count,
        error_message,
        started_at,
    } = failure;
    let reference_context_revision = state.session.tape.context_revision();

    if let Some(summary) =
        build_degraded_compaction_summary(sanitized_to_summarize, state.session.tape.summary())
    {
        let attempt_id = uuid::Uuid::new_v4().to_string();
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

        state.session.tape.compact(summary.clone(), keep_last);
        let output_prompt_tokens = state.session.tape.estimated_prompt_tokens();
        let output_messages = state.session.tape.len();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let duration_ms = duration_ms_since(started_at);
        state
            .session
            .record_compaction_attempt(build_compaction_attempt_snapshot(
                attempt_id.clone(),
                request,
                CompactionAttemptDetails {
                    result: CompactionResult::Degraded,
                    input_messages: Some(sanitized_to_summarize.len()),
                    output_messages: Some(output_messages),
                    input_prompt_tokens: Some(input_prompt_tokens),
                    output_prompt_tokens: Some(output_prompt_tokens),
                    retry_count,
                    tape_mutated: true,
                    warning_message: Some(warning_message),
                    error_message: Some(error_message),
                    failure_streak: Some(failure_streak),
                    reference_context_revision_before: Some(reference_context_revision),
                    reference_context_revision_after: Some(state.session.tape.context_revision()),
                    timestamp: timestamp.clone(),
                },
            ));
        state.session.record_compaction(CompactedItem {
            message: summary,
            attempt_id: Some(attempt_id),
            trigger: Some(request.trigger()),
            reason: Some(request.reason()),
            focus: request.focus().map(str::to_string),
            input_messages: Some(sanitized_to_summarize.len()),
            output_messages: Some(output_messages),
            input_tokens: Some(input_prompt_tokens),
            output_tokens: Some(output_prompt_tokens),
            duration_ms: Some(duration_ms),
            retry_count: Some(retry_count),
            result: Some(CompactionResult::Degraded),
            reference_context_revision: Some(reference_context_revision),
            timestamp,
        });

        return Ok(applied_outcome(
            request,
            input_prompt_tokens,
            output_prompt_tokens,
            retry_count,
            CompactionResult::Degraded,
        ));
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
    state
        .session
        .record_compaction_attempt(build_compaction_attempt_snapshot(
            uuid::Uuid::new_v4().to_string(),
            request,
            CompactionAttemptDetails {
                result: CompactionResult::Failure,
                input_messages: Some(sanitized_to_summarize.len()),
                output_messages: None,
                input_prompt_tokens: Some(input_prompt_tokens),
                output_prompt_tokens: None,
                retry_count,
                tape_mutated: false,
                warning_message: Some(warning_message),
                error_message: Some(error_message),
                failure_streak: Some(failure_streak),
                reference_context_revision_before: Some(reference_context_revision),
                reference_context_revision_after: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        ));

    Ok(failed_outcome(request, input_prompt_tokens, retry_count))
}

pub(crate) async fn maybe_compact_context_for_request<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    request: CompactionRequest,
) -> Result<CompactionOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let cancel = CancellationToken::new();
    maybe_compact_context_with_cancel(state, emit, &request, &cancel).await
}

pub(crate) async fn maybe_compact_context_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    request: &CompactionRequest,
    cancel: &CancellationToken,
) -> Result<CompactionOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let trigger_threshold = state.runtime_config.compaction_trigger_messages;
    let keep_last = state.runtime_config.compaction_keep_last;

    let message_count = state.session.tape.len();
    let estimated_prompt_tokens = state.session.tape.estimated_prompt_tokens();
    let context_window_tokens = state.runtime_config.context_window_tokens as usize;
    let emergency_mid_turn_compaction = matches!(request.mode(), CompactionMode::AutoMidTurn)
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
        return Ok(skipped_outcome(
            request,
            estimated_prompt_tokens,
            CompactionSkipReason::UnderThreshold,
        ));
    }

    let messages = state.session.tape.messages().to_vec();
    let retention_start = state.session.tape.compaction_retention_start(keep_last);
    let to_summarize = messages[..retention_start].to_vec();

    if to_summarize.is_empty() {
        return Ok(skipped_outcome(
            request,
            estimated_prompt_tokens,
            CompactionSkipReason::EmptySummarizeRegion,
        ));
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
        compaction_mode = ?request.mode(),
        "Compacting conversation history"
    );

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

    if let Some(focus) = request.focus() {
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
                    return Ok(skipped_outcome(
                        request,
                        estimated_prompt_tokens,
                        CompactionSkipReason::Cancelled,
                    ));
                }

                let removable_count = llm_messages
                    .iter()
                    .filter(|m| !matches!(m.role, crate::llm::MessageRole::Context))
                    .count();

                if trimmed_count < max_trim_retries
                    && removable_count > 1
                    && let Some(idx) = llm_messages
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

                warn!(error = %err, "Failed to generate compaction summary after retries");
                return handle_compaction_generation_failure(
                    state,
                    emit,
                    CompactionFailureContext {
                        request,
                        sanitized_to_summarize: &sanitized_to_summarize,
                        keep_last,
                        input_prompt_tokens: estimated_prompt_tokens,
                        retry_count: trimmed_count as u32,
                        error_message: err.to_string(),
                        started_at,
                    },
                )
                .await;
            }
        }
    };

    if summary.is_empty() {
        return handle_compaction_generation_failure(
            state,
            emit,
            CompactionFailureContext {
                request,
                sanitized_to_summarize: &sanitized_to_summarize,
                keep_last,
                input_prompt_tokens: estimated_prompt_tokens,
                retry_count: trimmed_count as u32,
                error_message: "compaction summary was empty".to_string(),
                started_at,
            },
        )
        .await;
    }

    let input_prompt_tokens = estimated_prompt_tokens;
    let success_result = compaction_success_result(trimmed_count);
    let reference_context_revision = state.session.tape.context_revision();
    let attempt_id = uuid::Uuid::new_v4().to_string();
    state.session.tape.compact(summary.clone(), keep_last);
    let output_prompt_tokens = state.session.tape.estimated_prompt_tokens();
    let output_messages = state.session.tape.len();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let duration_ms = duration_ms_since(started_at);
    state.session.reset_compaction_failure_streak();
    state
        .session
        .record_compaction_attempt(build_compaction_attempt_snapshot(
            attempt_id.clone(),
            request,
            CompactionAttemptDetails {
                result: success_result,
                input_messages: Some(to_summarize.len()),
                output_messages: Some(output_messages),
                input_prompt_tokens: Some(input_prompt_tokens),
                output_prompt_tokens: Some(output_prompt_tokens),
                retry_count: trimmed_count as u32,
                tape_mutated: true,
                warning_message: None,
                error_message: None,
                failure_streak: None,
                reference_context_revision_before: Some(reference_context_revision),
                reference_context_revision_after: Some(state.session.tape.context_revision()),
                timestamp: timestamp.clone(),
            },
        ));
    state.session.record_compaction(CompactedItem {
        message: summary,
        attempt_id: Some(attempt_id),
        trigger: Some(request.trigger()),
        reason: Some(request.reason()),
        focus: request.focus().map(str::to_string),
        input_messages: Some(to_summarize.len()),
        output_messages: Some(output_messages),
        input_tokens: Some(input_prompt_tokens),
        output_tokens: Some(output_prompt_tokens),
        duration_ms: Some(duration_ms),
        retry_count: Some(trimmed_count as u32),
        result: Some(success_result),
        reference_context_revision: Some(reference_context_revision),
        timestamp,
    });

    Ok(applied_outcome(
        request,
        input_prompt_tokens,
        output_prompt_tokens,
        trimmed_count as u32,
        success_result,
    ))
}
