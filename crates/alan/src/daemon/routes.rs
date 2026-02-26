//! REST and streaming route handlers for agentd.

use std::{
    convert::Infallible,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use alan_protocol::{Event, EventEnvelope, Submission};
use alan_runtime::{RolloutItem, RolloutRecorder};
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderValue, Response, StatusCode, header},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info, warn};

use super::state::AppState;

/// Health check response
pub async fn health() -> &'static str {
    "OK"
}

/// Response for session creation
#[derive(Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub websocket_url: String,
    pub events_url: String,
    pub submit_url: String,
    pub approval_policy: alan_protocol::ApprovalPolicy,
    pub sandbox_mode: alan_protocol::SandboxMode,
}

#[derive(Deserialize, Default)]
pub struct CreateSessionRequest {
    /// Optional workspace directory override for this agent session (agentd-local path)
    pub workspace_dir: Option<PathBuf>,
    /// Optional approval policy override
    pub approval_policy: Option<alan_protocol::ApprovalPolicy>,
    /// Optional sandbox mode override
    pub sandbox_mode: Option<alan_protocol::SandboxMode>,
}

/// Create a new session
pub async fn create_session(
    State(state): State<AppState>,
    payload: Option<Json<CreateSessionRequest>>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (workspace_dir, approval_policy, sandbox_mode) = payload
        .map(|Json(req)| {
            (
                req.workspace_dir.filter(|p| !p.as_os_str().is_empty()),
                req.approval_policy,
                req.sandbox_mode,
            )
        })
        .unwrap_or((None, None, None));

    let session_id = state
        .create_session_from_rollout(workspace_dir, None, approval_policy, sandbox_mode)
        .await
        .map_err(|err| {
            warn!(error = %err, "Failed to create session");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{:#}", err) })),
            )
        })?;
    info!(%session_id, "Created new session");

    Ok(Json(CreateSessionResponse {
        websocket_url: format!("/api/v1/sessions/{}/ws", session_id),
        events_url: format!("/api/v1/sessions/{}/events", session_id),
        submit_url: format!("/api/v1/sessions/{}/submit", session_id),
        session_id,
        approval_policy: approval_policy.unwrap_or_default(),
        sandbox_mode: sandbox_mode.unwrap_or_default(),
    }))
}

/// Get session info
#[derive(Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub active: bool,
    pub approval_policy: alan_protocol::ApprovalPolicy,
    pub sandbox_mode: alan_protocol::SandboxMode,
}

#[derive(Serialize)]
pub struct SessionListItem {
    pub session_id: String,
    pub workspace_id: String,
    pub active: bool,
    pub approval_policy: alan_protocol::ApprovalPolicy,
    pub sandbox_mode: alan_protocol::SandboxMode,
}

#[derive(Serialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionListItem>,
}

#[derive(Serialize)]
pub struct SessionReadResponse {
    pub session_id: String,
    pub workspace_id: String,
    pub active: bool,
    pub approval_policy: alan_protocol::ApprovalPolicy,
    pub sandbox_mode: alan_protocol::SandboxMode,
    pub rollout_path: Option<String>,
    pub messages: Vec<SessionHistoryMessage>,
}

#[derive(Serialize)]
pub struct ResumeSessionResponse {
    pub session_id: String,
    pub resumed: bool,
}

#[derive(Deserialize, Default)]
pub struct ForkSessionRequest {
    pub workspace_dir: Option<PathBuf>,
    pub approval_policy: Option<alan_protocol::ApprovalPolicy>,
    pub sandbox_mode: Option<alan_protocol::SandboxMode>,
}

#[derive(Serialize)]
pub struct ForkSessionResponse {
    pub session_id: String,
    pub forked_from_session_id: String,
    pub websocket_url: String,
    pub events_url: String,
    pub submit_url: String,
    pub approval_policy: alan_protocol::ApprovalPolicy,
    pub sandbox_mode: alan_protocol::SandboxMode,
}

#[derive(Deserialize)]
pub struct RollbackSessionRequest {
    pub num_turns: u32,
}

#[derive(Serialize)]
pub struct RollbackSessionResponse {
    pub submission_id: String,
    pub accepted: bool,
}

#[derive(Serialize)]
pub struct CompactSessionResponse {
    pub submission_id: String,
    pub accepted: bool,
}

#[derive(Serialize)]
pub struct SessionHistoryMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct SessionHistoryResponse {
    pub session_id: String,
    pub messages: Vec<SessionHistoryMessage>,
}

