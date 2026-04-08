use alan_auth::{
    BrowserLoginCompletion, BrowserLoginOptions, ChatgptAuthManager, ChatgptLoginSuccess,
    DeviceCodeLoginOptions, DeviceCodePrompt, ImportedChatgptTokenBundle, PendingBrowserLogin,
};
use alan_protocol::{
    AuthEvent, AuthEventEnvelope, AuthLoginMethod, AuthPendingLoginSummary, AuthProviderId,
    AuthStatusKind, AuthStatusSnapshot,
};
use chrono::{DateTime, Utc};
use std::{collections::HashMap, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock, broadcast};

const DEFAULT_AUTH_EVENT_BROADCAST_CAPACITY: usize = 64;
const DEFAULT_AUTH_EVENT_REPLAY_BUFFER_CAPACITY: usize = 256;
const DEVICE_CODE_TIMEOUT_MINUTES: i64 = 15;
pub const CHATGPT_BROWSER_CALLBACK_ROUTE_PREFIX: &str =
    "/api/v1/auth/providers/chatgpt/login/browser/callback";

#[derive(Debug, Clone)]
pub struct AuthEventReplayPage {
    pub events: Vec<AuthEventEnvelope>,
    pub gap: bool,
    pub oldest_event_id: Option<String>,
    pub latest_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthEventCursor {
    pub event_id: String,
    pub sequence: u64,
}

#[derive(Debug)]
pub struct AuthEventLog {
    next_sequence: u64,
    buffer: std::collections::VecDeque<AuthEventEnvelope>,
    capacity: usize,
}

impl AuthEventLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            next_sequence: 1,
            buffer: std::collections::VecDeque::with_capacity(capacity.min(16)),
            capacity: capacity.max(1),
        }
    }

    pub fn append(&mut self, event: AuthEvent) -> AuthEventEnvelope {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        let envelope = AuthEventEnvelope {
            event_id: format!("auth_evt_{sequence:016}"),
            sequence,
            timestamp_ms: now_timestamp_ms(),
            provider: AuthProviderId::Chatgpt,
            event,
        };
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(envelope.clone());
        envelope
    }

    pub fn replay_cursor(&self) -> AuthEventCursor {
        self.buffer.back().map_or(
            AuthEventCursor {
                event_id: format!("auth_evt_{:016}", 0),
                sequence: 0,
            },
            |envelope| AuthEventCursor {
                event_id: envelope.event_id.clone(),
                sequence: envelope.sequence,
            },
        )
    }

    pub fn read_after(&self, after_event_id: Option<&str>, limit: usize) -> AuthEventReplayPage {
        let limit = limit.clamp(1, 1000);
        let oldest_event_id = self.buffer.front().map(|e| e.event_id.clone());
        let latest_event_id = self.buffer.back().map(|e| e.event_id.clone());

        let Some(after_event_id) = after_event_id else {
            return AuthEventReplayPage {
                events: self.buffer.iter().take(limit).cloned().collect(),
                gap: false,
                oldest_event_id,
                latest_event_id,
            };
        };

        let after_sequence = parse_auth_event_sequence(after_event_id);
        if after_sequence == Some(0) {
            return AuthEventReplayPage {
                events: self.buffer.iter().take(limit).cloned().collect(),
                gap: false,
                oldest_event_id,
                latest_event_id,
            };
        }
        let mut gap = false;
        let start_idx = if let Some(idx) = self
            .buffer
            .iter()
            .position(|e| e.event_id == after_event_id)
        {
            idx + 1
        } else {
            if let (Some(seq), Some(oldest)) = (after_sequence, self.buffer.front())
                && seq < oldest.sequence
            {
                gap = true;
            }

            if let (Some(seq), Some(latest)) = (after_sequence, self.buffer.back()) {
                if seq >= latest.sequence {
                    self.buffer.len()
                } else {
                    gap = true;
                    0
                }
            } else {
                gap = true;
                0
            }
        };

        AuthEventReplayPage {
            events: self
                .buffer
                .iter()
                .skip(start_idx)
                .take(limit)
                .cloned()
                .collect(),
            gap,
            oldest_event_id,
            latest_event_id,
        }
    }
}

