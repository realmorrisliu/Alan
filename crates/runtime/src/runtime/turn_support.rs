use alan_protocol::Event;
use anyhow::Result;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

use crate::llm::LlmClient;

use super::agent_loop::{NormalizedToolCall, RuntimeLoopState};

pub(super) async fn cancel_current_task<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
) -> Result<()>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    warn!("Cancelling current task");
    // Clear turn-scoped pending state, but preserve session history so the user can
    // continue the same conversation after an interrupt/cancel.
    state.turn_state.clear();
    state.session.has_active_task = false;
    emit(Event::TaskCompleted {
        summary: "Task cancelled by user".to_string(),
        results: json!({"status": "cancelled"}),
    })
    .await;
    emit(Event::TurnCompleted { summary: None }).await;
    Ok(())
}

pub(super) async fn emit_task_completed_success<E, F>(emit: &mut E, summary: impl Into<String>)
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let summary = summary.into();
    emit(Event::TaskCompleted {
        summary: summary.clone(),
        results: json!({
            "status": "completed",
            "summary": summary
        }),
    })
    .await;
    emit(Event::TurnCompleted { summary: None }).await;
}

pub(super) fn normalize_tool_calls(
    tool_calls: Vec<crate::llm::ToolCall>,
) -> Vec<NormalizedToolCall> {
    let fallback_prefix = format!("tool_call_{}", Uuid::new_v4().simple());

    tool_calls
        .into_iter()
        .enumerate()
        .map(|(index, tc)| {
            let id = tc
                .id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{fallback_prefix}_{index}"));

            NormalizedToolCall {
                id,
                name: tc.name,
                arguments: tc.arguments,
            }
        })
        .collect()
}

pub(super) fn parse_plan_status(raw: &str) -> Option<alan_protocol::PlanItemStatus> {
    match raw {
        "pending" | "blocked" => Some(alan_protocol::PlanItemStatus::Pending),
        "in_progress" => Some(alan_protocol::PlanItemStatus::InProgress),
        "completed" | "skipped" => Some(alan_protocol::PlanItemStatus::Completed),
        _ => None,
    }
}

pub(super) fn parse_plan_items(value: &serde_json::Value) -> Option<Vec<alan_protocol::PlanItem>> {
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

pub(super) fn plan_update_from_todo_result(
    arguments: &serde_json::Value,
    result: &serde_json::Value,
) -> Option<(Option<String>, Vec<alan_protocol::PlanItem>)> {
    let items = parse_plan_items(result.get("items")?)?;
    let action = arguments
        .get("action")
        .and_then(|v| v.as_str())
        .map(|s| format!("todo_list {}", s));
    Some((action, items))
}

pub(super) fn detect_provider(llm_client: &LlmClient) -> &'static str {
    if llm_client.is_gemini() {
        "gemini"
    } else if llm_client.is_anthropic() {
        "anthropic_compatible"
    } else if llm_client.is_openai() {
        "openai_compatible"
    } else {
        "unknown"
    }
}

pub(super) fn split_text_for_typing(text: &str) -> Vec<String> {
    const TARGET_CHUNK_CHARS: usize = 32;

    if text.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;

    for ch in text.chars() {
        current.push(ch);
        current_len += 1;

        let boundary = ch.is_whitespace() || [',', '.', '!', '?', ';', ':'].contains(&ch);
        if current_len >= TARGET_CHUNK_CHARS && boundary {
            chunks.push(std::mem::take(&mut current));
            current_len = 0;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

pub(super) async fn emit_streaming_chunks<E, F>(emit: &mut E, content: &str)
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let chunks = split_text_for_typing(content);
    for chunk in &chunks {
        emit(Event::TextDelta {
            chunk: chunk.clone(),
            is_final: false,
        })
        .await;
    }
    emit(Event::TextDelta {
        chunk: String::new(),
        is_final: true,
    })
    .await;
}

pub(super) async fn emit_thinking_chunks<E, F>(emit: &mut E, thinking: &str)
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    let chunks = split_text_for_typing(thinking);
    for chunk in &chunks {
        emit(Event::ThinkingDelta {
            chunk: chunk.clone(),
            is_final: false,
        })
        .await;
    }
    emit(Event::ThinkingDelta {
        chunk: String::new(),
        is_final: true,
    })
    .await;
}

pub(super) async fn check_turn_cancelled<E, F>(
    state: &mut RuntimeLoopState,
    emit: &mut E,
    cancel: &CancellationToken,
) -> Result<bool>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    if !cancel.is_cancelled() {
        return Ok(false);
    }
    if !state.turn_state.is_turn_active() && !state.turn_state.has_pending_interaction() {
        emit(Event::Error {
            message: "No active turn to cancel.".to_string(),
            recoverable: true,
        })
        .await;
        return Ok(false);
    }
    cancel_current_task(state, emit).await?;
    Ok(true)
}
