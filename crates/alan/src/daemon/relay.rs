//! Relay MVP support for Phase B remote control.
//!
//! This module provides:
//! - relay-side node tunnel registration and heartbeat tracking
//! - relay-side request proxying through connected node tunnels
//! - node-side outbound tunnel client that forwards requests to local daemon APIs

use std::{
    collections::HashMap,
    env,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use anyhow::Context;
use axum::{
    Extension, Json,
    body::{Body, Bytes},
    extract::{OriginalUri, Path, WebSocketUpgrade, ws::Message as AxumWsMessage},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Response, StatusCode, Uri, header},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tracing::{debug, info, warn};
use uuid::Uuid;

const HEADER_NODE_ID: &str = "x-alan-node-id";
const HEADER_TRANSPORT_MODE: &str = "x-alan-transport-mode";

const ENV_RELAY_SERVER_ENABLED: &str = "ALAN_RELAY_SERVER_ENABLED";
const ENV_RELAY_NODE_TOKENS: &str = "ALAN_RELAY_NODE_TOKENS";
const ENV_RELAY_PROXY_TIMEOUT_SECS: &str = "ALAN_RELAY_PROXY_TIMEOUT_SECS";

const ENV_RELAY_URL: &str = "ALAN_RELAY_URL";
const ENV_RELAY_NODE_ID: &str = "ALAN_RELAY_NODE_ID";
const ENV_RELAY_NODE_TOKEN: &str = "ALAN_RELAY_NODE_TOKEN";
const ENV_RELAY_LOCAL_BASE_URL: &str = "ALAN_RELAY_LOCAL_BASE_URL";
const ENV_RELAY_HEARTBEAT_INTERVAL_SECS: &str = "ALAN_RELAY_HEARTBEAT_INTERVAL_SECS";
const ENV_RELAY_RECONNECT_DELAY_SECS: &str = "ALAN_RELAY_RECONNECT_DELAY_SECS";

const DEFAULT_PROXY_TIMEOUT_SECS: u64 = 20;
const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 15;
const DEFAULT_RECONNECT_DELAY_SECS: u64 = 2;
const DEFAULT_MPSC_BUFFER: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RelayFrame {
    NodeHello {
        node_id: String,
        connection_id: String,
        sent_at_ms: u64,
    },
    NodeHeartbeat {
        node_id: String,
        connection_id: String,
        sent_at_ms: u64,
    },
    RelayHeartbeat {
        node_id: String,
        connection_id: String,
        sent_at_ms: u64,
    },
    RelayProxyRequest {
        request_id: String,
        node_id: String,
        connection_id: String,
        method: String,
        path: String,
        headers: Vec<RelayHeader>,
        body: Option<String>,
    },
    NodeProxyResponse {
        request_id: String,
        node_id: String,
        connection_id: String,
        status: u16,
        headers: Vec<RelayHeader>,
        body: Option<String>,
        error: Option<String>,
    },
    RelayError {
        request_id: Option<String>,
        node_id: String,
        connection_id: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelayHeader {
    name: String,
    value: String,
}

#[derive(Debug, Clone)]
pub struct RelayServerConfig {
    enabled: bool,
    node_tokens: HashMap<String, String>,
    proxy_timeout: Duration,
}

impl RelayServerConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let enabled = env_var_truthy(ENV_RELAY_SERVER_ENABLED);
        let node_tokens_raw = env::var(ENV_RELAY_NODE_TOKENS).unwrap_or_default();
        let node_tokens = parse_node_tokens(&node_tokens_raw)?;
        let proxy_timeout = Duration::from_secs(env_var_u64(
            ENV_RELAY_PROXY_TIMEOUT_SECS,
            DEFAULT_PROXY_TIMEOUT_SECS,
        ));
        Ok(Self {
            enabled,
            node_tokens,
            proxy_timeout,
        })
    }

    fn authorize_node(&self, node_id: &str, headers: &HeaderMap) -> Result<(), RelayErrorCode> {
        if self.node_tokens.is_empty() {
            return Ok(());
        }

        let expected = self
            .node_tokens
            .get(node_id)
            .ok_or(RelayErrorCode::Unauthorized)?;
        let token = extract_bearer_token(headers).ok_or(RelayErrorCode::Unauthorized)?;
        if token != expected {
            return Err(RelayErrorCode::Unauthorized);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RelayHub {
    inner: Arc<RelayHubInner>,
}

#[derive(Debug)]
struct RelayHubInner {
    config: RelayServerConfig,
    nodes: RwLock<HashMap<String, Arc<NodeTunnel>>>,
}

#[derive(Debug)]
struct NodeTunnel {
    node_id: String,
    connection_id: String,
    connected_at_ms: u64,
    last_heartbeat_ms: AtomicU64,
    outbound_tx: mpsc::Sender<RelayFrame>,
    pending: Mutex<HashMap<String, oneshot::Sender<RelayNodeResponse>>>,
}

#[derive(Debug, Clone)]
struct RelayNodeResponse {
    status: u16,
    headers: Vec<RelayHeader>,
    body: Option<String>,
}

#[derive(Debug)]
enum RelayProxyError {
    NotConnected,
    Timeout,
    ForwardFailed,
    ResponseDropped,
}

#[derive(Debug, Clone, Serialize)]
struct RelayNodeStatus {
    node_id: String,
    connection_id: String,
    connected_at_ms: u64,
    last_heartbeat_ms: u64,
    pending_requests: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RelayNodeListResponse {
    nodes: Vec<RelayNodeStatus>,
}

#[derive(Debug, Clone, Serialize)]
struct RelayErrorResponse {
    code: &'static str,
    error: String,
}

#[derive(Debug, Clone, Copy)]
enum RelayErrorCode {
    BadRequest,
    Unauthorized,
    NotFound,
    Timeout,
    BadGateway,
}

impl RelayErrorCode {
    fn status(self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::BadGateway => StatusCode::BAD_GATEWAY,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::BadRequest => "relay_bad_request",
            Self::Unauthorized => "relay_unauthorized",
            Self::NotFound => "relay_node_not_found",
            Self::Timeout => "relay_timeout",
            Self::BadGateway => "relay_bad_gateway",
        }
    }
}

impl RelayHub {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self::new(RelayServerConfig::from_env()?))
    }

    pub fn new(config: RelayServerConfig) -> Self {
        Self {
            inner: Arc::new(RelayHubInner {
                config,
                nodes: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn enabled(&self) -> bool {
        self.inner.config.enabled
    }

    async fn register_node(
        &self,
        node_id: String,
        connection_id: String,
        outbound_tx: mpsc::Sender<RelayFrame>,
    ) {
        let tunnel = Arc::new(NodeTunnel {
            node_id: node_id.clone(),
            connection_id: connection_id.clone(),
            connected_at_ms: now_timestamp_ms(),
            last_heartbeat_ms: AtomicU64::new(now_timestamp_ms()),
            outbound_tx,
            pending: Mutex::new(HashMap::new()),
        });

        let replaced = self
            .inner
            .nodes
            .write()
            .await
            .insert(node_id.clone(), tunnel);
        if let Some(previous) = replaced {
            warn!(
                %node_id,
                old_connection_id = %previous.connection_id,
                new_connection_id = %connection_id,
                "Replacing existing relay tunnel for node"
            );
            previous.fail_all_pending("relay tunnel replaced").await;
        }

        info!(%node_id, %connection_id, "Relay tunnel connected");
    }

    async fn unregister_node(&self, node_id: &str, connection_id: &str) {
        let removed = {
            let mut nodes = self.inner.nodes.write().await;
            match nodes.get(node_id) {
                Some(tunnel) if tunnel.connection_id == connection_id => nodes.remove(node_id),
                _ => None,
            }
        };
        if let Some(tunnel) = removed {
            tunnel.fail_all_pending("relay tunnel disconnected").await;
            info!(%node_id, %connection_id, "Relay tunnel disconnected");
        }
    }

    async fn record_heartbeat(&self, node_id: &str, connection_id: &str) {
        let nodes = self.inner.nodes.read().await;
        if let Some(tunnel) = nodes.get(node_id)
            && tunnel.connection_id == connection_id
        {
            tunnel
                .last_heartbeat_ms
                .store(now_timestamp_ms(), Ordering::Relaxed);
        }
    }

    async fn resolve_proxy_response(
        &self,
        node_id: &str,
        connection_id: &str,
        request_id: String,
        response: RelayNodeResponse,
    ) {
        let nodes = self.inner.nodes.read().await;
        let Some(tunnel) = nodes.get(node_id) else {
            return;
        };
        if tunnel.connection_id != connection_id {
            return;
        }

        let tx = {
            let mut pending = tunnel.pending.lock().await;
            pending.remove(&request_id)
        };
        if let Some(tx) = tx {
            let _ = tx.send(response);
        }
    }

    async fn proxy_request(
        &self,
        node_id: &str,
        method: Method,
        path: String,
        headers: Vec<RelayHeader>,
        body: Option<String>,
    ) -> Result<RelayNodeResponse, RelayProxyError> {
        let tunnel = {
            let nodes = self.inner.nodes.read().await;
            nodes
                .get(node_id)
                .cloned()
                .ok_or(RelayProxyError::NotConnected)?
        };

        let request_id = Uuid::new_v4().to_string();
        let frame = RelayFrame::RelayProxyRequest {
            request_id: request_id.clone(),
            node_id: node_id.to_string(),
            connection_id: tunnel.connection_id.clone(),
            method: method.to_string(),
            path,
            headers,
            body,
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = tunnel.pending.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        if tunnel.outbound_tx.send(frame).await.is_err() {
            let mut pending = tunnel.pending.lock().await;
            pending.remove(&request_id);
            return Err(RelayProxyError::ForwardFailed);
        }

        let timeout = self.inner.config.proxy_timeout;
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(RelayProxyError::ResponseDropped),
            Err(_) => {
                let mut pending = tunnel.pending.lock().await;
                pending.remove(&request_id);
                Err(RelayProxyError::Timeout)
            }
        }
    }

    async fn list_nodes(&self) -> Vec<RelayNodeStatus> {
        let tunnels: Vec<Arc<NodeTunnel>> = {
            let nodes = self.inner.nodes.read().await;
            nodes.values().cloned().collect()
        };

        let mut statuses = Vec::with_capacity(tunnels.len());
        for tunnel in tunnels {
            let pending_requests = tunnel.pending.lock().await.len();
            statuses.push(RelayNodeStatus {
                node_id: tunnel.node_id.clone(),
                connection_id: tunnel.connection_id.clone(),
                connected_at_ms: tunnel.connected_at_ms,
                last_heartbeat_ms: tunnel.last_heartbeat_ms.load(Ordering::Relaxed),
                pending_requests,
            });
        }

        statuses.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        statuses
    }
}

impl NodeTunnel {
    async fn fail_all_pending(&self, message: &str) {
        let pending = {
            let mut guard = self.pending.lock().await;
            std::mem::take(&mut *guard)
        };
        for (_, tx) in pending {
            let _ = tx.send(RelayNodeResponse {
                status: StatusCode::BAD_GATEWAY.as_u16(),
                headers: vec![],
                body: Some(message.to_string()),
            });
        }
    }
}

pub async fn relay_list_nodes_handler(Extension(hub): Extension<RelayHub>) -> impl IntoResponse {
    let nodes = hub.list_nodes().await;
    Json(RelayNodeListResponse { nodes })
}

pub async fn relay_tunnel_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    Extension(hub): Extension<RelayHub>,
) -> Response<Body> {
    let Some(node_id) = parse_non_empty_header(&headers, HEADER_NODE_ID) else {
        return relay_error(
            RelayErrorCode::BadRequest,
            "missing required x-alan-node-id header",
        );
    };

    if let Err(code) = hub.inner.config.authorize_node(&node_id, &headers) {
        return relay_error(code, "node authentication failed");
    }

    ws.on_upgrade(move |socket| handle_relay_tunnel(socket, hub, node_id))
        .into_response()
}

pub async fn relay_proxy_handler(
    Path((node_id, tail_path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Extension(hub): Extension<RelayHub>,
    body: Bytes,
) -> Response<Body> {
    let Some(path) = build_forward_path(&tail_path, &uri) else {
        return relay_error(
            RelayErrorCode::BadRequest,
            "relay proxy path must target /api/v1/*",
        );
    };

    let forward_headers = collect_forward_headers(&headers, &node_id);
    let body = if body.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&body).into_owned())
    };

    match hub
        .proxy_request(&node_id, method, path, forward_headers, body)
        .await
    {
        Ok(response) => build_proxy_http_response(response),
        Err(err) => match err {
            RelayProxyError::NotConnected => {
                relay_error(RelayErrorCode::NotFound, "target node is not connected")
            }
            RelayProxyError::Timeout => relay_error(
                RelayErrorCode::Timeout,
                "relay timed out waiting for node response",
            ),
            RelayProxyError::ForwardFailed | RelayProxyError::ResponseDropped => relay_error(
                RelayErrorCode::BadGateway,
                "failed to proxy request through relay",
            ),
        },
    }
}

async fn handle_relay_tunnel(socket: axum::extract::ws::WebSocket, hub: RelayHub, node_id: String) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<RelayFrame>(DEFAULT_MPSC_BUFFER);

    hub.register_node(node_id.clone(), connection_id.clone(), outbound_tx.clone())
        .await;

    let writer = tokio::spawn(async move {
        while let Some(frame) = outbound_rx.recv().await {
            let Ok(payload) = serde_json::to_string(&frame) else {
                continue;
            };
            if ws_tx
                .send(AxumWsMessage::Text(payload.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let _ = outbound_tx
        .send(RelayFrame::RelayHeartbeat {
            node_id: node_id.clone(),
            connection_id: connection_id.clone(),
            sent_at_ms: now_timestamp_ms(),
        })
        .await;

    while let Some(result) = ws_rx.next().await {
        let message = match result {
            Ok(message) => message,
            Err(err) => {
                warn!(%node_id, %connection_id, error = %err, "Relay tunnel receive failed");
                break;
            }
        };

        match message {
            AxumWsMessage::Text(text) => {
                let frame: RelayFrame = match serde_json::from_str(&text) {
                    Ok(frame) => frame,
                    Err(err) => {
                        warn!(%node_id, %connection_id, error = %err, "Dropping invalid relay frame");
                        continue;
                    }
                };

                match frame {
                    RelayFrame::NodeHello {
                        node_id: hello_node,
                        connection_id: hello_conn,
                        ..
                    } => {
                        if hello_node != node_id || hello_conn != connection_id {
                            warn!(
                                expected_node = %node_id,
                                expected_connection = %connection_id,
                                actual_node = %hello_node,
                                actual_connection = %hello_conn,
                                "Node hello metadata mismatch"
                            );
                        } else {
                            debug!(%node_id, %connection_id, "Received relay node hello");
                        }
                    }
                    RelayFrame::NodeHeartbeat {
                        node_id: hb_node,
                        connection_id: hb_conn,
                        ..
                    } => {
                        if hb_node == node_id && hb_conn == connection_id {
                            hub.record_heartbeat(&node_id, &connection_id).await;
                            let _ = outbound_tx
                                .send(RelayFrame::RelayHeartbeat {
                                    node_id: node_id.clone(),
                                    connection_id: connection_id.clone(),
                                    sent_at_ms: now_timestamp_ms(),
                                })
                                .await;
                        }
                    }
                    RelayFrame::NodeProxyResponse {
                        request_id,
                        node_id: response_node,
                        connection_id: response_conn,
                        status,
                        headers,
                        body,
                        error,
                    } => {
                        if let Some(error_message) = error {
                            warn!(
                                %response_node,
                                %response_conn,
                                %request_id,
                                error = %error_message,
                                "Relay received node proxy error"
                            );
                        }
                        hub.resolve_proxy_response(
                            &response_node,
                            &response_conn,
                            request_id,
                            RelayNodeResponse {
                                status,
                                headers,
                                body,
                            },
                        )
                        .await;
                    }
                    RelayFrame::RelayError { message, .. } => {
                        warn!(%node_id, %connection_id, %message, "Relay tunnel peer reported error");
                    }
                    RelayFrame::RelayHeartbeat { .. } | RelayFrame::RelayProxyRequest { .. } => {
                        warn!(%node_id, %connection_id, "Ignoring unexpected relay frame from node");
                    }
                }
            }
            AxumWsMessage::Binary(_) => {
                warn!(%node_id, %connection_id, "Ignoring binary relay frame");
            }
            AxumWsMessage::Close(_) => {
                break;
            }
            AxumWsMessage::Ping(_) | AxumWsMessage::Pong(_) => {}
        }
    }

    writer.abort();
    hub.unregister_node(&node_id, &connection_id).await;
}

#[derive(Debug, Clone)]
pub struct RelayClientConfig {
    relay_url: String,
    node_id: String,
    node_token: Option<String>,
    local_base_url: String,
    heartbeat_interval: Duration,
    reconnect_delay: Duration,
}

impl RelayClientConfig {
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let relay_url = env::var(ENV_RELAY_URL).unwrap_or_default();
        if relay_url.trim().is_empty() {
            return Ok(None);
        }

        let node_id = env::var(ENV_RELAY_NODE_ID).with_context(|| {
            format!(
                "{} is required when {} is set",
                ENV_RELAY_NODE_ID, ENV_RELAY_URL
            )
        })?;

        if node_id.trim().is_empty() {
            anyhow::bail!("{} cannot be empty", ENV_RELAY_NODE_ID);
        }

        let node_token = env::var(ENV_RELAY_NODE_TOKEN)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let local_base_url = env::var(ENV_RELAY_LOCAL_BASE_URL)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(default_local_base_url);

        let heartbeat_interval = Duration::from_secs(env_var_u64(
            ENV_RELAY_HEARTBEAT_INTERVAL_SECS,
            DEFAULT_HEARTBEAT_INTERVAL_SECS,
        ));
        let reconnect_delay = Duration::from_secs(env_var_u64(
            ENV_RELAY_RECONNECT_DELAY_SECS,
            DEFAULT_RECONNECT_DELAY_SECS,
        ));

        Ok(Some(Self {
            relay_url: relay_url.trim().to_string(),
            node_id: node_id.trim().to_string(),
            node_token,
            local_base_url,
            heartbeat_interval,
            reconnect_delay,
        }))
    }
}

pub fn spawn_relay_client(config: RelayClientConfig) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(err) = run_relay_client_once(&config).await {
                warn!(
                    relay_url = %config.relay_url,
                    node_id = %config.node_id,
                    error = %err,
                    "Relay client disconnected"
                );
            }
            tokio::time::sleep(config.reconnect_delay).await;
        }
    })
}

async fn run_relay_client_once(config: &RelayClientConfig) -> anyhow::Result<()> {
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Message as TungsteniteMessage, client::IntoClientRequest},
    };

    let url = format!(
        "{}/api/v1/relay/tunnel",
        config.relay_url.trim_end_matches('/')
    );
    let mut request = url.clone().into_client_request()?;
    request.headers_mut().insert(
        HeaderName::from_static(HEADER_NODE_ID),
        HeaderValue::from_str(&config.node_id)?,
    );
    if let Some(token) = config.node_token.as_ref() {
        request.headers_mut().insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))?,
        );
    }

    let (socket, _) = connect_async(request)
        .await
        .with_context(|| format!("failed to connect relay tunnel: {url}"))?;
    let connection_id = Uuid::new_v4().to_string();
    let (mut ws_tx, mut ws_rx) = socket.split();

    let hello = RelayFrame::NodeHello {
        node_id: config.node_id.clone(),
        connection_id: connection_id.clone(),
        sent_at_ms: now_timestamp_ms(),
    };
    ws_tx
        .send(TungsteniteMessage::Text(
            serde_json::to_string(&hello)?.into(),
        ))
        .await?;

    let client = reqwest::Client::new();
    let mut heartbeat = tokio::time::interval(config.heartbeat_interval);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    info!(
        relay_url = %config.relay_url,
        node_id = %config.node_id,
        connection_id = %connection_id,
        "Relay client connected"
    );

    loop {
        tokio::select! {
        _ = heartbeat.tick() => {
            let frame = RelayFrame::NodeHeartbeat {
                node_id: config.node_id.clone(),
                connection_id: connection_id.clone(),
                sent_at_ms: now_timestamp_ms(),
            };
            ws_tx
                .send(TungsteniteMessage::Text(
                    serde_json::to_string(&frame)?.into(),
                ))
                .await?;
        }
        msg = ws_rx.next() => {
            let Some(msg) = msg else {
                anyhow::bail!("relay tunnel closed by peer");
            };
            let msg = msg?;
            match msg {
                TungsteniteMessage::Text(text) => {
                    let frame: RelayFrame = serde_json::from_str(&text)?;
                    match frame {
                        RelayFrame::RelayHeartbeat { .. } => {}
                        RelayFrame::RelayProxyRequest {
                            request_id,
                            node_id,
                            connection_id: frame_connection_id,
                            method,
                            path,
                            headers,
                            body,
                        } => {
                            let response = execute_local_proxy_request(
                                config,
                                &client,
                                request_id,
                                node_id,
                                &connection_id,
                                frame_connection_id,
                                method,
                                path,
                                headers,
                                body,
                            )
                            .await;
                            ws_tx
                                .send(TungsteniteMessage::Text(
                                    serde_json::to_string(&response)?.into(),
                                ))
                                .await?;
                        }
                        RelayFrame::RelayError { message, .. } => {
                            warn!(node_id = %config.node_id, %message, "Relay peer reported error");
                        }
                        RelayFrame::NodeHello { .. }
                        | RelayFrame::NodeHeartbeat { .. }
                        | RelayFrame::NodeProxyResponse { .. } => {
                            warn!(node_id = %config.node_id, "Ignoring unexpected frame on node relay client");
                        }
                    }
                }
                TungsteniteMessage::Binary(_) => {
                    warn!(node_id = %config.node_id, "Ignoring binary relay frame");
                }
                TungsteniteMessage::Close(_) => {
                    anyhow::bail!("relay tunnel closed");
                }
                    TungsteniteMessage::Ping(payload) => {
                        ws_tx.send(TungsteniteMessage::Pong(payload)).await?;
                    }
                    TungsteniteMessage::Pong(_) => {}
                    TungsteniteMessage::Frame(_) => {}
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_local_proxy_request(
    config: &RelayClientConfig,
    client: &reqwest::Client,
    request_id: String,
    node_id: String,
    active_connection_id: &str,
    incoming_connection_id: String,
    method: String,
    path: String,
    headers: Vec<RelayHeader>,
    body: Option<String>,
) -> RelayFrame {
    let response_for_error = |status: StatusCode, message: String| RelayFrame::NodeProxyResponse {
        request_id: request_id.clone(),
        node_id: node_id.clone(),
        connection_id: incoming_connection_id.clone(),
        status: status.as_u16(),
        headers: vec![],
        body: Some(message.clone()),
        error: Some(message),
    };

    if incoming_connection_id != active_connection_id {
        return response_for_error(
            StatusCode::BAD_REQUEST,
            "relay connection id mismatch".to_string(),
        );
    }

    if !is_allowed_forward_path(&path) {
        return response_for_error(
            StatusCode::BAD_REQUEST,
            "invalid proxied path; expected /api/v1/* excluding /api/v1/relay/*".to_string(),
        );
    }

    let parsed_method = match Method::from_bytes(method.as_bytes()) {
        Ok(method) => method,
        Err(_) => {
            return response_for_error(StatusCode::BAD_REQUEST, "invalid HTTP method".to_string());
        }
    };

    let url = format!("{}{}", config.local_base_url.trim_end_matches('/'), path);
    let mut request = client.request(parsed_method, url);

    for relay_header in headers {
        if is_hop_by_hop_header(&relay_header.name) {
            continue;
        }
        let Ok(name) = HeaderName::from_bytes(relay_header.name.as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_str(&relay_header.value) else {
            continue;
        };
        request = request.header(name, value);
    }

    request = request
        .header(HEADER_NODE_ID, &config.node_id)
        .header(HEADER_TRANSPORT_MODE, "relay");

    if let Some(body) = body {
        request = request.body(body);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            let mut relay_headers = vec![];
            for (name, value) in response.headers() {
                if is_hop_by_hop_header(name.as_str()) {
                    continue;
                }
                if let Ok(value) = value.to_str() {
                    relay_headers.push(RelayHeader {
                        name: name.as_str().to_string(),
                        value: value.to_string(),
                    });
                }
            }
            let body_text = response.text().await.ok();
            RelayFrame::NodeProxyResponse {
                request_id,
                node_id,
                connection_id: incoming_connection_id,
                status: status.as_u16(),
                headers: relay_headers,
                body: body_text,
                error: None,
            }
        }
        Err(err) => response_for_error(
            StatusCode::BAD_GATEWAY,
            format!("local proxy failed: {err}"),
        ),
    }
}

fn parse_node_tokens(raw: &str) -> anyhow::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    if raw.trim().is_empty() {
        return Ok(map);
    }

    for pair in raw.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (node_id, token) = pair.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("Invalid relay token binding `{pair}`; expected node_id=token")
        })?;
        let node_id = node_id.trim();
        let token = token.trim();
        if node_id.is_empty() || token.is_empty() {
            anyhow::bail!("Invalid relay token binding `{pair}`; node_id/token must be non-empty");
        }
        map.insert(node_id.to_string(), token.to_string());
    }

    Ok(map)
}

