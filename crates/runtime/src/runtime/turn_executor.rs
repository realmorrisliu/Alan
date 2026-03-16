use alan_protocol::Event;
use anyhow::Result;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::llm::build_generation_request;

use super::agent_loop::{
    CompactionExecution, CompactionRequest, RuntimeLoopState, generate_with_retry_with_cancel,
    maybe_compact_context_with_cancel,
};
use super::response_guardrails::{
    AssistantDraft, GuardrailDecision, ResponseGuardrailContext, ResponseGuardrails,
};
use super::tool_orchestrator::{
    ToolBatchOrchestratorOutcome, ToolOrchestratorInputs, ToolTurnOrchestrator,
};
use super::turn_driver::TurnInputBroker;
use super::turn_support::{
    check_turn_cancelled, detect_provider, emit_streaming_chunks, emit_task_completed_success,
    emit_thinking_chunks, normalize_tool_calls, split_text_for_typing,
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

const STREAM_RECOVERY_OUTPUT_SNIPPET_MAX_CHARS: usize = 2000;
const COMPACTION_TIMEOUT_SECS: u64 = 30;

#[derive(Default)]
struct StreamedToolCallBuffer {
    id: Option<String>,
    name: Option<String>,
    arguments_delta: String,
    final_arguments: Option<String>,
}

fn truncate_for_stream_recovery(text: &str) -> String {
    let truncated: String = text
        .chars()
        .take(STREAM_RECOVERY_OUTPUT_SNIPPET_MAX_CHARS)
        .collect();
    if truncated.chars().count() == text.chars().count() {
        truncated
    } else {
        format!("{truncated}...")
    }
}

fn inject_stream_recovery_instruction(
    request: &mut crate::llm::GenerationRequest,
    visible_text_so_far: &str,
) {
    let instruction = if visible_text_so_far.trim().is_empty() {
        "The prior streaming response was interrupted after visible output but before a complete final answer. Continue the response now. Do not restart from the beginning.".to_string()
    } else {
        format!(
            "The prior streaming response was interrupted after partially outputting text to the user.\nContinue from exactly where it stopped.\nDo not repeat already-emitted text.\nReturn only the continuation.\n\nAlready emitted text:\n<already_emitted>\n{}\n</already_emitted>",
            truncate_for_stream_recovery(visible_text_so_far)
        )
    };

    if let Some(system_prompt) = &mut request.system_prompt {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&instruction);
    } else {
        request.system_prompt = Some(instruction);
    }
}

fn strip_repeated_recovery_prefix(existing_text: &str, recovered_text: &str) -> String {
    if existing_text.is_empty() || recovered_text.is_empty() {
        return recovered_text.to_string();
    }

    if let Some(stripped) = recovered_text.strip_prefix(existing_text) {
        return stripped.to_string();
    }

    let mut overlap_bytes = 0usize;
    let mut overlap_chars = 0usize;
    for (byte_idx, _) in existing_text.char_indices() {
        let suffix = &existing_text[byte_idx..];
        if recovered_text.starts_with(suffix) {
            let suffix_chars = suffix.chars().count();
            if suffix_chars > overlap_chars {
                overlap_chars = suffix_chars;
                overlap_bytes = suffix.len();
            }
        }
    }

    let overlap_threshold_chars = {
        let shortest_len = existing_text
            .chars()
            .count()
            .min(recovered_text.chars().count());
        // Keep threshold low enough for short sentences / CJK text, but high enough
        // to avoid accidental one-character trimming.
        (shortest_len / 2).clamp(3, 16)
    };

    if overlap_chars >= overlap_threshold_chars {
        return recovered_text[overlap_bytes.min(recovered_text.len())..].to_string();
    }

    recovered_text.to_string()
}

fn resolve_skills_registry_cwd(state: &RuntimeLoopState) -> Option<std::path::PathBuf> {
    super::prompt_cache::resolve_skills_registry_cwd(
        state.tools.default_cwd().as_deref(),
        state.core_config.memory.workspace_dir.as_deref(),
    )
}

fn resolve_workspace_persona_dir(state: &RuntimeLoopState) -> Option<std::path::PathBuf> {
    crate::prompts::resolve_workspace_persona_dir_for_workspace(
        &state.core_config,
        state.workspace_persona_dir.as_deref(),
    )
}

fn build_domain_prompt_with_skills(
    state: &mut RuntimeLoopState,
    user_input: Option<&[crate::tape::ContentPart]>,
) -> super::prompt_cache::PromptAssemblyResult {
    state.prompt_cache.rebind_paths(
        resolve_skills_registry_cwd(state),
        resolve_workspace_persona_dir(state),
    );
    state.prompt_cache.build(user_input)
}

