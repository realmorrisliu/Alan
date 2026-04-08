use super::auth_control::{AuthControlError, AuthEventReplayPage};
use super::state::AppState;
use alan_auth::{BrowserLoginCompletion, ImportedChatgptTokenBundle};
use alan_protocol::{AuthErrorCode, AuthErrorResponse, AuthEvent, AuthStatusSnapshot};
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, Response, StatusCode, header},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub struct LogoutAuthResponse {
    pub removed: bool,
    pub snapshot: AuthStatusSnapshot,
}

#[derive(Debug, Deserialize, Default)]
pub struct ReadAuthEventsQuery {
    pub after_event_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Deserialize, Default)]
pub struct BrowserLoginCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportTokensRequest {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginSuccessResponse {
    pub account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    pub snapshot: AuthStatusSnapshot,
}

type AuthRouteError = (StatusCode, Json<AuthErrorResponse>);

pub async fn get_chatgpt_auth_status(
    State(state): State<AppState>,
) -> Result<Json<AuthStatusSnapshot>, AuthRouteError> {
    state
        .auth_control
        .status()
        .await
        .map(Json)
        .map_err(auth_control_error_response)
}

pub async fn post_chatgpt_auth_logout(
    State(state): State<AppState>,
) -> Result<Json<LogoutAuthResponse>, AuthRouteError> {
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
) -> Result<Json<ReadAuthEventsResponse>, AuthRouteError> {
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
) -> Result<Response<Body>, AuthRouteError> {
    let mut events_rx = state.auth_control.subscribe();
    let bootstrap_cursor = state.auth_control.replay_cursor().await;
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
                event_id: bootstrap_cursor.event_id,
                sequence: bootstrap_cursor.sequence,
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
) -> Result<Json<StartDeviceLoginResponse>, AuthRouteError> {
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
) -> Result<Json<LoginSuccessResponse>, AuthRouteError> {
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
    headers: HeaderMap,
    Json(request): Json<StartBrowserLoginRequest>,
) -> Result<Json<StartBrowserLoginResponse>, AuthRouteError> {
    let timeout_secs = request.timeout_secs.unwrap_or(300).clamp(30, 1800);
    let public_origin = derive_public_origin(&headers)?;
    let start = state
        .auth_control
        .start_browser_login(
            request.workspace_id,
            std::time::Duration::from_secs(timeout_secs),
            &public_origin,
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

pub async fn complete_chatgpt_browser_login_callback(
    State(state): State<AppState>,
    Path(login_id): Path<String>,
    Query(query): Query<BrowserLoginCallbackQuery>,
) -> Response<Body> {
    if let Some(error_code) = query.error {
        let user_message = query
            .error_description
            .unwrap_or_else(|| "Sign-in failed".to_string());
        let internal_message = format!("{error_code}: {user_message}");
        return match state
            .auth_control
            .fail_browser_login(&login_id, internal_message)
            .await
        {
            Ok(()) => html_response(
                StatusCode::BAD_REQUEST,
                "ChatGPT Login Failed",
                &user_message,
            ),
            Err(error) => browser_callback_error_response(error),
        };
    }

    let Some(code) = query.code else {
        return match state
            .auth_control
            .fail_browser_login(&login_id, "OAuth callback did not include code".to_string())
            .await
        {
            Ok(()) => html_response(
                StatusCode::BAD_REQUEST,
                "ChatGPT Login Failed",
                "OAuth callback did not include code.",
            ),
            Err(error) => browser_callback_error_response(error),
        };
    };

    let Some(state_token) = query.state else {
        return match state
            .auth_control
            .fail_browser_login(
                &login_id,
                "OAuth callback did not include state".to_string(),
            )
            .await
        {
            Ok(()) => html_response(
                StatusCode::BAD_REQUEST,
                "ChatGPT Login Failed",
                "OAuth callback did not include state.",
            ),
            Err(error) => browser_callback_error_response(error),
        };
    };

    match state
        .auth_control
        .complete_browser_login(
            &login_id,
            BrowserLoginCompletion {
                code,
                state: state_token,
            },
        )
        .await
    {
        Ok(_) => html_response(
            StatusCode::OK,
            "ChatGPT Login Complete",
            "Alan captured your ChatGPT session. You can close this window.",
        ),
        Err(error) => browser_callback_error_response(error),
    }
}

pub async fn complete_chatgpt_browser_login(
    State(state): State<AppState>,
    Json(request): Json<CompleteBrowserLoginRequest>,
) -> Result<Json<LoginSuccessResponse>, AuthRouteError> {
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
) -> Result<Json<LoginSuccessResponse>, AuthRouteError> {
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

fn auth_control_error_response(error: AuthControlError) -> (StatusCode, Json<AuthErrorResponse>) {
    let (status, code) = match &error {
        AuthControlError::UnknownPendingLogin { .. } => {
            (StatusCode::NOT_FOUND, AuthErrorCode::UnknownPendingLogin)
        }
        AuthControlError::ExpiredPendingLogin { .. } => {
            (StatusCode::GONE, AuthErrorCode::ExpiredPendingLogin)
        }
        AuthControlError::ExternalTokenHandoffDisabled => (
            StatusCode::FORBIDDEN,
            AuthErrorCode::ExternalTokenHandoffDisabled,
        ),
        AuthControlError::Chatgpt(chatgpt_error) => match chatgpt_error.kind() {
            Some(alan_auth::ChatgptAuthErrorKind::NotLoggedIn) => {
                (StatusCode::UNAUTHORIZED, AuthErrorCode::NotLoggedIn)
            }
            Some(alan_auth::ChatgptAuthErrorKind::MissingAccountIdentity) => (
                StatusCode::UNAUTHORIZED,
                AuthErrorCode::MissingAccountIdentity,
            ),
            Some(alan_auth::ChatgptAuthErrorKind::TokenExpired) => {
                (StatusCode::UNAUTHORIZED, AuthErrorCode::TokenExpired)
            }
            Some(alan_auth::ChatgptAuthErrorKind::RefreshFailed) => {
                (StatusCode::UNAUTHORIZED, AuthErrorCode::RefreshFailed)
            }
            Some(alan_auth::ChatgptAuthErrorKind::WorkspaceMismatch) => {
                (StatusCode::CONFLICT, AuthErrorCode::WorkspaceMismatch)
            }
            Some(alan_auth::ChatgptAuthErrorKind::UnauthorizedAfterRefresh) => (
                StatusCode::UNAUTHORIZED,
                AuthErrorCode::UnauthorizedAfterRefresh,
            ),
            Some(alan_auth::ChatgptAuthErrorKind::LoginFailed) => {
                (StatusCode::UNAUTHORIZED, AuthErrorCode::LoginFailed)
            }
            None => {
                if matches!(
                    chatgpt_error,
                    alan_auth::ChatgptAuthError::Io(io_error)
                        if io_error.kind() == std::io::ErrorKind::TimedOut
                ) {
                    (StatusCode::REQUEST_TIMEOUT, AuthErrorCode::Internal)
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, AuthErrorCode::Internal)
                }
            }
        },
    };
    (
        status,
        Json(AuthErrorResponse {
            code,
            message: error.to_string(),
        }),
    )
}

fn derive_public_origin(headers: &HeaderMap) -> Result<String, AuthRouteError> {
    let scheme = forwarded_param(headers, "proto")
        .or_else(|| first_header_value(headers, "x-forwarded-proto"))
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| matches!(value.as_str(), "http" | "https"))
        .unwrap_or_else(|| "http".to_string());
    let host = host_header_value(headers)
        .or_else(|| forwarded_param(headers, "host"))
        .or_else(|| first_header_value(headers, "x-forwarded-host"))
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    code: AuthErrorCode::Internal,
                    message: "Could not determine public host for browser login callback"
                        .to_string(),
                }),
            )
        })?;
    Ok(format!("{scheme}://{host}"))
}