fn parse_non_empty_header(headers: &HeaderMap, name: &str) -> Option<String> {
    let value = headers.get(name)?.to_str().ok()?.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?.trim();
    let split_at = value.find(char::is_whitespace)?;
    let (scheme, token_part) = value.split_at(split_at);
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token_part.trim();
    if token.is_empty() {
        return None;
    }
    Some(token)
}

fn relay_error(code: RelayErrorCode, message: &str) -> Response<Body> {
    let payload = RelayErrorResponse {
        code: code.code(),
        error: message.to_string(),
    };
    (code.status(), Json(payload)).into_response()
}

fn build_forward_path(tail_path: &str, uri: &Uri) -> Option<String> {
    let normalized = format!("/{}", tail_path.trim_start_matches('/'));
    if !is_allowed_forward_path(&normalized) {
        return None;
    }

    let mut path = normalized;
    if let Some(query) = uri.query()
        && !query.is_empty()
    {
        path.push('?');
        path.push_str(query);
    }
    Some(path)
}

fn is_allowed_forward_path(path: &str) -> bool {
    path.starts_with("/api/v1/") && !path.starts_with("/api/v1/relay/")
}

fn collect_forward_headers(headers: &HeaderMap, node_id: &str) -> Vec<RelayHeader> {
    let mut result = vec![];

    for (name, value) in headers {
        if is_hop_by_hop_header(name.as_str()) {
            continue;
        }
        if name.as_str().eq_ignore_ascii_case(HEADER_NODE_ID)
            || name.as_str().eq_ignore_ascii_case(HEADER_TRANSPORT_MODE)
        {
            continue;
        }
        if let Ok(value) = value.to_str() {
            result.push(RelayHeader {
                name: name.as_str().to_string(),
                value: value.to_string(),
            });
        }
    }

    result.push(RelayHeader {
        name: HEADER_NODE_ID.to_string(),
        value: node_id.to_string(),
    });
    result.push(RelayHeader {
        name: HEADER_TRANSPORT_MODE.to_string(),
        value: "relay".to_string(),
    });

    result
}

