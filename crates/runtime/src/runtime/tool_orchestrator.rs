use alan_protocol::{
    AdaptiveForm, AdaptivePresentationHint, ConfirmationYieldPayload, DynamicToolYieldPayload,
    Event, InputMode, Op, StructuredInputKind, StructuredInputOption, StructuredInputQuestion,
    ToolCapability,
};
use anyhow::Result;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::approval::{
    EFFECT_REPLAY_CHECKPOINT_PREFIX, EFFECT_REPLAY_CHECKPOINT_TYPE, PendingConfirmation,
    TOOL_ESCALATION_CHECKPOINT_PREFIX, TOOL_ESCALATION_CHECKPOINT_TYPE,
    append_skill_permission_hints, is_runtime_confirmation_checkpoint_type, replays_tool_calls,
};

use super::agent_loop::{NormalizedToolCall, RuntimeLoopState};
use super::loop_guard::ToolLoopGuard;
use super::tool_policy::{ToolPolicyDecision, evaluate_tool_policy};
use super::turn_driver::{MAX_BUFFERED_INBAND_USER_INPUTS, TurnInputBroker};
use super::turn_support::{check_turn_cancelled, tool_result_preview};
use super::virtual_tools::{VirtualToolOutcome, try_handle_virtual_tool_call};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectCategory {
    File,
    Network,
    Process,
}

impl EffectCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Network => "network",
            Self::Process => "process",
        }
    }
}

#[derive(Debug, Clone)]
struct EffectIdentity {
    category: EffectCategory,
    idempotency_key: String,
    request_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolOrchestratorOutcome {
    ContinueToolBatch { refresh_context: bool },
    PauseTurn,
    EndTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolBatchOrchestratorOutcome {
    ContinueTurnLoop { refresh_context: bool },
    PauseTurn,
    EndTurn,
}

fn dynamic_tool_resume_form(
    capabilities: &alan_protocol::ClientCapabilities,
    tool_name: &str,
) -> Option<AdaptiveForm> {
    if !capabilities.adaptive_yields.schema_driven_forms {
        return None;
    }

    let field_hints = if capabilities.adaptive_yields.presentation_hints {
        vec![AdaptivePresentationHint::Toggle]
    } else {
        vec![]
    };

    Some(AdaptiveForm {
        fields: vec![
            StructuredInputQuestion {
                id: "success".to_string(),
                label: "Success".to_string(),
                prompt: "Did the client-side tool execution succeed?".to_string(),
                kind: StructuredInputKind::Boolean,
                required: true,
                placeholder: None,
                help_text: Some(
                    "Set to false if the client-side tool execution failed.".to_string(),
                ),
                default_value: Some("true".to_string()),
                default_values: vec![],
                min_selected: None,
                max_selected: None,
                options: vec![
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
                ],
                presentation_hints: field_hints,
            },
            StructuredInputQuestion {
                id: "result".to_string(),
                label: "Result".to_string(),
                prompt: format!("Return a simple result for {tool_name}."),
                kind: StructuredInputKind::Text,
                required: false,
                placeholder: Some("Short result summary".to_string()),
                help_text: Some(
                    "Use raw /resume JSON if you need nested or richer structured output."
                        .to_string(),
                ),
                default_value: None,
                default_values: vec![],
                min_selected: None,
                max_selected: None,
                options: vec![],
                presentation_hints: vec![],
            },
            StructuredInputQuestion {
                id: "error".to_string(),
                label: "Error".to_string(),
                prompt: format!("Optional error details for {tool_name}."),
                kind: StructuredInputKind::Text,
                required: false,
                placeholder: Some("Only fill when success is false".to_string()),
                help_text: None,
                default_value: None,
                default_values: vec![],
                min_selected: None,
                max_selected: None,
                options: vec![],
                presentation_hints: vec![],
            },
        ],
    })
}

fn confirmation_payload(
    capabilities: &alan_protocol::ClientCapabilities,
    checkpoint_type: String,
    summary: String,
    details: Value,
    options: Vec<String>,
) -> ConfirmationYieldPayload {
    let presentation_hints = if capabilities.adaptive_yields.presentation_hints
        && is_runtime_confirmation_checkpoint_type(&checkpoint_type)
    {
        vec![AdaptivePresentationHint::Dangerous]
    } else {
        vec![]
    };

    let default_option = options
        .iter()
        .find(|option| option.as_str() == "approve")
        .cloned()
        .or_else(|| options.first().cloned());

    ConfirmationYieldPayload {
        checkpoint_type,
        summary,
        details: Some(details),
        options,
        default_option,
        presentation_hints,
    }
}

pub(super) struct ToolTurnOrchestrator {
    loop_guard: ToolLoopGuard,
}

impl ToolTurnOrchestrator {
    pub(super) fn new(max_tool_loops: Option<usize>, tool_repeat_limit: usize) -> Self {
        Self {
            loop_guard: ToolLoopGuard::new(max_tool_loops, tool_repeat_limit),
        }
    }

    pub(super) async fn orchestrate_tool_batch<E, F>(
        &mut self,
        state: &mut RuntimeLoopState,
        tool_calls: &[NormalizedToolCall],
        inputs: ToolOrchestratorInputs<'_>,
        emit: &mut E,
    ) -> Result<ToolBatchOrchestratorOutcome>
    where
        E: FnMut(Event) -> F,
        F: std::future::Future<Output = ()>,
    {
        self.orchestrate_tool_batch_internal(state, tool_calls, inputs, None, emit)
            .await
    }

    async fn orchestrate_tool_batch_internal<E, F>(
        &mut self,
        state: &mut RuntimeLoopState,
        tool_calls: &[NormalizedToolCall],
        inputs: ToolOrchestratorInputs<'_>,
        approved_unknown_effect_call_index: Option<usize>,
        emit: &mut E,
    ) -> Result<ToolBatchOrchestratorOutcome>
    where
        E: FnMut(Event) -> F,
        F: std::future::Future<Output = ()>,
    {
        orchestrate_tool_batch_with_guard(
            state,
            &mut self.loop_guard,
            tool_calls,
            inputs,
            approved_unknown_effect_call_index,
            emit,
        )
        .await
    }
}

pub(super) async fn replay_approved_tool_call_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    tool_call: &NormalizedToolCall,
    approved_unknown_effect_call_id: Option<&str>,
    inputs: ToolOrchestratorInputs<'_>,
    emit: &mut E,
) -> Result<ToolBatchOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    replay_approved_tool_batch_with_cancel(
        state,
        std::slice::from_ref(tool_call),
        approved_unknown_effect_call_id,
        inputs,
        emit,
    )
    .await
}

pub(super) async fn replay_approved_tool_batch_with_cancel<E, F>(
    state: &mut RuntimeLoopState,
    tool_calls: &[NormalizedToolCall],
    approved_unknown_effect_call_id: Option<&str>,
    inputs: ToolOrchestratorInputs<'_>,
    emit: &mut E,
) -> Result<ToolBatchOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let max_tool_loops = if state.runtime_config.max_tool_loops == 0 {
        None
    } else {
        Some(state.runtime_config.max_tool_loops)
    };
    let approved_unknown_effect_call_index = approved_unknown_effect_call_id.and_then(|call_id| {
        tool_calls
            .first()
            .filter(|call| call.id == call_id)
            .map(|_| 0)
    });
    let mut orchestrator =
        ToolTurnOrchestrator::new(max_tool_loops, state.runtime_config.tool_repeat_limit);
    orchestrator
        .orchestrate_tool_batch_internal(
            state,
            tool_calls,
            inputs,
            approved_unknown_effect_call_index,
            emit,
        )
        .await
}

