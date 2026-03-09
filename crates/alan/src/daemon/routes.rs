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
    extract::{Extension, Path, Query, State},
    http::{HeaderValue, Response, StatusCode, header},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info, warn};

use super::remote_control::{RemoteRequestContext, required_scope_for_op};
use super::state::AppState;
use super::task_store::{
    RunCheckpointRecord, RunResumeAction, RunStatus, ScheduleItemRecord, ScheduleStatus,
    ScheduleTriggerType,
};

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
    pub governance: alan_protocol::GovernanceConfig,
    pub streaming_mode: alan_runtime::StreamingMode,
    pub partial_stream_recovery_mode: alan_runtime::PartialStreamRecoveryMode,
    pub durability: SessionDurabilityInfo,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct SessionDurabilityInfo {
    pub durable: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct RollbackDurabilityInfo {
    pub durable: bool,
    pub scope: &'static str,
}

#[derive(Deserialize, Default)]
pub struct CreateSessionRequest {
    /// Optional workspace directory override for this agent session (agentd-local path)
    pub workspace_dir: Option<PathBuf>,
    /// Optional governance override
    pub governance: Option<alan_protocol::GovernanceConfig>,
    /// Optional streaming behavior override
    pub streaming_mode: Option<alan_runtime::StreamingMode>,
    /// Optional partial-stream recovery override
    pub partial_stream_recovery_mode: Option<alan_runtime::PartialStreamRecoveryMode>,
}

/// Create a new session
pub async fn create_session(
    State(state): State<AppState>,
    payload: Option<Json<CreateSessionRequest>>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (workspace_dir, governance, streaming_mode, partial_stream_recovery_mode) = payload
        .map(|Json(req)| {
            (
                req.workspace_dir.filter(|p| !p.as_os_str().is_empty()),
                req.governance,
                req.streaming_mode,
                req.partial_stream_recovery_mode,
            )
        })
        .unwrap_or((None, None, None, None));

    let session_id = state
        .create_session_from_rollout(
            workspace_dir,
            None,
            governance.clone(),
            streaming_mode,
            partial_stream_recovery_mode,
        )
        .await
        .map_err(|err| {
            warn!(error = %err, "Failed to create session");
            (
                status_for_session_creation_error(&err),
                Json(serde_json::json!({ "error": format!("{:#}", err) })),
            )
        })?;
    info!(%session_id, "Created new session");

    let (governance, streaming_mode, partial_stream_recovery_mode, durability) = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&session_id) else {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Session {session_id} missing after creation")
                })),
            ));
        };
        (
            entry.governance.clone(),
            entry.streaming_mode,
            entry.partial_stream_recovery_mode,
            session_durability_info(entry.durability_required, entry.durable),
        )
    };

    Ok(Json(CreateSessionResponse {
        websocket_url: format!("/api/v1/sessions/{}/ws", session_id),
        events_url: format!("/api/v1/sessions/{}/events", session_id),
        submit_url: format!("/api/v1/sessions/{}/submit", session_id),
        session_id,
        governance,
        streaming_mode,
        partial_stream_recovery_mode,
        durability,
    }))
}

/// Get session info
#[derive(Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub active: bool,
    pub governance: alan_protocol::GovernanceConfig,
    pub streaming_mode: alan_runtime::StreamingMode,
    pub partial_stream_recovery_mode: alan_runtime::PartialStreamRecoveryMode,
    pub durability: SessionDurabilityInfo,
}

#[derive(Serialize)]
pub struct SessionListItem {
    pub session_id: String,
    pub workspace_id: String,
    pub active: bool,
    pub governance: alan_protocol::GovernanceConfig,
    pub streaming_mode: alan_runtime::StreamingMode,
    pub partial_stream_recovery_mode: alan_runtime::PartialStreamRecoveryMode,
    pub durability: SessionDurabilityInfo,
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
    pub governance: alan_protocol::GovernanceConfig,
    pub streaming_mode: alan_runtime::StreamingMode,
    pub partial_stream_recovery_mode: alan_runtime::PartialStreamRecoveryMode,
    pub durability: SessionDurabilityInfo,
    pub rollout_path: Option<String>,
    pub messages: Vec<SessionHistoryMessage>,
}

#[derive(Serialize)]
pub struct ReconnectSnapshotResponse {
    pub session_id: String,
    pub workspace_id: String,
    pub captured_at_ms: u64,
    pub replay: ReconnectReplayState,
    pub execution: ReconnectExecutionState,
    pub notifications: ReconnectNotificationState,
}

#[derive(Serialize)]
pub struct ReconnectReplayState {
    pub oldest_event_id: Option<String>,
    pub latest_event_id: Option<String>,
    pub latest_submission_id: Option<String>,
    pub buffered_event_count: usize,
}

#[derive(Serialize)]
pub struct ReconnectExecutionState {
    pub run_status: Option<RunStatus>,
    pub next_action: Option<RunResumeAction>,
    pub resume_required: bool,
    pub latest_checkpoint: Option<ReconnectCheckpoint>,
}