#[derive(Debug, Clone)]
struct PendingDeviceLogin {
    login_id: String,
    prompt: DeviceCodePrompt,
    forced_workspace_id: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
enum PendingLogin {
    Browser(PendingBrowserLogin),
    Device(PendingDeviceLogin),
}

impl PendingLogin {
    fn login_id(&self) -> &str {
        match self {
            Self::Browser(login) => &login.login_id,
            Self::Device(login) => &login.login_id,
        }
    }

    fn created_at(&self) -> DateTime<Utc> {
        match self {
            Self::Browser(login) => login.created_at,
            Self::Device(login) => login.created_at,
        }
    }

    fn expires_at(&self) -> DateTime<Utc> {
        match self {
            Self::Browser(login) => login.expires_at,
            Self::Device(login) => login.expires_at,
        }
    }

    fn method(&self) -> AuthLoginMethod {
        match self {
            Self::Browser(_) => AuthLoginMethod::Browser,
            Self::Device(_) => AuthLoginMethod::DeviceCode,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthControlState {
    manager: ChatgptAuthManager,
    events_tx: broadcast::Sender<AuthEventEnvelope>,
    event_log: Arc<RwLock<AuthEventLog>>,
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    external_token_handoff_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct DeviceCodeLoginStart {
    pub login_id: String,
    pub verification_url: String,
    pub user_code: String,
    pub interval_secs: u64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct BrowserLoginStart {
    pub login_id: String,
    pub auth_url: String,
    pub redirect_uri: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum AuthControlError {
    #[error("Unknown pending login `{login_id}`")]
    UnknownPendingLogin { login_id: String },
    #[error("Pending login `{login_id}` has expired")]
    ExpiredPendingLogin { login_id: String },
    #[error("External ChatGPT token handoff is disabled on this host")]
    ExternalTokenHandoffDisabled,
    #[error(transparent)]
    Chatgpt(#[from] alan_auth::ChatgptAuthError),
}

impl AuthControlState {
    pub fn new(manager: ChatgptAuthManager, external_token_handoff_enabled: bool) -> Self {
        let (events_tx, _) = broadcast::channel(DEFAULT_AUTH_EVENT_BROADCAST_CAPACITY);
        Self {
            manager,
            events_tx,
            event_log: Arc::new(RwLock::new(AuthEventLog::new(
                DEFAULT_AUTH_EVENT_REPLAY_BUFFER_CAPACITY,
            ))),
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
            external_token_handoff_enabled,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AuthEventEnvelope> {
        self.events_tx.subscribe()
    }

    pub async fn status(&self) -> Result<AuthStatusSnapshot, AuthControlError> {
        self.prune_expired_pending_logins().await;
        let pending_login = {
            let pending = self.pending_logins.lock().await;
            latest_pending_login_summary(&pending)
        };
        Ok(match self.manager.status().await? {
            Some(auth) => AuthStatusSnapshot {
                provider: AuthProviderId::Chatgpt,
                kind: if pending_login.is_some() {
                    AuthStatusKind::Pending
                } else {
                    AuthStatusKind::LoggedIn
                },
                storage_path: Some(auth.storage_path.display().to_string()),
                account_id: Some(auth.account_id),
                email: auth.email,
                plan_type: auth.plan_type,
                user_id: auth.user_id,
                access_token_expires_at: auth.access_token_expires_at,
                last_refresh_at: auth.last_refresh_at,
                pending_login,
            },
            None => AuthStatusSnapshot {
                provider: AuthProviderId::Chatgpt,
                kind: if pending_login.is_some() {
                    AuthStatusKind::Pending
                } else {
                    AuthStatusKind::LoggedOut
                },
                storage_path: Some(self.manager.storage_path().display().to_string()),
                account_id: None,
                email: None,
                plan_type: None,
                user_id: None,
                access_token_expires_at: None,
                last_refresh_at: None,
                pending_login,
            },
        })
    }

    pub async fn logout(&self) -> Result<bool, AuthControlError> {
        let removed = self.manager.logout().await?;
        self.emit(AuthEvent::LogoutCompleted { removed }).await;
        self.emit_status_snapshot().await?;
        Ok(removed)
    }

    pub async fn read_events(
        &self,
        after_event_id: Option<&str>,
        limit: usize,
    ) -> AuthEventReplayPage {
        self.event_log
            .read()
            .await
            .read_after(after_event_id, limit)
    }

    pub async fn replay_cursor(&self) -> AuthEventCursor {
        self.event_log.read().await.replay_cursor()
    }

    pub async fn start_device_login(
        &self,
        forced_workspace_id: Option<String>,
    ) -> Result<DeviceCodeLoginStart, AuthControlError> {
        let prompt = self.manager.start_device_code().await?;
        let login_id = format!("device_{}", random_id());
        let created_at = Utc::now();
        let expires_at = created_at + chrono::Duration::minutes(DEVICE_CODE_TIMEOUT_MINUTES);
        self.pending_logins.lock().await.insert(
            login_id.clone(),
            PendingLogin::Device(PendingDeviceLogin {
                login_id: login_id.clone(),
                prompt: prompt.clone(),
                forced_workspace_id,
                created_at,
                expires_at,
            }),
        );
        self.emit(AuthEvent::LoginStarted {
            login_id: login_id.clone(),
            method: AuthLoginMethod::DeviceCode,
        })
        .await;
        self.emit(AuthEvent::DeviceCodeReady {
            login_id: login_id.clone(),
            verification_url: prompt.verification_url.clone(),
            user_code: prompt.user_code.clone(),
            interval_secs: prompt.interval_secs(),
        })
        .await;
        self.emit_status_snapshot().await?;
        let interval_secs = prompt.interval_secs();
        Ok(DeviceCodeLoginStart {
            login_id,
            verification_url: prompt.verification_url,
            user_code: prompt.user_code,
            interval_secs,
            created_at,
            expires_at,
        })
    }

    pub async fn complete_device_login(
        &self,
        login_id: &str,
    ) -> Result<ChatgptLoginSuccess, AuthControlError> {
        let login = match self.claim_pending_device_login(login_id).await {
            Ok(login) => login,
            Err(error @ AuthControlError::ExpiredPendingLogin { .. }) => {
                self.emit(AuthEvent::LoginFailed {
                    login_id: Some(login_id.to_string()),
                    message: error.to_string(),
                    recoverable: false,
                })
                .await;
                self.emit_status_snapshot().await?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };

        let result = self
            .manager
            .complete_device_code(
                &login.prompt,
                DeviceCodeLoginOptions {
                    forced_workspace_id: login.forced_workspace_id.clone(),
                },
            )
            .await;

        match result {
            Ok(success) => {
                self.emit(AuthEvent::LoginSucceeded {
                    login_id: login_id.to_string(),
                    account_id: success.account_id.clone(),
                    email: success.email.clone(),
                    plan_type: success.plan_type.clone(),
                })
                .await;
                self.emit_status_snapshot().await?;
                Ok(success)
            }
            Err(error) => {
                self.emit(AuthEvent::LoginFailed {
                    login_id: Some(login_id.to_string()),
                    message: error.to_string(),
                    recoverable: false,
                })
                .await;
                self.emit_status_snapshot().await?;
                Err(AuthControlError::from(error))
            }
        }
    }

    pub async fn start_browser_login(
        &self,
        forced_workspace_id: Option<String>,
        timeout: Duration,
        public_origin: &str,
    ) -> Result<BrowserLoginStart, AuthControlError> {
        let login_id = format!("browser_{}", random_id());
        let redirect_uri = format!(
            "{}{}/{}",
            public_origin.trim_end_matches('/'),
            CHATGPT_BROWSER_CALLBACK_ROUTE_PREFIX,
            login_id
        );
        let pending = self.manager.begin_browser_login(BrowserLoginOptions {
            open_browser: false,
            forced_workspace_id,
            timeout,
            redirect_uri: Some(redirect_uri),
            login_id: Some(login_id),
        })?;
        let summary = BrowserLoginStart {
            login_id: pending.login_id.clone(),
            auth_url: pending.auth_url.clone(),
            redirect_uri: pending.redirect_uri.clone(),
            created_at: pending.created_at,
            expires_at: pending.expires_at,
        };
        self.pending_logins
            .lock()
            .await
            .insert(pending.login_id.clone(), PendingLogin::Browser(pending));
        self.emit(AuthEvent::LoginStarted {
            login_id: summary.login_id.clone(),
            method: AuthLoginMethod::Browser,
        })
        .await;
        self.emit(AuthEvent::BrowserLoginReady {
            login_id: summary.login_id.clone(),
            auth_url: summary.auth_url.clone(),
            redirect_uri: summary.redirect_uri.clone(),
        })
        .await;
        self.emit_status_snapshot().await?;
        Ok(summary)
    }

    pub async fn complete_browser_login(
        &self,
        login_id: &str,
        completion: BrowserLoginCompletion,
    ) -> Result<ChatgptLoginSuccess, AuthControlError> {
        let login = match self.claim_pending_browser_login(login_id).await {
            Ok(login) => login,
            Err(error @ AuthControlError::ExpiredPendingLogin { .. }) => {
                self.emit(AuthEvent::LoginFailed {
                    login_id: Some(login_id.to_string()),
                    message: error.to_string(),
                    recoverable: false,
                })
                .await;
                self.emit_status_snapshot().await?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };

        let result = self
            .manager
            .complete_browser_login(&login, completion)
            .await;
        match result {
            Ok(success) => {
                self.emit(AuthEvent::LoginSucceeded {
                    login_id: login_id.to_string(),
                    account_id: success.account_id.clone(),
                    email: success.email.clone(),
                    plan_type: success.plan_type.clone(),
                })
                .await;
                self.emit_status_snapshot().await?;
                Ok(success)
            }
            Err(error) => {
                self.emit(AuthEvent::LoginFailed {
                    login_id: Some(login_id.to_string()),
                    message: error.to_string(),
                    recoverable: false,
                })
                .await;
                self.emit_status_snapshot().await?;
                Err(AuthControlError::from(error))
            }
        }
    }

    pub async fn fail_browser_login(
        &self,
        login_id: &str,
        message: impl Into<String>,
    ) -> Result<(), AuthControlError> {
        let message = message.into();
        let _login = match self.claim_pending_browser_login(login_id).await {
            Ok(login) => login,
            Err(error @ AuthControlError::ExpiredPendingLogin { .. }) => {
                self.emit(AuthEvent::LoginFailed {
                    login_id: Some(login_id.to_string()),
                    message: error.to_string(),
                    recoverable: false,
                })
                .await;
                self.emit_status_snapshot().await?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };

        self.emit(AuthEvent::LoginFailed {
            login_id: Some(login_id.to_string()),
            message,
            recoverable: false,
        })
        .await;
        self.emit_status_snapshot().await?;
        Ok(())
    }

    pub async fn import_chatgpt_tokens(
        &self,
        bundle: ImportedChatgptTokenBundle,
        forced_workspace_id: Option<String>,
    ) -> Result<ChatgptLoginSuccess, AuthControlError> {
        if !self.external_token_handoff_enabled {
            return Err(AuthControlError::ExternalTokenHandoffDisabled);
        }
        let success = self
            .manager
            .import_token_bundle(bundle, forced_workspace_id.as_deref())?;
        self.emit(AuthEvent::TokenImported {
            account_id: success.account_id.clone(),
            email: success.email.clone(),
            plan_type: success.plan_type.clone(),
        })
        .await;
        self.emit_status_snapshot().await?;
        Ok(success)
    }

    async fn emit_status_snapshot(&self) -> Result<(), AuthControlError> {
        let snapshot = self.status().await?;
        self.emit(AuthEvent::StatusSnapshot { snapshot }).await;
        Ok(())
    }

    async fn emit(&self, event: AuthEvent) {
        let envelope = self.event_log.write().await.append(event);
        let _ = self.events_tx.send(envelope);
    }

    async fn claim_pending_device_login(
        &self,
        login_id: &str,
    ) -> Result<PendingDeviceLogin, AuthControlError> {
        let mut pending = self.pending_logins.lock().await;
        let Some(existing) = pending.get(login_id) else {
            return Err(AuthControlError::UnknownPendingLogin {
                login_id: login_id.to_string(),
            });
        };
        let PendingLogin::Device(login) = existing else {
            return Err(AuthControlError::UnknownPendingLogin {
                login_id: login_id.to_string(),
            });
        };
        if login.expires_at <= Utc::now() {
            pending.remove(login_id);
            return Err(AuthControlError::ExpiredPendingLogin {
                login_id: login_id.to_string(),
            });
        }
        match pending.remove(login_id) {
            Some(PendingLogin::Device(login)) => Ok(login),
            _ => Err(AuthControlError::UnknownPendingLogin {
                login_id: login_id.to_string(),
            }),
        }
    }

    async fn claim_pending_browser_login(
        &self,
        login_id: &str,
    ) -> Result<PendingBrowserLogin, AuthControlError> {
        let mut pending = self.pending_logins.lock().await;
        let Some(existing) = pending.get(login_id) else {
            return Err(AuthControlError::UnknownPendingLogin {
                login_id: login_id.to_string(),
            });
        };
        let PendingLogin::Browser(login) = existing else {
            return Err(AuthControlError::UnknownPendingLogin {
                login_id: login_id.to_string(),
            });
        };
        if login.expires_at <= Utc::now() {
            pending.remove(login_id);
            return Err(AuthControlError::ExpiredPendingLogin {
                login_id: login_id.to_string(),
            });
        }
        match pending.remove(login_id) {
            Some(PendingLogin::Browser(login)) => Ok(login),
            _ => Err(AuthControlError::UnknownPendingLogin {
                login_id: login_id.to_string(),
            }),
        }
    }

    async fn prune_expired_pending_logins(&self) {
        let now = Utc::now();
        self.pending_logins
            .lock()
            .await
            .retain(|_, login| login.expires_at() > now);
    }
}

fn latest_pending_login_summary(
    pending: &HashMap<String, PendingLogin>,
) -> Option<AuthPendingLoginSummary> {
    pending
        .values()
        .max_by(|left, right| {
            left.created_at()
                .cmp(&right.created_at())
                .then_with(|| left.login_id().cmp(right.login_id()))
        })
        .map(|login| AuthPendingLoginSummary {
            login_id: login.login_id().to_string(),
            method: login.method(),
            created_at: login.created_at(),
            expires_at: Some(login.expires_at()),
        })
}

fn now_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn parse_auth_event_sequence(event_id: &str) -> Option<u64> {
    event_id.strip_prefix("auth_evt_")?.parse::<u64>().ok()
}

fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alan_auth::{BrowserLoginOptions, ChatgptAuthConfig};
    use axum::{Json, routing::post};
    use base64::Engine;
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::net::TcpListener;

    fn build_jwt(payload: serde_json::Value) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        format!("{header}.{payload}.sig")
    }

    fn test_control_state(temp_dir: &TempDir, external: bool) -> AuthControlState {
        test_control_state_with_issuer(temp_dir, external, "https://auth.example.com".to_string())
    }

    fn test_control_state_with_issuer(
        temp_dir: &TempDir,
        external: bool,
        issuer: String,
    ) -> AuthControlState {
        let manager = ChatgptAuthManager::new(ChatgptAuthConfig {
            storage_path: temp_dir.path().join("auth.json"),
            issuer,
            client_id: "client".to_string(),
            browser_callback_port: 1455,
        })
        .unwrap();
        AuthControlState::new(manager, external)
    }

    async fn spawn_device_code_server() -> (String, tokio::task::JoinHandle<()>) {
        async fn start_device_code() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "device_auth_id": "device_auth_old",
                "user_code": "AAAA-BBBB",
                "interval": "5"
            }))
        }

        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                axum::Router::new()
                    .route("/api/accounts/deviceauth/usercode", post(start_device_code)),
            )
            .await
            .unwrap();
        });
        (format!("http://{}", address), server)
    }

    #[tokio::test]
    async fn status_reports_logged_out_by_default() {
        let temp_dir = TempDir::new().unwrap();
        let control = test_control_state(&temp_dir, false);
        let snapshot = control.status().await.unwrap();
        assert_eq!(snapshot.kind, AuthStatusKind::LoggedOut);
    }

    #[tokio::test]
    async fn import_tokens_updates_status_and_events() {
        let temp_dir = TempDir::new().unwrap();
        let control = test_control_state(&temp_dir, true);
        let id_token = build_jwt(json!({
            "email": "user@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_plan_type": "pro",
                "chatgpt_user_id": "user_123",
                "chatgpt_account_id": "acct_123"
            }
        }));
        let access_token = build_jwt(json!({"exp": 4_102_444_800_i64}));

        let success = control
            .import_chatgpt_tokens(
                ImportedChatgptTokenBundle {
                    id_token,
                    access_token,
                    refresh_token: "refresh_token".to_string(),
                },
                None,
            )
            .await
            .unwrap();
        assert_eq!(success.account_id, "acct_123");

        let snapshot = control.status().await.unwrap();
        assert_eq!(snapshot.kind, AuthStatusKind::LoggedIn);
        assert_eq!(snapshot.account_id.as_deref(), Some("acct_123"));

        let page = control.read_events(None, 10).await;
        assert!(
            page.events
                .iter()
                .any(|event| matches!(event.event, AuthEvent::TokenImported { .. }))
        );
    }

    #[tokio::test]
    async fn external_handoff_can_be_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let control = test_control_state(&temp_dir, false);
        let error = control
            .import_chatgpt_tokens(
                ImportedChatgptTokenBundle {
                    id_token: build_jwt(json!({
                        "https://api.openai.com/auth": {
                            "chatgpt_account_id": "acct_123"
                        }
                    })),
                    access_token: build_jwt(json!({"exp": 4_102_444_800_i64})),
                    refresh_token: "refresh".to_string(),
                },
                None,
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("disabled"));
    }

    #[tokio::test]
    async fn status_prefers_most_recent_pending_login() {
        let temp_dir = TempDir::new().unwrap();
        let control = test_control_state(&temp_dir, false);
        let mut older = control
            .manager
            .begin_browser_login(BrowserLoginOptions {
                open_browser: false,
                forced_workspace_id: None,
                timeout: Duration::from_secs(300),
                redirect_uri: None,
                login_id: None,
            })
            .unwrap();
        older.login_id = "browser_old".to_string();
        older.created_at = Utc::now();
        older.expires_at = older.created_at + chrono::Duration::minutes(5);

        let mut newer = control
            .manager
            .begin_browser_login(BrowserLoginOptions {
                open_browser: false,
                forced_workspace_id: None,
                timeout: Duration::from_secs(300),
                redirect_uri: None,
                login_id: None,
            })
            .unwrap();
        newer.login_id = "browser_new".to_string();
        newer.created_at = older.created_at + chrono::Duration::seconds(1);
        newer.expires_at = newer.created_at + chrono::Duration::minutes(5);

        control
            .pending_logins
            .lock()
            .await
            .insert(older.login_id.clone(), PendingLogin::Browser(older));
        control
            .pending_logins
            .lock()
            .await
            .insert(newer.login_id.clone(), PendingLogin::Browser(newer));

        let snapshot = control.status().await.unwrap();
        assert_eq!(snapshot.kind, AuthStatusKind::Pending);
        assert_eq!(
            snapshot.pending_login.map(|login| login.login_id),
            Some("browser_new".to_string())
        );
    }

    #[tokio::test]
    async fn complete_device_login_rejects_expired_pending_login_immediately() {
        let temp_dir = TempDir::new().unwrap();
        let (issuer, server) = spawn_device_code_server().await;
        let control = test_control_state_with_issuer(&temp_dir, false, issuer);
        let created_at = Utc::now() - chrono::Duration::minutes(20);
        let login_id = "device_expired".to_string();
        let prompt = control.manager.start_device_code().await.unwrap();
        control.pending_logins.lock().await.insert(
            login_id.clone(),
            PendingLogin::Device(PendingDeviceLogin {
                login_id: login_id.clone(),
                prompt,
                forced_workspace_id: None,
                created_at,
                expires_at: created_at + chrono::Duration::minutes(15),
            }),
        );

        let error = control
            .complete_device_login(&login_id)
            .await
            .expect_err("expired login");
        assert!(matches!(
            error,
            AuthControlError::ExpiredPendingLogin { login_id: ref id } if id == &login_id
        ));
        assert!(!control.pending_logins.lock().await.contains_key(&login_id));
        let page = control.read_events(None, 10).await;
        assert!(page.events.iter().any(|event| matches!(
            &event.event,
            AuthEvent::LoginFailed { login_id: Some(id), .. } if id == &login_id
        )));
        server.abort();
    }

    #[tokio::test]
    async fn complete_browser_login_rejects_expired_pending_login_immediately() {
        let temp_dir = TempDir::new().unwrap();
        let control = test_control_state(&temp_dir, false);
        let created_at = Utc::now() - chrono::Duration::minutes(20);
        let mut pending = control
            .manager
            .begin_browser_login(BrowserLoginOptions {
                open_browser: false,
                forced_workspace_id: None,
                timeout: Duration::from_secs(300),
                redirect_uri: None,
                login_id: None,
            })
            .unwrap();
        pending.login_id = "browser_expired".to_string();
        pending.created_at = created_at;
        pending.expires_at = created_at + chrono::Duration::minutes(5);
        let login_id = pending.login_id.clone();
        control
            .pending_logins
            .lock()
            .await
            .insert(login_id.clone(), PendingLogin::Browser(pending));

        let error = control
            .complete_browser_login(
                &login_id,
                BrowserLoginCompletion {
                    code: "code".to_string(),
                    state: "state".to_string(),
                },
            )
            .await
            .expect_err("expired login");
        assert!(matches!(
            error,
            AuthControlError::ExpiredPendingLogin { login_id: ref id } if id == &login_id
        ));
        assert!(!control.pending_logins.lock().await.contains_key(&login_id));
        let page = control.read_events(None, 10).await;
        assert!(page.events.iter().any(|event| matches!(
            &event.event,
            AuthEvent::LoginFailed { login_id: Some(id), .. } if id == &login_id
        )));
    }

    #[test]
    fn auth_event_log_supports_cursor_reads() {
        let mut log = AuthEventLog::new(8);
        let first = log.append(AuthEvent::LogoutCompleted { removed: false });
        log.append(AuthEvent::LogoutCompleted { removed: true });
        let page = log.read_after(Some(&first.event_id), 8);
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].sequence, 2);
    }

    #[test]
    fn auth_event_log_zero_cursor_round_trips_before_first_event() {
        let mut log = AuthEventLog::new(8);
        let cursor = log.replay_cursor();
        assert_eq!(cursor.sequence, 0);
        assert_eq!(cursor.event_id, "auth_evt_0000000000000000");

        log.append(AuthEvent::LogoutCompleted { removed: true });
        let page = log.read_after(Some(&cursor.event_id), 8);
        assert!(!page.gap);
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].sequence, 1);
    }
}