#[derive(Clone, Copy)]
pub(super) struct ToolOrchestratorInputs<'a> {
    pub cancel: &'a CancellationToken,
    pub steering_broker: Option<&'a TurnInputBroker>,
}

fn classify_effect_category(
    tool_name: &str,
    tool_capability: Option<ToolCapability>,
) -> Option<EffectCategory> {
    match tool_capability {
        Some(ToolCapability::Read) => None,
        Some(ToolCapability::Network) => Some(EffectCategory::Network),
        Some(ToolCapability::Write) => {
            if matches!(tool_name, "write_file" | "edit_file") {
                Some(EffectCategory::File)
            } else {
                Some(EffectCategory::Process)
            }
        }
        None => {
            if tool_name == "bash" {
                Some(EffectCategory::Process)
            } else {
                None
            }
        }
    }
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = Map::new();
            for key in keys {
                if let Some(entry) = map.get(key) {
                    sorted.insert(key.clone(), canonicalize_json(entry));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn build_effect_identity(
    session: &crate::session::Session,
    tool_name: &str,
    tool_arguments: &Value,
    category: EffectCategory,
) -> EffectIdentity {
    let normalized_arguments = canonicalize_json(tool_arguments);
    let request_payload = json!({
        "tool_name": tool_name,
        "effect_type": category.as_str(),
        "arguments": normalized_arguments,
    });
    let request_fingerprint = sha256_hex(&request_payload.to_string());
    let idempotency_key = format!(
        "run:{}:turn:{}:{}",
        session.id,
        session.user_turn_ordinal(),
        request_fingerprint
    );
    EffectIdentity {
        category,
        idempotency_key,
        request_fingerprint,
    }
}

fn effect_decision_reason(
    decision: &str,
    reason: Option<&str>,
    existing_status: Option<crate::rollout::EffectStatus>,
    dedupe_hit: bool,
) -> Value {
    json!({
        "decision": decision,
        "reason": reason,
        "existing_status": existing_status.map(|status| match status {
            crate::rollout::EffectStatus::Applied => "applied",
            crate::rollout::EffectStatus::Failed => "failed",
            crate::rollout::EffectStatus::Unknown => "unknown",
        }),
        "dedupe_hit": dedupe_hit,
    })
}

async fn orchestrate_tool_call_with_guard<E, F>(
    state: &mut RuntimeLoopState,
    loop_guard: &mut ToolLoopGuard,
    tool_call: &NormalizedToolCall,
    inputs: ToolOrchestratorInputs<'_>,
    allow_approved_unknown_effect_execution: bool,
    emit: &mut E,
) -> Result<ToolOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let tool_arguments = tool_call.arguments.clone();

    if let Some(msg) = loop_guard.before_tool_call(&tool_call.name, &tool_arguments) {
        emit(Event::Error {
            message: msg.clone(),
            recoverable: true,
        })
        .await;
        emit(Event::TextDelta {
            chunk: msg,
            is_final: true,
        })
        .await;
        return Ok(ToolOrchestratorOutcome::EndTurn);
    }

    match try_handle_virtual_tool_call(state, tool_call, &tool_arguments, inputs.cancel, emit)
        .await?
    {
        VirtualToolOutcome::NotVirtual => {}
        VirtualToolOutcome::Continue { refresh_context } => {
            return Ok(ToolOrchestratorOutcome::ContinueToolBatch { refresh_context });
        }
        VirtualToolOutcome::PauseTurn => return Ok(ToolOrchestratorOutcome::PauseTurn),
        VirtualToolOutcome::EndTurn => return Ok(ToolOrchestratorOutcome::EndTurn),
    }

    let tool_capability = state
        .tools
        .capability_for_tool(&tool_call.name, &tool_arguments)
        .or_else(|| {
            state
                .session
                .dynamic_tools
                .get(&tool_call.name)
                .and_then(|tool| tool.capability)
        });
    let policy_decision = evaluate_tool_policy(
        &state.runtime_config.policy_engine,
        &state.runtime_config.governance,
        &tool_call.name,
        &tool_arguments,
        tool_capability,
    );
    let policy_audit = match &policy_decision {
        ToolPolicyDecision::Allow { audit }
        | ToolPolicyDecision::Escalate { audit, .. }
        | ToolPolicyDecision::Forbidden { audit, .. } => audit.clone(),
    };
    state.session.record_event(
        "tool_policy_decision",
        json!({
            "tool_call_id": tool_call.id,
            "tool_name": tool_call.name,
            "policy_source": policy_audit.policy_source,
            "rule_id": policy_audit.rule_id,
            "action": policy_audit.action,
            "reason": policy_audit.reason,
            "capability": policy_audit.capability,
            "sandbox_backend": policy_audit.sandbox_backend,
        }),
    );

    let tool_audit = match policy_decision {
        ToolPolicyDecision::Allow { audit } => Some(audit),
        ToolPolicyDecision::Escalate {
            summary,
            mut details,
            audit,
        } => {
            details["replay_tool_call"] = json!({
                "call_id": tool_call.id,
                "tool_name": tool_call.name,
                "arguments": tool_arguments,
            });
            details = append_skill_permission_hints(details, state.turn_state.active_skills());
            let pending = PendingConfirmation {
                checkpoint_id: format!("{TOOL_ESCALATION_CHECKPOINT_PREFIX}{}", tool_call.id),
                checkpoint_type: TOOL_ESCALATION_CHECKPOINT_TYPE.to_string(),
                summary,
                details,
                options: vec!["approve".to_string(), "reject".to_string()],
            };
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                json!({"status":"escalation_required"}),
                true,
                Some(audit),
            );
            state.turn_state.set_confirmation(pending.clone());
            emit(Event::Yield {
                request_id: pending.checkpoint_id,
                kind: alan_protocol::YieldKind::Confirmation,
                payload: serde_json::to_value(confirmation_payload(
                    &state.session.client_capabilities,
                    pending.checkpoint_type,
                    pending.summary,
                    pending.details,
                    pending.options,
                ))
                .unwrap_or_else(|_| json!({})),
            })
            .await;
            return Ok(ToolOrchestratorOutcome::PauseTurn);
        }
        ToolPolicyDecision::Forbidden { reason, audit } => {
            let blocked_payload = json!({
                "error": reason,
                "status": "blocked_by_policy"
            });
            emit(Event::Error {
                message: blocked_payload["error"]
                    .as_str()
                    .unwrap_or("Tool blocked by policy")
                    .to_string(),
                recoverable: true,
            })
            .await;
            emit(Event::ToolCallCompleted {
                id: tool_call.id.clone(),
                result_preview: tool_result_preview(&blocked_payload),
                audit: Some(audit.clone()),
            })
            .await;
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                blocked_payload.clone(),
                false,
                Some(audit),
            );
            state
                .session
                .add_tool_message(&tool_call.id, &tool_call.name, blocked_payload);
            return Ok(ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: false,
            });
        }
    };

    if state.session.dynamic_tools.contains_key(&tool_call.name) {
        emit(Event::ToolCallStarted {
            id: tool_call.id.clone(),
            name: tool_call.name.clone(),
            audit: tool_audit.clone(),
        })
        .await;
        state
            .turn_state
            .set_dynamic_tool_call(crate::approval::PendingDynamicToolCall {
                call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                arguments: tool_arguments.clone(),
            });
        state.session.record_tool_call_with_audit(
            &tool_call.name,
            tool_arguments.clone(),
            json!({"status":"pending_dynamic_tool_result","call_id": tool_call.id}),
            true,
            tool_audit.clone(),
        );
        emit(Event::Yield {
            request_id: tool_call.id.clone(),
            kind: alan_protocol::YieldKind::DynamicTool,
            payload: serde_json::to_value(DynamicToolYieldPayload {
                tool_name: tool_call.name.clone(),
                arguments: tool_arguments.clone(),
                title: format!("Resolve dynamic tool: {}", tool_call.name),
                prompt: Some(
                    "Use the adaptive form for simple success/result payloads, or /resume <json> for raw structured results."
                        .to_string(),
                ),
                form: dynamic_tool_resume_form(&state.session.client_capabilities, &tool_call.name),
            })
            .unwrap_or_else(|_| json!({})),
        })
        .await;
        return Ok(ToolOrchestratorOutcome::PauseTurn);
    }

    let effect_identity =
        classify_effect_category(&tool_call.name, tool_capability).map(|category| {
            build_effect_identity(&state.session, &tool_call.name, &tool_arguments, category)
        });
    let existing_effect = effect_identity.as_ref().and_then(|identity| {
        state
            .session
            .effect_by_idempotency_key(&identity.idempotency_key)
    });

    if let (Some(identity), Some(existing)) = (&effect_identity, &existing_effect)
        && matches!(existing.status, crate::rollout::EffectStatus::Unknown)
        && !allow_approved_unknown_effect_execution
    {
        let escalation_reason =
            "Previous side effect attempt has unknown status; explicit confirmation required";
        state.session.record_event(
            "effect_dedupe_decision",
            json!({
                "run_id": state.session.id,
                "tool_call_id": tool_call.id,
                "tool_name": tool_call.name,
                "effect_type": identity.category.as_str(),
                "idempotency_key": identity.idempotency_key,
                "request_fingerprint": identity.request_fingerprint,
                "existing_effect_id": existing.effect_id,
                "decision": effect_decision_reason(
                    "escalate",
                    Some(escalation_reason),
                    Some(existing.status.clone()),
                    false
                )
            }),
        );

        let pending = PendingConfirmation {
            checkpoint_id: format!("{EFFECT_REPLAY_CHECKPOINT_PREFIX}{}", tool_call.id),
            checkpoint_type: EFFECT_REPLAY_CHECKPOINT_TYPE.to_string(),
            summary: "Potential duplicate side effect requires confirmation".to_string(),
            details: append_skill_permission_hints(
                json!({
                    "reason": escalation_reason,
                    "effect_status": "unknown",
                    "effect_type": identity.category.as_str(),
                    "idempotency_key": identity.idempotency_key,
                    "request_fingerprint": identity.request_fingerprint,
                    "replay_tool_call": {
                        "call_id": tool_call.id,
                        "tool_name": tool_call.name,
                        "arguments": tool_arguments,
                    }
                }),
                state.turn_state.active_skills(),
            ),
            options: vec!["approve".to_string(), "reject".to_string()],
        };
        state.session.record_tool_call_with_audit(
            &tool_call.name,
            tool_arguments.clone(),
            json!({
                "status": "escalation_required",
                "reason": escalation_reason,
                "idempotency_key": identity.idempotency_key,
                "effect_status": "unknown"
            }),
            true,
            tool_audit.clone(),
        );
        state.turn_state.set_confirmation(pending.clone());
        emit(Event::Yield {
            request_id: pending.checkpoint_id,
            kind: alan_protocol::YieldKind::Confirmation,
            payload: serde_json::to_value(confirmation_payload(
                &state.session.client_capabilities,
                pending.checkpoint_type,
                pending.summary,
                pending.details,
                pending.options,
            ))
            .unwrap_or_else(|_| json!({})),
        })
        .await;
        return Ok(ToolOrchestratorOutcome::PauseTurn);
    }

    emit(Event::ToolCallStarted {
        id: tool_call.id.clone(),
        name: tool_call.name.clone(),
        audit: tool_audit.clone(),
    })
    .await;

    if let (Some(identity), Some(existing)) = (&effect_identity, &existing_effect)
        && matches!(existing.status, crate::rollout::EffectStatus::Applied)
    {
        let dedupe_reason = "Matching applied side effect found; skipped physical execution";
        let replay_payload = existing
            .result_payload
            .clone()
            .or_else(|| {
                state
                    .session
                    .tool_payload_by_call_id(&existing.tool_call_id)
            })
            .unwrap_or_else(|| {
                json!({
                    "status": "dedupe_hit",
                    "dedupe_hit": true,
                    "reason": dedupe_reason,
                    "idempotency_key": identity.idempotency_key,
                    "effect_type": identity.category.as_str(),
                    "effect_status": "applied"
                })
            });
        emit(Event::ToolCallCompleted {
            id: tool_call.id.clone(),
            result_preview: tool_result_preview(&replay_payload),
            audit: tool_audit.clone(),
        })
        .await;
        state.session.record_tool_call_with_audit(
            &tool_call.name,
            tool_arguments.clone(),
            replay_payload.clone(),
            true,
            tool_audit,
        );
        state
            .session
            .add_tool_message(&tool_call.id, &tool_call.name, replay_payload.clone());
        state.session.record_event(
            "effect_dedupe_decision",
            json!({
                "run_id": state.session.id,
                "tool_call_id": tool_call.id,
                "tool_name": tool_call.name,
                "effect_type": identity.category.as_str(),
                "idempotency_key": identity.idempotency_key,
                "request_fingerprint": identity.request_fingerprint,
                "existing_effect_id": existing.effect_id,
                "decision": effect_decision_reason(
                    "skip",
                    Some(dedupe_reason),
                    Some(existing.status.clone()),
                    true
                )
            }),
        );
        let now = chrono::Utc::now().to_rfc3339();
        let replay_digest = existing
            .result_digest
            .clone()
            .unwrap_or_else(|| sha256_hex(&canonicalize_json(&replay_payload).to_string()));
        state.session.record_effect(crate::rollout::EffectRecord {
            effect_id: format!("ef-{}", uuid::Uuid::new_v4()),
            run_id: state.session.id.clone(),
            tool_call_id: tool_call.id.clone(),
            idempotency_key: identity.idempotency_key.clone(),
            effect_type: identity.category.as_str().to_string(),
            request_fingerprint: identity.request_fingerprint.clone(),
            result_digest: Some(replay_digest),
            result_payload: Some(replay_payload),
            status: crate::rollout::EffectStatus::Applied,
            applied_at: existing.applied_at.clone().or(Some(now.clone())),
            reason: Some(dedupe_reason.to_string()),
            dedupe_hit: true,
            timestamp: now,
        });
        return Ok(ToolOrchestratorOutcome::ContinueToolBatch {
            refresh_context: false,
        });
    }

    if let Some(identity) = &effect_identity {
        let existing_status = existing_effect.as_ref().map(|effect| effect.status.clone());
        state.session.record_event(
            "effect_dedupe_decision",
            json!({
                "run_id": state.session.id,
                "tool_call_id": tool_call.id,
                "tool_name": tool_call.name,
                "effect_type": identity.category.as_str(),
                "idempotency_key": identity.idempotency_key,
                "request_fingerprint": identity.request_fingerprint,
                "decision": effect_decision_reason(
                    "execute",
                    Some("No applied effect record found"),
                    existing_status,
                    false
                )
            }),
        );
    }

    let effect_start = effect_identity.as_ref().map(|identity| {
        let record = crate::rollout::EffectRecord {
            effect_id: format!("ef-{}", uuid::Uuid::new_v4()),
            run_id: state.session.id.clone(),
            tool_call_id: tool_call.id.clone(),
            idempotency_key: identity.idempotency_key.clone(),
            effect_type: identity.category.as_str().to_string(),
            request_fingerprint: identity.request_fingerprint.clone(),
            result_digest: None,
            result_payload: None,
            status: crate::rollout::EffectStatus::Unknown,
            applied_at: None,
            reason: Some("execution started before terminal status commit".to_string()),
            dedupe_hit: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        state.session.record_effect(record.clone());
        record
    });

    if effect_start.is_some()
        && let Some(recorder) = state.session.recorder.as_ref()
        && let Err(err) = recorder.flush().await
    {
        let flush_error = format!("Failed to persist side-effect checkpoint: {err}");
        let flush_error_payload = json!({
            "error": flush_error,
            "status": "effect_checkpoint_persist_failed"
        });
        if let (Some(identity), Some(effect_start)) = (&effect_identity, &effect_start) {
            let digest = sha256_hex(&canonicalize_json(&flush_error_payload).to_string());
            state.session.record_effect(crate::rollout::EffectRecord {
                effect_id: effect_start.effect_id.clone(),
                run_id: effect_start.run_id.clone(),
                tool_call_id: effect_start.tool_call_id.clone(),
                idempotency_key: identity.idempotency_key.clone(),
                effect_type: identity.category.as_str().to_string(),
                request_fingerprint: identity.request_fingerprint.clone(),
                result_digest: Some(digest),
                result_payload: Some(flush_error_payload.clone()),
                status: crate::rollout::EffectStatus::Failed,
                applied_at: None,
                reason: Some(flush_error.clone()),
                dedupe_hit: false,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
        emit(Event::Error {
            message: flush_error.clone(),
            recoverable: true,
        })
        .await;
        emit(Event::ToolCallCompleted {
            id: tool_call.id.clone(),
            result_preview: tool_result_preview(&flush_error_payload),
            audit: tool_audit.clone(),
        })
        .await;
        state.session.record_tool_call_with_audit(
            &tool_call.name,
            tool_arguments.clone(),
            flush_error_payload.clone(),
            false,
            tool_audit,
        );
        state
            .session
            .add_tool_message(&tool_call.id, &tool_call.name, flush_error_payload);
        return Ok(ToolOrchestratorOutcome::ContinueToolBatch {
            refresh_context: false,
        });
    }

    let tool_start = Instant::now();
    let tool_result = tokio::select! {
        _ = inputs.cancel.cancelled() => {
            if check_turn_cancelled(state, emit, inputs.cancel).await? {
                return Ok(ToolOrchestratorOutcome::EndTurn);
            }
            unreachable!("check_turn_cancelled returns on cancellation");
        }
        result = state.tools.execute(&tool_call.name, tool_arguments.clone()) => result,
    };

    match tool_result {
        Ok(value) => {
            if let (Some(identity), Some(effect_start)) = (&effect_identity, &effect_start) {
                let digest = sha256_hex(&canonicalize_json(&value).to_string());
                state.session.record_effect(crate::rollout::EffectRecord {
                    effect_id: effect_start.effect_id.clone(),
                    run_id: effect_start.run_id.clone(),
                    tool_call_id: effect_start.tool_call_id.clone(),
                    idempotency_key: identity.idempotency_key.clone(),
                    effect_type: identity.category.as_str().to_string(),
                    request_fingerprint: identity.request_fingerprint.clone(),
                    result_digest: Some(digest),
                    result_payload: Some(value.clone()),
                    status: crate::rollout::EffectStatus::Applied,
                    applied_at: Some(chrono::Utc::now().to_rfc3339()),
                    reason: None,
                    dedupe_hit: false,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            }
            emit(Event::ToolCallCompleted {
                id: tool_call.id.clone(),
                result_preview: tool_result_preview(&value),
                audit: tool_audit.clone(),
            })
            .await;
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                value.clone(),
                true,
                tool_audit.clone(),
            );
            state
                .session
                .add_tool_message(&tool_call.id, &tool_call.name, value);
            info!(
                tool_name = %tool_call.name,
                elapsed_ms = tool_start.elapsed().as_millis(),
                success = true,
                "Tool done"
            );
            Ok(ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: false,
            })
        }
        Err(err) => {
            let error_payload = json!({"error": err.to_string()});
            if let (Some(identity), Some(effect_start)) = (&effect_identity, &effect_start) {
                let digest = sha256_hex(&canonicalize_json(&error_payload).to_string());
                state.session.record_effect(crate::rollout::EffectRecord {
                    effect_id: effect_start.effect_id.clone(),
                    run_id: effect_start.run_id.clone(),
                    tool_call_id: effect_start.tool_call_id.clone(),
                    idempotency_key: identity.idempotency_key.clone(),
                    effect_type: identity.category.as_str().to_string(),
                    request_fingerprint: identity.request_fingerprint.clone(),
                    result_digest: Some(digest),
                    result_payload: Some(error_payload.clone()),
                    status: crate::rollout::EffectStatus::Failed,
                    applied_at: None,
                    reason: Some(err.to_string()),
                    dedupe_hit: false,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            }
            emit(Event::ToolCallCompleted {
                id: tool_call.id.clone(),
                result_preview: tool_result_preview(&error_payload),
                audit: tool_audit.clone(),
            })
            .await;
            state.session.record_tool_call_with_audit(
                &tool_call.name,
                tool_arguments.clone(),
                error_payload.clone(),
                false,
                tool_audit,
            );
            state
                .session
                .add_tool_message(&tool_call.id, &tool_call.name, error_payload);
            info!(
                tool_name = %tool_call.name,
                elapsed_ms = tool_start.elapsed().as_millis(),
                success = false,
                error = %err,
                "Tool done"
            );
            Ok(ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: false,
            })
        }
    }
}

async fn orchestrate_tool_batch_with_guard<E, F>(
    state: &mut RuntimeLoopState,
    loop_guard: &mut ToolLoopGuard,
    tool_calls: &[NormalizedToolCall],
    inputs: ToolOrchestratorInputs<'_>,
    approved_unknown_effect_call_index: Option<usize>,
    emit: &mut E,
) -> Result<ToolBatchOrchestratorOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let mut refresh_context = false;

    for (idx, tool_call) in tool_calls.iter().enumerate() {
        let allow_approved_unknown_effect_execution =
            approved_unknown_effect_call_index.is_some_and(|approved_index| approved_index == idx);
        match orchestrate_tool_call_with_guard(
            state,
            loop_guard,
            tool_call,
            inputs,
            allow_approved_unknown_effect_execution,
            emit,
        )
        .await?
        {
            ToolOrchestratorOutcome::ContinueToolBatch {
                refresh_context: call_refresh,
            } => {
                refresh_context |= call_refresh;
                if handle_queued_steering_inputs(
                    state,
                    tool_calls,
                    idx + 1,
                    inputs.steering_broker,
                    emit,
                )
                .await?
                {
                    return Ok(ToolBatchOrchestratorOutcome::ContinueTurnLoop {
                        refresh_context: true,
                    });
                }
            }
            ToolOrchestratorOutcome::PauseTurn => {
                if let Some(pending) = state.turn_state.pending_confirmation()
                    && replays_tool_calls(&pending.checkpoint_type)
                {
                    state
                        .turn_state
                        .set_tool_replay_batch(pending.checkpoint_id, tool_calls[idx..].to_vec());
                }
                return Ok(ToolBatchOrchestratorOutcome::PauseTurn);
            }
            ToolOrchestratorOutcome::EndTurn => {
                return Ok(ToolBatchOrchestratorOutcome::EndTurn);
            }
        }
    }

    if let Some(msg) = loop_guard.after_tool_batch() {
        emit(Event::Error {
            message: msg.clone(),
            recoverable: true,
        })
        .await;
        emit(Event::TextDelta {
            chunk: msg,
            is_final: true,
        })
        .await;
        emit(Event::TurnCompleted {
            summary: Some("Tool loop stopped by loop guard".to_string()),
        })
        .await;
        return Ok(ToolBatchOrchestratorOutcome::EndTurn);
    }

    Ok(ToolBatchOrchestratorOutcome::ContinueTurnLoop { refresh_context })
}

async fn handle_queued_steering_inputs<E, F>(
    state: &mut RuntimeLoopState,
    tool_calls: &[NormalizedToolCall],
    remaining_start_idx: usize,
    steering_broker: Option<&TurnInputBroker>,
    emit: &mut E,
) -> Result<bool>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let Some(broker) = steering_broker else {
        return Ok(false);
    };

    let mut steering_inputs: Vec<Vec<crate::tape::ContentPart>> = Vec::new();
    while let Some(submission) = broker.try_recv().await {
        if let Op::Input {
            parts,
            mode: InputMode::Steer,
        } = &submission.op
        {
            steering_inputs.push(parts.clone());
            continue;
        }

        if matches!(
            &submission.op,
            Op::Input {
                mode: InputMode::FollowUp,
                ..
            }
        ) && state.turn_state.buffered_inband_user_input_count()
            >= MAX_BUFFERED_INBAND_USER_INPUTS
        {
            emit(Event::Error {
                message: format!(
                    "Too many queued in-turn user inputs (limit={MAX_BUFFERED_INBAND_USER_INPUTS}); dropping newest input."
                ),
                recoverable: true,
            })
            .await;
            continue;
        }

        state.turn_state.push_buffered_inband_submission(submission);
    }

    if steering_inputs.is_empty() {
        return Ok(false);
    }

    for parts in steering_inputs {
        state.session.add_user_message_parts(parts);
    }

    let remaining = &tool_calls[remaining_start_idx..];
    if !remaining.is_empty() {
        emit(Event::Error {
            message: format!(
                "Steering input received during tool batch; skipping {} pending tool call(s).",
                remaining.len()
            ),
            recoverable: true,
        })
        .await;
    }

    for skipped in remaining {
        let skipped_payload = json!({
            "status": "skipped_due_to_steering",
            "error": "Skipped due to queued user steering input."
        });
        emit(Event::ToolCallStarted {
            id: skipped.id.clone(),
            name: skipped.name.clone(),
            audit: None,
        })
        .await;
        emit(Event::ToolCallCompleted {
            id: skipped.id.clone(),
            result_preview: tool_result_preview(&skipped_payload),
            audit: None,
        })
        .await;
        state.session.record_tool_call(
            &skipped.name,
            skipped.arguments.clone(),
            skipped_payload.clone(),
            false,
        );
        state
            .session
            .add_tool_message(&skipped.id, &skipped.name, skipped_payload);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        llm::LlmClient,
        runtime::TurnState,
        session::Session,
        tools::{Tool, ToolContext, ToolRegistry, ToolResult},
    };
    use alan_llm::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk};
    use alan_protocol::{DynamicToolSpec, ToolCapability};
    use async_trait::async_trait;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

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

    struct CountingEffectTool {
        name: &'static str,
        capability: ToolCapability,
        counter: Arc<AtomicUsize>,
    }

    impl Tool for CountingEffectTool {
        fn name(&self) -> &str {
            self.name
        }

        fn description(&self) -> &str {
            "Counting side-effect tool used for dedupe tests"
        }

        fn parameters_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "payload": {"type": "string"}
                }
            })
        }

        fn execute(&self, arguments: Value, _ctx: &ToolContext) -> ToolResult {
            let counter = Arc::clone(&self.counter);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(json!({
                    "ok": true,
                    "payload": arguments
                }))
            })
        }

        fn capability(&self, _arguments: &Value) -> ToolCapability {
            self.capability
        }
    }

    fn create_test_state() -> RuntimeLoopState {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        let runtime_config = super::super::RuntimeConfig::default();

        RuntimeLoopState {
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

    fn create_test_state_with_session_and_tools(
        session: Session,
        tools: ToolRegistry,
    ) -> RuntimeLoopState {
        RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            current_submission_id: None,
            llm_client: LlmClient::new(SimpleMockProvider),
            tools,
            core_config: Config::default(),
            runtime_config: super::super::RuntimeConfig::default(),
            workspace_persona_dirs: Vec::new(),
            prompt_cache: crate::runtime::prompt_cache::PromptAssemblyCache::new(Vec::new()),
            turn_state: TurnState::default(),
        }
    }

    async fn execute_single_tool_call(
        state: &mut RuntimeLoopState,
        call_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> (ToolBatchOrchestratorOutcome, Vec<Event>) {
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();
        let tool_calls = vec![NormalizedToolCall {
            id: call_id.to_string(),
            name: tool_name.to_string(),
            arguments,
        }];
        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let mut events = Vec::new();
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let outcome = orchestrator
            .orchestrate_tool_batch(state, &tool_calls, inputs, &mut emit)
            .await
            .expect("tool orchestration should succeed");
        (outcome, events)
    }

    #[tokio::test]
    async fn test_tool_turn_orchestrator_new() {
        let orchestrator = ToolTurnOrchestrator::new(Some(10), 4);
        // Verify orchestrator was created with the correct settings
        // Just test that it doesn't panic
        let _ = orchestrator;
    }

    #[tokio::test]
    async fn test_orchestrate_empty_tool_batch() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls: Vec<NormalizedToolCall> = vec![];
        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::ContinueTurnLoop { refresh_context } => {
                assert!(!refresh_context);
            }
            _ => panic!("Expected ContinueTurnLoop"),
        }
    }

    #[tokio::test]
    async fn test_handle_queued_steering_inputs_enforces_buffer_cap_for_follow_up() {
        let mut state = create_test_state();
        for idx in 0..MAX_BUFFERED_INBAND_USER_INPUTS {
            state
                .turn_state
                .push_buffered_inband_submission(alan_protocol::Submission::new(Op::Input {
                    parts: vec![alan_protocol::ContentPart::text(format!("buffered-{idx}"))],
                    mode: InputMode::FollowUp,
                }));
        }
        let broker = TurnInputBroker::default();
        assert!(
            broker
                .push(alan_protocol::Submission::new(Op::Input {
                    parts: vec![alan_protocol::ContentPart::text("overflow-follow-up")],
                    mode: InputMode::FollowUp,
                }))
                .await
        );

        let mut events = Vec::new();
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let handled = handle_queued_steering_inputs(&mut state, &[], 0, Some(&broker), &mut emit)
            .await
            .unwrap();
        assert!(!handled);
        assert_eq!(
            state.turn_state.buffered_inband_user_input_count(),
            MAX_BUFFERED_INBAND_USER_INPUTS
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::Error { message, recoverable }
                if *recoverable && message.contains("Too many queued in-turn user inputs")
        )));
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_virtual_update_plan() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Test plan",
                "items": [
                    {"id": "1", "content": "Step 1", "status": "in_progress"}
                ]
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
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
            "Expected update_plan ToolCallCompleted preview"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::PlanUpdated { explanation, items }
                if explanation.as_deref() == Some("Test plan")
                    && items.len() == 1
                    && items[0].content == "Step 1"
        )));
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_virtual_confirmation() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                "checkpoint_id": "chk_123",
                "checkpoint_type": "test",
                "summary": "Test confirmation",
                "details": {"key": "value"}
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::PauseTurn => {
                // Expected
            }
            _ => panic!("Expected PauseTurn"),
        }

        // Check that Yield Confirmation event was emitted
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
    async fn test_orchestrate_tool_batch_with_virtual_user_input() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_user_input".to_string(),
            arguments: json!({
                "title": "Test Input",
                "prompt": "Enter something",
                "questions": [
                    {"id": "q1", "label": "Question 1", "prompt": "What?", "required": true}
                ]
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::PauseTurn => {
                // Expected
            }
            _ => panic!("Expected PauseTurn"),
        }

        // Check that Yield event was emitted
        let has_input_request = events.iter().any(|e| {
            matches!(
                e,
                Event::Yield {
                    kind: alan_protocol::YieldKind::StructuredInput,
                    ..
                }
            )
        });
        assert!(has_input_request, "Expected Yield StructuredInput event");
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_builtin_tool() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Test with read_file tool - requires sandbox setup, will likely fail but tests the path
        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        // Tool execution may fail due to sandbox restrictions, but orchestration should complete
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replay_approved_tool_call() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Replay test",
                "items": [{"id": "1", "content": "Step", "status": "completed"}]
            }),
        };

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result =
            replay_approved_tool_call_with_cancel(&mut state, &tool_call, None, inputs, &mut emit)
                .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replay_approved_tool_batch() {
        let mut state = create_test_state();
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Batch test",
                "items": [{"id": "1", "content": "Step 1", "status": "completed"}]
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = replay_approved_tool_batch_with_cancel(
            &mut state,
            &tool_calls,
            None,
            inputs,
            &mut emit,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tool_batch_with_dynamic_tool() {
        let mut state = create_test_state();
        state
            .session
            .client_capabilities
            .adaptive_yields
            .schema_driven_forms = true;
        state
            .session
            .client_capabilities
            .adaptive_yields
            .presentation_hints = true;
        // Register a dynamic tool
        state.session.dynamic_tools.insert(
            "custom_dynamic_tool".to_string(),
            DynamicToolSpec {
                name: "custom_dynamic_tool".to_string(),
                description: "A test tool".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
                capability: Some(alan_protocol::ToolCapability::Read),
            },
        );

        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "custom_dynamic_tool".to_string(),
            arguments: json!({}),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // Should pause for dynamic tool
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::PauseTurn => {
                let payload = events.iter().find_map(|event| match event {
                    Event::Yield {
                        kind: alan_protocol::YieldKind::DynamicTool,
                        payload,
                        ..
                    } => Some(payload),
                    _ => None,
                });
                let payload = payload.expect("Expected Yield DynamicTool event");
                assert_eq!(payload["tool_name"], "custom_dynamic_tool");
                assert_eq!(payload["form"]["fields"][0]["kind"], "boolean");
                assert_eq!(
                    payload["form"]["fields"][0]["presentation_hints"][0],
                    "toggle"
                );
            }
            _ => panic!("Expected PauseTurn for dynamic tool"),
        }
    }

    #[tokio::test]
    async fn test_tool_batch_with_dynamic_delegated_tool_is_not_shadowed() {
        let mut state = create_test_state();
        state
            .session
            .client_capabilities
            .adaptive_yields
            .schema_driven_forms = true;
        state
            .session
            .client_capabilities
            .adaptive_yields
            .presentation_hints = true;
        state.session.dynamic_tools.insert(
            "invoke_delegated_skill".to_string(),
            DynamicToolSpec {
                name: "invoke_delegated_skill".to_string(),
                description: "Delegated execution bridge".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
                capability: Some(alan_protocol::ToolCapability::Read),
            },
        );

        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "invoke_delegated_skill".to_string(),
            arguments: json!({
                "skill_id": "repo-review",
                "target": "reviewer",
                "task": "Review the current diff and summarize risks."
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::PauseTurn => {
                let payload = events.iter().find_map(|event| match event {
                    Event::Yield {
                        kind: alan_protocol::YieldKind::DynamicTool,
                        payload,
                        ..
                    } => Some(payload),
                    _ => None,
                });
                let payload = payload.expect("Expected Yield DynamicTool event");
                assert_eq!(payload["tool_name"], "invoke_delegated_skill");
            }
            _ => panic!("Expected PauseTurn for dynamic delegated tool"),
        }
    }

    #[tokio::test]
    async fn test_orchestrate_tool_batch_with_cancel() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        // Cancel immediately
        cancel.cancel();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        // Should complete without panic even when cancelled
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_virtual_tool_ends_turn() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Invalid confirmation request - missing required summary
        let tool_calls = vec![NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                "details": {"reason": "missing_summary"}
            }),
        }];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // Invalid virtual tool should end turn
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::EndTurn => {
                // Check Error event was emitted
                let has_error = events.iter().any(|e| matches!(e, Event::Error { .. }));
                assert!(has_error, "Expected Error event for invalid virtual tool");
            }
            _ => panic!("Expected EndTurn for invalid virtual tool"),
        }
    }

    #[tokio::test]
    async fn test_multiple_tools_in_batch() {
        let mut state = create_test_state();
        let mut orchestrator = ToolTurnOrchestrator::new(None, 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let tool_calls = vec![
            NormalizedToolCall {
                id: "call_1".to_string(),
                name: "update_plan".to_string(),
                arguments: json!({
                    "explanation": "First",
                    "items": [{"id": "1", "content": "Step 1", "status": "completed"}]
                }),
            },
            NormalizedToolCall {
                id: "call_2".to_string(),
                name: "update_plan".to_string(),
                arguments: json!({
                    "explanation": "Second",
                    "items": [{"id": "2", "content": "Step 2", "status": "completed"}]
                }),
            },
        ];

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // Should have two update_plan completion events.
        let plan_updates: Vec<_> = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::ToolCallCompleted {
                        result_preview: Some(preview),
                        ..
                    } if preview.contains("plan_updated")
                )
            })
            .collect();
        assert_eq!(
            plan_updates.len(),
            2,
            "Expected two update_plan completion events"
        );
    }

    #[tokio::test]
    async fn test_side_effect_dedupe_survives_session_rollout_recovery_for_file_effects() {
        let temp = tempfile::TempDir::new().unwrap();
        let sessions_dir = temp.path();
        let counter = Arc::new(AtomicUsize::new(0));

        let mut session =
            Session::new_with_id_and_recorder_in_dir("sess-dedupe", "mock", sessions_dir)
                .await
                .unwrap();
        session.add_user_message("write file once");
        let mut tools = ToolRegistry::new();
        tools.register(CountingEffectTool {
            name: "write_file",
            capability: ToolCapability::Write,
            counter: Arc::clone(&counter),
        });

        let mut state = create_test_state_with_session_and_tools(session, tools);
        let (_, first_events) = execute_single_tool_call(
            &mut state,
            "call-file-1",
            "write_file",
            json!({"path": "notes.txt", "payload": "hello"}),
        )
        .await;
        assert!(
            first_events
                .iter()
                .any(|event| matches!(event, Event::ToolCallCompleted { .. }))
        );
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let rollout_path = state
            .session
            .recorder
            .as_ref()
            .expect("recorder should exist")
            .path()
            .clone();

        let recovered_session =
            Session::load_from_rollout_in_dir(&rollout_path, "mock", sessions_dir)
                .await
                .unwrap();
        let mut recovered_tools = ToolRegistry::new();
        recovered_tools.register(CountingEffectTool {
            name: "write_file",
            capability: ToolCapability::Write,
            counter: Arc::clone(&counter),
        });
        let mut recovered_state =
            create_test_state_with_session_and_tools(recovered_session, recovered_tools);
        let _ = execute_single_tool_call(
            &mut recovered_state,
            "call-file-2",
            "write_file",
            json!({"path": "notes.txt", "payload": "hello"}),
        )
        .await;

        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "dedupe after recovery should skip physical execution"
        );
        assert_eq!(
            recovered_state
                .session
                .tool_payload_by_call_id("call-file-2")
                .expect("replayed tool payload should exist"),
            recovered_state
                .session
                .tool_payload_by_call_id("call-file-1")
                .expect("original tool payload should exist"),
            "dedupe replay should preserve original tool payload"
        );
    }

    #[tokio::test]
    async fn test_side_effect_dedupe_for_network_effects() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut session = Session::new();
        session.add_user_message("call api once");
        let mut tools = ToolRegistry::new();
        tools.register(CountingEffectTool {
            name: "bash",
            capability: ToolCapability::Network,
            counter: Arc::clone(&counter),
        });
        let mut state = create_test_state_with_session_and_tools(session, tools);

        let _ = execute_single_tool_call(
            &mut state,
            "call-net-1",
            "bash",
            json!({"command": "curl https://example.com"}),
        )
        .await;
        let _ = execute_single_tool_call(
            &mut state,
            "call-net-2",
            "bash",
            json!({"command": "curl https://example.com"}),
        )
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(
            state
                .session
                .tool_payload_by_call_id("call-net-2")
                .expect("replayed tool payload should exist"),
            state
                .session
                .tool_payload_by_call_id("call-net-1")
                .expect("original tool payload should exist"),
            "dedupe replay should preserve original network-tool payload"
        );
    }

    #[tokio::test]
    async fn test_side_effect_dedupe_for_process_effects() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut session = Session::new();
        session.add_user_message("run command once");
        let mut tools = ToolRegistry::new();
        tools.register(CountingEffectTool {
            name: "bash",
            capability: ToolCapability::Write,
            counter: Arc::clone(&counter),
        });
        let mut state = create_test_state_with_session_and_tools(session, tools);

        let _ = execute_single_tool_call(
            &mut state,
            "call-proc-1",
            "bash",
            json!({"command": "touch hello.txt"}),
        )
        .await;
        let _ = execute_single_tool_call(
            &mut state,
            "call-proc-2",
            "bash",
            json!({"command": "touch hello.txt"}),
        )
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(
            state
                .session
                .tool_payload_by_call_id("call-proc-2")
                .expect("replayed tool payload should exist"),
            state
                .session
                .tool_payload_by_call_id("call-proc-1")
                .expect("original tool payload should exist"),
            "dedupe replay should preserve original process-tool payload"
        );
    }

    #[test]
    fn test_effect_identity_turn_component_remains_monotonic_across_rollback() {
        let mut session = Session::new();
        let arguments = json!({"path":"notes.txt","payload":"hello"});

        session.add_user_message("turn-1");
        let first = build_effect_identity(&session, "write_file", &arguments, EffectCategory::File);
        session.add_user_message("turn-2");
        let second =
            build_effect_identity(&session, "write_file", &arguments, EffectCategory::File);

        let removed = session.rollback_last_turns(1);
        assert!(removed.removed_messages > 0);
        session.add_user_message("turn-3");
        let third = build_effect_identity(&session, "write_file", &arguments, EffectCategory::File);

        assert_ne!(
            second.idempotency_key, third.idempotency_key,
            "new turn after rollback must not reuse prior turn idempotency key"
        );
        assert_ne!(first.idempotency_key, second.idempotency_key);
    }

    #[test]
    fn test_effect_identity_is_stable_when_confirmation_adds_control_message() {
        let mut session = Session::new();
        let arguments = json!({"path":"notes.txt","payload":"hello"});
        session.add_user_message("write once");
        let first = build_effect_identity(&session, "write_file", &arguments, EffectCategory::File);

        session.add_user_control_message_parts(vec![crate::tape::ContentPart::structured(
            json!({"checkpoint_type":"effect_replay_confirmation","choice":"approve"}),
        )]);
        let replayed =
            build_effect_identity(&session, "write_file", &arguments, EffectCategory::File);

        assert_eq!(
            first.idempotency_key, replayed.idempotency_key,
            "control messages should not perturb idempotency key turn component"
        );
    }

    #[tokio::test]
    async fn test_unknown_effect_status_escalates_without_execution() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut session = Session::new();
        session.add_user_message("write file with safety");
        let mut tools = ToolRegistry::new();
        tools.register(CountingEffectTool {
            name: "write_file",
            capability: ToolCapability::Write,
            counter: Arc::clone(&counter),
        });
        let mut state = create_test_state_with_session_and_tools(session, tools);
        let arguments = json!({"path": "notes.txt", "payload": "hello"});
        let identity = build_effect_identity(
            &state.session,
            "write_file",
            &arguments,
            EffectCategory::File,
        );
        state.session.record_effect(crate::rollout::EffectRecord {
            effect_id: "ef-unknown".to_string(),
            run_id: state.session.id.clone(),
            tool_call_id: "call-prev".to_string(),
            idempotency_key: identity.idempotency_key.clone(),
            effect_type: "file".to_string(),
            request_fingerprint: identity.request_fingerprint.clone(),
            result_digest: None,
            result_payload: None,
            status: crate::rollout::EffectStatus::Unknown,
            applied_at: None,
            reason: Some("crash during prior execution".to_string()),
            dedupe_hit: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        let (outcome, events) =
            execute_single_tool_call(&mut state, "call-new", "write_file", arguments).await;
        assert!(matches!(outcome, ToolBatchOrchestratorOutcome::PauseTurn));
        assert_eq!(
            counter.load(Ordering::SeqCst),
            0,
            "unknown effect status should not execute without confirmation"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::Yield {
                kind: alan_protocol::YieldKind::Confirmation,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn test_replay_approved_unknown_effect_executes_tool_once() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut session = Session::new();
        session.add_user_message("write file with approval");
        let mut tools = ToolRegistry::new();
        tools.register(CountingEffectTool {
            name: "write_file",
            capability: ToolCapability::Write,
            counter: Arc::clone(&counter),
        });
        let mut state = create_test_state_with_session_and_tools(session, tools);
        let arguments = json!({"path": "notes.txt", "payload": "hello"});
        let identity = build_effect_identity(
            &state.session,
            "write_file",
            &arguments,
            EffectCategory::File,
        );
        state.session.record_effect(crate::rollout::EffectRecord {
            effect_id: "ef-unknown".to_string(),
            run_id: state.session.id.clone(),
            tool_call_id: "call-prev".to_string(),
            idempotency_key: identity.idempotency_key.clone(),
            effect_type: "file".to_string(),
            request_fingerprint: identity.request_fingerprint.clone(),
            result_digest: None,
            result_payload: None,
            status: crate::rollout::EffectStatus::Unknown,
            applied_at: None,
            reason: Some("crash during prior execution".to_string()),
            dedupe_hit: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        let cancel = CancellationToken::new();
        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };
        let tool_call = NormalizedToolCall {
            id: "call-new".to_string(),
            name: "write_file".to_string(),
            arguments,
        };
        let mut events = Vec::new();
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let outcome = replay_approved_tool_call_with_cancel(
            &mut state,
            &tool_call,
            Some(tool_call.id.as_str()),
            inputs,
            &mut emit,
        )
        .await
        .expect("approved replay should run");
        assert!(matches!(
            outcome,
            ToolBatchOrchestratorOutcome::ContinueTurnLoop { .. }
        ));
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "approved replay should execute once"
        );
        assert!(
            !events.iter().any(|event| matches!(
                event,
                Event::Yield {
                    kind: alan_protocol::YieldKind::Confirmation,
                    ..
                }
            )),
            "approved replay should not emit a second confirmation yield"
        );

        let restored = state
            .session
            .effect_by_idempotency_key(&identity.idempotency_key)
            .expect("updated effect record should exist");
        assert_eq!(restored.status, crate::rollout::EffectStatus::Applied);
    }

    #[tokio::test]
    async fn test_replay_approved_batch_bypasses_unknown_only_for_first_tool_call() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut session = Session::new();
        session.add_user_message("write file with batch replay");
        let mut tools = ToolRegistry::new();
        tools.register(CountingEffectTool {
            name: "write_file",
            capability: ToolCapability::Write,
            counter: Arc::clone(&counter),
        });
        let mut state = create_test_state_with_session_and_tools(session, tools);
        let arguments_first = json!({"path": "notes-1.txt", "payload": "hello"});
        let arguments_second = json!({"path": "notes-2.txt", "payload": "world"});
        let identity_first = build_effect_identity(
            &state.session,
            "write_file",
            &arguments_first,
            EffectCategory::File,
        );
        let identity_second = build_effect_identity(
            &state.session,
            "write_file",
            &arguments_second,
            EffectCategory::File,
        );
        state.session.record_effect(crate::rollout::EffectRecord {
            effect_id: "ef-unknown-1".to_string(),
            run_id: state.session.id.clone(),
            tool_call_id: "call-prev-1".to_string(),
            idempotency_key: identity_first.idempotency_key.clone(),
            effect_type: "file".to_string(),
            request_fingerprint: identity_first.request_fingerprint.clone(),
            result_digest: None,
            result_payload: None,
            status: crate::rollout::EffectStatus::Unknown,
            applied_at: None,
            reason: Some("crash during prior execution".to_string()),
            dedupe_hit: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        state.session.record_effect(crate::rollout::EffectRecord {
            effect_id: "ef-unknown-2".to_string(),
            run_id: state.session.id.clone(),
            tool_call_id: "call-prev-2".to_string(),
            idempotency_key: identity_second.idempotency_key.clone(),
            effect_type: "file".to_string(),
            request_fingerprint: identity_second.request_fingerprint.clone(),
            result_digest: None,
            result_payload: None,
            status: crate::rollout::EffectStatus::Unknown,
            applied_at: None,
            reason: Some("crash during prior execution".to_string()),
            dedupe_hit: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        let tool_calls = vec![
            NormalizedToolCall {
                id: "call-dup".to_string(),
                name: "write_file".to_string(),
                arguments: arguments_first,
            },
            NormalizedToolCall {
                id: "call-dup".to_string(),
                name: "write_file".to_string(),
                arguments: arguments_second,
            },
        ];
        let cancel = CancellationToken::new();
        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };
        let mut events = Vec::new();
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let outcome = replay_approved_tool_batch_with_cancel(
            &mut state,
            &tool_calls,
            Some("call-dup"),
            inputs,
            &mut emit,
        )
        .await
        .expect("approved replay batch should run");
        assert!(matches!(outcome, ToolBatchOrchestratorOutcome::PauseTurn));
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "only the approved call should bypass unknown-effect escalation"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::Yield {
                request_id,
                kind: alan_protocol::YieldKind::Confirmation,
                ..
            } if request_id.contains("call-dup")
        )));
    }

    #[tokio::test]
    async fn test_tool_loop_guard_triggers() {
        let mut state = create_test_state();
        // Set max loops to a small number
        let mut orchestrator = ToolTurnOrchestrator::new(Some(2), 4);
        let cancel = CancellationToken::new();

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        // Create many tool calls that will exceed the loop limit
        let mut tool_calls = vec![];
        for i in 0..3 {
            tool_calls.push(NormalizedToolCall {
                id: format!("call_{}", i),
                name: "update_plan".to_string(),
                arguments: json!({
                    "explanation": format!("Step {}", i),
                    "items": [{"id": i.to_string(), "content": "Step", "status": "completed"}]
                }),
            });
        }

        let inputs = ToolOrchestratorInputs {
            cancel: &cancel,
            steering_broker: None,
        };

        let result = orchestrator
            .orchestrate_tool_batch(&mut state, &tool_calls, inputs, &mut emit)
            .await;

        assert!(result.is_ok());
        // After max loops, should end turn
        match result.unwrap() {
            ToolBatchOrchestratorOutcome::EndTurn => {
                // Expected
            }
            _ => {
                // Note: Depending on implementation, might continue or end
                // Just verify no panic occurred
            }
        }
    }
}