#[derive(Serialize)]
pub struct ReconnectCheckpoint {
    pub checkpoint_id: String,
    pub checkpoint_type: String,
    pub summary: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ReconnectNotificationState {
    pub latest_signal_cursor: Option<String>,
    pub signals: Vec<ReconnectNotificationSignal>,
}

#[derive(Serialize)]
pub struct ReconnectNotificationSignal {
    pub signal_id: String,
    pub signal_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_kind: Option<String>,
    pub summary: String,
    pub created_at: String,
    pub informational: bool,
}

#[derive(Serialize)]
pub struct ResumeSessionResponse {
    pub session_id: String,
    pub resumed: bool,
}

#[derive(Deserialize, Default)]
pub struct ForkSessionRequest {
    pub workspace_dir: Option<PathBuf>,
    pub governance: Option<alan_protocol::GovernanceConfig>,
    pub streaming_mode: Option<alan_runtime::StreamingMode>,
    pub partial_stream_recovery_mode: Option<alan_runtime::PartialStreamRecoveryMode>,
}

#[derive(Serialize)]
pub struct ForkSessionResponse {
    pub session_id: String,
    pub forked_from_session_id: String,
    pub websocket_url: String,
    pub events_url: String,
    pub submit_url: String,
    pub governance: alan_protocol::GovernanceConfig,
    pub streaming_mode: alan_runtime::StreamingMode,
    pub partial_stream_recovery_mode: alan_runtime::PartialStreamRecoveryMode,
    pub durability: SessionDurabilityInfo,
}

#[derive(Deserialize)]
pub struct RollbackSessionRequest {
    pub turns: u32,
}

#[derive(Serialize)]
pub struct RollbackSessionResponse {
    pub submission_id: String,
    pub accepted: bool,
    pub durability: RollbackDurabilityInfo,
    pub warning: String,
}

#[derive(Serialize)]
pub struct CompactSessionResponse {
    pub submission_id: String,
    pub accepted: bool,
}

#[derive(Deserialize)]
pub struct ScheduleAtRequest {
    pub wake_at: String,
}

#[derive(Deserialize)]
pub struct SleepUntilRequest {
    pub wake_at: String,
}

#[derive(Serialize)]
pub struct ScheduleResponse {
    pub session_id: String,
    pub schedule_id: String,
    pub run_id: String,
    pub trigger_type: String,
    pub status: String,
    pub wake_at: String,
    pub idempotency_key: String,
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

fn session_durability_info(required: bool, durable: bool) -> SessionDurabilityInfo {
    SessionDurabilityInfo { durable, required }
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SessionInfo>, StatusCode> {
    debug!(%id, "Getting session info");

    let exists = state.get_session(&id).await.map_err(|err| {
        warn!(%id, error = %err, "Failed to recover sessions before get_session");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if exists {
        let (governance, streaming_mode, partial_stream_recovery_mode, durability) = {
            let sessions = state.sessions.read().await;
            let Some(entry) = sessions.get(&id) else {
                return Err(StatusCode::NOT_FOUND);
            };
            (
                entry.governance.clone(),
                entry.streaming_mode,
                entry.partial_stream_recovery_mode,
                session_durability_info(entry.durability_required, entry.durable),
            )
        };
        Ok(Json(SessionInfo {
            session_id: id,
            active: true,
            governance,
            streaming_mode,
            partial_stream_recovery_mode,
            durability,
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
            governance: entry.governance.clone(),
            streaming_mode: entry.streaming_mode,
            partial_stream_recovery_mode: entry.partial_stream_recovery_mode,
            durability: session_durability_info(entry.durability_required, entry.durable),
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
    let (
        workspace_id,
        governance,
        streaming_mode,
        partial_stream_recovery_mode,
        durability,
        rollout_path,
    ) = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&session_id) else {
            return Err(StatusCode::NOT_FOUND);
        };
        (
            entry.workspace_id.clone(),
            entry.governance.clone(),
            entry.streaming_mode,
            entry.partial_stream_recovery_mode,
            session_durability_info(entry.durability_required, entry.durable),
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
        governance,
        streaming_mode,
        partial_stream_recovery_mode,
        durability,
        rollout_path,
        messages: history.messages,
    }))
}

/// Read reconnect snapshot for mobile/offline recovery without mutating execution state.
pub async fn reconnect_snapshot(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ReconnectSnapshotResponse>, StatusCode> {
    state.ensure_sessions_recovered().await.map_err(|err| {
        warn!(
            %session_id,
            error = %err,
            "Failed to recover sessions before reconnect snapshot read"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (workspace_id, event_log) = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&session_id) else {
            return Err(StatusCode::NOT_FOUND);
        };
        (entry.workspace_id.clone(), Arc::clone(&entry.event_log))
    };

    state
        .touch_session_inbound(&session_id)
        .await
        .map_err(|err| {
            warn!(
                %session_id,
                error = %err,
                "Failed to update inbound activity before reconnect snapshot read"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let replay_summary = {
        let guard = event_log.read().await;
        guard.replay_summary()
    };

    let run_snapshot = match state.restore_run(&session_id) {
        Ok(snapshot) => Some(snapshot),
        Err(err) => {
            warn!(
                %session_id,
                error = %err,
                "Unable to restore run snapshot for reconnect read; continuing with replay data only"
            );
            None
        }
    };

    let (execution, notifications) = if let Some(snapshot) = run_snapshot {
        let checkpoint = snapshot
            .checkpoint
            .as_ref()
            .map(reconnect_checkpoint_from_record);
        let signal = snapshot.checkpoint.as_ref().and_then(|checkpoint| {
            reconnect_signal_from_checkpoint(checkpoint, snapshot.run.status, snapshot.next_action)
        });
        (
            ReconnectExecutionState {
                run_status: Some(snapshot.run.status),
                next_action: Some(snapshot.next_action),
                resume_required: matches!(snapshot.next_action, RunResumeAction::AwaitUserResume),
                latest_checkpoint: checkpoint,
            },
            ReconnectNotificationState {
                latest_signal_cursor: signal.as_ref().map(|s| s.signal_id.clone()),
                signals: signal.into_iter().collect(),
            },
        )
    } else {
        (
            ReconnectExecutionState {
                run_status: None,
                next_action: None,
                resume_required: false,
                latest_checkpoint: None,
            },
            ReconnectNotificationState {
                latest_signal_cursor: None,
                signals: vec![],
            },
        )
    };

    Ok(Json(ReconnectSnapshotResponse {
        session_id,
        workspace_id,
        captured_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        replay: ReconnectReplayState {
            oldest_event_id: replay_summary.oldest_event_id,
            latest_event_id: replay_summary.latest_event_id,
            latest_submission_id: replay_summary.latest_submission_id,
            buffered_event_count: replay_summary.buffered_event_count,
        },
        execution,
        notifications,
    }))
}

/// Resume (ensure running) the runtime for a known session.
pub async fn resume_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ResumeSessionResponse>, StatusCode> {
    let exists = state.get_session(&session_id).await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to recover sessions before resume");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }
    state
        .touch_session_inbound(&session_id)
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to update inbound activity before resume");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
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
    let (
        source_workspace_id,
        source_governance,
        source_streaming_mode,
        source_partial_stream_recovery_mode,
        stored_rollout_path,
    ) = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&session_id) else {
            return Err(StatusCode::NOT_FOUND);
        };
        (
            entry.workspace_id.clone(),
            entry.governance.clone(),
            entry.streaming_mode,
            entry.partial_stream_recovery_mode,
            entry.rollout_path.clone(),
        )
    };

    state
        .touch_session_inbound(&session_id)
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to update inbound activity before fork");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let JsonLikeFork {
        workspace_dir,
        governance,
        streaming_mode,
        partial_stream_recovery_mode,
    } = JsonLikeFork::from(payload);
    let effective_governance = governance.unwrap_or(source_governance);
    let effective_streaming_mode = streaming_mode.unwrap_or(source_streaming_mode);
    let effective_partial_stream_recovery_mode =
        partial_stream_recovery_mode.unwrap_or(source_partial_stream_recovery_mode);

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
            Some(effective_governance.clone()),
            Some(effective_streaming_mode),
            Some(effective_partial_stream_recovery_mode),
        )
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to fork session");
            status_for_session_creation_error(&err)
        })?;

    let durability = {
        let sessions = state.sessions.read().await;
        let Some(entry) = sessions.get(&new_session_id) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };
        session_durability_info(entry.durability_required, entry.durable)
    };

    Ok(Json(ForkSessionResponse {
        websocket_url: format!("/api/v1/sessions/{}/ws", new_session_id),
        events_url: format!("/api/v1/sessions/{}/events", new_session_id),
        submit_url: format!("/api/v1/sessions/{}/submit", new_session_id),
        session_id: new_session_id,
        forked_from_session_id: session_id,
        governance: effective_governance,
        streaming_mode: effective_streaming_mode,
        partial_stream_recovery_mode: effective_partial_stream_recovery_mode,
        durability,
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

    state.touch_session_inbound(&session_id).await.map_err(|err| {
        warn!(%session_id, error = %err, "Failed to update inbound activity before history read");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let rollout_path = if let Some(path) = stored_rollout_path {
        if path.exists() {
            Some(path)
        } else {
            let refreshed =
                latest_rollout_path_for_workspace(&state, &session_id, &workspace_id).await?;
            state
                .set_session_rollout_path(&session_id, refreshed.clone())
                .await
                .map_err(|err| {
                    warn!(
                        %session_id,
                        error = %err,
                        "Failed to persist refreshed rollout path for history read"
                    );
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            refreshed
        }
    } else {
        let refreshed =
            latest_rollout_path_for_workspace(&state, &session_id, &workspace_id).await?;
        state
            .set_session_rollout_path(&session_id, refreshed.clone())
            .await
            .map_err(|err| {
                warn!(
                    %session_id,
                    error = %err,
                    "Failed to persist refreshed rollout path for history read"
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
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
    let sessions_dir = match state.get_sessions_dir(session_id).await.map_err(|err| {
        warn!(
            %session_id,
            error = %err,
            "Failed to recover sessions before resolving sessions directory"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
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
    remote_context: Option<Extension<RemoteRequestContext>>,
    Json(request): Json<SubmitRequest>,
) -> Result<Json<SubmitResponse>, StatusCode> {
    debug!(%session_id, "Submitting operation");
    let required_scope = required_scope_for_op(&request.op);
    if let Some(context) = remote_context.as_ref().map(|ext| &ext.0)
        && !context.allows_scope(required_scope)
    {
        warn!(
            %session_id,
            required_scope = ?required_scope,
            "Rejecting submit operation due to insufficient remote scope"
        );
        return Err(StatusCode::FORBIDDEN);
    }

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

    state
        .touch_session_inbound(&session_id)
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to update inbound activity before submit");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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
        None,
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

/// Request in-memory, non-durable rollback of the latest N turns for a session.
pub async fn rollback_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<RollbackSessionRequest>,
) -> Result<Json<RollbackSessionResponse>, StatusCode> {
    if request.turns == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let Json(resp) = submit_operation(
        State(state),
        Path(session_id),
        None,
        Json(SubmitRequest {
            op: alan_protocol::Op::Rollback {
                turns: request.turns,
            },
        }),
    )
    .await?;
    Ok(Json(RollbackSessionResponse {
        submission_id: resp.submission_id,
        accepted: resp.accepted,
        durability: RollbackDurabilityInfo {
            durable: false,
            scope: "in_memory",
        },
        warning: alan_runtime::ROLLBACK_NON_DURABLE_WARNING.to_string(),
    }))
}

/// Persist a one-shot `schedule_at` wake for a session.
pub async fn schedule_session_at(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<ScheduleAtRequest>,
) -> Result<Json<ScheduleResponse>, StatusCode> {
    let wake_at = parse_wake_at_or_bad_request(&request.wake_at)?;
    let schedule = state
        .schedule_at(&session_id, wake_at)
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to register schedule_at");
            if err.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(schedule_response(&session_id, schedule)))
}

/// Transition a session to sleeping and register wakeup.
pub async fn sleep_session_until(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<SleepUntilRequest>,
) -> Result<Json<ScheduleResponse>, StatusCode> {
    let wake_at = parse_wake_at_or_bad_request(&request.wake_at)?;
    let schedule = state
        .sleep_until(&session_id, wake_at)
        .await
        .map_err(|err| {
            warn!(%session_id, error = %err, "Failed to register sleep_until");
            if err.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(schedule_response(&session_id, schedule)))
}

fn parse_wake_at_or_bad_request(raw: &str) -> Result<DateTime<Utc>, StatusCode> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| StatusCode::BAD_REQUEST)
}

fn schedule_response(session_id: &str, schedule: ScheduleItemRecord) -> ScheduleResponse {
    let trigger_type = match schedule.trigger_type {
        ScheduleTriggerType::At => "at",
        ScheduleTriggerType::Interval => "interval",
        ScheduleTriggerType::RetryBackoff => "retry_backoff",
    };
    let status = match schedule.status {
        ScheduleStatus::Waiting => "waiting",
        ScheduleStatus::Due => "due",
        ScheduleStatus::Dispatching => "dispatching",
        ScheduleStatus::Cancelled => "cancelled",
        ScheduleStatus::Completed => "completed",
        ScheduleStatus::Failed => "failed",
    };

    ScheduleResponse {
        session_id: session_id.to_string(),
        schedule_id: schedule.schedule_id,
        run_id: schedule.run_id,
        trigger_type: trigger_type.to_string(),
        status: status.to_string(),
        wake_at: schedule.next_wake_at,
        idempotency_key: schedule.idempotency_key,
    }
}

fn reconnect_checkpoint_from_record(checkpoint: &RunCheckpointRecord) -> ReconnectCheckpoint {
    ReconnectCheckpoint {
        checkpoint_id: checkpoint.checkpoint_id.clone(),
        checkpoint_type: checkpoint.checkpoint_type.clone(),
        summary: checkpoint.summary.clone(),
        created_at: checkpoint.created_at.clone(),
        payload: checkpoint.payload.clone(),
    }
}

fn reconnect_signal_from_checkpoint(
    checkpoint: &RunCheckpointRecord,
    run_status: RunStatus,
    next_action: RunResumeAction,
) -> Option<ReconnectNotificationSignal> {
    if checkpoint.checkpoint_type != "yield" {
        return None;
    }
    if !matches!(run_status, RunStatus::Yielded)
        || !matches!(next_action, RunResumeAction::AwaitUserResume)
    {
        return None;
    }

    let request_id = checkpoint
        .payload
        .as_ref()
        .and_then(|payload| payload.get("request_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let yield_kind = checkpoint
        .payload
        .as_ref()
        .and_then(|payload| payload.get("kind"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let signal_type = if yield_kind
        .as_deref()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("structured_input"))
    {
        "pending_structured_input".to_string()
    } else {
        "pending_yield".to_string()
    };

    Some(ReconnectNotificationSignal {
        signal_id: checkpoint.checkpoint_id.clone(),
        signal_type,
        request_id,
        yield_kind,
        summary: checkpoint.summary.clone(),
        created_at: checkpoint.created_at.clone(),
        informational: true,
    })
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

    state
        .touch_session_inbound(&session_id)
        .await
        .map_err(|err| {
            warn!(
                %session_id,
                error = %err,
                "Failed to update inbound activity before event replay read"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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
    let mut events_rx = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session) => session.events_tx.subscribe(),
            None => return Err(StatusCode::NOT_FOUND),
        }
    };
    let resume_error_message = match state.resume_session_runtime(&session_id).await {
        Ok(()) => None,
        Err(err) => {
            warn!(
                %session_id,
                error = %err,
                "Failed to resume session runtime before streaming events"
            );
            Some(err.to_string())
        }
    };

    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(256);
    let state_clone = state.clone();
    let session_id_clone = session_id.clone();

    tokio::spawn(async move {
        if let Some(error_message) = resume_error_message {
            let resume_error = stream_resume_failed_envelope(&session_id_clone, error_message);
            let mut payload = match serde_json::to_vec(&resume_error) {
                Ok(bytes) => bytes,
                Err(err) => {
                    warn!(
                        session_id = %session_id_clone,
                        error = %err,
                        "Failed to serialize resume error event"
                    );
                    return;
                }
            };
            payload.push(b'\n');
            if tx.send(Ok(Bytes::from(payload))).await.is_err() {
                return;
            }
        }

        let mut last_event_id: Option<String> = None;
        loop {
            match events_rx.recv().await {
                Ok(envelope) => {
                    if let Err(err) = state_clone.touch_session_outbound(&session_id_clone).await {
                        warn!(
                            session_id = %session_id_clone,
                            error = %err,
                            "Failed to update outbound activity while streaming events"
                        );
                    }
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
    let replay_hint = replay_from_event_id
        .as_deref()
        .map(|event_id| format!(" Replay from event_id={event_id}."))
        .unwrap_or_default();
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
        event: Event::Error {
            message: format!(
                "Event stream lagged and skipped {skipped} event(s).{}",
                replay_hint
            ),
            recoverable: true,
        },
    }
}

fn stream_resume_failed_envelope(session_id: &str, error_message: String) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("control_resume_failed_{}", uuid::Uuid::new_v4()),
        sequence: 0,
        session_id: session_id.to_string(),
        submission_id: None,
        turn_id: "turn_control".to_string(),
        item_id: "item_control".to_string(),
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        event: Event::Error {
            message: format!(
                "Failed to resume session runtime before streaming events: {error_message}"
            ),
            recoverable: true,
        },
    }
}

fn latest_rollout_path(sessions_dir: &FsPath) -> std::io::Result<Option<PathBuf>> {
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let root = sessions_dir.to_path_buf();
    let mut dirs = vec![root.clone()];
    let mut latest: Option<(SystemTime, PathBuf)> = None;
    while let Some(dir) = dirs.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                if dir == root {
                    return Err(err);
                }
                warn!(
                    path = %dir.display(),
                    error = %err,
                    "Failed to inspect nested sessions directory while scanning rollouts"
                );
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    warn!(
                        path = %dir.display(),
                        error = %err,
                        "Failed to inspect nested sessions entry while scanning rollouts"
                    );
                    continue;
                }
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                dirs.push(path);
                continue;
            }

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
    }

    Ok(latest.map(|(_, path)| path))
}

fn status_for_session_creation_error(err: &anyhow::Error) -> StatusCode {
    let message = format!("{:#}", err);
    if message.contains("Workspace already has an active session runtime") {
        StatusCode::CONFLICT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

fn rollout_items_to_history_messages(items: Vec<RolloutItem>) -> Vec<SessionHistoryMessage> {
    items
        .into_iter()
        .filter_map(|item| match item {
            RolloutItem::Message(msg) => {
                if let Some(message) = msg.message {
                    let (role, content, tool_name) = match &message {
                        alan_runtime::tape::Message::User { .. } => {
                            ("user".to_string(), message.text_content(), None)
                        }
                        alan_runtime::tape::Message::Assistant { .. } => (
                            "assistant".to_string(),
                            message.non_thinking_text_content(),
                            None,
                        ),
                        alan_runtime::tape::Message::Tool { responses } => (
                            "tool".to_string(),
                            message.text_content(),
                            responses
                                .first()
                                .map(|response| response.id.trim().to_string())
                                .filter(|id| !id.is_empty()),
                        ),
                        alan_runtime::tape::Message::System { .. }
                        | alan_runtime::tape::Message::Context { .. } => return None,
                    };

                    if content.trim().is_empty() {
                        return None;
                    }

                    return Some(SessionHistoryMessage {
                        role,
                        content,
                        tool_name,
                        timestamp: msg.timestamp,
                    });
                }

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
    governance: Option<alan_protocol::GovernanceConfig>,
    streaming_mode: Option<alan_runtime::StreamingMode>,
    partial_stream_recovery_mode: Option<alan_runtime::PartialStreamRecoveryMode>,
}

impl JsonLikeFork {
    fn from(payload: Option<Json<ForkSessionRequest>>) -> Self {
        payload
            .map(|Json(req)| Self {
                workspace_dir: req.workspace_dir.filter(|p| !p.as_os_str().is_empty()),
                governance: req.governance,
                streaming_mode: req.streaming_mode,
                partial_stream_recovery_mode: req.partial_stream_recovery_mode,
            })
            .unwrap_or(Self {
                workspace_dir: None,
                governance: None,
                streaming_mode: None,
                partial_stream_recovery_mode: None,
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
        runtime::{RuntimeEventEnvelope, SessionDurabilityState, WorkspaceRuntimeConfig},
    };
    use axum::body::to_bytes;

    fn runtime_event_with_submission(
        event: Event,
        submission_id: Option<&str>,
    ) -> RuntimeEventEnvelope {
        RuntimeEventEnvelope {
            submission_id: submission_id.map(str::to_owned),
            event,
        }
    }

    fn runtime_event(event: Event) -> RuntimeEventEnvelope {
        runtime_event_with_submission(event, Some("sub-test"))
    }

    fn test_runtime_config() -> Config {
        Config::for_openai("sk-test", None, Some("gpt-5.4"))
    }

    fn test_state() -> AppState {
        test_state_with_runtime_limit(10)
    }

    fn test_state_with_runtime_limit(max_concurrent_runtimes: usize) -> AppState {
        test_state_with_runtime_limit_and_config(max_concurrent_runtimes, test_runtime_config())
    }

    fn test_state_with_runtime_limit_and_config(
        max_concurrent_runtimes: usize,
        config: Config,
    ) -> AppState {
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
        let runtime_manager = crate::daemon::runtime_manager::RuntimeManager::new(
            crate::daemon::runtime_manager::RuntimeManagerConfig {
                max_concurrent_runtimes,
                runtime_config_template: WorkspaceRuntimeConfig::from(config.clone()),
            },
        );
        let store = std::sync::Arc::new(
            crate::daemon::session_store::SessionStore::with_dir(base_dir.join("sessions"))
                .unwrap(),
        );
        let task_store = std::sync::Arc::new(
            crate::daemon::task_store::TaskStore::new(
                crate::daemon::task_store::JsonFileTaskStoreBackend::with_storage_dir(
                    base_dir.join("tasks"),
                )
                .unwrap(),
            )
            .unwrap(),
        );

        AppState::from_parts_with_task_store(
            config,
            std::sync::Arc::new(resolver),
            std::sync::Arc::new(runtime_manager),
            store,
            task_store,
            3600,
        )
    }

    fn session_entry(
        workspace_path: &std::path::Path,
    ) -> (SessionEntry, mpsc::Receiver<Submission>) {
        session_entry_with_replay_capacity(workspace_path, 32)
    }

    fn session_entry_with_replay_capacity(
        workspace_path: &std::path::Path,
        replay_capacity: usize,
    ) -> (SessionEntry, mpsc::Receiver<Submission>) {
        let (submission_tx, submission_rx) = mpsc::channel(8);
        let (events_tx, _) = broadcast::channel(8);
        let event_log = Arc::new(tokio::sync::RwLock::new(SessionEventLog::new(
            replay_capacity,
        )));
        let entry = SessionEntry::new(
            workspace_path.to_path_buf(),
            workspace_path.join(".alan"),
            alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Conservative,
                policy_path: None,
            },
            alan_runtime::StreamingMode::Auto,
            alan_runtime::PartialStreamRecoveryMode::ContinueOnce,
            SessionDurabilityState {
                durable: true,
                required: false,
            },
            submission_tx,
            events_tx,
            event_log,
            None,
            None,
        );
        (entry, submission_rx)
    }

    struct SessionsDirPermissionGuard {
        path: std::path::PathBuf,
    }

    impl SessionsDirPermissionGuard {
        fn new(path: std::path::PathBuf) -> Self {
            set_directory_writable(&path, false);
            Self { path }
        }
    }

    impl Drop for SessionsDirPermissionGuard {
        fn drop(&mut self) {
            set_directory_writable(&self.path, true);
        }
    }

    fn prepare_recorder_blocked_workspace(
        base_dir: &std::path::Path,
    ) -> (std::path::PathBuf, SessionsDirPermissionGuard) {
        let workspace_path = base_dir.join("workspace");
        let alan_dir = workspace_path.join(".alan");
        std::fs::create_dir_all(alan_dir.join("skills")).unwrap();
        std::fs::create_dir_all(alan_dir.join("sessions")).unwrap();
        std::fs::create_dir_all(alan_dir.join("memory")).unwrap();
        std::fs::create_dir_all(alan_dir.join("persona")).unwrap();
        std::fs::write(alan_dir.join("memory").join("MEMORY.md"), "# Memory\n").unwrap();

        let guard = SessionsDirPermissionGuard::new(alan_dir.join("sessions"));
        (workspace_path, guard)
    }

    fn set_directory_writable(path: &std::path::Path, writable: bool) {
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            permissions.set_mode(if writable { 0o755 } else { 0o555 });
        }
        #[cfg(not(unix))]
        {
            permissions.set_readonly(!writable);
        }
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[tokio::test]
    async fn health_returns_ok() {
        assert_eq!(health().await, "OK");
    }

    #[tokio::test]
    async fn create_session_returns_500_when_runtime_cannot_start() {
        let state = test_state_with_runtime_limit_and_config(10, Config::default());
        let (status, _body) = create_session(State(state), None).await.err().unwrap();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn create_session_reports_non_durable_mode_and_warning_when_recorder_is_unavailable() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (workspace_path, _guard) = prepare_recorder_blocked_workspace(temp.path());

        let Json(resp) = create_session(
            State(state.clone()),
            Some(Json(CreateSessionRequest {
                workspace_dir: Some(workspace_path),
                governance: None,
                streaming_mode: None,
                partial_stream_recovery_mode: None,
            })),
        )
        .await
        .unwrap();

        assert!(!resp.durability.durable);
        assert!(!resp.durability.required);

        let Json(events) = read_events(
            State(state.clone()),
            Path(resp.session_id.clone()),
            Query(ReadEventsQuery::default()),
        )
        .await
        .unwrap();
        assert!(events.events.iter().any(|event| {
            matches!(
                &event.event,
                Event::Warning { message } if message.contains("in-memory mode")
            )
        }));

        let Json(read_resp) = read_session(State(state.clone()), Path(resp.session_id.clone()))
            .await
            .unwrap();
        assert!(!read_resp.durability.durable);
        assert!(!read_resp.durability.required);

        state.remove_session(&resp.session_id).await.unwrap();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn create_session_fails_in_strict_durability_mode_when_recorder_is_unavailable() {
        let mut config = test_runtime_config();
        config.durability.required = true;
        let state = test_state_with_runtime_limit_and_config(10, config);
        let temp = tempfile::TempDir::new().unwrap();
        let (workspace_path, _guard) = prepare_recorder_blocked_workspace(temp.path());

        let (status, body) = create_session(
            State(state),
            Some(Json(CreateSessionRequest {
                workspace_dir: Some(workspace_path),
                governance: None,
                streaming_mode: None,
                partial_stream_recovery_mode: None,
            })),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            body.0["error"]
                .as_str()
                .unwrap_or_default()
                .contains("Strict durability required")
        );
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
        assert!(!state.get_session("sess-del").await.unwrap());
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
                message: None,
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
                message: None,
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
            None,
            Json(SubmitRequest { op: Op::Interrupt }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn schedule_session_at_returns_bad_request_for_invalid_wake_at() {
        let state = test_state();
        let err = schedule_session_at(
            State(state),
            Path("missing".to_string()),
            Json(ScheduleAtRequest {
                wake_at: "not-a-timestamp".to_string(),
            }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn schedule_session_at_returns_not_found_for_missing_session() {
        let state = test_state();
        let err = schedule_session_at(
            State(state),
            Path("missing".to_string()),
            Json(ScheduleAtRequest {
                wake_at: chrono::Utc::now().to_rfc3339(),
            }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn sleep_session_until_returns_waiting_schedule_for_existing_session() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _rx) = session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-sleep-route".to_string(), entry);

        let wake_at = chrono::Utc::now() + chrono::Duration::minutes(2);
        let Json(resp) = sleep_session_until(
            State(state.clone()),
            Path("sess-sleep-route".to_string()),
            Json(SleepUntilRequest {
                wake_at: wake_at.to_rfc3339(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(resp.session_id, "sess-sleep-route");
        assert_eq!(resp.status, "waiting");
        assert_eq!(resp.trigger_type, "at");

        let run = state
            .task_store
            .get_run("sess-sleep-route")
            .unwrap()
            .unwrap();
        assert_eq!(run.status, crate::daemon::task_store::RunStatus::Sleeping);
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
        entry2.governance = alan_protocol::GovernanceConfig {
            profile: alan_protocol::GovernanceProfile::Autonomous,
            policy_path: None,
        };
        entry2.streaming_mode = alan_runtime::StreamingMode::Off;
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
            resp.sessions[0].governance.profile,
            alan_protocol::GovernanceProfile::Conservative
        );
        assert_eq!(
            resp.sessions[0].streaming_mode,
            alan_runtime::StreamingMode::Auto
        );
        assert!(resp.sessions[0].durability.durable);
        assert!(!resp.sessions[0].durability.required);
        assert_eq!(resp.sessions[0].governance.policy_path, None);
        assert_eq!(
            resp.sessions[1].governance.profile,
            alan_protocol::GovernanceProfile::Autonomous
        );
        assert_eq!(
            resp.sessions[1].streaming_mode,
            alan_runtime::StreamingMode::Off
        );
        assert!(resp.sessions[1].durability.durable);
        assert!(!resp.sessions[1].durability.required);
        assert_eq!(resp.sessions[1].governance.policy_path, None);
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
                message: None,
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
            resp.governance.profile,
            alan_protocol::GovernanceProfile::Conservative
        );
        assert_eq!(resp.streaming_mode, alan_runtime::StreamingMode::Auto);
        assert!(resp.durability.durable);
        assert!(!resp.durability.required);
        assert_eq!(resp.messages.len(), 1);
        assert_eq!(resp.messages[0].content, "hello");
        assert!(resp.rollout_path.unwrap().ends_with("read.jsonl"));
    }

    #[tokio::test]
    async fn recovered_session_metadata_reports_non_durable_until_runtime_resumes() {
        let mut config = test_runtime_config();
        config.durability.required = true;
        let state = test_state_with_runtime_limit_and_config(10, config);
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_path = temp.path().join("workspace");
        let sessions_dir = workspace_path.join(".alan").join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        let rollout_path = sessions_dir.join("recovered.jsonl");
        let items = [
            alan_runtime::RolloutItem::SessionMeta(alan_runtime::SessionMeta {
                session_id: "sess-recovered".to_string(),
                started_at: "2026-02-23T00:00:00Z".to_string(),
                cwd: ".".to_string(),
                model: "test-model".to_string(),
            }),
            alan_runtime::RolloutItem::Message(alan_runtime::MessageRecord {
                role: "user".to_string(),
                content: Some("hello".to_string()),
                tool_name: None,
                message: None,
                timestamp: "2026-02-23T00:00:01Z".to_string(),
            }),
        ];
        std::fs::write(
            &rollout_path,
            items
                .iter()
                .map(serde_json::to_string)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
                + "\n",
        )
        .unwrap();

        state
            .session_store
            .save(crate::daemon::session_store::SessionBinding {
                session_id: "sess-recovered".to_string(),
                workspace_path: workspace_path.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
                governance: alan_protocol::GovernanceConfig::default(),
                streaming_mode: Some(alan_runtime::StreamingMode::Auto),
                partial_stream_recovery_mode: Some(
                    alan_runtime::PartialStreamRecoveryMode::ContinueOnce,
                ),
                rollout_path: Some(rollout_path),
                durability_required: Some(true),
                durable: Some(true),
            })
            .unwrap();

        let Json(info) = get_session(State(state.clone()), Path("sess-recovered".to_string()))
            .await
            .unwrap();
        assert!(info.durability.required);
        assert!(!info.durability.durable);

        let Json(list) = list_sessions(State(state.clone())).await.unwrap();
        let listed = list
            .sessions
            .into_iter()
            .find(|session| session.session_id == "sess-recovered")
            .expect("recovered session should be listed");
        assert!(listed.durability.required);
        assert!(!listed.durability.durable);

        let Json(read) = read_session(State(state), Path("sess-recovered".to_string()))
            .await
            .unwrap();
        assert!(read.durability.required);
        assert!(!read.durability.durable);
        assert_eq!(read.messages.len(), 1);
        assert_eq!(read.messages[0].content, "hello");
    }

    #[tokio::test]
    async fn reconnect_snapshot_returns_not_found_for_missing_session() {
        let state = test_state();
        let err = reconnect_snapshot(State(state), Path("missing".to_string()))
            .await
            .err()
            .unwrap();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn reconnect_snapshot_returns_replay_and_pending_yield_signal() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _rx) = session_entry(temp.path());
        {
            let mut log = entry.event_log.write().await;
            let _ = log.append_runtime_event(
                "sess-reconnect",
                runtime_event(Event::ThinkingDelta {
                    chunk: "plan".to_string(),
                    is_final: false,
                }),
            );
            let _ = log.append_runtime_event(
                "sess-reconnect",
                runtime_event(Event::Yield {
                    request_id: "req-mobile".to_string(),
                    kind: alan_protocol::YieldKind::Confirmation,
                    payload: serde_json::json!({"reason": "approve"}),
                }),
            );
        }
        state
            .sessions
            .write()
            .await
            .insert("sess-reconnect".to_string(), entry);

        let mut run = crate::daemon::task_store::RunRecord::new("sess-reconnect", "task-1", 1);
        run.status = crate::daemon::task_store::RunStatus::Yielded;
        state.task_store.save_run(run).unwrap();
        state
            .task_store
            .record_run_checkpoint(
                "sess-reconnect",
                "yield",
                "runtime yielded awaiting external input",
                Some(serde_json::json!({
                    "request_id": "req-mobile",
                    "kind": "confirmation"
                })),
            )
            .unwrap();

        let Json(snapshot) = reconnect_snapshot(State(state), Path("sess-reconnect".to_string()))
            .await
            .unwrap();
        assert_eq!(snapshot.session_id, "sess-reconnect");
        assert_eq!(
            snapshot.replay.latest_submission_id.as_deref(),
            Some("sub-test")
        );
        assert_eq!(snapshot.replay.buffered_event_count, 2);
        assert_eq!(
            snapshot.execution.run_status,
            Some(crate::daemon::task_store::RunStatus::Yielded)
        );
        assert_eq!(
            snapshot.execution.next_action,
            Some(crate::daemon::task_store::RunResumeAction::AwaitUserResume)
        );
        assert!(snapshot.execution.resume_required);
        assert_eq!(snapshot.notifications.signals.len(), 1);
        let signal = &snapshot.notifications.signals[0];
        assert_eq!(signal.signal_type, "pending_yield");
        assert_eq!(signal.request_id.as_deref(), Some("req-mobile"));
        assert_eq!(signal.yield_kind.as_deref(), Some("confirmation"));
        assert!(signal.informational);
    }

    #[tokio::test]
    async fn reconnect_snapshot_degrades_gracefully_when_run_snapshot_missing() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _rx) = session_entry(temp.path());
        {
            let mut log = entry.event_log.write().await;
            let _ = log.append_runtime_event(
                "sess-reconnect-empty",
                runtime_event(Event::TextDelta {
                    chunk: "hello".to_string(),
                    is_final: true,
                }),
            );
        }
        state
            .sessions
            .write()
            .await
            .insert("sess-reconnect-empty".to_string(), entry);

        let Json(snapshot) =
            reconnect_snapshot(State(state), Path("sess-reconnect-empty".to_string()))
                .await
                .unwrap();
        assert_eq!(snapshot.execution.run_status, None);
        assert_eq!(snapshot.execution.next_action, None);
        assert!(!snapshot.execution.resume_required);
        assert!(snapshot.notifications.signals.is_empty());
        assert_eq!(
            snapshot.replay.latest_submission_id.as_deref(),
            Some("sub-test")
        );
    }

    #[tokio::test]
    async fn reconnect_snapshot_uses_full_buffer_for_replay_metadata() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _rx) = session_entry_with_replay_capacity(temp.path(), 1105);
        {
            let mut log = entry.event_log.write().await;
            for index in 0..1101 {
                let event = Event::TextDelta {
                    chunk: format!("chunk-{index}"),
                    is_final: index == 1100,
                };
                let runtime_event = if index == 1100 {
                    runtime_event_with_submission(event, Some("sub-tail"))
                } else {
                    runtime_event_with_submission(event, None)
                };
                let _ = log.append_runtime_event("sess-reconnect-large", runtime_event);
            }
        }
        state
            .sessions
            .write()
            .await
            .insert("sess-reconnect-large".to_string(), entry);

        let Json(snapshot) =
            reconnect_snapshot(State(state), Path("sess-reconnect-large".to_string()))
                .await
                .unwrap();
        assert_eq!(
            snapshot.replay.oldest_event_id.as_deref(),
            Some("evt_0000000000000001")
        );
        assert_eq!(
            snapshot.replay.latest_event_id.as_deref(),
            Some("evt_0000000000001101")
        );
        assert_eq!(
            snapshot.replay.latest_submission_id.as_deref(),
            Some("sub-tail")
        );
        assert_eq!(snapshot.replay.buffered_event_count, 1101);
    }

    #[tokio::test]
    async fn reconnect_snapshot_maps_structured_input_signal_type() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _rx) = session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-reconnect-structured".to_string(), entry);

        let mut run =
            crate::daemon::task_store::RunRecord::new("sess-reconnect-structured", "task-1", 1);
        run.status = crate::daemon::task_store::RunStatus::Yielded;
        state.task_store.save_run(run).unwrap();
        state
            .task_store
            .record_run_checkpoint(
                "sess-reconnect-structured",
                "yield",
                "runtime yielded awaiting external input",
                Some(serde_json::json!({
                    "request_id": "req-structured",
                    "kind": "structured_input"
                })),
            )
            .unwrap();

        let Json(snapshot) =
            reconnect_snapshot(State(state), Path("sess-reconnect-structured".to_string()))
                .await
                .unwrap();
        assert_eq!(snapshot.notifications.signals.len(), 1);
        assert_eq!(
            snapshot.notifications.signals[0].signal_type,
            "pending_structured_input"
        );
        assert_eq!(
            snapshot.notifications.signals[0].request_id.as_deref(),
            Some("req-structured")
        );
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
            info.0.governance.profile,
            alan_protocol::GovernanceProfile::Conservative
        );
        assert_eq!(info.0.streaming_mode, alan_runtime::StreamingMode::Auto);
        assert!(info.0.durability.durable);
        assert!(!info.0.durability.required);

        let resp = submit_operation(
            State(state.clone()),
            Path("sess-1".to_string()),
            None,
            Json(SubmitRequest {
                op: Op::Input {
                    parts: vec![alan_protocol::ContentPart::text("hello")],
                    mode: alan_protocol::InputMode::Steer,
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
            Op::Input { parts, mode } => {
                assert_eq!(alan_protocol::parts_to_text(&parts), "hello");
                assert_eq!(mode, alan_protocol::InputMode::Steer);
            }
            other => panic!("Unexpected op: {:?}", other),
        }
    }

    #[tokio::test]
    async fn submit_operation_forbids_privileged_op_with_write_only_scope() {
        let state = test_state();
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, mut submission_rx) = session_entry(temp.path());
        state
            .sessions
            .write()
            .await
            .insert("sess-scope".to_string(), entry);

        let context = RemoteRequestContext {
            node_id: Some("node-1".to_string()),
            client_id: Some("client-1".to_string()),
            trace_id: Some("trace-1".to_string()),
            transport_mode: None,
            required_scope: Some(super::super::remote_control::SessionScope::Write),
            granted_scopes: Some(std::collections::HashSet::from([
                super::super::remote_control::SessionScope::Write,
            ])),
            auth_enabled: true,
            authenticated: true,
        };

        let err = submit_operation(
            State(state.clone()),
            Path("sess-scope".to_string()),
            Some(Extension(context)),
            Json(SubmitRequest {
                op: Op::Rollback { turns: 1 },
            }),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, StatusCode::FORBIDDEN);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), submission_rx.recv())
                .await
                .is_err(),
            "forbidden op should not be forwarded to runtime"
        );
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
            Json(RollbackSessionRequest { turns: 0 }),
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
            Json(RollbackSessionRequest { turns: 2 }),
        )
        .await
        .unwrap();
        assert!(resp.accepted);
        assert!(!resp.durability.durable);
        assert_eq!(resp.durability.scope, "in_memory");
        assert_eq!(resp.warning, alan_runtime::ROLLBACK_NON_DURABLE_WARNING);

        let submission = submission_rx.recv().await.unwrap();
        match submission.op {
            Op::Rollback { turns } => assert_eq!(turns, 2),
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
    async fn stream_events_emits_error_event_when_resume_fails() {
        let state = test_state_with_runtime_limit(0);
        let temp = tempfile::TempDir::new().unwrap();
        let (entry, _submission_rx) = session_entry(temp.path());
        let events_tx = entry.events_tx.clone();
        state
            .sessions
            .write()
            .await
            .insert("sess-resume-fail".to_string(), entry);

        let resp = stream_events(State(state.clone()), Path("sess-resume-fail".to_string()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Drop all senders so the stream terminates after control/error events are emitted.
        state.sessions.write().await.remove("sess-resume-fail");
        drop(events_tx);

        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("\"type\":\"error\""));
        assert!(text.contains("Failed to resume session runtime before streaming events"));
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
                message: None,
                timestamp: "2026-02-23T00:00:00Z".to_string(),
            }),
            RolloutItem::Message(MessageRecord {
                role: "assistant".to_string(),
                content: Some("world".to_string()),
                tool_name: None,
                message: None,
                timestamp: "2026-02-23T00:00:01Z".to_string(),
            }),
            RolloutItem::Message(MessageRecord {
                role: "system".to_string(),
                content: Some("internal".to_string()),
                tool_name: None,
                message: None,
                timestamp: "2026-02-23T00:00:02Z".to_string(),
            }),
            RolloutItem::Message(MessageRecord {
                role: "assistant".to_string(),
                content: Some("   ".to_string()),
                tool_name: None,
                message: None,
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
    fn rollout_items_to_history_messages_supports_rich_message_payload() {
        let items = vec![RolloutItem::Message(MessageRecord {
            role: "assistant".to_string(),
            content: None,
            tool_name: None,
            message: Some(alan_runtime::tape::Message::Assistant {
                parts: vec![
                    alan_runtime::tape::ContentPart::thinking("internal"),
                    alan_runtime::tape::ContentPart::text("visible"),
                ],
                tool_requests: vec![],
            }),
            timestamp: "2026-02-23T00:00:00Z".to_string(),
        })];

        let messages = rollout_items_to_history_messages(items);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "assistant");
        assert_eq!(messages[0].content, "visible");
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
    fn latest_rollout_path_searches_nested_directories() {
        let dir = std::env::temp_dir().join(format!("agentd-routes-{}", uuid::Uuid::new_v4()));
        let nested = dir.join("2026").join("02").join("28");
        std::fs::create_dir_all(&nested).unwrap();
        let top = dir.join("a.jsonl");
        let nested_latest = nested.join("b.jsonl");

        std::fs::write(&top, "{}\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&nested_latest, "{}\n").unwrap();

        let found = latest_rollout_path(&dir).unwrap().unwrap();
        assert_eq!(found, nested_latest);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_like_fork_parses_policy_overrides() {
        let parsed = JsonLikeFork::from(Some(Json(ForkSessionRequest {
            workspace_dir: Some(PathBuf::from("/tmp/ws")),
            governance: Some(alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: Some(".alan/policy.yaml".to_string()),
            }),
            streaming_mode: Some(alan_runtime::StreamingMode::On),
            partial_stream_recovery_mode: Some(alan_runtime::PartialStreamRecoveryMode::Off),
        })));

        assert_eq!(parsed.workspace_dir, Some(PathBuf::from("/tmp/ws")));
        assert_eq!(
            parsed.governance,
            Some(alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: Some(".alan/policy.yaml".to_string()),
            })
        );
        assert_eq!(parsed.streaming_mode, Some(alan_runtime::StreamingMode::On));
        assert_eq!(
            parsed.partial_stream_recovery_mode,
            Some(alan_runtime::PartialStreamRecoveryMode::Off)
        );
    }

    #[test]
    fn submit_request_parses_legacy_steer_alias_as_input_mode_steer() {
        let payload = serde_json::json!({
            "op": {
                "type": "steer",
                "parts": [
                    { "type": "text", "text": "legacy steer payload" }
                ]
            }
        });

        let parsed: SubmitRequest = serde_json::from_value(payload).unwrap();
        match parsed.op {
            Op::Input { parts, mode } => {
                assert_eq!(mode, alan_protocol::InputMode::Steer);
                assert_eq!(alan_protocol::parts_to_text(&parts), "legacy steer payload");
            }
            other => panic!("unexpected op: {other:?}"),
        }
    }
}