/// Run a single agent turn
pub(super) async fn run_turn_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    turn_kind: TurnRunKind,
    user_input: Option<Vec<crate::tape::ContentPart>>,
    emit: &mut E,
    cancel: &CancellationToken,
    steering_broker: Option<&TurnInputBroker>,
) -> Result<TurnExecutionOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    if matches!(turn_kind, TurnRunKind::NewTurn) {
        state.turn_state.reset_auto_mid_turn_compaction_state();
        emit(Event::TurnStarted {}).await;
    }

    let compaction_request = CompactionRequest::automatic_pre_turn();
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(COMPACTION_TIMEOUT_SECS),
        maybe_compact_context_with_cancel(state, emit, &compaction_request, cancel),
    )
    .await
    {
        Ok(Ok(_)) => {}
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

    let user_input_for_skills = user_input.clone();
    if let Some(user_input) = user_input {
        state.session.add_user_message_parts(user_input);
    }

    let prompt_build = build_domain_prompt_with_skills(state, user_input_for_skills.as_deref());
    debug!(
        elapsed_ms = prompt_build.elapsed_ms,
        skills_cache_hit = prompt_build.skills_cache_hit,
        persona_cache_hit = prompt_build.persona_cache_hit,
        cache_builds = prompt_build.metrics.builds,
        cache_hits = prompt_build.metrics.hits,
        "Prepared prompt assembly inputs"
    );
    let _domain_prompt = prompt_build.domain_prompt;
    let system_prompt = prompt_build.system_prompt;

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
    let mut response_guardrails = ResponseGuardrails::default();
    loop {
        if check_turn_cancelled(state, emit, cancel).await? {
            return Ok(TurnExecutionOutcome::Finished);
        }
        let provider = detect_provider(&state.llm_client);

        let prompt_view = state.session.tape.prompt_view();
        let estimated_prompt_tokens = prompt_view.estimated_tokens;
        let context_revision = prompt_view.reference_context.revision;
        let messages = prompt_view.messages;
        let llm_messages = state.llm_client.project_messages(&messages);
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

        let streaming_requested = match state.runtime_config.streaming_mode {
            crate::config::StreamingMode::Off => false,
            crate::config::StreamingMode::On | crate::config::StreamingMode::Auto => true,
        };
        let mut used_streaming = false;
        let mut response_may_be_incomplete = false;

        let response = if streaming_requested {
            // Streaming path: emit thinking/text deltas in real time
            match state.llm_client.generate_stream(request.clone()).await {
                Ok(mut rx) => {
                    used_streaming = true;
                    let mut accumulated_thinking = String::new();
                    let mut accumulated_thinking_signature: Option<String> = None;
                    let mut accumulated_redacted_thinking: Vec<String> = Vec::new();
                    let mut accumulated_content = String::new();
                    let mut accumulated_tool_calls: Vec<crate::llm::ToolCall> = Vec::new();
                    let mut accumulated_usage: Option<crate::llm::TokenUsage> = None;
                    // Track tool call assembly from deltas
                    let mut tool_call_buffers: std::collections::HashMap<
                        usize,
                        StreamedToolCallBuffer,
                    > = std::collections::HashMap::new();
                    let mut thinking_finalized = false;
                    let mut stream_finished = false;
                    let mut stream_finish_reason: Option<String> = None;
                    let mut emitted_stream_output = false;
                    let mut emitted_visible_stream_output = false;
                    let mut stream_interrupted_after_partial = false;

                    while let Some(chunk) = rx.recv().await {
                        if cancel.is_cancelled() {
                            break;
                        }

                        // Handle thinking delta
                        if let Some(ref thinking) = chunk.thinking
                            && !thinking.is_empty()
                        {
                            accumulated_thinking.push_str(thinking);
                            emitted_stream_output = true;
                            emitted_visible_stream_output = true;
                            emit(Event::ThinkingDelta {
                                chunk: thinking.clone(),
                                is_final: false,
                            })
                            .await;
                        }
                        if let Some(signature) = chunk.thinking_signature
                            && !signature.is_empty()
                        {
                            match &mut accumulated_thinking_signature {
                                Some(existing) => existing.push_str(&signature),
                                None => accumulated_thinking_signature = Some(signature),
                            }
                        }
                        if let Some(redacted) = chunk.redacted_thinking
                            && !redacted.is_empty()
                        {
                            accumulated_redacted_thinking.push(redacted);
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
                                emitted_stream_output = true;
                                emitted_visible_stream_output = true;
                                emit(Event::TextDelta {
                                    chunk: text.clone(),
                                    is_final: false,
                                })
                                .await;
                            }
                        }

                        // Handle tool call deltas
                        if let Some(ref delta) = chunk.tool_call_delta {
                            emitted_stream_output = true;
                            let entry = tool_call_buffers.entry(delta.index).or_default();
                            if let Some(ref id) = delta.id {
                                entry.id = Some(id.clone());
                            }
                            if let Some(ref name) = delta.name {
                                entry.name = Some(name.clone());
                            }
                            if let Some(ref args) = delta.arguments_delta {
                                entry.arguments_delta.push_str(args);
                            }
                            if let Some(ref arguments) = delta.arguments {
                                entry.final_arguments = Some(arguments.clone());
                            }
                        }

                        if let Some(usage) = chunk.usage {
                            accumulated_usage = Some(usage);
                        }

                        if chunk.is_finished {
                            stream_finished = true;
                            stream_finish_reason = chunk.finish_reason.clone();
                            break;
                        }
                    }

                    if cancel.is_cancelled() && check_turn_cancelled(state, emit, cancel).await? {
                        return Ok(TurnExecutionOutcome::Finished);
                    }

                    let terminal_stream_error = stream_finish_reason
                        .as_deref()
                        .map(|reason| {
                            let normalized = reason.to_ascii_lowercase();
                            normalized == "stream_closed"
                                || normalized == "stream_error"
                                || normalized == "error"
                                || normalized.contains("error")
                        })
                        .unwrap_or(false);

                    let mut fallback_response: Option<crate::llm::GenerationResponse> = None;
                    if !stream_finished || terminal_stream_error {
                        let has_any_stream_payload = emitted_stream_output
                            || !accumulated_content.is_empty()
                            || !accumulated_thinking.is_empty()
                            || !tool_call_buffers.is_empty();

                        if !has_any_stream_payload {
                            warn!(
                                elapsed_ms = request_start.elapsed().as_millis(),
                                "LLM stream ended before producing output; falling back to non-streaming generation"
                            );
                            used_streaming = false;
                            fallback_response = Some(
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
                                        if cancel.is_cancelled()
                                            && check_turn_cancelled(state, emit, cancel).await?
                                        {
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
                                },
                            );
                        } else if !emitted_visible_stream_output {
                            // Only tool deltas/metadata were observed; prefer safe fallback generation.
                            warn!(
                                elapsed_ms = request_start.elapsed().as_millis(),
                                "LLM stream interrupted before visible output; falling back to non-streaming generation"
                            );
                            used_streaming = false;
                            fallback_response = Some(
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
                                        if cancel.is_cancelled()
                                            && check_turn_cancelled(state, emit, cancel).await?
                                        {
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
                                },
                            );
                        } else {
                            warn!(
                                elapsed_ms = request_start.elapsed().as_millis(),
                                finish_reason = ?stream_finish_reason,
                                "LLM stream ended unexpectedly after partial output; preserving partial response"
                            );
                            stream_interrupted_after_partial = true;
                            let detail = stream_finish_reason
                                .as_deref()
                                .unwrap_or("stream interrupted");
                            emit(Event::Warning {
                                message: format!(
                                    "Stream interrupted after partial output ({detail}); response may be incomplete."
                                ),
                            })
                            .await;
                            response_may_be_incomplete = true;

                            if matches!(
                                state.runtime_config.partial_stream_recovery_mode,
                                crate::config::PartialStreamRecoveryMode::ContinueOnce
                            ) {
                                let mut recovery_request = request.clone();
                                inject_stream_recovery_instruction(
                                    &mut recovery_request,
                                    &accumulated_content,
                                );
                                match generate_with_retry_with_cancel(
                                    &mut state.llm_client,
                                    recovery_request,
                                    state.runtime_config.llm_request_timeout_secs,
                                    cancel,
                                )
                                .await
                                {
                                    Ok(recovered) => {
                                        if cancel.is_cancelled()
                                            && check_turn_cancelled(state, emit, cancel).await?
                                        {
                                            return Ok(TurnExecutionOutcome::Finished);
                                        }

                                        let crate::llm::GenerationResponse {
                                            content: recovered_content,
                                            thinking: recovered_thinking,
                                            thinking_signature: recovered_thinking_signature,
                                            redacted_thinking: recovered_redacted_thinking,
                                            tool_calls: recovered_tool_calls,
                                            usage: recovered_usage,
                                            warnings: recovered_warnings,
                                        } = recovered;

                                        if let Some(recovered_thinking) = recovered_thinking
                                            && !recovered_thinking.is_empty()
                                        {
                                            if accumulated_content.is_empty() && !thinking_finalized
                                            {
                                                for chunk in
                                                    split_text_for_typing(&recovered_thinking)
                                                {
                                                    emit(Event::ThinkingDelta {
                                                        chunk,
                                                        is_final: false,
                                                    })
                                                    .await;
                                                }
                                            }
                                            accumulated_thinking.push_str(&recovered_thinking);
                                        }

                                        if let Some(signature) = recovered_thinking_signature
                                            && !signature.is_empty()
                                        {
                                            match &mut accumulated_thinking_signature {
                                                Some(existing) => existing.push_str(&signature),
                                                None => {
                                                    accumulated_thinking_signature = Some(signature)
                                                }
                                            }
                                        }

                                        if !recovered_redacted_thinking.is_empty() {
                                            accumulated_redacted_thinking
                                                .extend(recovered_redacted_thinking);
                                        }

                                        let continuation = strip_repeated_recovery_prefix(
                                            &accumulated_content,
                                            &recovered_content,
                                        );
                                        if !continuation.is_empty() {
                                            if !thinking_finalized
                                                && !accumulated_thinking.is_empty()
                                            {
                                                emit(Event::ThinkingDelta {
                                                    chunk: String::new(),
                                                    is_final: true,
                                                })
                                                .await;
                                                thinking_finalized = true;
                                            }
                                            for chunk in split_text_for_typing(&continuation) {
                                                emit(Event::TextDelta {
                                                    chunk,
                                                    is_final: false,
                                                })
                                                .await;
                                            }
                                            accumulated_content.push_str(&continuation);
                                        }

                                        if !recovered_tool_calls.is_empty() {
                                            accumulated_tool_calls.extend(recovered_tool_calls);
                                        }
                                        if let Some(usage) = recovered_usage {
                                            accumulated_usage = Some(usage);
                                        }
                                        for warning in recovered_warnings {
                                            emit(Event::Warning { message: warning }).await;
                                        }
                                    }
                                    Err(error) => {
                                        if cancel.is_cancelled()
                                            && check_turn_cancelled(state, emit, cancel).await?
                                        {
                                            return Ok(TurnExecutionOutcome::Finished);
                                        }
                                        warn!(
                                            elapsed_ms = request_start.elapsed().as_millis(),
                                            error = %error,
                                            "Partial stream recovery failed; preserving partial output"
                                        );
                                        emit(Event::Warning {
                                            message: format!(
                                                "Failed to recover interrupted stream: {error}"
                                            ),
                                        })
                                        .await;
                                    }
                                }
                            }
                        }
                    }

                    if let Some(response) = fallback_response {
                        response
                    } else {
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
                        if stream_interrupted_after_partial && !tool_call_buffers.is_empty() {
                            let skipped = tool_call_buffers.len();
                            tool_call_buffers.clear();
                            warn!(
                                skipped,
                                "Skipping streamed tool calls after partial stream interruption"
                            );
                            emit(Event::Warning {
                                message: format!(
                                    "Skipped {skipped} streamed tool call(s) because the stream ended early."
                                ),
                            })
                            .await;
                        } else {
                            let mut indices: Vec<usize> =
                                tool_call_buffers.keys().copied().collect();
                            indices.sort();
                            for idx in indices {
                                if let Some(StreamedToolCallBuffer {
                                    id,
                                    name: Some(name),
                                    arguments_delta,
                                    final_arguments,
                                }) = tool_call_buffers.remove(&idx)
                                {
                                    let arguments_json = final_arguments.unwrap_or(arguments_delta);
                                    match serde_json::from_str(&arguments_json) {
                                        Ok(arguments) => {
                                            accumulated_tool_calls.push(crate::llm::ToolCall {
                                                id,
                                                name,
                                                arguments,
                                            });
                                        }
                                        Err(err) => {
                                            warn!(
                                                tool_name = %name,
                                                error = %err,
                                                "Dropping malformed streamed tool call arguments"
                                            );
                                            emit(Event::Warning {
                                                message: format!(
                                                    "Dropped malformed streamed tool call `{name}` arguments."
                                                ),
                                            })
                                            .await;
                                        }
                                    }
                                }
                            }
                        }

                        crate::llm::GenerationResponse {
                            content: accumulated_content,
                            thinking: if accumulated_thinking.is_empty() {
                                None
                            } else {
                                Some(accumulated_thinking)
                            },
                            thinking_signature: accumulated_thinking_signature,
                            redacted_thinking: accumulated_redacted_thinking,
                            tool_calls: accumulated_tool_calls,
                            usage: accumulated_usage,
                            warnings: Vec::new(),
                        }
                    }
                }
                Err(error) => {
                    if cancel.is_cancelled() && check_turn_cancelled(state, emit, cancel).await? {
                        return Ok(TurnExecutionOutcome::Finished);
                    }
                    warn!(
                        elapsed_ms = request_start.elapsed().as_millis(),
                        error = %error,
                        "LLM stream initialization failed; falling back to non-streaming generation"
                    );
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
                            if cancel.is_cancelled()
                                && check_turn_cancelled(state, emit, cancel).await?
                            {
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

        if let Some(usage) = response.usage {
            info!(
                prompt_tokens = usage.prompt_tokens,
                completion_tokens = usage.completion_tokens,
                total_tokens = usage.total_tokens,
                reasoning_tokens = ?usage.reasoning_tokens,
                "LLM usage"
            );
        }

        for warning in &response.warnings {
            emit(Event::Warning {
                message: warning.clone(),
            })
            .await;
        }

        let tool_calls = normalize_tool_calls(response.tool_calls);

        let guardrail_context = ResponseGuardrailContext::from_state(state);
        let guardrail_draft = AssistantDraft::new(&response.content, !tool_calls.is_empty());
        match response_guardrails.evaluate(&guardrail_context, &guardrail_draft) {
            GuardrailDecision::Accept => {}
            GuardrailDecision::Warn {
                rule_id,
                reason,
                instruction,
            } => {
                warn!(
                    rule_id,
                    reason = %reason,
                    "Response guardrail triggered for assistant output"
                );
                emit(Event::Warning {
                    message: format!("Guardrail warning ({rule_id}): {reason}"),
                })
                .await;
                if instruction.is_some() {
                    emit(Event::Warning {
                        message: format!(
                            "Automatic guardrail regeneration is disabled for parity across streaming and non-streaming outputs ({rule_id})."
                        ),
                    })
                    .await;
                }
            }
        }

        if !used_streaming {
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
            state
                .session
                .add_assistant_message_with_tool_calls_and_reasoning(
                    &response.content,
                    session_tool_calls,
                    response.thinking.as_deref(),
                    response.thinking_signature.as_deref(),
                    &response.redacted_thinking,
                );
        } else if !response.content.is_empty() {
            state.session.add_assistant_message_with_reasoning(
                &response.content,
                response.thinking.as_deref(),
                response.thinking_signature.as_deref(),
                &response.redacted_thinking,
            );
        }

        if !tool_calls.is_empty() {
            match tool_orchestrator
                .orchestrate_tool_batch(
                    state,
                    &tool_calls,
                    ToolOrchestratorInputs {
                        cancel,
                        steering_broker,
                    },
                    emit,
                )
                .await?
            {
                ToolBatchOrchestratorOutcome::ContinueTurnLoop { .. } => {
                    maybe_compact_mid_turn_if_needed(state, emit, cancel).await?;
                    if check_turn_cancelled(state, emit, cancel).await? {
                        return Ok(TurnExecutionOutcome::Finished);
                    }
                }
                ToolBatchOrchestratorOutcome::PauseTurn => return Ok(TurnExecutionOutcome::Paused),
                ToolBatchOrchestratorOutcome::EndTurn => {
                    return Ok(TurnExecutionOutcome::Finished);
                }
            }
            continue;
        }

        if response.content.is_empty() {
            let fallback_text = "I apologize, but I couldn't generate a response.";
            // Persist fallback output (and any reasoning metadata) to tape so
            // subsequent turns can reference what the assistant actually emitted.
            state.session.add_assistant_message_with_reasoning(
                fallback_text,
                response.thinking.as_deref(),
                response.thinking_signature.as_deref(),
                &response.redacted_thinking,
            );
            emit(Event::TextDelta {
                chunk: fallback_text.to_string(),
                is_final: true,
            })
            .await;
            emit(Event::TurnCompleted {
                summary: Some("Turn completed with empty response fallback".to_string()),
            })
            .await;
            return Ok(TurnExecutionOutcome::Finished);
        }

        if response_may_be_incomplete {
            emit_task_completed_success(
                emit,
                "Task completed with interrupted stream; response may be incomplete.",
            )
            .await;
        } else {
            emit_task_completed_success(emit, "Task completed").await;
        }
        return Ok(TurnExecutionOutcome::Finished);
    }
}

async fn maybe_compact_mid_turn_if_needed<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    cancel: &CancellationToken,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let estimated_prompt_tokens = state.session.tape.estimated_prompt_tokens();
    let context_window_tokens = state.runtime_config.context_window_tokens as usize;
    if !state
        .turn_state
        .can_auto_mid_turn_compact(estimated_prompt_tokens, context_window_tokens)
    {
        return Ok(());
    }

    let compaction_request = CompactionRequest::automatic_mid_turn();
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(COMPACTION_TIMEOUT_SECS),
        maybe_compact_context_with_cancel(state, emit, &compaction_request, cancel),
    )
    .await
    {
        Ok(Ok(CompactionExecution::Applied {
            output_prompt_tokens,
            ..
        })) => {
            state
                .turn_state
                .record_auto_mid_turn_compaction(output_prompt_tokens);
        }
        Ok(Ok(CompactionExecution::Skipped)) => {}
        Ok(Err(e)) => {
            warn!(error = %e, "Mid-turn context compaction failed");
        }
        Err(_) => {
            warn!("Mid-turn context compaction timeout - continuing without compaction");
        }
    }

    Ok(())
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
        tools::{Tool, ToolContext, ToolRegistry, ToolResult},
    };
    use alan_llm::{
        GenerationRequest, GenerationResponse, LlmProvider, StreamChunk, ToolCall, ToolCallDelta,
    };
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    // Mock provider that returns content without tool calls
    struct ContentMockProvider {
        content: String,
        thinking: Option<String>,
    }

    impl ContentMockProvider {
        fn new(content: impl Into<String>) -> Self {
            Self {
                content: content.into(),
                thinking: None,
            }
        }

        fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
            self.thinking = Some(thinking.into());
            self
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
                thinking: self.thinking.clone(),
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
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
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: self.tool_calls.clone(),
                usage: None,
                warnings: Vec::new(),
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
            "tool_mock"
        }
    }

    struct StreamedFinalToolArgumentsProvider {
        stream_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for StreamedFinalToolArgumentsProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            Ok(GenerationResponse {
                content: "stream fallback should not be used".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("streamed-final-tool-arguments".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let call = self.stream_calls.fetch_add(1, Ordering::SeqCst);
            let (tx, rx) = tokio::sync::mpsc::channel(4);

            if call == 0 {
                let _ = tx
                    .send(StreamChunk {
                        text: None,
                        thinking: None,
                        thinking_signature: None,
                        redacted_thinking: None,
                        usage: None,
                        tool_call_delta: Some(ToolCallDelta {
                            index: 0,
                            id: Some("call_1".to_string()),
                            name: Some("update_plan".to_string()),
                            arguments_delta: Some("{\"explanation\":".to_string()),
                            arguments: None,
                        }),
                        is_finished: false,
                        finish_reason: None,
                    })
                    .await;
                let _ = tx
                    .send(StreamChunk {
                        text: None,
                        thinking: None,
                        thinking_signature: None,
                        redacted_thinking: None,
                        usage: None,
                        tool_call_delta: Some(ToolCallDelta {
                            index: 0,
                            id: Some("call_1".to_string()),
                            name: Some("update_plan".to_string()),
                            arguments_delta: None,
                            arguments: Some(
                                json!({
                                    "explanation": "Streamed final args",
                                    "items": [
                                        {
                                            "id": "1",
                                            "content": "Step 1",
                                            "status": "completed"
                                        }
                                    ]
                                })
                                .to_string(),
                            ),
                        }),
                        is_finished: false,
                        finish_reason: None,
                    })
                    .await;
                let _ = tx
                    .send(StreamChunk {
                        text: None,
                        thinking: None,
                        thinking_signature: None,
                        redacted_thinking: None,
                        usage: None,
                        tool_call_delta: None,
                        is_finished: true,
                        finish_reason: Some("tool_calls".to_string()),
                    })
                    .await;
            } else {
                let _ = tx
                    .send(StreamChunk {
                        text: Some("Plan updated".to_string()),
                        thinking: None,
                        thinking_signature: None,
                        redacted_thinking: None,
                        usage: None,
                        tool_call_delta: None,
                        is_finished: true,
                        finish_reason: Some("stop".to_string()),
                    })
                    .await;
            }

            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "streamed_final_tool_arguments_mock"
        }
    }

    struct SequenceMockProvider {
        responses: VecDeque<GenerationResponse>,
        generate_calls: Arc<AtomicUsize>,
    }

    impl SequenceMockProvider {
        fn new(responses: Vec<GenerationResponse>, generate_calls: Arc<AtomicUsize>) -> Self {
            Self {
                responses: responses.into(),
                generate_calls,
            }
        }
    }

    #[async_trait]
    impl LlmProvider for SequenceMockProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.generate_calls.fetch_add(1, Ordering::SeqCst);
            self.responses
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("No more scripted responses"))
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("sequence mock".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            Err(anyhow::anyhow!(
                "streaming not supported in SequenceMockProvider"
            ))
        }

        fn provider_name(&self) -> &'static str {
            "sequence_mock"
        }
    }

    struct NetworkCapabilityTool;

    impl Tool for NetworkCapabilityTool {
        fn name(&self) -> &str {
            "network_probe"
        }

        fn description(&self) -> &str {
            "Test tool classified as network capability."
        }

        fn parameters_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {}
            })
        }

        fn execute(&self, _arguments: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
            Box::pin(async move { Ok(json!({"ok": true})) })
        }

        fn capability(&self, _arguments: &serde_json::Value) -> alan_protocol::ToolCapability {
            alan_protocol::ToolCapability::Network
        }
    }

    struct LargeOutputTool {
        output: String,
    }

    impl LargeOutputTool {
        fn new(output: impl Into<String>) -> Self {
            Self {
                output: output.into(),
            }
        }
    }

    impl Tool for LargeOutputTool {
        fn name(&self) -> &str {
            "emit_large_output"
        }

        fn description(&self) -> &str {
            "Emit a large text payload for compaction tests."
        }

        fn parameters_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {}
            })
        }

        fn execute(&self, _arguments: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
            let payload = serde_json::to_value(ContentPart::text(self.output.clone())).unwrap();
            Box::pin(async move { Ok(payload) })
        }
    }

    fn create_test_state_with_provider<P: LlmProvider + 'static>(provider: P) -> RuntimeLoopState {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        // Keep turn-executor tests deterministic by defaulting to non-streaming unless a test
        // explicitly opts into streaming semantics.
        let runtime_config = RuntimeConfig {
            streaming_mode: crate::config::StreamingMode::Off,
            ..RuntimeConfig::default()
        };

        RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(provider),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dir: None,
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(None, None),
            turn_state: TurnState::default(),
        }
    }

    fn create_repo_skill(
        workspace_root: &std::path::Path,
        dir_name: &str,
        skill_name: &str,
        description: &str,
        body: &str,
    ) {
        let skill_dir = workspace_root.join(".alan/skills").join(dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                r#"---
name: {skill_name}
description: {description}
---

{body}
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_strip_repeated_recovery_prefix_does_not_trim_tiny_overlap() {
        let result = strip_repeated_recovery_prefix("abc", "apple pie");
        assert_eq!(result, "apple pie");
    }

    #[test]
    fn test_strip_repeated_recovery_prefix_trims_full_existing_prefix() {
        let result = strip_repeated_recovery_prefix("partial ", "partial and recovered");
        assert_eq!(result, "and recovered");
    }

    #[test]
    fn test_strip_repeated_recovery_prefix_trims_long_suffix_overlap() {
        let existing = "The quick brown fox jumps over ";
        let recovered = "brown fox jumps over the lazy dog";
        let result = strip_repeated_recovery_prefix(existing, recovered);
        assert_eq!(result, "the lazy dog");
    }

    #[test]
    fn test_strip_repeated_recovery_prefix_handles_short_overlap() {
        let existing = "今天北京天气";
        let recovered = "北京天气很好，适合出门";
        let result = strip_repeated_recovery_prefix(existing, recovered);
        assert_eq!(result, "很好，适合出门");
    }

    #[test]
    fn test_resolve_skills_registry_cwd_normalizes_alan_tool_cwd_to_workspace_root() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let alan_dir = workspace_root.join(".alan");
        std::fs::create_dir_all(&alan_dir).unwrap();

        let mut state = create_test_state_with_provider(ContentMockProvider::new("ok"));
        state.tools.set_default_cwd(alan_dir);

        let resolved = resolve_skills_registry_cwd(&state).unwrap();
        assert_eq!(resolved, workspace_root);
    }

    #[test]
    fn test_resolve_skills_registry_cwd_falls_back_to_memory_workspace_dir() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let memory_dir = workspace_root.join(".alan/memory");
        std::fs::create_dir_all(&memory_dir).unwrap();

        let mut state = create_test_state_with_provider(ContentMockProvider::new("ok"));
        state.core_config.memory.workspace_dir = Some(memory_dir);

        let resolved = resolve_skills_registry_cwd(&state).unwrap();
        assert_eq!(resolved, workspace_root);
    }

    #[test]
    fn test_build_domain_prompt_with_skills_includes_mentioned_repo_skill_instructions() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill(
            &workspace_root,
            "my-skill",
            "My Skill",
            "Custom test skill",
            "# Instructions\nUse this skill when asked.",
        );

        let mut state = create_test_state_with_provider(ContentMockProvider::new("ok"));
        state.tools.set_default_cwd(workspace_root);

        let user_input = vec![ContentPart::text("please use $my-skill for this task")];
        let prompt = build_domain_prompt_with_skills(&mut state, Some(&user_input));

        assert!(prompt.system_prompt.contains("## Available Skills"));
        assert!(
            prompt
                .system_prompt
                .contains("## Active Skill Instructions")
        );
        assert!(prompt.system_prompt.contains("## Skill: My Skill"));
        assert!(prompt.system_prompt.contains("Use this skill when asked."));
    }

    #[test]
    fn test_build_domain_prompt_with_skills_uses_persona_fallback_from_memory_dir() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let alan_dir = workspace_root.join(".alan");
        let persona_dir = alan_dir.join("persona");
        let memory_dir = alan_dir.join("memory");
        std::fs::create_dir_all(&memory_dir).unwrap();
        crate::prompts::ensure_workspace_bootstrap_files_at(&persona_dir).unwrap();
        std::fs::write(persona_dir.join("SOUL.md"), "custom fallback persona").unwrap();

        let mut state = create_test_state_with_provider(ContentMockProvider::new("ok"));
        state.core_config.memory.workspace_dir = Some(memory_dir);

        let prompt = build_domain_prompt_with_skills(&mut state, None);

        assert!(prompt.system_prompt.contains("Workspace Persona Context"));
        assert!(prompt.system_prompt.contains("custom fallback persona"));
    }

    struct StreamEndsImmediatelyProvider {
        fallback_content: String,
        generate_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for StreamEndsImmediatelyProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.generate_calls.fetch_add(1, Ordering::SeqCst);
            Ok(GenerationResponse {
                content: self.fallback_content.clone(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok(self.fallback_content.clone())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            drop(tx);
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "stream_ends_immediately_mock"
        }
    }

    struct PartialStreamThenCloseProvider {
        generate_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for PartialStreamThenCloseProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.generate_calls.fetch_add(1, Ordering::SeqCst);
            Ok(GenerationResponse {
                content: "partial and recovered response".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("partial-stream".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(2);
            let _ = tx
                .send(StreamChunk {
                    text: Some("partial ".to_string()),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: false,
                    finish_reason: None,
                })
                .await;
            drop(tx);
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "partial_stream_then_close_mock"
        }
    }

    struct TerminalErrorNoPayloadProvider {
        fallback_content: String,
        generate_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for TerminalErrorNoPayloadProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.generate_calls.fetch_add(1, Ordering::SeqCst);
            Ok(GenerationResponse {
                content: self.fallback_content.clone(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok(self.fallback_content.clone())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(2);
            let _ = tx
                .send(StreamChunk {
                    text: None,
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stream_error".to_string()),
                })
                .await;
            drop(tx);
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "terminal_error_no_payload_mock"
        }
    }

    struct TerminalErrorAfterPartialProvider {
        generate_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for TerminalErrorAfterPartialProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.generate_calls.fetch_add(1, Ordering::SeqCst);
            Ok(GenerationResponse {
                content: "partial resumed".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("partial-stream".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(3);
            let _ = tx
                .send(StreamChunk {
                    text: Some("partial ".to_string()),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: false,
                    finish_reason: None,
                })
                .await;
            let _ = tx
                .send(StreamChunk {
                    text: None,
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stream_error".to_string()),
                })
                .await;
            drop(tx);
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "terminal_error_after_partial_mock"
        }
    }

    struct ThinkingThenCloseProvider {
        generate_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for ThinkingThenCloseProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.generate_calls.fetch_add(1, Ordering::SeqCst);
            Ok(GenerationResponse {
                content: "final recovered answer".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("thinking-then-close".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(2);
            let _ = tx
                .send(StreamChunk {
                    text: None,
                    thinking: Some("reasoning...".to_string()),
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: false,
                    finish_reason: None,
                })
                .await;
            drop(tx);
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "thinking_then_close_mock"
        }
    }

    struct StreamingGuardrailRetryProvider {
        stream_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for StreamingGuardrailRetryProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            Ok(GenerationResponse {
                content: "should_not_use_generate".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("unused-chat".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let call = self.stream_calls.fetch_add(1, Ordering::SeqCst);
            let (tx, rx) = tokio::sync::mpsc::channel(3);
            let text = if call == 0 {
                "I can't access the internet right now."
            } else {
                "I'll check that using available tools."
            };
            let _ = tx
                .send(StreamChunk {
                    text: Some(text.to_string()),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: false,
                    finish_reason: None,
                })
                .await;
            let _ = tx
                .send(StreamChunk {
                    text: None,
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stop".to_string()),
                })
                .await;
            drop(tx);
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "streaming_guardrail_retry_mock"
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
            Some(vec![ContentPart::text("Test input")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));

        // Check events
        let has_turn_started = events.iter().any(|e| matches!(e, Event::TurnStarted {}));
        let has_turn_completed = events.iter().any(|e| {
            matches!(
                e,
                Event::TurnCompleted {
                    summary: Some(_),
                    ..
                }
            )
        });

        assert!(has_turn_started, "Expected TurnStarted event");
        assert!(has_turn_completed, "Expected TurnCompleted event");
    }

    #[tokio::test]
    async fn test_run_turn_warns_unavailability_claim_when_network_tool_exists() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let provider = SequenceMockProvider::new(
            vec![
                GenerationResponse {
                    content: "I don't have access to real-time weather data.".to_string(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![],
                    usage: None,
                    warnings: Vec::new(),
                },
                GenerationResponse {
                    content: "I'll check that using available tools.".to_string(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![],
                    usage: None,
                    warnings: Vec::new(),
                },
            ],
            Arc::clone(&generate_calls),
        );
        let mut state = create_test_state_with_provider(provider);
        state.tools.register(NetworkCapabilityTool);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("how's the weather today?")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));
        assert_eq!(
            generate_calls.load(Ordering::SeqCst),
            1,
            "Guardrail should not auto-regenerate in non-streaming mode"
        );

        let has_guardrail_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message }
                    if message.contains("Guardrail warning")
                        && message.contains("capability_contradiction")
            )
        });
        assert!(has_guardrail_warning);
        let has_parity_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message }
                    if message.contains("disabled for parity")
            )
        });
        assert!(has_parity_warning);

        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();

        assert!(emitted_text.contains("I don't have access to real-time weather data."));
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
            Some(vec![ContentPart::text("Test input")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());

        // Check for empty response fallback
        let has_fallback = events.iter().any(|e| {
            matches!(e, Event::TurnCompleted { summary } if summary.as_deref() == Some("Turn completed with empty response fallback"))
        });
        assert!(has_fallback, "Expected empty response fallback");

        let assistant_messages: Vec<_> = state
            .session
            .tape
            .messages()
            .iter()
            .filter(|m| matches!(m, crate::session::Message::Assistant { .. }))
            .collect();
        assert_eq!(
            assistant_messages.len(),
            1,
            "Expected fallback assistant message"
        );
        assert_eq!(
            assistant_messages[0].non_thinking_text_content(),
            "I apologize, but I couldn't generate a response."
        );
    }

    #[tokio::test]
    async fn test_run_turn_empty_content_with_thinking_persists_reasoning() {
        let mut state = create_test_state_with_provider(
            ContentMockProvider::new("").with_thinking("internal reasoning"),
        );
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("Test input")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));

        let assistant_messages: Vec<_> = state
            .session
            .tape
            .messages()
            .iter()
            .filter(|m| matches!(m, crate::session::Message::Assistant { .. }))
            .collect();
        assert_eq!(
            assistant_messages.len(),
            1,
            "Expected a single assistant message"
        );
        assert_eq!(
            assistant_messages[0].thinking_content().as_deref(),
            Some("internal reasoning")
        );
        assert_eq!(
            assistant_messages[0].non_thinking_text_content(),
            "I apologize, but I couldn't generate a response."
        );
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn test_run_turn_performs_mid_turn_compaction_before_follow_up_generation() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let provider = SequenceMockProvider::new(
            vec![
                GenerationResponse {
                    content: String::new(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![ToolCall {
                        id: Some("call-mid-turn".to_string()),
                        name: "emit_large_output".to_string(),
                        arguments: json!({}),
                    }],
                    usage: None,
                    warnings: Vec::new(),
                },
                GenerationResponse {
                    content: "Mid-turn compaction summary".to_string(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![],
                    usage: None,
                    warnings: Vec::new(),
                },
                GenerationResponse {
                    content: "Finished after compaction".to_string(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![],
                    usage: None,
                    warnings: Vec::new(),
                },
            ],
            Arc::clone(&generate_calls),
        );
        let mut state = create_test_state_with_provider(provider);
        state
            .tools
            .register(LargeOutputTool::new("very long tool output\n".repeat(600)));
        state.runtime_config.compaction_trigger_messages = 1_000;
        state.runtime_config.compaction_keep_last = 1;
        state.runtime_config.context_window_tokens = 512;
        state.runtime_config.compaction_trigger_ratio = 0.5;

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("Use the tool and continue")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));
        assert_eq!(generate_calls.load(Ordering::SeqCst), 3);
        assert_eq!(
            state.session.tape.summary(),
            Some("Mid-turn compaction summary")
        );
        assert_eq!(state.turn_state.compactions_this_turn(), 1);
        assert!(
            state
                .session
                .tape
                .messages()
                .iter()
                .any(|message| message.text_content().contains("Finished after compaction"))
        );
        assert!(events.iter().any(|event| {
            matches!(
                event,
                Event::TurnCompleted {
                    summary: Some(summary)
                } if summary.contains("Task completed")
            )
        }));
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn test_run_turn_resets_mid_turn_compaction_budget_for_new_turns() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let provider = SequenceMockProvider::new(
            vec![
                GenerationResponse {
                    content: String::new(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![ToolCall {
                        id: Some("call-mid-turn".to_string()),
                        name: "emit_large_output".to_string(),
                        arguments: json!({}),
                    }],
                    usage: None,
                    warnings: Vec::new(),
                },
                GenerationResponse {
                    content: "Mid-turn compaction summary".to_string(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![],
                    usage: None,
                    warnings: Vec::new(),
                },
                GenerationResponse {
                    content: "Finished after compaction".to_string(),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: Vec::new(),
                    tool_calls: vec![],
                    usage: None,
                    warnings: Vec::new(),
                },
            ],
            Arc::clone(&generate_calls),
        );
        let mut state = create_test_state_with_provider(provider);
        state
            .tools
            .register(LargeOutputTool::new("very long tool output\n".repeat(600)));
        state.runtime_config.compaction_trigger_messages = 1_000;
        state.runtime_config.compaction_keep_last = 1;
        state.runtime_config.context_window_tokens = 512;
        state.runtime_config.compaction_trigger_ratio = 0.5;
        state.turn_state.record_auto_mid_turn_compaction(256);
        state.turn_state.record_auto_mid_turn_compaction(512);

        let cancel = CancellationToken::new();
        let mut emit = |_event: Event| async {};
        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("Use the tool and continue")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));
        assert_eq!(generate_calls.load(Ordering::SeqCst), 3);
        assert_eq!(
            state.session.tape.summary(),
            Some("Mid-turn compaction summary")
        );
        assert_eq!(state.turn_state.compactions_this_turn(), 1);
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
            None,
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
            Some(vec![ContentPart::text("Test input")]),
            &mut emit,
            &cancel,
            None,
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
            Some(vec![ContentPart::text("Test input")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());

        // Should report update_plan completion via tool lifecycle event.
        let has_update_plan_completion = events.iter().any(|e| {
            matches!(
                e,
                Event::ToolCallCompleted {
                    id,
                    result_preview: Some(preview),
                    ..
                } if id == "call_1" && preview.contains("plan_updated")
            )
        });
        assert!(
            has_update_plan_completion,
            "Expected ToolCallCompleted preview for update_plan"
        );
    }

    #[tokio::test]
    async fn test_streamed_tool_execution_prefers_final_arguments_over_deltas() {
        let mut state = create_test_state_with_provider(StreamedFinalToolArgumentsProvider {
            stream_calls: Arc::new(AtomicUsize::new(0)),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("Test streamed tool args")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), TurnExecutionOutcome::Finished));

        let has_update_plan_completion = events.iter().any(|event| {
            matches!(
                event,
                Event::ToolCallCompleted {
                    id,
                    result_preview: Some(preview),
                    ..
                } if id == "call_1" && preview.contains("plan_updated")
            )
        });
        assert!(
            has_update_plan_completion,
            "Expected streamed tool execution to use final arguments"
        );

        let dropped_malformed_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message }
                    if message.contains("Dropped malformed streamed tool call")
            )
        });
        assert!(
            !dropped_malformed_warning,
            "Expected final tool arguments to override malformed deltas"
        );
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
            Some(vec![ContentPart::text("Test input")]),
            &mut emit,
            &cancel,
            None,
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
            Some(vec![ContentPart::text("Test input")]),
            &mut emit,
            &cancel,
            None,
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

    #[tokio::test]
    async fn test_stream_end_without_output_falls_back_to_non_streaming() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let mut state = create_test_state_with_provider(StreamEndsImmediatelyProvider {
            fallback_content: "fallback non-stream response".to_string(),
            generate_calls: Arc::clone(&generate_calls),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("Test fallback")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(generate_calls.load(Ordering::SeqCst), 1);
        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert!(emitted_text.contains("fallback non-stream response"));
    }

    #[tokio::test]
    async fn test_partial_stream_attempts_recovery_and_emits_warning() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let mut state = create_test_state_with_provider(PartialStreamThenCloseProvider {
            generate_calls: Arc::clone(&generate_calls),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("Test partial stream")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(generate_calls.load(Ordering::SeqCst), 1);

        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(emitted_text, "partial and recovered response");

        let has_partial_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message }
                    if message.contains("Stream interrupted after partial output")
            )
        });
        assert!(has_partial_warning);
        let has_incomplete_summary = events.iter().any(|event| {
            matches!(
                event,
                Event::TurnCompleted { summary: Some(summary) }
                    if summary.contains("response may be incomplete")
            )
        });
        assert!(has_incomplete_summary);
    }

    #[tokio::test]
    async fn test_partial_stream_recovery_can_be_disabled() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let mut state = create_test_state_with_provider(PartialStreamThenCloseProvider {
            generate_calls: Arc::clone(&generate_calls),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;
        state.runtime_config.partial_stream_recovery_mode =
            crate::config::PartialStreamRecoveryMode::Off;

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text(
                "Test partial stream with recovery off",
            )]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(generate_calls.load(Ordering::SeqCst), 0);
        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(emitted_text, "partial ");
    }

    #[tokio::test]
    async fn test_terminal_stream_error_without_payload_falls_back_to_non_streaming() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let mut state = create_test_state_with_provider(TerminalErrorNoPayloadProvider {
            fallback_content: "fallback from terminal stream error".to_string(),
            generate_calls: Arc::clone(&generate_calls),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text(
                "Test terminal stream error fallback",
            )]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(generate_calls.load(Ordering::SeqCst), 1);
        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert!(emitted_text.contains("fallback from terminal stream error"));
    }

    #[tokio::test]
    async fn test_terminal_stream_error_after_partial_output_preserves_partial_and_warns() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let mut state = create_test_state_with_provider(TerminalErrorAfterPartialProvider {
            generate_calls: Arc::clone(&generate_calls),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text(
                "Test terminal stream error with partial output",
            )]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(generate_calls.load(Ordering::SeqCst), 1);

        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(emitted_text, "partial resumed");

        let has_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message }
                    if message.contains("Stream interrupted after partial output")
                        && message.contains("stream_error")
            )
        });
        assert!(has_warning);
    }

    #[tokio::test]
    async fn test_run_turn_streaming_warns_unavailability_claim_when_network_tool_exists() {
        let stream_calls = Arc::new(AtomicUsize::new(0));
        let mut state = create_test_state_with_provider(StreamingGuardrailRetryProvider {
            stream_calls: Arc::clone(&stream_calls),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;
        state.tools.register(NetworkCapabilityTool);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("how's the weather today?")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(
            stream_calls.load(Ordering::SeqCst),
            1,
            "Guardrail should not auto-regenerate in streaming mode"
        );

        let has_guardrail_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message } if message.contains("Guardrail warning")
            )
        });
        assert!(has_guardrail_warning);
        let has_skip_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message }
                    if message.contains("disabled for parity")
            )
        });
        assert!(has_skip_warning);

        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert!(emitted_text.contains("I can't access the internet right now."));

        let assistant_messages: Vec<_> = state
            .session
            .tape
            .messages()
            .iter()
            .filter(|m| matches!(m, crate::session::Message::Assistant { .. }))
            .collect();
        assert_eq!(assistant_messages.len(), 1);
        assert_eq!(
            assistant_messages[0].non_thinking_text_content(),
            "I can't access the internet right now."
        );
    }

    #[tokio::test]
    async fn test_thinking_only_interruption_is_treated_as_visible_and_recovered() {
        let generate_calls = Arc::new(AtomicUsize::new(0));
        let mut state = create_test_state_with_provider(ThinkingThenCloseProvider {
            generate_calls: Arc::clone(&generate_calls),
        });
        state.runtime_config.streaming_mode = crate::config::StreamingMode::On;

        let cancel = CancellationToken::new();
        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = run_turn_with_cancel(
            &mut state,
            TurnRunKind::NewTurn,
            Some(vec![ContentPart::text("test thinking-only interruption")]),
            &mut emit,
            &cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(generate_calls.load(Ordering::SeqCst), 1);

        let emitted_text = events
            .iter()
            .filter_map(|event| match event {
                Event::TextDelta { chunk, .. } if !chunk.is_empty() => Some(chunk.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(emitted_text, "final recovered answer");

        let has_interruption_warning = events.iter().any(|event| {
            matches!(
                event,
                Event::Warning { message }
                    if message.contains("Stream interrupted after partial output")
            )
        });
        assert!(has_interruption_warning);

        let has_thinking_output = events.iter().any(|event| {
            matches!(
                event,
                Event::ThinkingDelta { chunk, is_final: false } if chunk.contains("reasoning")
            )
        });
        assert!(has_thinking_output);
    }
}
