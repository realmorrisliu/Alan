use alan_protocol::{
    AdaptivePresentationHint, ConfirmationYieldPayload, Event, SpawnHandle, SpawnLaunchInputs,
    SpawnSpec, StructuredInputKind, StructuredInputOption, StructuredInputQuestion,
    StructuredInputYieldPayload, YieldKind,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::approval::{PendingConfirmation, append_skill_permission_hints};
use crate::llm::ToolDefinition;
use crate::skills::{
    DelegatedSkillInvocationRecord, DelegatedSkillResult, DelegatedSkillResultStatus,
};

use super::agent_loop::{NormalizedToolCall, RuntimeLoopState};
use super::child_agents::{
    ChildRuntimeResult, ChildRuntimeStatus, infer_workspace_root_from_memory_dir,
    spawn_child_runtime_cancellable,
};
use super::turn_support::{check_turn_cancelled, tool_result_preview};

const MAX_DELEGATED_SKILL_ID_CHARS: usize = 120;
const MAX_DELEGATED_TARGET_CHARS: usize = 120;
const MAX_DELEGATED_TASK_CHARS: usize = 1_000;
const MAX_DELEGATED_RESULT_SUMMARY_CHARS: usize = 320;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VirtualToolOutcome {
    NotVirtual,
    Continue { refresh_context: bool },
    PauseTurn,
    EndTurn,
}

pub(super) fn virtual_tool_definitions(include_delegated_skill: bool) -> Vec<ToolDefinition> {
    let mut defs = vec![
        request_confirmation_tool_definition(),
        request_user_input_tool_definition(),
        update_plan_tool_definition(),
    ];
    if include_delegated_skill {
        defs.push(invoke_delegated_skill_tool_definition());
    }
    defs
}

pub(super) async fn try_handle_virtual_tool_call<E, F>(
    state: &mut RuntimeLoopState,
    tool_call: &NormalizedToolCall,
    tool_arguments: &serde_json::Value,
    cancel: &CancellationToken,
    emit: &mut E,
) -> Result<VirtualToolOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    if cancel.is_cancelled() && check_turn_cancelled(state, emit, cancel).await? {
        return Ok(VirtualToolOutcome::EndTurn);
    }

    match tool_call.name.as_str() {
        "request_confirmation" => {
            emit(Event::ToolCallStarted {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                audit: None,
            })
            .await;

            if let Some(mut pending) = parse_confirmation_request(&tool_call.id, tool_arguments) {
                pending.details = append_skill_permission_hints(
                    pending.details,
                    state.turn_state.active_skills(),
                );
                let pending_payload = json!({
                    "status": "pending_confirmation",
                    "request_id": pending.checkpoint_id
                });
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&pending_payload),
                    audit: None,
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    pending_payload,
                    true,
                );
                state.turn_state.set_confirmation(pending.clone());
                emit(Event::Yield {
                    request_id: pending.checkpoint_id,
                    kind: alan_protocol::YieldKind::Confirmation,
                    payload: serde_json::to_value(ConfirmationYieldPayload {
                        checkpoint_type: pending.checkpoint_type,
                        summary: pending.summary,
                        details: Some(pending.details),
                        default_option: pending
                            .options
                            .iter()
                            .find(|option| option.as_str() == "approve")
                            .cloned()
                            .or_else(|| pending.options.first().cloned()),
                        options: pending.options,
                        presentation_hints: vec![],
                    })
                    .unwrap_or_else(|_| json!({})),
                })
                .await;
            } else {
                let error_payload = json!({
                    "status": "invalid_request",
                    "error": "Invalid confirmation request."
                });
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&error_payload),
                    audit: None,
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    error_payload,
                    false,
                );
                emit(Event::Error {
                    message: "Invalid confirmation request.".to_string(),
                    recoverable: true,
                })
                .await;
                return Ok(VirtualToolOutcome::EndTurn);
            }
            Ok(VirtualToolOutcome::PauseTurn)
        }
        "request_user_input" => {
            emit(Event::ToolCallStarted {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                audit: None,
            })
            .await;

            if let Some(request) =
                parse_structured_user_input_request(&tool_call.id, tool_arguments)
            {
                let request_id = request.request_id.clone();
                let pending_payload =
                    json!({"status": "pending_structured_input", "request_id": request_id});
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&pending_payload),
                    audit: None,
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    pending_payload,
                    true,
                );
                state.turn_state.set_structured_input(request.clone());
                emit(Event::Yield {
                    request_id: request.request_id,
                    kind: alan_protocol::YieldKind::StructuredInput,
                    payload: serde_json::to_value(structured_input_yield_payload(
                        &state.session.client_capabilities,
                        request.title,
                        request.prompt,
                        request.questions,
                    ))
                    .unwrap_or_else(|_| json!({})),
                })
                .await;
            } else {
                let error_payload = json!({
                    "status": "invalid_request",
                    "error": "Invalid structured user input request."
                });
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&error_payload),
                    audit: None,
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    error_payload,
                    false,
                );
                emit(Event::Error {
                    message: "Invalid structured user input request.".to_string(),
                    recoverable: true,
                })
                .await;
                return Ok(VirtualToolOutcome::EndTurn);
            }
            Ok(VirtualToolOutcome::PauseTurn)
        }
        "update_plan" => {
            emit(Event::ToolCallStarted {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                audit: None,
            })
            .await;
            match parse_plan_update(tool_arguments) {
                Some((explanation, items)) => {
                    state
                        .turn_state
                        .set_plan_snapshot(explanation.clone(), items.clone());
                    let payload = json!({
                        "status": "plan_updated",
                        "explanation": explanation,
                        "items_count": items.len()
                    });
                    emit(Event::ToolCallCompleted {
                        id: tool_call.id.clone(),
                        result_preview: tool_result_preview(&payload),
                        audit: None,
                    })
                    .await;
                    state.session.record_tool_call(
                        &tool_call.name,
                        tool_arguments.clone(),
                        payload.clone(),
                        true,
                    );
                    state
                        .session
                        .add_tool_message(&tool_call.id, &tool_call.name, payload);
                    Ok(VirtualToolOutcome::Continue {
                        refresh_context: true,
                    })
                }
                None => {
                    let error_payload = json!({
                        "status": "invalid_request",
                        "error": "Invalid plan update payload."
                    });
                    emit(Event::ToolCallCompleted {
                        id: tool_call.id.clone(),
                        result_preview: tool_result_preview(&error_payload),
                        audit: None,
                    })
                    .await;
                    state.session.record_tool_call(
                        &tool_call.name,
                        tool_arguments.clone(),
                        error_payload,
                        false,
                    );
                    emit(Event::Error {
                        message: "Invalid plan update payload.".to_string(),
                        recoverable: true,
                    })
                    .await;
                    Ok(VirtualToolOutcome::Continue {
                        refresh_context: false,
                    })
                }
            }
        }
        "invoke_delegated_skill" => {
            // A host may provide a real delegated-execution bridge as a dynamic tool.
            // In that case, do not shadow it with the runtime placeholder branch.
            if state
                .session
                .dynamic_tools
                .contains_key("invoke_delegated_skill")
            {
                return Ok(VirtualToolOutcome::NotVirtual);
            }
            handle_invoke_delegated_skill(
                state,
                tool_call,
                tool_arguments,
                cancel,
                emit,
                |state, spec, cancel| Box::pin(spawn_and_join_delegated_child(state, spec, cancel)),
            )
            .await
        }
        _ => Ok(VirtualToolOutcome::NotVirtual),
    }
}