#[derive(Deserialize, Default)]
pub struct ReadEventsQuery {
    pub after_event_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct ReadEventsResponse {
    pub session_id: String,
    pub gap: bool,
    pub oldest_event_id: Option<String>,
    pub latest_event_id: Option<String>,
    pub events: Vec<EventEnvelope>,
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SessionInfo>, StatusCode> {
    debug!(%id, "Getting session info");

    if state.get_session(&id).await.is_some() {
        let (approval_policy, sandbox_mode) = {
            let sessions = state.sessions.read().await;
            let Some(entry) = sessions.get(&id) else {
                return Err(StatusCode::NOT_FOUND);
            };
            (entry.approval_policy, entry.sandbox_mode)
        };
        Ok(Json(SessionInfo {
            session_id: id,
            active: true,
            approval_policy,
            sandbox_mode,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// List active sessions known to agentd.
pub async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<SessionListResponse>, StatusCode> {
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(error = %err, "Failed to recover sessions before listing");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let sessions = state.sessions.read().await;
    let mut data: Vec<SessionListItem> = sessions
        .iter()
        .map(|(session_id, entry)| SessionListItem {
            session_id: session_id.clone(),
            workspace_id: entry.workspace_id.clone(),
            active: true,
            approval_policy: entry.approval_policy,
            sandbox_mode: entry.sandbox_mode,
        })
        .collect();
    data.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(Json(SessionListResponse { sessions: data }))
}

/// Read session metadata and persisted message history in one response.
pub async fn read_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionReadResponse>, StatusCode> {
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to recover sessions before read");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let (workspace_id, approval_policy, sandbox_mode, rollout_path) = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&session_id) else {
            return Err(StatusCode::NOT_FOUND);
        };
        (
            entry.workspace_id.clone(),
            entry.approval_policy,
            entry.sandbox_mode,
            entry
                .rollout_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
        )
    };

    let Json(history) = get_session_history(State(state.clone()), Path(session_id.clone())).await?;
    Ok(Json(SessionReadResponse {
        session_id,
        workspace_id,
        active: true,
        approval_policy,
        sandbox_mode,
        rollout_path,
        messages: history.messages,
    }))
}

/// Resume (ensure running) the runtime for a known session.
pub async fn resume_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ResumeSessionResponse>, StatusCode> {
    if state.get_session(&session_id).await.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    state.touch_session_inbound(&session_id).await;
    state
        .resume_session_runtime(&session_id)
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to resume session runtime");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(ResumeSessionResponse {
        session_id,
        resumed: true,
    }))
}

/// Fork a session by starting a new runtime seeded from the source rollout.
pub async fn fork_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    payload: Option<Json<ForkSessionRequest>>,
) -> Result<Json<ForkSessionResponse>, StatusCode> {
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to recover sessions before fork");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let (source_workspace_id, source_approval_policy, source_sandbox_mode, stored_rollout_path) = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&session_id) else {
            return Err(StatusCode::NOT_FOUND);
        };
        (
            entry.workspace_id.clone(),
            entry.approval_policy,
            entry.sandbox_mode,
            entry.rollout_path.clone(),
        )
    };

    state.touch_session_inbound(&session_id).await;

    let JsonLikeFork {
        workspace_dir,
        approval_policy,
        sandbox_mode,
    } = JsonLikeFork::from(payload);
    let effective_approval_policy = approval_policy.unwrap_or(source_approval_policy);
    let effective_sandbox_mode = sandbox_mode.unwrap_or(source_sandbox_mode);

    let rollout_path = if let Some(path) = stored_rollout_path {
        if path.exists() {
            Some(path)
        } else {
            latest_rollout_path_for_workspace(&state, &session_id, &source_workspace_id).await?
        }
    } else {
        latest_rollout_path_for_workspace(&state, &session_id, &source_workspace_id).await?
    };

    let Some(rollout_path) = rollout_path else {
        return Err(StatusCode::CONFLICT);
    };

    let new_session_id = state
        .create_session_from_rollout(
            workspace_dir,
            Some(rollout_path),
            Some(effective_approval_policy),
            Some(effective_sandbox_mode),
        )
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to fork session");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ForkSessionResponse {
        websocket_url: format!("/api/v1/sessions/{}/ws", new_session_id),
        events_url: format!("/api/v1/sessions/{}/events", new_session_id),
        submit_url: format!("/api/v1/sessions/{}/submit", new_session_id),
        session_id: new_session_id,
        forked_from_session_id: session_id,
        approval_policy: effective_approval_policy,
        sandbox_mode: effective_sandbox_mode,
    }))
}

