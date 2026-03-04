//! Remote-control request metadata and scope-based authn/authz for daemon APIs.
//!
//! This module implements Phase A direct-remote controls:
//! - additive routing metadata headers (`node_id`, `client_id`, `trace_id`, `transport_mode`)
//! - optional bearer-token scope enforcement for session APIs
//! - request-context injection for downstream audit/logging

use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};

use axum::{
    Json,
    extract::Request,
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tracing::{debug, warn};

const HEADER_NODE_ID: &str = "x-alan-node-id";
const HEADER_CLIENT_ID: &str = "x-alan-client-id";
const HEADER_TRACE_ID: &str = "x-alan-trace-id";
const HEADER_TRANSPORT_MODE: &str = "x-alan-transport-mode";

const ENV_REMOTE_AUTH_ENABLED: &str = "ALAN_REMOTE_AUTH_ENABLED";
const ENV_REMOTE_AUTH_TOKENS: &str = "ALAN_REMOTE_AUTH_TOKENS";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionScope {
    Read,
    Write,
    Resume,
    Admin,
}

impl SessionScope {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "session.read" => Some(Self::Read),
            "session.write" => Some(Self::Write),
            "session.resume" => Some(Self::Resume),
            "session.admin" => Some(Self::Admin),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "session.read",
            Self::Write => "session.write",
            Self::Resume => "session.resume",
            Self::Admin => "session.admin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    Direct,
    Relay,
}

impl TransportMode {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "direct" => Some(Self::Direct),
            "relay" => Some(Self::Relay),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Relay => "relay",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteRequestContext {
    pub node_id: Option<String>,
    pub client_id: Option<String>,
    pub trace_id: Option<String>,
    pub transport_mode: Option<TransportMode>,
    pub required_scope: Option<SessionScope>,
    pub auth_enabled: bool,
    pub authenticated: bool,
}

struct ParsedRemoteHeaders {
    node_id: Option<String>,
    client_id: Option<String>,
    trace_id: Option<String>,
    transport_mode: Option<TransportMode>,
}

#[derive(Debug, Clone, Default)]
pub struct RemoteAccessControl {
    enabled: bool,
    token_scopes: HashMap<String, HashSet<SessionScope>>,
}

impl RemoteAccessControl {
    pub fn from_env() -> anyhow::Result<Self> {
        let enabled = env_var_truthy(ENV_REMOTE_AUTH_ENABLED);
        let token_scopes =
            parse_remote_auth_tokens(&env::var(ENV_REMOTE_AUTH_TOKENS).unwrap_or_default())?;

        if enabled && token_scopes.is_empty() {
            anyhow::bail!(
                "{} is enabled but {} is empty",
                ENV_REMOTE_AUTH_ENABLED,
                ENV_REMOTE_AUTH_TOKENS
            );
        }

        Ok(Self {
            enabled,
            token_scopes,
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn authorize_request(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
    ) -> Result<RemoteRequestContext, AuthError> {
        let required_scope = required_scope_for_request(method, path);
        let parsed_headers = parse_remote_headers(headers)?;
        let mut context = RemoteRequestContext {
            node_id: parsed_headers.node_id,
            client_id: parsed_headers.client_id,
            trace_id: parsed_headers.trace_id,
            transport_mode: parsed_headers.transport_mode,
            required_scope,
            auth_enabled: self.enabled,
            authenticated: false,
        };

        // Keep compatibility by default; strict scope auth is opt-in via env.
        if !self.enabled {
            return Ok(context);
        }

        let required_scope = match required_scope {
            Some(scope) => scope,
            None => return Ok(context),
        };
        let token = extract_bearer_token(headers)
            .ok_or(AuthError::unauthorized("missing or invalid bearer token"))?;
        let granted_scopes = self
            .token_scopes
            .get(token)
            .ok_or(AuthError::unauthorized("unknown bearer token"))?;
        if !granted_scopes.contains(&required_scope) {
            return Err(AuthError::forbidden(required_scope));
        }

        context.authenticated = true;
        Ok(context)
    }
}

#[derive(Debug)]
pub struct AuthError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl AuthError {
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "remote_auth_unauthorized",
            message: message.into(),
        }
    }

    fn forbidden(required: SessionScope) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "remote_auth_forbidden",
            message: format!("missing required scope: {}", required.as_str()),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "remote_auth_bad_request",
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
struct AuthErrorResponse<'a> {
    error: &'a str,
    code: &'a str,
}

pub async fn remote_access_middleware(
    control: Arc<RemoteAccessControl>,
    mut request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    match control.authorize_request(&method, &path, request.headers()) {
        Ok(context) => {
            debug!(
                method = %method,
                path = %path,
                auth_enabled = context.auth_enabled,
                authenticated = context.authenticated,
                required_scope = context.required_scope.map(|scope| scope.as_str()),
                node_id = context.node_id.as_deref(),
                client_id = context.client_id.as_deref(),
                trace_id = context.trace_id.as_deref(),
                transport_mode = context.transport_mode.map(|m| m.as_str()),
                "remote access check passed"
            );
            request.extensions_mut().insert(context);
            next.run(request).await
        }
        Err(err) => {
            warn!(
                method = %method,
                path = %path,
                status = %err.status,
                code = err.code,
                message = %err.message,
                "remote access check failed"
            );
            (
                err.status,
                Json(AuthErrorResponse {
                    error: &err.message,
                    code: err.code,
                }),
            )
                .into_response()
        }
    }
}

fn env_var_truthy(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn parse_remote_auth_tokens(raw: &str) -> anyhow::Result<HashMap<String, HashSet<SessionScope>>> {
    let mut bindings = HashMap::new();
    if raw.trim().is_empty() {
        return Ok(bindings);
    }

    for binding in raw.split(';') {
        let binding = binding.trim();
        if binding.is_empty() {
            continue;
        }
        let (token, scopes_raw) = binding.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("Invalid token binding `{binding}`; expected token=scopes")
        })?;
        let token = token.trim();
        let scopes_raw = scopes_raw.trim();
        if token.is_empty() || scopes_raw.is_empty() {
            anyhow::bail!("Invalid token binding `{binding}`; token and scopes must be non-empty");
        }

        let mut scopes = HashSet::new();
        for scope in scopes_raw.split(',') {
            let scope = scope.trim();
            if scope.is_empty() {
                continue;
            }
            if scope == "*" {
                scopes.insert(SessionScope::Read);
                scopes.insert(SessionScope::Write);
                scopes.insert(SessionScope::Resume);
                scopes.insert(SessionScope::Admin);
                continue;
            }
            let Some(parsed) = SessionScope::parse(scope) else {
                anyhow::bail!("Unknown scope `{scope}` in binding `{binding}`");
            };
            scopes.insert(parsed);
        }
        if scopes.is_empty() {
            anyhow::bail!("Invalid token binding `{binding}`; no scopes parsed");
        }

        bindings.insert(token.to_string(), scopes);
    }

    Ok(bindings)
}

fn parse_remote_headers(headers: &HeaderMap) -> Result<ParsedRemoteHeaders, AuthError> {
    let node_id = parse_optional_header(headers, HEADER_NODE_ID)?;
    let client_id = parse_optional_header(headers, HEADER_CLIENT_ID)?;
    let trace_id = parse_optional_header(headers, HEADER_TRACE_ID)?;
    let transport_mode = parse_optional_header(headers, HEADER_TRANSPORT_MODE)?
        .map(|value| {
            TransportMode::parse(&value).ok_or_else(|| {
                AuthError::bad_request(format!(
                    "invalid {} header; expected `direct` or `relay`",
                    HEADER_TRANSPORT_MODE
                ))
            })
        })
        .transpose()?;
    Ok(ParsedRemoteHeaders {
        node_id,
        client_id,
        trace_id,
        transport_mode,
    })
}

fn parse_optional_header(headers: &HeaderMap, name: &str) -> Result<Option<String>, AuthError> {
    let Some(value) = headers.get(name) else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| AuthError::bad_request(format!("invalid `{name}` header encoding")))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("`{name}` cannot be empty")));
    }
    Ok(Some(value.to_string()))
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn required_scope_for_request(method: &Method, path: &str) -> Option<SessionScope> {
    // Non-session routes are not subject to remote-session scope checks.
    if !path.starts_with("/api/v1/") {
        return None;
    }

    if method == Method::DELETE {
        return Some(SessionScope::Admin);
    }

    if method == Method::POST {
        if path.ends_with("/resume") {
            return Some(SessionScope::Resume);
        }
        if path.ends_with("/fork")
            || path.ends_with("/rollback")
            || path.ends_with("/compact")
            || path.ends_with("/schedule_at")
            || path.ends_with("/sleep_until")
        {
            return Some(SessionScope::Admin);
        }
        return Some(SessionScope::Write);
    }

    if method == Method::GET {
        if path.ends_with("/ws") {
            // WebSocket sessions can submit operations bidirectionally.
            return Some(SessionScope::Write);
        }
        return Some(SessionScope::Read);
    }

    Some(SessionScope::Write)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Method};

    #[test]
    fn parse_remote_auth_tokens_parses_multi_token_scopes() {
        let parsed = parse_remote_auth_tokens(
            "reader=session.read;writer=session.read,session.write;admin=*",
        )
        .unwrap();

        assert_eq!(parsed.len(), 3);
        assert!(parsed["reader"].contains(&SessionScope::Read));
        assert!(!parsed["reader"].contains(&SessionScope::Write));
        assert!(parsed["writer"].contains(&SessionScope::Read));
        assert!(parsed["writer"].contains(&SessionScope::Write));
        assert!(parsed["admin"].contains(&SessionScope::Read));
        assert!(parsed["admin"].contains(&SessionScope::Write));
        assert!(parsed["admin"].contains(&SessionScope::Resume));
        assert!(parsed["admin"].contains(&SessionScope::Admin));
    }

    #[test]
    fn required_scope_for_request_maps_session_routes() {
        assert_eq!(
            required_scope_for_request(&Method::GET, "/api/v1/sessions"),
            Some(SessionScope::Read)
        );
        assert_eq!(
            required_scope_for_request(&Method::POST, "/api/v1/sessions"),
            Some(SessionScope::Write)
        );
        assert_eq!(
            required_scope_for_request(&Method::POST, "/api/v1/sessions/s1/resume"),
            Some(SessionScope::Resume)
        );
        assert_eq!(
            required_scope_for_request(&Method::POST, "/api/v1/sessions/s1/rollback"),
            Some(SessionScope::Admin)
        );
        assert_eq!(
            required_scope_for_request(&Method::DELETE, "/api/v1/sessions/s1"),
            Some(SessionScope::Admin)
        );
        assert_eq!(
            required_scope_for_request(&Method::GET, "/api/v1/sessions/s1/ws"),
            Some(SessionScope::Write)
        );
        assert_eq!(required_scope_for_request(&Method::GET, "/health"), None);
    }

    #[test]
    fn authorize_request_allows_when_auth_disabled() {
        let control = RemoteAccessControl::default();
        let headers = HeaderMap::new();
        let context = control
            .authorize_request(&Method::POST, "/api/v1/sessions/s1/submit", &headers)
            .unwrap();
        assert!(!context.auth_enabled);
        assert!(!context.authenticated);
        assert_eq!(context.required_scope, Some(SessionScope::Write));
    }

    #[test]
    fn authorize_request_requires_valid_bearer_when_enabled() {
        let mut token_scopes = HashMap::new();
        token_scopes.insert("token-r".to_string(), HashSet::from([SessionScope::Read]));
        let control = RemoteAccessControl {
            enabled: true,
            token_scopes,
        };

        let headers = HeaderMap::new();
        let err = control
            .authorize_request(&Method::GET, "/api/v1/sessions", &headers)
            .unwrap_err();
        assert_eq!(err.status, StatusCode::UNAUTHORIZED);

        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token-r"),
        );
        let context = control
            .authorize_request(&Method::GET, "/api/v1/sessions", &headers)
            .unwrap();
        assert!(context.auth_enabled);
        assert!(context.authenticated);
        assert_eq!(context.required_scope, Some(SessionScope::Read));

        let err = control
            .authorize_request(&Method::POST, "/api/v1/sessions/s1/submit", &headers)
            .unwrap_err();
        assert_eq!(err.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn authorize_request_rejects_invalid_transport_mode_header() {
        let control = RemoteAccessControl::default();
        let mut headers = HeaderMap::new();
        headers.insert(
            HEADER_TRANSPORT_MODE,
            HeaderValue::from_static("carrier-pigeon"),
        );
        let err = control
            .authorize_request(&Method::GET, "/api/v1/sessions", &headers)
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }
}
