use super::auth_control::{AuthControlError, AuthEventReplayPage};
use super::state::AppState;
use alan_auth::{BrowserLoginCompletion, ImportedChatgptTokenBundle};
use alan_protocol::{AuthEvent, AuthStatusSnapshot};
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Query, State},
    http::{HeaderValue, Response, StatusCode, header},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

#[derive(Debug, Serialize)]
pub struct LogoutAuthResponse {
    pub removed: bool,
    pub snapshot: AuthStatusSnapshot,
}

#[derive(Debug, Deserialize, Default)]
pub struct ReadAuthEventsQuery {
    pub after_event_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ReadAuthEventsResponse {
    pub gap: bool,
    pub oldest_event_id: Option<String>,
    pub latest_event_id: Option<String>,
    pub events: Vec<alan_protocol::AuthEventEnvelope>,
}

#[derive(Debug, Deserialize, Default)]
pub struct StartDeviceLoginRequest {
    pub workspace_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartDeviceLoginResponse {
    pub login_id: String,
    pub verification_url: String,
    pub user_code: String,
    pub interval_secs: u64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteDeviceLoginRequest {
    pub login_id: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct StartBrowserLoginRequest {
    pub workspace_id: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct StartBrowserLoginResponse {
    pub login_id: String,
    pub auth_url: String,
    pub redirect_uri: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteBrowserLoginRequest {
    pub login_id: String,
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportTokensRequest {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginSuccessResponse {
    pub account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    pub snapshot: AuthStatusSnapshot,
}

pub async fn get_chatgpt_auth_status(
    State(state): State<AppState>,
) -> Result<Json<AuthStatusSnapshot>, (StatusCode, Json<serde_json::Value>)> {
    state
        .auth_control
        .status()
        .await
        .map(Json)
        .map_err(auth_control_error_response)
}

pub async fn post_chatgpt_auth_logout(
    State(state): State<AppState>,
) -> Result<Json<LogoutAuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let removed = state
        .auth_control
        .logout()
        .await
        .map_err(auth_control_error_response)?;
    let snapshot = state
        .auth_control
        .status()
        .await
        .map_err(auth_control_error_response)?;
    Ok(Json(LogoutAuthResponse { removed, snapshot }))
}

pub async fn read_chatgpt_auth_events(
    State(state): State<AppState>,
    Query(query): Query<ReadAuthEventsQuery>,
) -> Result<Json<ReadAuthEventsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    let AuthEventReplayPage {
        events,
        gap,
        oldest_event_id,
        latest_event_id,
    } = state
        .auth_control
        .read_events(query.after_event_id.as_deref(), limit)
        .await;
    Ok(Json(ReadAuthEventsResponse {
        gap,
        oldest_event_id,
        latest_event_id,
        events,
    }))
}

pub async fn stream_chatgpt_auth_events(
    State(state): State<AppState>,
) -> Result<Response<Body>, (StatusCode, Json<serde_json::Value>)> {
    let mut events_rx = state.auth_control.subscribe();
    let initial_snapshot = state
        .auth_control
        .status()
        .await
        .map_err(auth_control_error_response)?;
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::convert::Infallible>>(64);

    tokio::spawn(async move {
        if send_event(
            &tx,
            &alan_protocol::AuthEventEnvelope {
                event_id: "auth_evt_snapshot".to_string(),
                sequence: 0,
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_millis() as u64)
                    .unwrap_or(0),
                provider: alan_protocol::AuthProviderId::Chatgpt,
                event: AuthEvent::StatusSnapshot {
                    snapshot: initial_snapshot,
                },
            },
        )
        .await
        .is_err()
        {
            return;
        }

        loop {
            match events_rx.recv().await {
                Ok(envelope) => {
                    if send_event(&tx, &envelope).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "Host auth event stream lagged");
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

pub async fn start_chatgpt_device_login(
    State(state): State<AppState>,
    Json(request): Json<StartDeviceLoginRequest>,
) -> Result<Json<StartDeviceLoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let start = state
        .auth_control
        .start_device_login(request.workspace_id)
        .await
        .map_err(auth_control_error_response)?;
    Ok(Json(StartDeviceLoginResponse {
        login_id: start.login_id,
        verification_url: start.verification_url,
        user_code: start.user_code,
        interval_secs: start.interval_secs,
        created_at: start.created_at,
        expires_at: start.expires_at,
    }))
}

pub async fn complete_chatgpt_device_login(
    State(state): State<AppState>,
    Json(request): Json<CompleteDeviceLoginRequest>,
) -> Result<Json<LoginSuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let login = state
        .auth_control
        .complete_device_login(&request.login_id)
        .await
        .map_err(auth_control_error_response)?;
    let snapshot = state
        .auth_control
        .status()
        .await
        .map_err(auth_control_error_response)?;
    Ok(Json(LoginSuccessResponse {
        account_id: login.account_id,
        email: login.email,
        plan_type: login.plan_type,
        snapshot,
    }))
}

pub async fn start_chatgpt_browser_login(
    State(state): State<AppState>,
    Json(request): Json<StartBrowserLoginRequest>,
) -> Result<Json<StartBrowserLoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let timeout_secs = request.timeout_secs.unwrap_or(300).clamp(30, 1800);
    let start = state
        .auth_control
        .start_browser_login(
            request.workspace_id,
            std::time::Duration::from_secs(timeout_secs),
        )
        .await
        .map_err(auth_control_error_response)?;
    Ok(Json(StartBrowserLoginResponse {
        login_id: start.login_id,
        auth_url: start.auth_url,
        redirect_uri: start.redirect_uri,
        created_at: start.created_at,
        expires_at: start.expires_at,
    }))
}

pub async fn complete_chatgpt_browser_login(
    State(state): State<AppState>,
    Json(request): Json<CompleteBrowserLoginRequest>,
) -> Result<Json<LoginSuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let login = state
        .auth_control
        .complete_browser_login(
            &request.login_id,
            BrowserLoginCompletion {
                code: request.code,
                state: request.state,
            },
        )
        .await
        .map_err(auth_control_error_response)?;
    let snapshot = state
        .auth_control
        .status()
        .await
        .map_err(auth_control_error_response)?;
    Ok(Json(LoginSuccessResponse {
        account_id: login.account_id,
        email: login.email,
        plan_type: login.plan_type,
        snapshot,
    }))
}

pub async fn import_chatgpt_tokens(
    State(state): State<AppState>,
    Json(request): Json<ImportTokensRequest>,
) -> Result<Json<LoginSuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let login = state
        .auth_control
        .import_chatgpt_tokens(
            ImportedChatgptTokenBundle {
                id_token: request.id_token,
                access_token: request.access_token,
                refresh_token: request.refresh_token,
            },
            request.workspace_id,
        )
        .await
        .map_err(auth_control_error_response)?;
    let snapshot = state
        .auth_control
        .status()
        .await
        .map_err(auth_control_error_response)?;
    Ok(Json(LoginSuccessResponse {
        account_id: login.account_id,
        email: login.email,
        plan_type: login.plan_type,
        snapshot,
    }))
}

async fn send_event(
    tx: &mpsc::Sender<Result<Bytes, std::convert::Infallible>>,
    envelope: &alan_protocol::AuthEventEnvelope,
) -> Result<(), ()> {
    let mut payload = serde_json::to_vec(envelope).map_err(|_| ())?;
    payload.push(b'\n');
    tx.send(Ok(Bytes::from(payload))).await.map_err(|_| ())
}

fn auth_control_error_response(error: AuthControlError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &error {
        AuthControlError::UnknownPendingLogin { .. } => StatusCode::NOT_FOUND,
        AuthControlError::ExpiredPendingLogin { .. } => StatusCode::GONE,
        AuthControlError::ExternalTokenHandoffDisabled => StatusCode::FORBIDDEN,
        AuthControlError::Chatgpt(alan_auth::ChatgptAuthError::NotLoggedIn)
        | AuthControlError::Chatgpt(alan_auth::ChatgptAuthError::TokenExpired)
        | AuthControlError::Chatgpt(alan_auth::ChatgptAuthError::RefreshFailed(_))
        | AuthControlError::Chatgpt(alan_auth::ChatgptAuthError::Unauthorized(_)) => {
            StatusCode::UNAUTHORIZED
        }
        AuthControlError::Chatgpt(alan_auth::ChatgptAuthError::WorkspaceMismatch { .. }) => {
            StatusCode::CONFLICT
        }
        AuthControlError::Chatgpt(alan_auth::ChatgptAuthError::Io(error))
            if error.kind() == std::io::ErrorKind::TimedOut =>
        {
            StatusCode::REQUEST_TIMEOUT
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(serde_json::json!({
            "error": error.to_string()
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::auth_control::AuthControlState;
    use crate::daemon::state::AppState;
    use crate::daemon::{
        runtime_manager::{RuntimeManager, RuntimeManagerConfig},
        session_store::SessionStore,
        task_store::{JsonFileTaskStoreBackend, TaskStore},
        workspace_resolver::WorkspaceResolver,
    };
    use alan_auth::{ChatgptAuthConfig, ChatgptAuthManager};
    use alan_runtime::{Config, runtime::WorkspaceRuntimeConfig};
    use axum::{
        Router,
        body::to_bytes,
        http::{Request, StatusCode as HttpStatusCode, header},
        routing::{get, post},
    };
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn test_state(temp_dir: &TempDir, external_handoff: bool) -> AppState {
        let config = Config::for_openai_responses("sk-test", None, Some("gpt-5.4"));
        let resolver = Arc::new(WorkspaceResolver::with_registry(
            crate::registry::WorkspaceRegistry {
                version: 1,
                workspaces: vec![],
            },
            temp_dir.path().to_path_buf(),
        ));
        let runtime_manager = Arc::new(RuntimeManager::new(RuntimeManagerConfig {
            max_concurrent_runtimes: 2,
            runtime_config_template: WorkspaceRuntimeConfig::from(config.clone()),
        }));
        let session_store =
            Arc::new(SessionStore::with_dir(temp_dir.path().join("sessions")).unwrap());
        let task_store = Arc::new(
            TaskStore::new(
                JsonFileTaskStoreBackend::with_storage_dir(temp_dir.path().join("tasks")).unwrap(),
            )
            .unwrap(),
        );
        let auth_control = Arc::new(AuthControlState::new(
            ChatgptAuthManager::new(ChatgptAuthConfig {
                storage_path: temp_dir.path().join("auth.json"),
                issuer: "https://auth.example.com".to_string(),
                client_id: "client".to_string(),
                browser_callback_port: 1455,
            })
            .unwrap(),
            external_handoff,
        ));
        AppState::from_parts_with_task_store_and_auth_control(
            config,
            resolver,
            runtime_manager,
            session_store,
            task_store,
            auth_control,
            3600,
        )
    }

    #[tokio::test]
    async fn auth_status_route_returns_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/status",
                get(get_chatgpt_auth_status),
            )
            .with_state(test_state(&temp_dir, false));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/providers/chatgpt/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), HttpStatusCode::OK);
    }

    #[tokio::test]
    async fn auth_import_route_respects_handoff_flag() {
        let temp_dir = TempDir::new().unwrap();
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/import",
                post(import_chatgpt_tokens),
            )
            .with_state(test_state(&temp_dir, false));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/providers/chatgpt/import")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "id_token": "id",
                            "access_token": "access",
                            "refresh_token": "refresh"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("disabled"));
    }
}