async fn handle_invoke_delegated_skill<E, F, S>(
    state: &mut RuntimeLoopState,
    tool_call: &NormalizedToolCall,
    tool_arguments: &serde_json::Value,
    cancel: &CancellationToken,
    emit: &mut E,
    spawn_child: S,
) -> Result<VirtualToolOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
    S: for<'a> FnOnce(
        &'a RuntimeLoopState,
        SpawnSpec,
        &'a CancellationToken,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<ChildRuntimeResult>> + Send + 'a>,
    >,
{
    emit(Event::ToolCallStarted {
        id: tool_call.id.clone(),
        name: tool_call.name.clone(),
        audit: None,
    })
    .await;

    let Some(request) = parse_delegated_skill_invocation_request(tool_arguments) else {
        let error_payload = json!({
            "status": "invalid_request",
            "error": "Invalid delegated skill invocation payload."
        });
        emit(Event::ToolCallCompleted {
            id: tool_call.id.clone(),
            result_preview: tool_result_preview(&error_payload),
            audit: None,
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
            .add_tool_message(&tool_call.id, &tool_call.name, error_payload.clone());
        emit(Event::Error {
            message: "Invalid delegated skill invocation payload.".to_string(),
            recoverable: true,
        })
        .await;
        return Ok(VirtualToolOutcome::Continue {
            refresh_context: true,
        });
    };

    let (result, child_run) = match resolve_delegated_skill_invocation(state, &request) {
        Ok(spec) => match spawn_child(state, spec, cancel).await {
            Ok(child_result) => {
                if cancel.is_cancelled()
                    && matches!(child_result.status, ChildRuntimeStatus::Cancelled)
                    && check_turn_cancelled(state, emit, cancel).await?
                {
                    return Ok(VirtualToolOutcome::EndTurn);
                }

                (
                    delegated_result_from_child_result(&child_result),
                    Some(delegated_child_run_reference(&child_result)),
                )
            }
            Err(err) => {
                if cancel.is_cancelled() && check_turn_cancelled(state, emit, cancel).await? {
                    return Ok(VirtualToolOutcome::EndTurn);
                }

                (
                    DelegatedSkillResult::failed(
                        format!(
                            "Failed to launch delegated child agent for skill '{}': {err}",
                            request.skill_id
                        ),
                        Some(json!({
                            "error_kind": "child_launch_failed"
                        })),
                    ),
                    None,
                )
            }
        },
        Err(result) => (result, None),
    };

    let (persisted_arguments, tape_record, rollout_record) =
        build_bounded_delegated_invocation_persistence(&request, result, child_run);
    let preview = tool_result_preview(&json!(tape_record.result.summary.clone()));
    let tape_payload = serde_json::to_value(&tape_record).unwrap_or_else(|_| {
        json!({
            "status": "invalid_result_encoding",
            "error": "Failed to serialize delegated skill result."
        })
    });
    let rollout_payload =
        serde_json::to_value(&rollout_record).unwrap_or_else(|_| tape_payload.clone());
    emit(Event::ToolCallCompleted {
        id: tool_call.id.clone(),
        result_preview: preview,
        audit: None,
    })
    .await;
    let invocation_succeeded = matches!(
        tape_record.result.status,
        DelegatedSkillResultStatus::Completed
    );
    state.session.record_tool_call(
        &tool_call.name,
        persisted_arguments,
        rollout_payload,
        invocation_succeeded,
    );
    state
        .session
        .add_tool_message(&tool_call.id, &tool_call.name, tape_payload);
    Ok(VirtualToolOutcome::Continue {
        refresh_context: true,
    })
}

async fn spawn_and_join_delegated_child(
    state: &RuntimeLoopState,
    spec: SpawnSpec,
    cancel: &CancellationToken,
) -> Result<ChildRuntimeResult> {
    if cancel.is_cancelled() {
        return Ok(ChildRuntimeResult {
            status: ChildRuntimeStatus::Cancelled,
            session_id: String::new(),
            rollout_path: None,
            output_text: String::new(),
            turn_summary: None,
            warnings: Vec::new(),
            error_message: None,
            pause: None,
        });
    }

    let controller = spawn_child_runtime_cancellable(state, spec, cancel).await?;
    controller.join_until_cancelled(cancel).await
}

fn resolve_delegated_skill_invocation(
    state: &RuntimeLoopState,
    request: &DelegatedSkillInvocationRequest,
) -> std::result::Result<SpawnSpec, DelegatedSkillResult> {
    let Some(skill) = state
        .turn_state
        .active_skills()
        .iter()
        .find(|skill| skill.metadata.id == request.skill_id)
    else {
        return Err(DelegatedSkillResult::failed(
            format!(
                "Delegated skill '{}' is not active in the current turn.",
                request.skill_id
            ),
            Some(json!({
                "error_kind": "skill_not_active"
            })),
        ));
    };

    if !skill.availability.is_available() {
        return Err(DelegatedSkillResult::failed(
            format!(
                "Delegated skill '{}' is {}.",
                request.skill_id,
                skill.availability.render_label()
            ),
            Some(json!({
                "error_kind": "skill_unavailable"
            })),
        ));
    }

    let Some(resolved_target) = skill.metadata.execution.delegate_target() else {
        return Err(DelegatedSkillResult::failed(
            format!(
                "Skill '{}' is not resolved for delegated execution.",
                request.skill_id
            ),
            Some(json!({
                "error_kind": "skill_not_delegated"
            })),
        ));
    };

    if resolved_target != request.target {
        return Err(DelegatedSkillResult::failed(
            format!(
                "Delegated skill '{}' resolves to child agent '{}' rather than '{}'.",
                request.skill_id, resolved_target, request.target
            ),
            Some(json!({
                "error_kind": "delegate_target_mismatch",
                "resolved_target": resolved_target
            })),
        ));
    }

    let Some(spawn_target) = skill.metadata.delegated_spawn_target() else {
        return Err(DelegatedSkillResult::failed(
            format!(
                "Delegated skill '{}' does not expose a package-local child-agent target.",
                request.skill_id
            ),
            Some(json!({
                "error_kind": "delegate_target_missing"
            })),
        ));
    };

    Ok(build_delegated_spawn_spec(state, request, spawn_target))
}

fn build_delegated_spawn_spec(
    state: &RuntimeLoopState,
    request: &DelegatedSkillInvocationRequest,
    target: alan_protocol::SpawnTarget,
) -> SpawnSpec {
    let cwd = state.tools.default_cwd();
    let workspace_root =
        infer_workspace_root_from_memory_dir(state.core_config.memory.workspace_dir.as_deref());
    let timeout_secs = (state.core_config.tool_timeout_secs > 0)
        .then_some(state.core_config.tool_timeout_secs as u64);

    SpawnSpec {
        target,
        launch: SpawnLaunchInputs {
            task: request.task.clone(),
            cwd,
            workspace_root,
            timeout_secs,
            budget_tokens: None,
            output_dir: None,
        },
        handles: vec![SpawnHandle::Workspace, SpawnHandle::ApprovalScope],
        runtime_overrides: Default::default(),
    }
}

fn delegated_result_from_child_result(result: &ChildRuntimeResult) -> DelegatedSkillResult {
    match result.status {
        ChildRuntimeStatus::Completed => delegated_result_from_completed_child(result),
        ChildRuntimeStatus::Failed => DelegatedSkillResult::failed(
            format!(
                "Delegated child agent failed: {}",
                result
                    .error_message
                    .clone()
                    .or_else(|| non_empty_trimmed(&result.output_text))
                    .unwrap_or_else(|| "unknown failure".to_string())
            ),
            Some(json!({
                "error_kind": "child_failed"
            })),
        ),
        ChildRuntimeStatus::TimedOut => DelegatedSkillResult::failed(
            "Delegated child agent timed out.".to_string(),
            Some(json!({
                "error_kind": "child_timed_out"
            })),
        ),
        ChildRuntimeStatus::Cancelled => DelegatedSkillResult::failed(
            "Delegated child agent was cancelled.".to_string(),
            Some(json!({
                "error_kind": "child_cancelled"
            })),
        ),
        ChildRuntimeStatus::Paused => {
            let (pause_kind, request_id) = result
                .pause
                .as_ref()
                .map(|pause| {
                    (
                        yield_kind_label(&pause.kind),
                        Some(pause.request_id.clone()),
                    )
                })
                .unwrap_or_else(|| ("unknown".to_string(), None));
            DelegatedSkillResult::failed(
                format!(
                    "Delegated child agent paused for {} and cannot continue in v1 delegated execution.",
                    pause_kind
                ),
                Some(json!({
                    "error_kind": "child_paused",
                    "pause_kind": pause_kind,
                    "request_id": request_id
                })),
            )
        }
    }
}

fn delegated_result_from_completed_child(result: &ChildRuntimeResult) -> DelegatedSkillResult {
    DelegatedSkillResult::completed(completed_child_summary(result), None)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DelegatedChildRunReference {
    session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rollout_path: Option<PathBuf>,
    terminal_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DelegatedSkillRolloutRecord {
    #[serde(flatten)]
    invocation: DelegatedSkillInvocationRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    child_run: Option<DelegatedChildRunReference>,
}

fn delegated_child_run_reference(result: &ChildRuntimeResult) -> DelegatedChildRunReference {
    DelegatedChildRunReference {
        session_id: result.session_id.clone(),
        rollout_path: result.rollout_path.clone(),
        terminal_status: child_runtime_status_label(result.status.clone()),
    }
}

fn child_runtime_status_label(status: ChildRuntimeStatus) -> String {
    match status {
        ChildRuntimeStatus::Completed => "completed".to_string(),
        ChildRuntimeStatus::Paused => "paused".to_string(),
        ChildRuntimeStatus::Cancelled => "cancelled".to_string(),
        ChildRuntimeStatus::TimedOut => "timed_out".to_string(),
        ChildRuntimeStatus::Failed => "failed".to_string(),
    }
}

fn build_bounded_delegated_invocation_persistence(
    request: &DelegatedSkillInvocationRequest,
    result: DelegatedSkillResult,
    child_run: Option<DelegatedChildRunReference>,
) -> (
    serde_json::Value,
    DelegatedSkillInvocationRecord,
    DelegatedSkillRolloutRecord,
) {
    let (arguments, record) = build_bounded_delegated_tape_record(request, result);
    let rollout_record = DelegatedSkillRolloutRecord {
        invocation: record.clone(),
        child_run,
    };
    (arguments, record, rollout_record)
}

fn build_bounded_delegated_tape_record(
    request: &DelegatedSkillInvocationRequest,
    result: DelegatedSkillResult,
) -> (serde_json::Value, DelegatedSkillInvocationRecord) {
    let skill_id =
        truncate_text_with_suffix(&request.skill_id, MAX_DELEGATED_SKILL_ID_CHARS, "...");
    let target = truncate_text_with_suffix(&request.target, MAX_DELEGATED_TARGET_CHARS, "...");
    let task = truncate_text_with_suffix(&request.task, MAX_DELEGATED_TASK_CHARS, "...");
    let result = DelegatedSkillResult {
        summary: truncate_text_with_suffix(
            &result.summary,
            MAX_DELEGATED_RESULT_SUMMARY_CHARS,
            "...",
        ),
        ..result
    };

    let record = DelegatedSkillInvocationRecord {
        skill_id,
        target,
        task,
        result,
    };
    let arguments = json!({
        "skill_id": record.skill_id.clone(),
        "target": record.target.clone(),
        "task": record.task.clone(),
    });

    (arguments, record)
}

fn completed_child_summary(result: &ChildRuntimeResult) -> String {
    non_empty_trimmed(result.turn_summary.as_deref().unwrap_or_default())
        .or_else(|| non_empty_trimmed(&result.output_text))
        .unwrap_or_else(|| "Delegated child agent completed without textual output.".to_string())
}

fn non_empty_trimmed(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn yield_kind_label(kind: &YieldKind) -> String {
    match kind {
        YieldKind::Confirmation => "confirmation".to_string(),
        YieldKind::StructuredInput => "structured_input".to_string(),
        YieldKind::DynamicTool => "dynamic_tool".to_string(),
        YieldKind::Custom(kind) => kind.clone(),
    }
}

pub(super) fn parse_confirmation_request(
    tool_call_id: &str,
    args: &serde_json::Value,
) -> Option<PendingConfirmation> {
    let checkpoint_type = args
        .get("checkpoint_type")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("confirmation")
        .to_string();
    let summary = args.get("summary")?.as_str()?.trim().to_string();
    if summary.is_empty() {
        return None;
    }
    let details = args.get("details").cloned().unwrap_or(json!({}));
    let options = args
        .get("options")
        .and_then(|o| o.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                })
                .collect()
        })
        .filter(|opts: &Vec<String>| !opts.is_empty())
        .unwrap_or_else(|| {
            vec![
                "approve".to_string(),
                "modify".to_string(),
                "reject".to_string(),
            ]
        });

    Some(PendingConfirmation {
        checkpoint_id: tool_call_id.to_string(),
        checkpoint_type,
        summary,
        details,
        options,
    })
}

fn parse_structured_user_input_request(
    tool_call_id: &str,
    arguments: &serde_json::Value,
) -> Option<crate::approval::PendingStructuredInputRequest> {
    let title = arguments.get("title")?.as_str()?.trim().to_string();
    let prompt = arguments.get("prompt")?.as_str()?.trim().to_string();
    if title.is_empty() || prompt.is_empty() {
        return None;
    }
    let request_id = tool_call_id.to_string();

    let questions = arguments
        .get("questions")?
        .as_array()?
        .iter()
        .filter_map(|raw| {
            let id = parse_non_empty_string(raw.get("id"))?;
            let label = parse_non_empty_string(raw.get("label"))?;
            let prompt = parse_non_empty_string(raw.get("prompt"))?;
            let required = raw
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let options = raw
                .get("options")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|opt| {
                            Some(StructuredInputOption {
                                value: parse_non_empty_string(opt.get("value"))?,
                                label: parse_non_empty_string(opt.get("label"))?,
                                description: parse_optional_string(opt.get("description")),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let kind = parse_structured_input_kind(raw.get("kind"), !options.is_empty())?;
            let placeholder = parse_optional_string(raw.get("placeholder"));
            let help_text = parse_optional_string(raw.get("help_text"));
            let default_value = parse_optional_string(raw.get("default"));
            let default_values = parse_string_array(raw.get("defaults"));
            let min_selected = parse_optional_u32(raw.get("min_selected"));
            let max_selected = parse_optional_u32(raw.get("max_selected"));
            let presentation_hints = parse_presentation_hints(raw.get("presentation_hints"));
            let options = normalize_question_options(kind, options);

            if matches!(
                kind,
                StructuredInputKind::Boolean
                    | StructuredInputKind::SingleSelect
                    | StructuredInputKind::MultiSelect
            ) && options.is_empty()
            {
                return None;
            }

            let option_values = options
                .iter()
                .map(|opt| opt.value.as_str())
                .collect::<Vec<_>>();
            let normalized_default_value = match kind {
                StructuredInputKind::Text
                | StructuredInputKind::Number
                | StructuredInputKind::Integer => default_value.clone(),
                StructuredInputKind::Boolean | StructuredInputKind::SingleSelect => {
                    normalize_single_default(default_value.clone(), option_values.as_slice())
                }
                StructuredInputKind::MultiSelect => None,
            };
            let normalized_default_values = if matches!(kind, StructuredInputKind::MultiSelect) {
                normalize_multi_defaults(
                    default_value.as_deref(),
                    default_values,
                    option_values.as_slice(),
                )
            } else {
                Vec::new()
            };
            let (min_selected, max_selected) =
                normalize_selection_constraints(min_selected, max_selected, options.len());

            Some(StructuredInputQuestion {
                id,
                label,
                prompt,
                kind,
                required,
                placeholder,
                help_text,
                default_value: normalized_default_value,
                default_values: normalized_default_values,
                min_selected: if matches!(kind, StructuredInputKind::MultiSelect) {
                    min_selected
                } else {
                    None
                },
                max_selected: if matches!(kind, StructuredInputKind::MultiSelect) {
                    max_selected
                } else {
                    None
                },
                options,
                presentation_hints,
            })
        })
        .collect::<Vec<_>>();

    if questions.is_empty() {
        return None;
    }

    Some(crate::approval::PendingStructuredInputRequest {
        request_id,
        title,
        prompt,
        questions,
    })
}

fn parse_non_empty_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(|raw| raw.as_str())
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(ToString::to_string)
}

fn parse_optional_string(value: Option<&serde_json::Value>) -> Option<String> {
    parse_non_empty_string(value)
}

fn parse_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|raw| raw.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parse_non_empty_string(Some(item)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_optional_u32(value: Option<&serde_json::Value>) -> Option<u32> {
    value
        .and_then(|raw| raw.as_u64())
        .and_then(|raw| u32::try_from(raw).ok())
}

fn parse_structured_input_kind(
    value: Option<&serde_json::Value>,
    has_options: bool,
) -> Option<StructuredInputKind> {
    match value.and_then(|raw| raw.as_str()) {
        Some("text") => Some(StructuredInputKind::Text),
        Some("boolean") => Some(StructuredInputKind::Boolean),
        Some("number") => Some(StructuredInputKind::Number),
        Some("integer") => Some(StructuredInputKind::Integer),
        Some("single_select") => Some(StructuredInputKind::SingleSelect),
        Some("multi_select") => Some(StructuredInputKind::MultiSelect),
        Some(_) => None,
        None => Some(if has_options {
            StructuredInputKind::SingleSelect
        } else {
            StructuredInputKind::Text
        }),
    }
}

fn parse_presentation_hints(value: Option<&serde_json::Value>) -> Vec<AdaptivePresentationHint> {
    value
        .and_then(|raw| raw.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item.as_str() {
                    Some("radio") => Some(AdaptivePresentationHint::Radio),
                    Some("toggle") => Some(AdaptivePresentationHint::Toggle),
                    Some("searchable") => Some(AdaptivePresentationHint::Searchable),
                    Some("multiline") => Some(AdaptivePresentationHint::Multiline),
                    Some("compact") => Some(AdaptivePresentationHint::Compact),
                    Some("dangerous") => Some(AdaptivePresentationHint::Dangerous),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_question_options(
    kind: StructuredInputKind,
    options: Vec<StructuredInputOption>,
) -> Vec<StructuredInputOption> {
    if matches!(kind, StructuredInputKind::Boolean) && options.is_empty() {
        return boolean_options();
    }
    options
}

fn boolean_options() -> Vec<StructuredInputOption> {
    vec![
        StructuredInputOption {
            value: "true".to_string(),
            label: "Yes".to_string(),
            description: None,
        },
        StructuredInputOption {
            value: "false".to_string(),
            label: "No".to_string(),
            description: None,
        },
    ]
}

fn structured_input_yield_payload(
    capabilities: &alan_protocol::ClientCapabilities,
    title: String,
    prompt: String,
    questions: Vec<StructuredInputQuestion>,
) -> StructuredInputYieldPayload {
    let questions = if capabilities.adaptive_yields.presentation_hints {
        questions
    } else {
        questions
            .into_iter()
            .map(|mut question| {
                question.presentation_hints.clear();
                question
            })
            .collect()
    };

    StructuredInputYieldPayload {
        title,
        prompt: Some(prompt),
        questions,
    }
}

fn normalize_single_default(
    default_value: Option<String>,
    option_values: &[&str],
) -> Option<String> {
    default_value
        .filter(|value| option_values.is_empty() || option_values.contains(&value.as_str()))
}

fn normalize_multi_defaults(
    default_value: Option<&str>,
    default_values: Vec<String>,
    option_values: &[&str],
) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in default_values {
        if option_values.contains(&value.as_str()) && !normalized.contains(&value) {
            normalized.push(value);
        }
    }

    if normalized.is_empty()
        && let Some(value) = default_value
        && option_values.contains(&value)
    {
        normalized.push(value.to_string());
    }

    normalized
}

fn normalize_selection_constraints(
    min_selected: Option<u32>,
    max_selected: Option<u32>,
    option_count: usize,
) -> (Option<u32>, Option<u32>) {
    let option_limit = u32::try_from(option_count).ok();
    let min = min_selected.filter(|value| Some(*value) <= option_limit);
    let max = max_selected.filter(|value| Some(*value) <= option_limit);

    match (min, max) {
        (Some(min), Some(max)) if max < min => (Some(min), None),
        other => other,
    }
}

fn parse_plan_status(raw: &str) -> Option<alan_protocol::PlanItemStatus> {
    match raw {
        "pending" | "blocked" => Some(alan_protocol::PlanItemStatus::Pending),
        "in_progress" => Some(alan_protocol::PlanItemStatus::InProgress),
        "completed" | "skipped" => Some(alan_protocol::PlanItemStatus::Completed),
        _ => None,
    }
}

fn parse_plan_items(value: &serde_json::Value) -> Option<Vec<alan_protocol::PlanItem>> {
    let items = value.as_array()?;
    let parsed = items
        .iter()
        .filter_map(|raw| {
            let id = raw.get("id")?.as_str()?.to_string();
            let content = raw
                .get("content")
                .or_else(|| raw.get("description"))?
                .as_str()?
                .to_string();
            let status_raw = raw.get("status")?.as_str()?;
            let status = parse_plan_status(status_raw)?;
            Some(alan_protocol::PlanItem {
                id,
                content,
                status,
            })
        })
        .collect::<Vec<_>>();
    (!parsed.is_empty()).then_some(parsed)
}

fn parse_plan_update(
    arguments: &serde_json::Value,
) -> Option<(Option<String>, Vec<alan_protocol::PlanItem>)> {
    let explanation = arguments
        .get("explanation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let items = parse_plan_items(arguments.get("items")?)?;
    Some((explanation, items))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DelegatedSkillInvocationRequest {
    skill_id: String,
    target: String,
    task: String,
}

fn parse_delegated_skill_invocation_request(
    arguments: &serde_json::Value,
) -> Option<DelegatedSkillInvocationRequest> {
    let skill_id = arguments.get("skill_id")?.as_str()?.trim().to_string();
    let target = arguments.get("target")?.as_str()?.trim().to_string();
    let task = arguments.get("task")?.as_str()?.trim().to_string();
    if skill_id.is_empty() || target.is_empty() || task.is_empty() {
        return None;
    }
    Some(DelegatedSkillInvocationRequest {
        skill_id,
        target,
        task,
    })
}

fn truncate_text_with_suffix(text: &str, max_chars: usize, suffix: &str) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let suffix_len = suffix.chars().count();
    if max_chars <= suffix_len {
        return suffix.chars().take(max_chars).collect();
    }

    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(suffix_len))
        .collect::<String>();
    truncated.push_str(suffix);
    truncated
}

fn request_confirmation_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "request_confirmation".to_string(),
        description: "Request user confirmation or approval before proceeding with a significant action. Use this when you need explicit user approval before making changes or proceeding with a recommendation.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "checkpoint_id": {
                    "type": "string",
                    "description": "Optional legacy field. Runtime uses the tool call id as request_id."
                },
                "checkpoint_type": {
                    "type": "string",
                    "description": "The type of checkpoint (e.g., 'business_understanding', 'supplier_recommendation', 'final_confirmation'). Defaults to 'confirmation'."
                },
                "summary": {
                    "type": "string",
                    "description": "A clear summary of what is being proposed or what the user should confirm"
                },
                "details": {
                    "type": "object",
                    "description": "Additional structured details relevant to the confirmation"
                }
            },
            "required": ["summary"]
        }),
    }
}

fn request_user_input_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "request_user_input".to_string(),
        description: "Request structured user input (questions/options) from the client UI and wait for a structured response before continuing.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "request_id": {
                    "type": "string",
                    "description": "Optional legacy field. Runtime uses the tool call id as request_id."
                },
                "title": { "type": "string" },
                "prompt": { "type": "string" },
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "label": { "type": "string" },
                            "prompt": { "type": "string" },
                            "kind": {
                                "type": "string",
                                "enum": ["text", "boolean", "number", "integer", "single_select", "multi_select"]
                            },
                            "required": { "type": "boolean" },
                            "placeholder": { "type": "string" },
                            "help_text": { "type": "string" },
                            "presentation_hints": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "enum": ["radio", "toggle", "searchable", "multiline", "compact", "dangerous"]
                                }
                            },
                            "default": { "type": "string" },
                            "defaults": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "min_selected": { "type": "integer", "minimum": 0 },
                            "max_selected": { "type": "integer", "minimum": 0 },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "value": { "type": "string" },
                                        "label": { "type": "string" },
                                        "description": { "type": "string" }
                                    },
                                    "required": ["value", "label"]
                                }
                            }
                        },
                        "required": ["id", "label", "prompt"]
                    }
                }
            },
            "required": ["title", "prompt", "questions"]
        }),
    }
}