fn build_proxy_http_response(response: RelayNodeResponse) -> Response<Body> {
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);

    for relay_header in response.headers {
        if is_hop_by_hop_header(&relay_header.name) {
            continue;
        }
        let Ok(name) = HeaderName::from_bytes(relay_header.name.as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_str(&relay_header.value) else {
            continue;
        };
        builder = builder.header(name, value);
    }

    builder
        .body(Body::from(response.body.unwrap_or_default()))
        .unwrap_or_else(|_| {
            relay_error(RelayErrorCode::BadGateway, "failed to build proxy response")
        })
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "proxy-connection"
            | "keep-alive"
            | "transfer-encoding"
            | "upgrade"
            | "te"
            | "trailers"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "content-length"
            | "host"
    )
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

fn env_var_u64(name: &str, default_value: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

fn default_local_base_url() -> String {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8090".to_string());
    let port = bind_address
        .rsplit(':')
        .next()
        .and_then(|raw| raw.parse::<u16>().ok())
        .unwrap_or(8090);
    format!("http://127.0.0.1:{port}")
}

fn now_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Message as TungsteniteMessage, client::IntoClientRequest},
    };

    #[test]
    fn parse_node_tokens_parses_bindings() {
        let parsed = parse_node_tokens("node-a=token-a;node-b=token-b").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed["node-a"], "token-a");
        assert_eq!(parsed["node-b"], "token-b");
    }

    #[test]
    fn parse_node_tokens_rejects_invalid_input() {
        assert!(parse_node_tokens("broken").is_err());
        assert!(parse_node_tokens("node=").is_err());
        assert!(parse_node_tokens("=token").is_err());
    }

    #[tokio::test]
    async fn relay_tunnel_requires_valid_node_token_when_configured() {
        let hub = RelayHub::new(RelayServerConfig {
            enabled: true,
            node_tokens: HashMap::from([("node-1".to_string(), "token-1".to_string())]),
            proxy_timeout: Duration::from_secs(3),
        });

        let app = Router::new()
            .route("/api/v1/relay/tunnel", get(relay_tunnel_handler))
            .layer(Extension(hub));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut req = format!("ws://{addr}/api/v1/relay/tunnel")
            .into_client_request()
            .unwrap();
        req.headers_mut().insert(
            HeaderName::from_static(HEADER_NODE_ID),
            HeaderValue::from_static("node-1"),
        );

        let err = connect_async(req)
            .await
            .expect_err("missing token should fail websocket upgrade");
        match err {
            tokio_tungstenite::tungstenite::Error::Http(response) => {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            }
            other => panic!("expected HTTP unauthorized error, got {other:?}"),
        }

        server.abort();
    }

    #[tokio::test]
    async fn relay_proxy_forwards_http_request_over_tunnel() {
        let hub = RelayHub::new(RelayServerConfig {
            enabled: true,
            node_tokens: HashMap::new(),
            proxy_timeout: Duration::from_secs(5),
        });

        let app = Router::new()
            .route("/api/v1/relay/tunnel", get(relay_tunnel_handler))
            .route(
                "/api/v1/relay/nodes/{node_id}/{*path}",
                axum::routing::any(relay_proxy_handler),
            )
            .layer(Extension(hub));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Connect fake node tunnel.
        let mut ws_req = format!("ws://{addr}/api/v1/relay/tunnel")
            .into_client_request()
            .unwrap();
        ws_req.headers_mut().insert(
            HeaderName::from_static(HEADER_NODE_ID),
            HeaderValue::from_static("node-a"),
        );
        let (ws_stream, _) = connect_async(ws_req).await.unwrap();
        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        // The first relay frame can be heartbeat; capture connection metadata from hello/request.
        let node_task = tokio::spawn(async move {
            let mut active_connection_id = String::new();

            // Send hello after connect once we know connection_id from relay heartbeat.
            while let Some(Ok(msg)) = ws_rx.next().await {
                let TungsteniteMessage::Text(text) = msg else {
                    continue;
                };
                let frame: RelayFrame = serde_json::from_str(&text).unwrap();
                match frame {
                    RelayFrame::RelayHeartbeat {
                        node_id,
                        connection_id,
                        ..
                    } => {
                        active_connection_id = connection_id.clone();
                        let hello = RelayFrame::NodeHello {
                            node_id,
                            connection_id: connection_id.clone(),
                            sent_at_ms: now_timestamp_ms(),
                        };
                        ws_tx
                            .send(TungsteniteMessage::Text(
                                serde_json::to_string(&hello).unwrap().into(),
                            ))
                            .await
                            .unwrap();
                    }
                    RelayFrame::RelayProxyRequest {
                        request_id,
                        node_id,
                        connection_id,
                        method,
                        path,
                        headers,
                        body,
                    } => {
                        assert_eq!(node_id, "node-a");
                        assert_eq!(connection_id, active_connection_id);
                        assert_eq!(method, "POST");
                        assert_eq!(path, "/api/v1/sessions/s1/submit?mode=test");
                        assert_eq!(body.as_deref(), Some("{\"op\":\"ping\"}"));
                        assert!(headers.iter().any(|h| {
                            h.name.eq_ignore_ascii_case(HEADER_TRANSPORT_MODE)
                                && h.value.eq_ignore_ascii_case("relay")
                        }));

                        let response = RelayFrame::NodeProxyResponse {
                            request_id,
                            node_id,
                            connection_id,
                            status: StatusCode::OK.as_u16(),
                            headers: vec![RelayHeader {
                                name: "content-type".to_string(),
                                value: "application/json".to_string(),
                            }],
                            body: Some("{\"ok\":true}".to_string()),
                            error: None,
                        };
                        ws_tx
                            .send(TungsteniteMessage::Text(
                                serde_json::to_string(&response).unwrap().into(),
                            ))
                            .await
                            .unwrap();
                        break;
                    }
                    _ => {}
                }
            }
        });

        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "http://{addr}/api/v1/relay/nodes/node-a/api/v1/sessions/s1/submit?mode=test"
            ))
            .header("authorization", "Bearer client-token")
            .body("{\"op\":\"ping\"}")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.text().await.unwrap(), "{\"ok\":true}");

        node_task.await.unwrap();
        server.abort();
    }
}