fn host_header_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn forwarded_param(headers: &HeaderMap, key: &str) -> Option<String> {
    let forwarded = first_header_value(headers, "forwarded")?;
    let first_item = forwarded.split(',').next()?.trim();
    first_item.split(';').find_map(|segment| {
        let (name, value) = segment.split_once('=')?;
        if name.trim().eq_ignore_ascii_case(key) {
            let value = value.trim().trim_matches('"').trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        } else {
            None
        }
    })
}

fn first_header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn browser_callback_error_response(error: AuthControlError) -> Response<Body> {
    let (status, payload) = auth_control_error_response(error);
    html_response(status, "ChatGPT Login Failed", &payload.0.message)
}

fn html_response(status: StatusCode, title: &str, message: &str) -> Response<Body> {
    let mut response = Response::new(Body::from(render_html(title, message)));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn render_html(title: &str, message: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1><p>{}</p></body></html>",
        html_escape(title),
        html_escape(title),
        html_escape(message)
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::auth_control::{AuthControlState, CHATGPT_BROWSER_CALLBACK_ROUTE_PREFIX};
    use crate::daemon::state::AppState;
    use crate::daemon::{
        runtime_manager::{RuntimeManager, RuntimeManagerConfig},
        session_store::SessionStore,
        task_store::{JsonFileTaskStoreBackend, TaskStore},
        workspace_resolver::WorkspaceResolver,
    };
    use alan_auth::{ChatgptAuthConfig, ChatgptAuthManager};
    use alan_protocol::AuthStatusKind;
    use alan_runtime::{Config, runtime::WorkspaceRuntimeConfig};
    use axum::{
        Json, Router,
        body::to_bytes,
        http::{Request, StatusCode as HttpStatusCode, header},
        routing::{get, post},
    };
    use base64::Engine;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    fn test_state(temp_dir: &TempDir, external_handoff: bool) -> AppState {
        test_state_with_issuer(
            temp_dir,
            external_handoff,
            "https://auth.example.com".to_string(),
        )
    }

    fn test_state_with_issuer(
        temp_dir: &TempDir,
        external_handoff: bool,
        issuer: String,
    ) -> AppState {
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
                issuer,
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

    fn build_jwt(payload: serde_json::Value) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        format!("{header}.{payload}.sig")
    }

    async fn spawn_oauth_token_server() -> (String, tokio::task::JoinHandle<()>) {
        async fn exchange_token() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "id_token": build_jwt(serde_json::json!({
                    "email": "user@example.com",
                    "https://api.openai.com/auth": {
                        "chatgpt_plan_type": "pro",
                        "chatgpt_user_id": "user_123",
                        "chatgpt_account_id": "acct_123"
                    }
                })),
                "access_token": build_jwt(serde_json::json!({"exp": 4_102_444_800_i64})),
                "refresh_token": "refresh_token"
            }))
        }

        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                axum::Router::new().route("/oauth/token", post(exchange_token)),
            )
            .await
            .unwrap();
        });
        (format!("http://{}", address), server)
    }

    async fn spawn_device_auth_server() -> (String, tokio::task::JoinHandle<()>) {
        async fn start_device_code() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "device_auth_id": "device_auth_test",
                "user_code": "AAAA-BBBB",
                "interval": "1"
            }))
        }

        async fn device_token() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "authorization_code": "auth_code",
                "code_verifier": "verifier"
            }))
        }

        async fn exchange_token() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "id_token": build_jwt(serde_json::json!({
                    "email": "user@example.com",
                    "https://api.openai.com/auth": {
                        "chatgpt_plan_type": "pro",
                        "chatgpt_user_id": "user_123",
                        "chatgpt_account_id": "acct_123"
                    }
                })),
                "access_token": build_jwt(serde_json::json!({"exp": 4_102_444_800_i64})),
                "refresh_token": "refresh_token"
            }))
        }

        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                axum::Router::new()
                    .route("/api/accounts/deviceauth/usercode", post(start_device_code))
                    .route("/api/accounts/deviceauth/token", post(device_token))
                    .route("/oauth/token", post(exchange_token)),
            )
            .await
            .unwrap();
        });
        (format!("http://{}", address), server)
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
    async fn auth_events_stream_route_uses_ndjson() {
        let temp_dir = TempDir::new().unwrap();
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/events",
                get(stream_chatgpt_auth_events),
            )
            .with_state(test_state(&temp_dir, false));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/providers/chatgpt/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), HttpStatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/x-ndjson"
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );
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
        let payload: AuthErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.code, AuthErrorCode::ExternalTokenHandoffDisabled);
        assert!(payload.message.contains("disabled"));
    }

    #[tokio::test]
    async fn auth_import_route_returns_structured_workspace_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/import",
                post(import_chatgpt_tokens),
            )
            .with_state(test_state(&temp_dir, true));

        let id_token = {
            use base64::Engine;
            let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(r#"{"alg":"none","typ":"JWT"}"#);
            let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
                serde_json::json!({
                    "https://api.openai.com/auth": {
                        "chatgpt_account_id": "acct_actual"
                    }
                })
                .to_string(),
            );
            format!("{header}.{payload}.sig")
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/providers/chatgpt/import")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "id_token": id_token,
                            "access_token": "opaque-access-token",
                            "refresh_token": "refresh",
                            "workspace_id": "acct_expected"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), HttpStatusCode::CONFLICT);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: AuthErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.code, AuthErrorCode::WorkspaceMismatch);
        assert!(payload.message.contains("acct_expected"));
    }

    #[tokio::test]
    async fn auth_import_route_returns_structured_missing_account_identity() {
        let temp_dir = TempDir::new().unwrap();
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/import",
                post(import_chatgpt_tokens),
            )
            .with_state(test_state(&temp_dir, true));

        let id_token = {
            use base64::Engine;
            let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(r#"{"alg":"none","typ":"JWT"}"#);
            let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
                serde_json::json!({
                    "https://api.openai.com/auth": {
                        "chatgpt_user_id": "user_123"
                    }
                })
                .to_string(),
            );
            format!("{header}.{payload}.sig")
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/providers/chatgpt/import")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "id_token": id_token,
                            "access_token": "opaque-access-token",
                            "refresh_token": "refresh"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), HttpStatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: AuthErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.code, AuthErrorCode::MissingAccountIdentity);
    }

    #[tokio::test]
    async fn auth_device_routes_complete_and_emit_replayable_events() {
        let temp_dir = TempDir::new().unwrap();
        let (issuer, server) = spawn_device_auth_server().await;
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/login/device/start",
                post(start_chatgpt_device_login),
            )
            .route(
                "/api/v1/auth/providers/chatgpt/login/device/complete",
                post(complete_chatgpt_device_login),
            )
            .route(
                "/api/v1/auth/providers/chatgpt/events/read",
                get(read_chatgpt_auth_events),
            )
            .route(
                "/api/v1/auth/providers/chatgpt/status",
                get(get_chatgpt_auth_status),
            )
            .with_state(test_state_with_issuer(&temp_dir, false, issuer));

        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/providers/chatgpt/login/device/start")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_response.status(), HttpStatusCode::OK);
        let start_body = to_bytes(start_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let start: StartDeviceLoginResponse = serde_json::from_slice(&start_body).unwrap();
        let login_id = start.login_id.clone();
        assert!(start.login_id.starts_with("device_"));
        assert_eq!(start.user_code, "AAAA-BBBB");

        let events_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/providers/chatgpt/events/read?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(events_response.status(), HttpStatusCode::OK);
        let events_body = to_bytes(events_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: ReadAuthEventsResponse = serde_json::from_slice(&events_body).unwrap();
        assert!(events.events.iter().any(|event| matches!(
            &event.event,
            AuthEvent::LoginStarted { login_id, .. } if login_id == &start.login_id
        )));
        assert!(events.events.iter().any(|event| matches!(
            &event.event,
            AuthEvent::DeviceCodeReady { login_id, user_code, .. }
                if login_id == &start.login_id && user_code == "AAAA-BBBB"
        )));

        let complete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/providers/chatgpt/login/device/complete")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "login_id": login_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(complete_response.status(), HttpStatusCode::OK);
        let complete_body = to_bytes(complete_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let login: LoginSuccessResponse = serde_json::from_slice(&complete_body).unwrap();
        assert_eq!(login.account_id, "acct_123");

        let status_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/providers/chatgpt/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status_response.status(), HttpStatusCode::OK);
        let status_body = to_bytes(status_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: AuthStatusSnapshot = serde_json::from_slice(&status_body).unwrap();
        assert_eq!(snapshot.kind, AuthStatusKind::LoggedIn);
        assert_eq!(snapshot.account_id.as_deref(), Some("acct_123"));

        let events_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/providers/chatgpt/events/read?limit=20")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(events_response.status(), HttpStatusCode::OK);
        let events_body = to_bytes(events_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: ReadAuthEventsResponse = serde_json::from_slice(&events_body).unwrap();
        assert!(events.events.iter().any(|event| matches!(
            &event.event,
            AuthEvent::LoginSucceeded { login_id: event_login_id, account_id, .. }
                if event_login_id == &start.login_id && account_id == "acct_123"
        )));

        server.abort();
    }

    #[tokio::test]
    async fn auth_browser_start_and_callback_complete_daemon_owned_flow() {
        let temp_dir = TempDir::new().unwrap();
        let (issuer, server) = spawn_oauth_token_server().await;
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/login/browser/start",
                post(start_chatgpt_browser_login),
            )
            .route(
                "/api/v1/auth/providers/chatgpt/login/browser/callback/{login_id}",
                get(complete_chatgpt_browser_login_callback),
            )
            .route(
                "/api/v1/auth/providers/chatgpt/status",
                get(get_chatgpt_auth_status),
            )
            .with_state(test_state_with_issuer(&temp_dir, false, issuer));

        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/providers/chatgpt/login/browser/start")
                    .header(header::HOST, "alan.example.com:8090")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_response.status(), HttpStatusCode::OK);
        let start_body = to_bytes(start_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let start: StartBrowserLoginResponse = serde_json::from_slice(&start_body).unwrap();
        assert_eq!(
            start.redirect_uri,
            format!(
                "http://alan.example.com:8090{}/{}",
                CHATGPT_BROWSER_CALLBACK_ROUTE_PREFIX, start.login_id
            )
        );

        let state = start
            .auth_url
            .split('?')
            .nth(1)
            .and_then(|query| {
                query
                    .split('&')
                    .find_map(|part| part.strip_prefix("state=").map(str::to_string))
            })
            .expect("state query");

        let callback_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "{}/{}?code=auth_code&state={}",
                        CHATGPT_BROWSER_CALLBACK_ROUTE_PREFIX, start.login_id, state
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(callback_response.status(), HttpStatusCode::OK);
        let callback_body = to_bytes(callback_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let callback_html = String::from_utf8_lossy(&callback_body);
        assert!(callback_html.contains("ChatGPT Login Complete"));

        let status_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/providers/chatgpt/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status_response.status(), HttpStatusCode::OK);
        let status_body = to_bytes(status_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let snapshot: AuthStatusSnapshot = serde_json::from_slice(&status_body).unwrap();
        assert_eq!(snapshot.kind, AuthStatusKind::LoggedIn);
        assert_eq!(snapshot.account_id.as_deref(), Some("acct_123"));

        server.abort();
    }

    #[tokio::test]
    async fn auth_browser_start_prefers_host_header_over_forwarded_host() {
        let temp_dir = TempDir::new().unwrap();
        let app = Router::new()
            .route(
                "/api/v1/auth/providers/chatgpt/login/browser/start",
                post(start_chatgpt_browser_login),
            )
            .with_state(test_state(&temp_dir, false));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/providers/chatgpt/login/browser/start")
                    .header(header::HOST, "alan.example.com:8090")
                    .header("x-forwarded-host", "evil.example.com")
                    .header("x-forwarded-proto", "https")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), HttpStatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let start: StartBrowserLoginResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            start.redirect_uri,
            format!(
                "https://alan.example.com:8090{}/{}",
                CHATGPT_BROWSER_CALLBACK_ROUTE_PREFIX, start.login_id
            )
        );
    }
}