fn update_plan_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "update_plan".to_string(),
        description: "Publish a normalized plan/progress update to the client UI. Use this when the task plan changes or step status changes.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "explanation": { "type": "string" },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "content": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                        },
                        "required": ["id", "content", "status"]
                    }
                }
            },
            "required": ["items"]
        }),
    }
}

fn invoke_delegated_skill_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "invoke_delegated_skill".to_string(),
        description: "Invoke a delegated skill through Alan's runtime-owned child-agent path. Use this for delegated skills listed in the skills catalog or in active-skill runtime context.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "Resolved delegated skill id exposed in the skills catalog or active-skill runtime context.",
                    "maxLength": MAX_DELEGATED_SKILL_ID_CHARS
                },
                "target": {
                    "type": "string",
                    "description": "Resolved package-local child-agent export target for this delegated skill.",
                    "maxLength": MAX_DELEGATED_TARGET_CHARS
                },
                "task": {
                    "type": "string",
                    "description": "A concise bounded task for the delegated child agent.",
                    "maxLength": MAX_DELEGATED_TASK_CHARS
                }
            },
            "required": ["skill_id", "target", "task"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        llm::LlmClient,
        rollout::{RolloutItem, RolloutRecorder},
        runtime::{RuntimeConfig, TurnState, turn_state::TurnActivityState},
        session::Session,
        skills::{
            ActiveSkillEnvelope, ResolvedSkillExecution, SkillActivationReason,
            SkillExecutionResolutionSource, SkillMetadata, SkillScope,
        },
        tools::ToolRegistry,
    };
    use alan_llm::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk};
    use async_trait::async_trait;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

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
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
                warnings: Vec::new(),
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

    fn create_test_agent_loop_state() -> super::super::agent_loop::RuntimeLoopState {
        let config = Config::default();
        let session = Session::new();
        let mut tools = ToolRegistry::new();
        tools.set_default_cwd(PathBuf::from("/tmp/alan-delegated-parent"));
        let runtime_config = RuntimeConfig::default();

        super::super::agent_loop::RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            current_submission_id: None,
            llm_client: LlmClient::new(SimpleMockProvider),
            tools,
            core_config: config,
            runtime_config,
            workspace_persona_dirs: Vec::new(),
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(Vec::new()),
            turn_state: TurnState::default(),
        }
    }

    fn delegated_test_skill_metadata(skill_id: &str, target: &str) -> SkillMetadata {
        SkillMetadata {
            id: skill_id.to_string(),
            package_id: Some(format!("skill:{skill_id}")),
            name: skill_id.to_string(),
            description: format!("Delegated test skill {skill_id}"),
            short_description: None,
            path: PathBuf::from(format!("/tmp/{skill_id}/SKILL.md")),
            package_root: Some(PathBuf::from(format!("/tmp/{skill_id}"))),
            resource_root: Some(PathBuf::from(format!("/tmp/{skill_id}"))),
            scope: SkillScope::Repo,
            tags: Vec::new(),
            capabilities: None,
            compatibility: Default::default(),
            source: Default::default(),
            enabled: true,
            allow_implicit_invocation: true,
            alan_metadata: Default::default(),
            compatible_metadata: Default::default(),
            execution: ResolvedSkillExecution::Delegate {
                target: target.to_string(),
                source: SkillExecutionResolutionSource::ExplicitMetadata,
            },
        }
    }

    fn activate_test_delegated_skill(
        state: &mut super::super::agent_loop::RuntimeLoopState,
        skill_id: &str,
        target: &str,
    ) {
        state
            .turn_state
            .set_active_skills(vec![ActiveSkillEnvelope::available(
                delegated_test_skill_metadata(skill_id, target),
                SkillActivationReason::ExplicitMention {
                    mention: skill_id.to_string(),
                },
            )]);
    }

    async fn try_handle_virtual_tool_call_for_test<E, F>(
        state: &mut super::super::agent_loop::RuntimeLoopState,
        tool_call: &NormalizedToolCall,
        emit: &mut E,
    ) -> Result<VirtualToolOutcome>
    where
        E: FnMut(Event) -> F,
        F: std::future::Future<Output = ()>,
    {
        let cancel = CancellationToken::new();
        try_handle_virtual_tool_call(state, tool_call, &tool_call.arguments, &cancel, emit).await
    }

    #[test]
    fn test_virtual_tool_definitions_include_all_runtime_virtual_tools() {
        let defs = virtual_tool_definitions(false);
        assert_eq!(defs.len(), 3);
        assert!(defs.iter().any(|d| d.name == "request_confirmation"));
        assert!(defs.iter().any(|d| d.name == "request_user_input"));
        assert!(defs.iter().any(|d| d.name == "update_plan"));
        assert!(!defs.iter().any(|d| d.name == "invoke_delegated_skill"));
    }

    #[test]
    fn test_virtual_tool_definitions_can_include_delegated_skill() {
        let defs = virtual_tool_definitions(true);
        assert!(defs.iter().any(|d| d.name == "invoke_delegated_skill"));
    }

    #[test]
    fn test_request_confirmation_tool_definition_schema_shape() {
        let def = request_confirmation_tool_definition();
        assert_eq!(def.name, "request_confirmation");
        assert!(def.description.contains("confirmation"));
        assert_eq!(def.parameters["type"], "object");
        assert_eq!(
            def.parameters["properties"]["checkpoint_id"]["type"],
            "string"
        );
        assert_eq!(
            def.parameters["properties"]["checkpoint_type"]["type"],
            "string"
        );
        assert_eq!(def.parameters["properties"]["summary"]["type"], "string");
        assert_eq!(def.parameters["properties"]["details"]["type"], "object");
    }

    #[test]
    fn test_request_user_input_tool_definition() {
        let def = request_user_input_tool_definition();
        assert_eq!(def.name, "request_user_input");
        assert!(def.description.contains("structured"));
        assert_eq!(def.parameters["type"], "object");
        assert!(def.parameters["properties"].get("title").is_some());
        assert!(def.parameters["properties"].get("prompt").is_some());
        assert!(def.parameters["properties"].get("questions").is_some());
        assert_eq!(
            def.parameters["properties"]["questions"]["items"]["properties"]["kind"]["enum"],
            json!([
                "text",
                "boolean",
                "number",
                "integer",
                "single_select",
                "multi_select"
            ])
        );
    }

    #[test]
    fn test_update_plan_tool_definition() {
        let def = update_plan_tool_definition();
        assert_eq!(def.name, "update_plan");
        assert!(def.description.contains("plan"));
        assert_eq!(def.parameters["type"], "object");
        assert!(def.parameters["properties"].get("explanation").is_some());
        assert!(def.parameters["properties"].get("items").is_some());
    }

    #[test]
    fn test_invoke_delegated_skill_tool_definition() {
        let def = invoke_delegated_skill_tool_definition();
        assert_eq!(def.name, "invoke_delegated_skill");
        assert!(def.description.contains("delegated skill"));
        assert_eq!(def.parameters["type"], "object");
        assert_eq!(def.parameters["properties"]["skill_id"]["type"], "string");
        assert_eq!(
            def.parameters["properties"]["skill_id"]["maxLength"],
            MAX_DELEGATED_SKILL_ID_CHARS
        );
        assert_eq!(def.parameters["properties"]["target"]["type"], "string");
        assert_eq!(
            def.parameters["properties"]["target"]["maxLength"],
            MAX_DELEGATED_TARGET_CHARS
        );
        assert_eq!(def.parameters["properties"]["task"]["type"], "string");
        assert_eq!(
            def.parameters["properties"]["task"]["maxLength"],
            MAX_DELEGATED_TASK_CHARS
        );
    }

    // Tests for parse_confirmation_request
    #[test]
    fn test_parse_confirmation_request_valid() {
        let args = json!({
            "checkpoint_type": "test_type",
            "summary": "Test summary",
            "details": {"key": "value"},
            "options": ["approve", "reject"]
        });

        let result = parse_confirmation_request("call_1", &args);
        assert!(result.is_some());

        let pending = result.unwrap();
        assert_eq!(pending.checkpoint_id, "call_1");
        assert_eq!(pending.checkpoint_type, "test_type");
        assert_eq!(pending.summary, "Test summary");
        assert_eq!(pending.options, vec!["approve", "reject"]);
    }

    #[test]
    fn test_parse_confirmation_request_default_options() {
        let args = json!({
            "checkpoint_type": "test_type",
            "summary": "Test summary"
        });

        let result = parse_confirmation_request("call_1", &args);
        assert!(result.is_some());

        let pending = result.unwrap();
        assert_eq!(pending.checkpoint_id, "call_1");
        assert_eq!(pending.options, vec!["approve", "modify", "reject"]);
    }

    #[test]
    fn test_parse_confirmation_request_missing_required() {
        // Missing summary
        let args = json!({
            "checkpoint_type": "test_type",
            "details": {"k": "v"}
        });
        assert!(parse_confirmation_request("call_1", &args).is_none());

        // Missing checkpoint_type falls back to default
        let args = json!({
            "summary": "Test summary"
        });
        let parsed = parse_confirmation_request("call_1", &args).unwrap();
        assert_eq!(parsed.checkpoint_type, "confirmation");
    }

    #[test]
    fn test_parse_confirmation_request_non_string_fields() {
        // summary must be a non-empty string
        let args = json!({
            "checkpoint_type": "test_type",
            "summary": 123
        });
        assert!(parse_confirmation_request("call_1", &args).is_none());
    }

    // Tests for parse_structured_user_input_request
    #[test]
    fn test_parse_structured_user_input_request_valid() {
        let args = json!({
            "title": "Test Title",
            "prompt": "Test Prompt",
            "questions": [
                {
                    "id": "q1",
                    "label": "Question 1",
                    "prompt": "What is your name?",
                    "required": true
                }
            ]
        });

        let result = parse_structured_user_input_request("call_1", &args);
        assert!(result.is_some());

        let request = result.unwrap();
        assert_eq!(request.title, "Test Title");
        assert_eq!(request.prompt, "Test Prompt");
        assert_eq!(request.questions.len(), 1);
        assert_eq!(request.questions[0].id, "q1");
        assert_eq!(
            request.questions[0].kind,
            alan_protocol::StructuredInputKind::Text
        );
        assert!(request.questions[0].required);
    }

    #[test]
    fn test_parse_structured_user_input_request_with_options() {
        let args = json!({
            "title": "Test",
            "prompt": "Prompt",
            "questions": [
                {
                    "id": "q1",
                    "label": "Label",
                    "prompt": "Prompt?",
                    "required": false,
                    "options": [
                        {"value": "yes", "label": "Yes", "description": "Yes option"}
                    ]
                }
            ]
        });

        let result = parse_structured_user_input_request("call_1", &args);
        assert!(result.is_some());

        let request = result.unwrap();
        assert_eq!(
            request.questions[0].kind,
            alan_protocol::StructuredInputKind::SingleSelect
        );
        assert_eq!(request.questions[0].options.len(), 1);
        assert_eq!(request.questions[0].options[0].value, "yes");
        assert_eq!(request.questions[0].options[0].label, "Yes");
    }

    #[test]
    fn test_parse_structured_user_input_request_with_explicit_metadata() {
        let args = json!({
            "title": "Deployment settings",
            "prompt": "Review and adjust the requested values.",
            "questions": [
                {
                    "id": "branch",
                    "label": "Branch",
                    "prompt": "Branch name",
                    "kind": "text",
                    "required": true,
                    "placeholder": "feature/adaptive-yield-ui",
                    "help_text": "Use the exact git ref that should be deployed.",
                    "default": "main"
                },
                {
                    "id": "envs",
                    "label": "Environments",
                    "prompt": "Pick deployment targets",
                    "kind": "multi_select",
                    "options": [
                        {"value": "staging", "label": "Staging"},
                        {"value": "prod", "label": "Production"}
                    ],
                    "defaults": ["prod", "staging", "prod"],
                    "min_selected": 1,
                    "max_selected": 2
                }
            ]
        });

        let result = parse_structured_user_input_request("call_1", &args).unwrap();
        assert_eq!(
            result.questions[0].placeholder.as_deref(),
            Some("feature/adaptive-yield-ui")
        );
        assert_eq!(
            result.questions[0].help_text.as_deref(),
            Some("Use the exact git ref that should be deployed.")
        );
        assert_eq!(result.questions[0].default_value.as_deref(), Some("main"));
        assert_eq!(
            result.questions[1].kind,
            alan_protocol::StructuredInputKind::MultiSelect
        );
        assert_eq!(result.questions[1].default_values, vec!["prod", "staging"]);
        assert_eq!(result.questions[1].min_selected, Some(1));
        assert_eq!(result.questions[1].max_selected, Some(2));
    }

    #[test]
    fn test_parse_structured_user_input_request_rejects_select_without_options() {
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [
                {
                    "id": "q1",
                    "label": "Label",
                    "prompt": "Prompt?",
                    "kind": "single_select"
                }
            ]
        });

        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_missing_required() {
        // Missing title
        let args = json!({
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing prompt
        let args = json!({
            "title": "Title",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing questions
        let args = json!({
            "title": "Title",
            "prompt": "Prompt"
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_empty_fields() {
        // Empty title
        let args = json!({
            "title": "",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Empty prompt
        let args = json!({
            "title": "Title",
            "prompt": "  ",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_empty_questions() {
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": []
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_invalid_question() {
        // Missing question id
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing question label
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing question prompt
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_custom_request_id() {
        let args = json!({
            "request_id": "custom_id",
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });

        let result = parse_structured_user_input_request("call_1", &args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().request_id, "call_1");
    }

    #[test]
    fn test_parse_structured_user_input_request_fallback_request_id() {
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });

        let result = parse_structured_user_input_request("fallback_id", &args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().request_id, "fallback_id");
    }

    // Tests for parse_plan_update
    #[test]
    fn test_parse_plan_update_valid() {
        let args = json!({
            "explanation": "Test explanation",
            "items": [
                {"id": "1", "content": "Step 1", "status": "pending"},
                {"id": "2", "content": "Step 2", "status": "in_progress"}
            ]
        });

        let result = parse_plan_update(&args);
        assert!(result.is_some());

        let (explanation, items) = result.unwrap();
        assert_eq!(explanation, Some("Test explanation".to_string()));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "1");
        assert_eq!(items[1].content, "Step 2");
    }

    #[test]
    fn test_parse_plan_update_without_explanation() {
        let args = json!({
            "items": [{"id": "1", "content": "Step 1", "status": "completed"}]
        });

        let result = parse_plan_update(&args);
        assert!(result.is_some());

        let (explanation, items) = result.unwrap();
        assert_eq!(explanation, None);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_plan_update_missing_items() {
        let args = json!({
            "explanation": "Test"
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_empty_items() {
        let args = json!({
            "items": []
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_missing_id() {
        let args = json!({
            "items": [{"content": "Step 1", "status": "pending"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_missing_content() {
        let args = json!({
            "items": [{"id": "1", "status": "pending"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_missing_status() {
        let args = json!({
            "items": [{"id": "1", "content": "Step 1"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_invalid_status() {
        let args = json!({
            "items": [{"id": "1", "content": "Step 1", "status": "invalid_status"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_using_description() {
        // Tests that "description" field can be used as fallback for "content"
        let args = json!({
            "items": [{"id": "1", "description": "Step description", "status": "pending"}]
        });

        let result = parse_plan_update(&args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().1[0].content, "Step description");
    }

    #[test]
    fn test_parse_delegated_skill_invocation_request_valid() {
        let args = json!({
            "skill_id": "repo-review",
            "target": "reviewer",
            "task": "Review the current diff and summarize risks."
        });

        let result = parse_delegated_skill_invocation_request(&args).unwrap();
        assert_eq!(result.skill_id, "repo-review");
        assert_eq!(result.target, "reviewer");
        assert_eq!(result.task, "Review the current diff and summarize risks.");
    }

    #[test]
    fn test_parse_delegated_skill_invocation_request_rejects_empty_fields() {
        let missing = json!({
            "skill_id": "repo-review",
            "target": "reviewer"
        });
        assert!(parse_delegated_skill_invocation_request(&missing).is_none());

        let empty = json!({
            "skill_id": "repo-review",
            "target": "reviewer",
            "task": "   "
        });
        assert!(parse_delegated_skill_invocation_request(&empty).is_none());
    }

    #[test]
    fn test_build_bounded_delegated_invocation_persistence_truncates_fields() {
        let request = DelegatedSkillInvocationRequest {
            skill_id: "s".repeat(MAX_DELEGATED_SKILL_ID_CHARS + 40),
            target: "t".repeat(MAX_DELEGATED_TARGET_CHARS + 40),
            task: "x".repeat(MAX_DELEGATED_TASK_CHARS + 200),
        };
        let result = DelegatedSkillResult::failed(
            format!(
                "Delegated skill '{}' resolved to child agent '{}', but child-agent spawn support is not yet available in this runtime.",
                request.skill_id, request.target
            ),
            Some(json!({
                "error_kind": "runtime_child_launch_unavailable"
            })),
        );

        let child_run = Some(DelegatedChildRunReference {
            session_id: "child-session".to_string(),
            rollout_path: Some(PathBuf::from("/tmp/child-rollout.jsonl")),
            terminal_status: "completed".to_string(),
        });
        let (arguments, record, rollout_record) =
            build_bounded_delegated_invocation_persistence(&request, result, child_run);

        let skill_id = arguments["skill_id"].as_str().unwrap();
        let target = arguments["target"].as_str().unwrap();
        let task = arguments["task"].as_str().unwrap();
        assert!(skill_id.chars().count() <= MAX_DELEGATED_SKILL_ID_CHARS);
        assert!(target.chars().count() <= MAX_DELEGATED_TARGET_CHARS);
        assert!(task.chars().count() <= MAX_DELEGATED_TASK_CHARS);
        assert!(skill_id.ends_with("..."));
        assert!(target.ends_with("..."));
        assert!(task.ends_with("..."));
        assert!(record.result.summary.chars().count() <= MAX_DELEGATED_RESULT_SUMMARY_CHARS);
        assert!(record.result.summary.ends_with("..."));
        assert_eq!(
            rollout_record.child_run.as_ref().unwrap().session_id,
            "child-session"
        );
    }

    #[test]
    fn test_build_bounded_delegated_invocation_persistence_keeps_child_run_out_of_tape_record() {
        let request = DelegatedSkillInvocationRequest {
            skill_id: "repo-review".to_string(),
            target: "reviewer".to_string(),
            task: "Review the current diff and summarize risks.".to_string(),
        };
        let result = DelegatedSkillResult::completed("Delegated review completed.", None);
        let child_run = Some(DelegatedChildRunReference {
            session_id: "child-session".to_string(),
            rollout_path: Some(PathBuf::from("/tmp/child-rollout.jsonl")),
            terminal_status: "completed".to_string(),
        });

        let (_, tape_record, rollout_record) =
            build_bounded_delegated_invocation_persistence(&request, result, child_run);
        let tape_payload = serde_json::to_value(&tape_record).unwrap();
        let rollout_payload = serde_json::to_value(&rollout_record).unwrap();

        assert!(tape_payload.get("child_run").is_none());
        assert_eq!(
            rollout_payload["child_run"]["session_id"],
            json!("child-session")
        );
        assert_eq!(
            rollout_payload["child_run"]["rollout_path"],
            json!("/tmp/child-rollout.jsonl")
        );
    }

    // Tests for parse_plan_status
    #[test]
    fn test_parse_plan_status_valid_values() {
        assert!(parse_plan_status("pending").is_some());
        assert!(parse_plan_status("blocked").is_some());
        assert!(parse_plan_status("in_progress").is_some());
        assert!(parse_plan_status("completed").is_some());
        assert!(parse_plan_status("skipped").is_some());
    }

    #[test]
    fn test_parse_plan_status_invalid_values() {
        assert!(parse_plan_status("unknown").is_none());
        assert!(parse_plan_status("").is_none());
        assert!(parse_plan_status("PENDING").is_none()); // Case sensitive
    }

    // Tests for try_handle_virtual_tool_call
    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_request_confirmation() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                "checkpoint_id": "chk_123",
                "checkpoint_type": "test",
                "summary": "Test confirmation"
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::PauseTurn));

        // Verify confirmation was set
        assert!(state.turn_state.pending_confirmation().is_some());
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invalid_confirmation() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                // Invalid summary type
                "summary": 42
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::EndTurn));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_request_user_input() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_user_input".to_string(),
            arguments: json!({
                "title": "Test Input",
                "prompt": "Enter value",
                "questions": [{"id": "q1", "label": "Q1", "prompt": "What?"}]
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::PauseTurn));

        // Verify structured input was set
        assert!(state.turn_state.has_pending_interaction());
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invalid_user_input() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_user_input".to_string(),
            arguments: json!({
                // Missing required fields
                "title": "Test"
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::EndTurn));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_update_plan() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Test plan",
                "items": [{"id": "1", "content": "Step 1", "status": "in_progress"}]
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: true
            }
        ));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invalid_update_plan() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                // Missing items
                "explanation": "Test"
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: false
            }
        ));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invoke_delegated_skill() {
        let mut state = create_test_agent_loop_state();
        state.core_config.memory.workspace_dir =
            Some(PathBuf::from("/tmp/alan-delegated-parent/.alan/memory"));
        activate_test_delegated_skill(&mut state, "repo-review", "reviewer");

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let captured_spec = Arc::new(Mutex::new(None));
        let captured_spec_for_spawn = Arc::clone(&captured_spec);
        let cancel = CancellationToken::new();
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, spec, _cancel| {
                let captured_spec = Arc::clone(&captured_spec_for_spawn);
                Box::pin(async move {
                    *captured_spec.lock().unwrap() = Some(spec);
                    Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::Completed,
                        session_id: "child-session".to_string(),
                        rollout_path: Some(PathBuf::from("/tmp/child-rollout.jsonl")),
                        output_text: String::new(),
                        turn_summary: Some("Delegated review completed.".to_string()),
                        warnings: Vec::new(),
                        error_message: None,
                        pause: None,
                    })
                })
            },
        )
        .await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: true
            }
        ));
        let spec = captured_spec
            .lock()
            .unwrap()
            .clone()
            .expect("expected delegated spawn spec");
        assert_eq!(
            spec.target,
            alan_protocol::SpawnTarget::PackageChildAgent {
                package_id: "skill:repo-review".to_string(),
                export_name: "reviewer".to_string(),
            }
        );
        assert_eq!(
            spec.handles,
            vec![SpawnHandle::Workspace, SpawnHandle::ApprovalScope]
        );
        assert_eq!(
            spec.launch.workspace_root,
            Some(PathBuf::from("/tmp/alan-delegated-parent"))
        );
        assert_eq!(
            spec.launch.cwd,
            Some(PathBuf::from("/tmp/alan-delegated-parent"))
        );
        assert_eq!(
            spec.launch.timeout_secs,
            Some(state.core_config.tool_timeout_secs as u64)
        );

        let prompt_view = state.session.tape.prompt_view();
        let tool_result = prompt_view
            .messages
            .iter()
            .find_map(|message| match message {
                crate::tape::Message::Tool { responses } => responses
                    .iter()
                    .find(|response| response.id == "call_1")
                    .map(crate::tape::ToolResponse::text_content),
                _ => None,
            })
            .expect("expected delegated skill tool result");
        assert!(tool_result.contains("\"task\":\"Review the current diff and summarize risks.\""));
        assert!(tool_result.contains("\"status\":\"completed\""));
        assert!(tool_result.contains("Delegated review completed."));
        assert!(!tool_result.contains("child_run"));
        assert!(!tool_result.contains("child-session"));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invoke_delegated_skill_keeps_workspace_root_separate_from_nested_cwd()
     {
        let mut state = create_test_agent_loop_state();
        state.core_config.memory.workspace_dir =
            Some(PathBuf::from("/tmp/alan-delegated-parent/.alan/memory"));
        state
            .tools
            .set_default_cwd(PathBuf::from("/tmp/alan-delegated-parent/nested/src"));
        activate_test_delegated_skill(&mut state, "repo-review", "reviewer");

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let captured_spec = Arc::new(Mutex::new(None));
        let captured_spec_for_spawn = Arc::clone(&captured_spec);
        let cancel = CancellationToken::new();
        let mut emit = |_event: Event| async {};
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, spec, _cancel| {
                let captured_spec = Arc::clone(&captured_spec_for_spawn);
                Box::pin(async move {
                    *captured_spec.lock().unwrap() = Some(spec);
                    Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::Completed,
                        session_id: "child-session".to_string(),
                        rollout_path: None,
                        output_text: String::new(),
                        turn_summary: Some("done".to_string()),
                        warnings: Vec::new(),
                        error_message: None,
                        pause: None,
                    })
                })
            },
        )
        .await;
        assert!(result.is_ok());

        let spec = captured_spec
            .lock()
            .unwrap()
            .clone()
            .expect("expected delegated spawn spec");
        assert_eq!(
            spec.launch.cwd,
            Some(PathBuf::from("/tmp/alan-delegated-parent/nested/src"))
        );
        assert_eq!(
            spec.launch.workspace_root,
            Some(PathBuf::from("/tmp/alan-delegated-parent"))
        );
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invoke_delegated_skill_leaves_workspace_root_unset_without_memory_context()
     {
        let mut state = create_test_agent_loop_state();
        state
            .tools
            .set_default_cwd(PathBuf::from("/tmp/alan-delegated-parent/nested/src"));
        activate_test_delegated_skill(&mut state, "repo-review", "reviewer");

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let captured_spec = Arc::new(Mutex::new(None));
        let captured_spec_for_spawn = Arc::clone(&captured_spec);
        let cancel = CancellationToken::new();
        let mut emit = |_event: Event| async {};
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, spec, _cancel| {
                let captured_spec = Arc::clone(&captured_spec_for_spawn);
                Box::pin(async move {
                    *captured_spec.lock().unwrap() = Some(spec);
                    Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::Completed,
                        session_id: "child-session".to_string(),
                        rollout_path: None,
                        output_text: String::new(),
                        turn_summary: Some("done".to_string()),
                        warnings: Vec::new(),
                        error_message: None,
                        pause: None,
                    })
                })
            },
        )
        .await;
        assert!(result.is_ok());

        let spec = captured_spec
            .lock()
            .unwrap()
            .clone()
            .expect("expected delegated spawn spec");
        assert_eq!(
            spec.launch.cwd,
            Some(PathBuf::from("/tmp/alan-delegated-parent/nested/src"))
        );
        assert_eq!(spec.launch.workspace_root, None);
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invoke_delegated_skill_records_successful_tool_call()
    {
        let temp = TempDir::new().unwrap();
        let mut state = create_test_agent_loop_state();
        state.session = Session::new_with_recorder_in_dir("gpt-5-mini", temp.path())
            .await
            .unwrap();
        activate_test_delegated_skill(&mut state, "repo-review", "reviewer");

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let mut emit = |_event: Event| async {};
        let cancel = CancellationToken::new();
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, _spec, _cancel| {
                Box::pin(async {
                    Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::Completed,
                        session_id: "child-session".to_string(),
                        rollout_path: Some(PathBuf::from("/tmp/child-rollout.jsonl")),
                        output_text: String::new(),
                        turn_summary: Some("Delegated review completed.".to_string()),
                        warnings: Vec::new(),
                        error_message: None,
                        pause: None,
                    })
                })
            },
        )
        .await;
        assert!(result.is_ok());

        let rollout_path = state.session.rollout_path().unwrap().clone();
        let mut tool_call = None;
        for _ in 0..20 {
            let items = RolloutRecorder::load_history(&rollout_path).await.unwrap();
            tool_call = items.into_iter().find_map(|item| match item {
                RolloutItem::ToolCall(tool_call) => Some(tool_call),
                _ => None,
            });
            if tool_call.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let tool_call = tool_call.expect("expected delegated tool-call rollout record");
        assert_eq!(tool_call.name, "invoke_delegated_skill");
        assert!(tool_call.success);
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invoke_delegated_skill_bounds_preview_and_payload() {
        let mut state = create_test_agent_loop_state();
        let long_skill_id = format!("repo-review-{}", "x".repeat(150));
        let long_target = format!("reviewer-{}", "y".repeat(150));
        let long_task = "Review the current diff and summarize risks. ".repeat(80);
        activate_test_delegated_skill(&mut state, &long_skill_id, &long_target);

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": long_skill_id,
                "target": long_target,
                "task": long_task
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let cancel = CancellationToken::new();
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, _spec, _cancel| {
                Box::pin(async {
                    Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::Completed,
                        session_id: "child-session".to_string(),
                        rollout_path: Some(PathBuf::from("/tmp/child-rollout.jsonl")),
                        output_text: String::new(),
                        turn_summary: Some("delegated-result ".repeat(40)),
                        warnings: Vec::new(),
                        error_message: None,
                        pause: None,
                    })
                })
            },
        )
        .await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: true
            }
        ));

        let preview = events
            .iter()
            .find_map(|event| match event {
                Event::ToolCallCompleted {
                    id,
                    result_preview: Some(preview),
                    ..
                } if id == "call_1" => Some(preview.as_str()),
                _ => None,
            })
            .expect("expected delegated skill preview");
        assert!(preview.chars().count() <= 163);
        assert!(preview.ends_with("..."));

        let prompt_view = state.session.tape.prompt_view();
        let tool_result = prompt_view
            .messages
            .iter()
            .find_map(|message| match message {
                crate::tape::Message::Tool { responses } => responses
                    .iter()
                    .find(|response| response.id == "call_1")
                    .map(crate::tape::ToolResponse::text_content),
                _ => None,
            })
            .expect("expected delegated skill tool result");
        let payload: serde_json::Value = serde_json::from_str(&tool_result).unwrap();
        assert!(
            payload["skill_id"].as_str().unwrap().chars().count() <= MAX_DELEGATED_SKILL_ID_CHARS
        );
        assert!(payload["target"].as_str().unwrap().chars().count() <= MAX_DELEGATED_TARGET_CHARS);
        assert!(payload["task"].as_str().unwrap().chars().count() <= MAX_DELEGATED_TASK_CHARS);
        assert!(
            payload["result"]["summary"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= MAX_DELEGATED_RESULT_SUMMARY_CHARS
        );
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invoke_delegated_skill_honors_interrupt() {
        let mut state = create_test_agent_loop_state();
        activate_test_delegated_skill(&mut state, "repo-review", "reviewer");
        state
            .turn_state
            .set_turn_activity(TurnActivityState::Running);

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let cancel = CancellationToken::new();
        let cancel_for_task = cancel.clone();
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, _spec, _cancel| {
                let cancel_for_task = cancel_for_task.clone();
                Box::pin(async move {
                    cancel_for_task.cancelled().await;
                    Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::Cancelled,
                        session_id: "child-session".to_string(),
                        rollout_path: Some(PathBuf::from("/tmp/child-rollout.jsonl")),
                        output_text: String::new(),
                        turn_summary: None,
                        warnings: Vec::new(),
                        error_message: None,
                        pause: None,
                    })
                })
            },
        );
        tokio::task::yield_now().await;
        cancel.cancel();

        let result = result.await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::EndTurn));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::TurnCompleted { summary: Some(summary) } if summary == "Task cancelled by user"
        )));

        let prompt_view = state.session.tape.prompt_view();
        assert!(!prompt_view.messages.iter().any(|message| matches!(
            message,
            crate::tape::Message::Tool { responses }
                if responses.iter().any(|response| response.id == "call_1")
        )));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invoke_delegated_skill_honors_interrupt_during_startup()
     {
        let mut state = create_test_agent_loop_state();
        activate_test_delegated_skill(&mut state, "repo-review", "reviewer");
        state
            .turn_state
            .set_turn_activity(TurnActivityState::Running);

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let cancel = CancellationToken::new();
        let cancel_for_task = cancel.clone();
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, _spec, _cancel| {
                let cancel_for_task = cancel_for_task.clone();
                Box::pin(async move {
                    cancel_for_task.cancelled().await;
                    Err(anyhow::anyhow!("Child-agent launch cancelled"))
                })
            },
        );
        tokio::task::yield_now().await;
        cancel.cancel();

        let result = result.await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::EndTurn));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::TurnCompleted { summary: Some(summary) } if summary == "Task cancelled by user"
        )));

        let prompt_view = state.session.tape.prompt_view();
        assert!(!prompt_view.messages.iter().any(|message| matches!(
            message,
            crate::tape::Message::Tool { responses }
                if responses.iter().any(|response| response.id == "call_1")
        )));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invalid_delegated_skill_request() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer"
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: true
            }
        ));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                Event::Error {
                    message,
                    recoverable: true
                } if message.contains("delegated skill invocation")
            )
        }));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_rejects_target_mismatch() {
        let mut state = create_test_agent_loop_state();
        activate_test_delegated_skill(&mut state, "repo-review", "reviewer");

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "grader",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let cancel = CancellationToken::new();
        let result = handle_invoke_delegated_skill(
            &mut state,
            &tool_call,
            &tool_call.arguments,
            &cancel,
            &mut emit,
            |_state, _spec, _cancel| {
                panic!("target mismatch should not attempt child launch");
                #[allow(unreachable_code)]
                Box::pin(async move {
                    Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::Completed,
                        session_id: String::new(),
                        rollout_path: None,
                        output_text: String::new(),
                        turn_summary: None,
                        warnings: Vec::new(),
                        error_message: None,
                        pause: None,
                    })
                })
            },
        )
        .await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: true
            }
        ));

        let prompt_view = state.session.tape.prompt_view();
        let tool_result = prompt_view
            .messages
            .iter()
            .find_map(|message| match message {
                crate::tape::Message::Tool { responses } => responses
                    .iter()
                    .find(|response| response.id == "call_1")
                    .map(crate::tape::ToolResponse::text_content),
                _ => None,
            })
            .expect("expected delegated skill tool result");
        assert!(tool_result.contains("\"status\":\"failed\""));
        assert!(tool_result.contains("delegate_target_mismatch"));
        assert!(tool_result.contains("\"resolved_target\":\"reviewer\""));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ToolCallCompleted { id, .. } if id == "call_1"
        )));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_deferred_to_dynamic_delegated_tool() {
        let mut state = create_test_agent_loop_state();
        state.session.dynamic_tools.insert(
            "invoke_delegated_skill".to_string(),
            alan_protocol::DynamicToolSpec {
                name: "invoke_delegated_skill".to_string(),
                description: "Delegated execution bridge".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
                capability: Some(alan_protocol::ToolCapability::Read),
            },
        );

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::NotVirtual));
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_try_handle_non_virtual_tool() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::NotVirtual));
    }

    #[tokio::test]
    async fn test_try_handle_unknown_tool() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "unknown_tool".to_string(),
            arguments: json!({}),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result = try_handle_virtual_tool_call_for_test(&mut state, &tool_call, &mut emit).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::NotVirtual));
    }
}