/// Get persisted message history for a session (final messages only; no streaming deltas)
pub async fn get_session_history(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionHistoryResponse>, StatusCode> {
    debug!(%session_id, "Getting session message history");
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to recover sessions before history read");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (workspace_id, stored_rollout_path) = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session) => (session.workspace_id.clone(), session.rollout_path.clone()),
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    state.touch_session_inbound(&session_id).await;

    let rollout_path = if let Some(path) = stored_rollout_path {
        if path.exists() {
            Some(path)
        } else {
            let refreshed =
                latest_rollout_path_for_workspace(&state, &session_id, &workspace_id).await?;
            state
                .set_session_rollout_path(&session_id, refreshed.clone())
                .await;
            refreshed
        }
    } else {
        let refreshed =
            latest_rollout_path_for_workspace(&state, &session_id, &workspace_id).await?;
        state
            .set_session_rollout_path(&session_id, refreshed.clone())
            .await;
        refreshed
    };

    let messages = match rollout_path {
        Some(rollout_path) => {
            let items = RolloutRecorder::load_history(&rollout_path)
                .await
                .map_err(|err| {
                    warn!(
                        %session_id,
                        %workspace_id,
                        path = %rollout_path.display(),
                        error = %err,
                        "Failed to read rollout history"
                    );
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            rollout_items_to_history_messages(items)
        }
        None => Vec::new(),
    };

    Ok(Json(SessionHistoryResponse {
        session_id,
        messages,
    }))
}

async fn latest_rollout_path_for_workspace(
    state: &AppState,
    session_id: &str,
    _workspace_id: &str,
) -> Result<Option<PathBuf>, StatusCode> {
    let sessions_dir = match state.get_sessions_dir(session_id).await {
        Some(dir) => dir,
        None => {
            warn!(%session_id, "Session not found when looking up sessions directory");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    latest_rollout_path(&sessions_dir).map_err(|err| {
        warn!(
            %session_id,
            path = %sessions_dir.display(),
            error = %err,
            "Failed to inspect sessions directory for history"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// Delete (destroy) a session and its backing runtime
pub async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    debug!(%id, "Deleting session");
    state
        .remove_session(&id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|err| {
            warn!(session_id = %id, error = %err, "Failed to delete session");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Request body for submitting an operation
#[derive(Deserialize)]
pub struct SubmitRequest {
    pub op: alan_protocol::Op,
}

/// Response for operation submission
#[derive(Serialize)]
pub struct SubmitResponse {
    pub submission_id: String,
    pub accepted: bool,
}

/// Submit an operation to a session
pub async fn submit_operation(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<SubmitRequest>,
) -> Result<Json<SubmitResponse>, StatusCode> {
    debug!(%session_id, "Submitting operation");
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to recover sessions before submit");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let submission_tx = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session) => session.submission_tx.clone(),
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    state.touch_session_inbound(&session_id).await;

    let submission = Submission::new(request.op);
    let submission_id = submission.id.clone();

    if submission_tx.send(submission.clone()).await.is_ok() {
        return Ok(Json(SubmitResponse {
            submission_id,
            accepted: true,
        }));
    }

    // Recovery path: placeholder channel after agentd restart. Resume runtime and retry once.
    if let Err(err) = state.resume_session_runtime(&session_id).await {
        warn!(%session_id, error = %err, "Failed to resume runtime after submit channel send failure");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let retry_tx = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session) => session.submission_tx.clone(),
            None => return Err(StatusCode::NOT_FOUND),
        }
    };
    if retry_tx.send(submission).await.is_ok() {
        Ok(Json(SubmitResponse {
            submission_id,
            accepted: true,
        }))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Request manual context compaction for a session.
pub async fn compact_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<CompactSessionResponse>, StatusCode> {
    let Json(resp) = submit_operation(
        State(state),
        Path(session_id),
        Json(SubmitRequest {
            op: alan_protocol::Op::Compact,
        }),
    )
    .await?;
    Ok(Json(CompactSessionResponse {
        submission_id: resp.submission_id,
        accepted: resp.accepted,
    }))
}

/// Request in-memory rollback of the latest N turns for a session.
pub async fn rollback_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<RollbackSessionRequest>,
) -> Result<Json<RollbackSessionResponse>, StatusCode> {
    if request.num_turns == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let Json(resp) = submit_operation(
        State(state),
        Path(session_id),
        Json(SubmitRequest {
            op: alan_protocol::Op::Rollback {
                num_turns: request.num_turns,
            },
        }),
    )
    .await?;
    Ok(Json(RollbackSessionResponse {
        submission_id: resp.submission_id,
        accepted: resp.accepted,
    }))
}

/// Read buffered transport events with cursor-based replay semantics.
pub async fn read_events(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    query: Query<ReadEventsQuery>,
) -> Result<Json<ReadEventsResponse>, StatusCode> {
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to recover sessions before event replay read");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let Query(query) = query;
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);

    let event_log = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&session_id) else {
            return Err(StatusCode::NOT_FOUND);
        };
        Arc::clone(&entry.event_log)
    };

    state.touch_session_inbound(&session_id).await;

    let page = {
        let guard = event_log.read().await;
        guard.read_after(query.after_event_id.as_deref(), limit)
    };

    Ok(Json(ReadEventsResponse {
        session_id,
        gap: page.gap,
        oldest_event_id: page.oldest_event_id,
        latest_event_id: page.latest_event_id,
        events: page.events,
    }))
}

/// Stream agent events as NDJSON (`application/x-ndjson`)
pub async fn stream_events(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Response<Body>, StatusCode> {
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to recover sessions before streaming events");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let _ = state.resume_session_runtime(&session_id).await;

    let mut events_rx = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session) => session.events_tx.subscribe(),
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(256);
    let state_clone = state.clone();
    let session_id_clone = session_id.clone();

    tokio::spawn(async move {
        let mut last_event_id: Option<String> = None;
        loop {
            match events_rx.recv().await {
                Ok(envelope) => {
                    state_clone.touch_session_outbound(&session_id_clone).await;
                    last_event_id = Some(envelope.event_id.clone());
                    let mut payload = match serde_json::to_vec(&envelope) {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            warn!(session_id = %session_id_clone, error = %err, "Failed to serialize event");
                            continue;
                        }
                    };
                    payload.push(b'\n');
                    if tx.send(Ok(Bytes::from(payload))).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    warn!(session_id = %session_id_clone, missed = count, "Event stream lagged");
                    let lagged =
                        stream_lagged_envelope(&session_id_clone, count, last_event_id.clone());
                    let mut payload = match serde_json::to_vec(&lagged) {
                        Ok(bytes) => bytes,
                        Err(_) => break,
                    };
                    payload.push(b'\n');
                    let _ = tx.send(Ok(Bytes::from(payload))).await;
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let body = Body::from_stream(ReceiverStream::new(rx));
    let mut response = Response::new(body);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}

fn stream_lagged_envelope(
    session_id: &str,
    skipped: u64,
    replay_from_event_id: Option<String>,
) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("control_lagged_{}", uuid::Uuid::new_v4()),
        sequence: 0,
        session_id: session_id.to_string(),
        submission_id: None,
        turn_id: "turn_control".to_string(),
        item_id: "item_control".to_string(),
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        event: Event::StreamLagged {
            skipped,
            replay_from_event_id,
        },
    }
}

fn latest_rollout_path(sessions_dir: &FsPath) -> std::io::Result<Option<PathBuf>> {
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<(SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(sessions_dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_jsonl = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("jsonl"))
            .unwrap_or(false);
        if !is_jsonl {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);

        match &latest {
            Some((best_time, best_path))
                if modified < *best_time || (modified == *best_time && path <= *best_path) => {}
            _ => latest = Some((modified, path)),
        }
    }

    Ok(latest.map(|(_, path)| path))
}

fn rollout_items_to_history_messages(items: Vec<RolloutItem>) -> Vec<SessionHistoryMessage> {
    items
        .into_iter()
        .filter_map(|item| match item {
            RolloutItem::Message(msg) => {
                let role = msg.role;
                if role != "user" && role != "assistant" && role != "tool" {
                    return None;
                }

                let content = msg.content.unwrap_or_default();
                if content.trim().is_empty() {
                    return None;
                }

                Some(SessionHistoryMessage {
                    role,
                    content,
                    tool_name: msg.tool_name,
                    timestamp: msg.timestamp,
                })
            }
            _ => None,
        })
        .collect()
}

struct JsonLikeFork {
    workspace_dir: Option<PathBuf>,
    approval_policy: Option<alan_protocol::ApprovalPolicy>,
    sandbox_mode: Option<alan_protocol::SandboxMode>,
}

impl JsonLikeFork {
    fn from(payload: Option<Json<ForkSessionRequest>>) -> Self {
        payload
            .map(|Json(req)| Self {
                workspace_dir: req.workspace_dir.filter(|p| !p.as_os_str().is_empty()),
                approval_policy: req.approval_policy,
                sandbox_mode: req.sandbox_mode,
            })
            .unwrap_or(Self {
                workspace_dir: None,
                approval_policy: None,
                sandbox_mode: None,
            })
    }
}

#[cfg(test)]
mod tests {

    use super::super::state::{SessionEntry, SessionEventLog};
    use super::*;
    use alan_protocol::{Event, Op};
    use alan_runtime::{
        Config, MessageRecord,
        runtime::{RuntimeEventEnvelope, WorkspaceRuntimeConfig},
    };
    use axum::body::to_bytes;

    fn runtime_event(event: Event) -> RuntimeEventEnvelope {
        RuntimeEventEnvelope {
            submission_id: Some("sub-test".to_string()),
            event,
        }
    }

    fn test_state() -> AppState {
        let base_dir =
            std::env::temp_dir().join(format!("agentd-routes-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).unwrap();

        // Create test resolver and runtime manager
        let resolver = crate::daemon::workspace_resolver::WorkspaceResolver::with_registry(
            crate::registry::WorkspaceRegistry {
                version: 1,
                workspaces: vec![],
            },
            base_dir.clone(),
        );
        let runtime_manager = crate::daemon::runtime_manager::RuntimeManager::with_template(
            WorkspaceRuntimeConfig::from(Config::default()),
        );
        let store = std::sync::Arc::new(
            crate::daemon::session_store::SessionStore::with_dir(base_dir.join("sessions"))
                .unwrap(),
        );

        AppState::from_parts(
            Config::default(),
            std::sync::Arc::new(resolver),
            std::sync::Arc::new(runtime_manager),
            store,
            3600,
        )
    }

    fn session_entry(
        workspace_path: &std::path::Path,
    ) -> (SessionEntry, mpsc::Receiver<Submission>) {
        let (submission_tx, submission_rx) = mpsc::channel(8);
        let (events_tx, _) = broadcast::channel(8);
        let event_log = Arc::new(tokio::sync::RwLock::new(SessionEventLog::new(32)));
        let entry = SessionEntry::new(
            workspace_path.to_path_buf(),
            alan_protocol::ApprovalPolicy::OnRequest,
            alan_protocol::SandboxMode::WorkspaceWrite,
            submission_tx,
            events_tx,
            event_log,
            None,
            None,
        );
        (entry, submission_rx)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        assert_eq!(health().await, "OK");
    }

    #[tokio::test]
    async fn create_session_returns_500_when_runtime_cannot_start() {
        let state = test_state();
        let (status, _body) = create_session(State(state), None).await.err().unwrap();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn get_session_returns_not_found_for_missing_session() {
        let state = test_state();
        let err = get_session(State(state), Path("missing".to_string()))
            .await
            .err()
            .unwrap();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_missing_session_is_idempotent() {
        let state = test_state();
        let status = delete_session(State(state), Path("missing".to_string()))
            .await
            .unwrap();
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_session_removes_session_even_if_workspace_id_is_stale() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _rx) = session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-del".to_string(), entry);

        let status = delete_session(State(state.clone()), Path("sess-del".to_string()))
            .await
            .unwrap();
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(state.get_session("sess-del").await.is_none());
    }

    #[tokio::test]
    async fn get_session_history_prefers_stored_rollout_path_over_latest_file() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (mut entry, _submission_rx) = session_entry(temp.path());

        let temp = tempfile::TempDir::new().unwrap();
        let older = temp.path().join("older.jsonl");
        let newer = temp.path().join("newer.jsonl");

        let old_items = [
            alan_runtime::RolloutItem::SessionMeta(alan_runtime::SessionMeta {
                session_id: "runtime-old".to_string(),
                started_at: "2026-02-23T00:00:00Z".to_string(),
                cwd: ".".to_string(),
                model: "test-model".to_string(),
            }),
            alan_runtime::RolloutItem::Message(alan_runtime::MessageRecord {
                role: "assistant".to_string(),
                content: Some("expected-from-stored".to_string()),
                tool_name: None,
                timestamp: "2026-02-23T00:00:01Z".to_string(),
            }),
        ];
        let new_items = [
            alan_runtime::RolloutItem::SessionMeta(alan_runtime::SessionMeta {
                session_id: "runtime-new".to_string(),
                started_at: "2026-02-23T00:01:00Z".to_string(),
                cwd: ".".to_string(),
                model: "test-model".to_string(),
            }),
            alan_runtime::RolloutItem::Message(alan_runtime::MessageRecord {
                role: "assistant".to_string(),
                content: Some("wrong-latest".to_string()),
                tool_name: None,
                timestamp: "2026-02-23T00:01:01Z".to_string(),
            }),
        ];

        std::fs::write(
            &older,
            old_items
                .iter()
                .map(serde_json::to_string)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
                + "\n",
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::write(
            &newer,
            new_items
                .iter()
                .map(serde_json::to_string)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
                + "\n",
        )
        .unwrap();

        entry.rollout_path = Some(older.clone());
        state
            .sessions
            .write()
            .await
            .insert("sess-history".to_string(), entry);

        let Json(resp) = get_session_history(State(state), Path("sess-history".to_string()))
            .await
            .unwrap();

        assert_eq!(resp.messages.len(), 1);
        assert_eq!(resp.messages[0].content, "expected-from-stored");
    }

    #[tokio::test]
    async fn submit_operation_returns_not_found_for_missing_session() {
        let state = test_state();
        let err = submit_operation(
            State(state),
            Path("missing".to_string()),
            Json(SubmitRequest { op: Op::Interrupt }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn stream_events_returns_not_found_for_missing_session() {
        let state = test_state();
        let err = stream_events(State(state), Path("missing".to_string()))
            .await
            .err()
            .unwrap();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_session_history_returns_not_found_for_missing_session() {
        let state = test_state();
        let err = get_session_history(State(state), Path("missing".to_string()))
            .await
            .err()
            .unwrap();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_sessions_returns_registered_sessions() {
        let state = test_state();
        let (entry1, _rx1) = session_entry(std::path::Path::new("/tmp/ws-a"));
        let (mut entry2, _rx2) = session_entry(std::path::Path::new("/tmp/ws-b"));
        entry2.approval_policy = alan_protocol::ApprovalPolicy::Never;
        entry2.sandbox_mode = alan_protocol::SandboxMode::DangerFullAccess;
        {
            let mut sessions = state.sessions.write().await;
            sessions.insert("sess-b".to_string(), entry2);
            sessions.insert("sess-a".to_string(), entry1);
        }

        let Json(resp) = list_sessions(State(state)).await.unwrap();
        assert_eq!(resp.sessions.len(), 2);
        assert_eq!(resp.sessions[0].session_id, "sess-a");
        assert_eq!(resp.sessions[1].session_id, "sess-b");
        assert!(resp.sessions.iter().all(|s| s.active));
        assert_eq!(
            resp.sessions[0].approval_policy,
            alan_protocol::ApprovalPolicy::OnRequest
        );
        assert_eq!(
            resp.sessions[0].sandbox_mode,
            alan_protocol::SandboxMode::WorkspaceWrite
        );
        assert_eq!(
            resp.sessions[1].approval_policy,
            alan_protocol::ApprovalPolicy::Never
        );
        assert_eq!(
            resp.sessions[1].sandbox_mode,
            alan_protocol::SandboxMode::DangerFullAccess
        );
    }

    #[tokio::test]
    async fn read_session_returns_metadata_and_history() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (mut entry, _rx) = session_entry(temp.path());

        // Get the generated workspace_id from the entry
        let expected_workspace_id = entry.workspace_id.clone();

        let temp = tempfile::TempDir::new().unwrap();
        let rollout = temp.path().join("read.jsonl");
        let items = [
            alan_runtime::RolloutItem::SessionMeta(alan_runtime::SessionMeta {
                session_id: "runtime-read".to_string(),
                started_at: "2026-02-23T00:00:00Z".to_string(),
                cwd: ".".to_string(),
                model: "test-model".to_string(),
            }),
            alan_runtime::RolloutItem::Message(alan_runtime::MessageRecord {
                role: "user".to_string(),
                content: Some("hello".to_string()),
                tool_name: None,
                timestamp: "2026-02-23T00:00:01Z".to_string(),
            }),
        ];
        std::fs::write(
            &rollout,
            items
                .iter()
                .map(serde_json::to_string)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
                + "\n",
        )
        .unwrap();
        entry.rollout_path = Some(rollout.clone());

        state
            .sessions
            .write()
            .await
            .insert("sess-read".to_string(), entry);

        let Json(resp) = read_session(State(state), Path("sess-read".to_string()))
            .await
            .unwrap();
        assert_eq!(resp.session_id, "sess-read");
        assert_eq!(resp.workspace_id, expected_workspace_id);
        assert_eq!(
            resp.approval_policy,
            alan_protocol::ApprovalPolicy::OnRequest
        );
        assert_eq!(
            resp.sandbox_mode,
            alan_protocol::SandboxMode::WorkspaceWrite
        );
        assert_eq!(resp.messages.len(), 1);
        assert_eq!(resp.messages[0].content, "hello");
        assert!(resp.rollout_path.unwrap().ends_with("read.jsonl"));
    }

    #[tokio::test]
    async fn get_session_and_submit_operation_work_for_existing_session() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, mut submission_rx) = session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-1".to_string(), entry);

        let info = get_session(State(state.clone()), Path("sess-1".to_string()))
            .await
            .unwrap();
        assert_eq!(info.0.session_id, "sess-1");
        assert!(info.0.active);
        assert_eq!(
            info.0.approval_policy,
            alan_protocol::ApprovalPolicy::OnRequest
        );
        assert_eq!(
            info.0.sandbox_mode,
            alan_protocol::SandboxMode::WorkspaceWrite
        );

        let resp = submit_operation(
            State(state.clone()),
            Path("sess-1".to_string()),
            Json(SubmitRequest {
                op: Op::Input {
                    content: "hello".to_string(),
                },
            }),
        )
        .await
        .unwrap();
        assert!(resp.0.accepted);

        let submission =
            tokio::time::timeout(std::time::Duration::from_secs(2), submission_rx.recv())
                .await
                .unwrap()
                .unwrap();
        match submission.op {
            Op::Input { content } => assert_eq!(content, "hello"),
            other => panic!("Unexpected op: {:?}", other),
        }
    }

    #[tokio::test]
    async fn compact_session_submits_compact_op() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, mut submission_rx) = session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-compact".to_string(), entry);

        let Json(resp) = compact_session(State(state), Path("sess-compact".to_string()))
            .await
            .unwrap();
        assert!(resp.accepted);

        let submission = submission_rx.recv().await.unwrap();
        assert!(matches!(submission.op, Op::Compact));
    }

    #[tokio::test]
    async fn rollback_session_validates_and_submits_rollback_op() {
        let state = test_state();
        let err = rollback_session(
            State(state.clone()),
            Path("missing".to_string()),
            Json(RollbackSessionRequest { num_turns: 0 }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(err, StatusCode::BAD_REQUEST);

        let temp = tempfile::TempDir::new().unwrap();
        let (entry, mut submission_rx) = session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-rb".to_string(), entry);

        let Json(resp) = rollback_session(
            State(state),
            Path("sess-rb".to_string()),
            Json(RollbackSessionRequest { num_turns: 2 }),
        )
        .await
        .unwrap();
        assert!(resp.accepted);

        let submission = submission_rx.recv().await.unwrap();
        match submission.op {
            Op::Rollback { num_turns } => assert_eq!(num_turns, 2),
            other => panic!("Unexpected op: {:?}", other),
        }
    }

    #[tokio::test]
    async fn resume_and_fork_return_not_found_for_missing_session() {
        let state = test_state();
        let resume_err = resume_session(State(state.clone()), Path("missing".to_string()))
            .await
            .err()
            .unwrap();
        assert_eq!(resume_err, StatusCode::NOT_FOUND);

        let fork_err = fork_session(State(state), Path("missing".to_string()), None)
            .await
            .err()
            .unwrap();
        assert_eq!(fork_err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn stream_events_emits_ndjson_and_sets_headers() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _submission_rx) = session_entry(temp.path());
        let events_tx = entry.events_tx.clone();
        state
            .sessions
            .write()
            .await
            .insert("sess-stream".to_string(), entry);

        let resp = stream_events(State(state.clone()), Path("sess-stream".to_string()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/x-ndjson"
        );
        assert_eq!(
            resp.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );

        events_tx
            .send(EventEnvelope {
                event_id: "evt_0000000000000001".to_string(),
                sequence: 1,
                session_id: "sess-stream".to_string(),
                submission_id: Some("sub-test".to_string()),
                turn_id: "turn_000001".to_string(),
                item_id: "item_000001_0001".to_string(),
                timestamp_ms: 1,
                event: Event::ThinkingDelta {
                    chunk: "ping".to_string(),
                    is_final: false,
                },
            })
            .unwrap();

        // Drop all senders so the NDJSON stream terminates and body can be collected.
        state.sessions.write().await.remove("sess-stream");
        drop(events_tx);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("\"type\":\"thinking_delta\""));
        assert!(text.contains("\"chunk\":\"ping\""));
        assert!(text.contains("\"event_id\":\"evt_0000000000000001\""));
        assert!(text.ends_with('\n'));
    }

    #[tokio::test]
    async fn read_events_returns_buffered_envelopes_with_cursor_and_gap_flag() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _submission_rx) = session_entry(temp.path());
        {
            let mut log = entry.event_log.write().await;
            let e1 = log.append_runtime_event(
                "sess-events",
                runtime_event(Event::ThinkingDelta {
                    chunk: "start".to_string(),
                    is_final: false,
                }),
            );
            let _e2 = log.append_runtime_event(
                "sess-events",
                runtime_event(Event::TextDelta {
                    chunk: "hi".to_string(),
                    is_final: true,
                }),
            );
            let _ = entry.events_tx.send(e1);
        }
        state
            .sessions
            .write()
            .await
            .insert("sess-events".to_string(), entry);

        let Json(all) = read_events(
            State(state.clone()),
            Path("sess-events".to_string()),
            Query(ReadEventsQuery::default()),
        )
        .await
        .unwrap();
        assert_eq!(all.events.len(), 2);
        assert!(!all.gap);
        assert_eq!(all.events[0].event_id, "evt_0000000000000001");

        let Json(after_first) = read_events(
            State(state.clone()),
            Path("sess-events".to_string()),
            Query(ReadEventsQuery {
                after_event_id: Some("evt_0000000000000001".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap();
        assert_eq!(after_first.events.len(), 1);
        assert_eq!(after_first.events[0].event_id, "evt_0000000000000002");
        assert!(!after_first.gap);

        let Json(gap_page) = read_events(
            State(state),
            Path("sess-events".to_string()),
            Query(ReadEventsQuery {
                after_event_id: Some("evt_not_a_number".to_string()),
                limit: Some(10),
            }),
        )
        .await
        .unwrap();
        assert!(gap_page.gap);
    }

    #[test]
    fn rollout_items_to_history_messages_keeps_final_messages_only() {
        let items = vec![
            RolloutItem::Message(MessageRecord {
                role: "user".to_string(),
                content: Some("hello".to_string()),
                tool_name: None,
                timestamp: "2026-02-23T00:00:00Z".to_string(),
            }),
            RolloutItem::Message(MessageRecord {
                role: "assistant".to_string(),
                content: Some("world".to_string()),
                tool_name: None,
                timestamp: "2026-02-23T00:00:01Z".to_string(),
            }),
            RolloutItem::Message(MessageRecord {
                role: "system".to_string(),
                content: Some("internal".to_string()),
                tool_name: None,
                timestamp: "2026-02-23T00:00:02Z".to_string(),
            }),
            RolloutItem::Message(MessageRecord {
                role: "assistant".to_string(),
                content: Some("   ".to_string()),
                tool_name: None,
                timestamp: "2026-02-23T00:00:03Z".to_string(),
            }),
        ];

        let messages = rollout_items_to_history_messages(items);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "world");
    }

    #[test]
    fn latest_rollout_path_picks_latest_jsonl() {
        let dir = std::env::temp_dir().join(format!("agentd-routes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let older = dir.join("a.jsonl");
        let newer = dir.join("b.jsonl");
        let other = dir.join("ignore.txt");

        std::fs::write(&older, "{}\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&newer, "{}\n").unwrap();
        std::fs::write(&other, "x").unwrap();

        let found = latest_rollout_path(&dir).unwrap().unwrap();
        assert_eq!(found, newer);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_like_fork_parses_policy_overrides() {
        let parsed = JsonLikeFork::from(Some(Json(ForkSessionRequest {
            workspace_dir: Some(PathBuf::from("/tmp/ws")),
            approval_policy: Some(alan_protocol::ApprovalPolicy::Never),
            sandbox_mode: Some(alan_protocol::SandboxMode::ReadOnly),
        })));

        assert_eq!(parsed.workspace_dir, Some(PathBuf::from("/tmp/ws")));
        assert_eq!(
            parsed.approval_policy,
            Some(alan_protocol::ApprovalPolicy::Never)
        );
        assert_eq!(
            parsed.sandbox_mode,
            Some(alan_protocol::SandboxMode::ReadOnly)
        );
    }
}
