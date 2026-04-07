//! Daemon server — runs the HTTP/WS API server.
//!
//! Extracted from the original `agentd` main function so it can be called
//! from the `alan daemon start` subcommand.

use alan_runtime::{Config, LoadedConfig};
use anyhow::Result;
use axum::{
    Extension, Router, middleware,
    routing::{any, delete, get, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};

use super::relay::{self, RelayClientConfig, RelayHub};
use super::remote_control::{RemoteAccessControl, remote_access_middleware};
use super::state::AppState;
use super::websocket;
use super::{auth_routes, routes};
use crate::host_config::HostConfig;

/// Run the daemon server with the given configuration.
///
/// This function blocks until the server is shut down.
#[allow(dead_code)]
pub async fn run_server(config: Config) -> Result<()> {
    run_server_with_loaded_config(LoadedConfig {
        config,
        path: None,
        source: alan_runtime::ConfigSourceKind::Default,
    })
    .await
}

pub async fn run_server_with_loaded_config(loaded_config: LoadedConfig) -> Result<()> {
    info!("Starting Alan daemon");

    // Create app state
    let state = AppState::from_loaded_config(loaded_config);
    state.start_cleanup_task();
    state.start_scheduler_task();
    if let Err(err) = state.ensure_sessions_recovered().await {
        warn!(error = %err, "Failed to recover persisted sessions during daemon startup");
    }
    let remote_access = Arc::new(RemoteAccessControl::from_env()?);
    let relay_hub = RelayHub::from_env()?;
    info!(
        remote_auth_enabled = remote_access.enabled(),
        "Remote access control initialized"
    );
    info!(
        relay_server_enabled = relay_hub.enabled(),
        "Relay server configuration initialized"
    );

    if let Some(relay_client_config) = RelayClientConfig::from_env()? {
        relay::spawn_relay_client(relay_client_config);
        info!("Relay outbound tunnel client started");
    }

    let remote_access_layer = {
        let remote_access = Arc::clone(&remote_access);
        middleware::from_fn(move |request, next| {
            let remote_access = Arc::clone(&remote_access);
            async move { remote_access_middleware(remote_access, request, next).await }
        })
    };

    // Build router
    let mut app = Router::new()
        // Health check
        .route("/health", get(routes::health))
        .route(
            "/api/v1/auth/providers/chatgpt/status",
            get(auth_routes::get_chatgpt_auth_status),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/logout",
            post(auth_routes::post_chatgpt_auth_logout),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/events",
            get(auth_routes::stream_chatgpt_auth_events),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/events/read",
            get(auth_routes::read_chatgpt_auth_events),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/login/device/start",
            post(auth_routes::start_chatgpt_device_login),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/login/device/complete",
            post(auth_routes::complete_chatgpt_device_login),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/login/browser/start",
            post(auth_routes::start_chatgpt_browser_login),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/login/browser/complete",
            post(auth_routes::complete_chatgpt_browser_login),
        )
        .route(
            "/api/v1/auth/providers/chatgpt/import",
            post(auth_routes::import_chatgpt_tokens),
        )
        // API routes
        .route(
            "/api/v1/sessions",
            post(routes::create_session).get(routes::list_sessions),
        )
        .route("/api/v1/skills/catalog", get(routes::get_skill_catalog))
        .route(
            "/api/v1/skills/changed",
            get(routes::get_skill_catalog_changed),
        )
        .route(
            "/api/v1/skills/overrides",
            post(routes::write_skill_override_route),
        )
        .route("/api/v1/sessions/{id}", get(routes::get_session))
        .route("/api/v1/sessions/{id}/read", get(routes::read_session))
        .route(
            "/api/v1/sessions/{id}/reconnect_snapshot",
            get(routes::reconnect_snapshot),
        )
        .route(
            "/api/v1/sessions/{id}/history",
            get(routes::get_session_history),
        )
        .route(
            "/api/v1/sessions/{id}/events/read",
            get(routes::read_events),
        )
        .route("/api/v1/sessions/{id}", delete(routes::delete_session))
        .route("/api/v1/sessions/{id}/resume", post(routes::resume_session))
        .route("/api/v1/sessions/{id}/fork", post(routes::fork_session))
        .route(
            "/api/v1/sessions/{id}/rollback",
            post(routes::rollback_session),
        )
        .route(
            "/api/v1/sessions/{id}/compact",
            post(routes::compact_session),
        )
        .route(
            "/api/v1/sessions/{id}/schedule_at",
            post(routes::schedule_session_at),
        )
        .route(
            "/api/v1/sessions/{id}/sleep_until",
            post(routes::sleep_session_until),
        )
        .route(
            "/api/v1/sessions/{id}/submit",
            post(routes::submit_operation),
        )
        .route("/api/v1/sessions/{id}/events", get(routes::stream_events))
        .route("/api/v1/sessions/{id}/ws", get(websocket::ws_handler))
        .with_state(state);

    if relay_hub.enabled() {
        let relay_router = Router::new()
            .route("/api/v1/relay/nodes", get(relay::relay_list_nodes_handler))
            .route("/api/v1/relay/tunnel", get(relay::relay_tunnel_handler))
            .route(
                "/api/v1/relay/nodes/{node_id}/{*path}",
                any(relay::relay_proxy_handler),
            )
            .layer(Extension(relay_hub.clone()));
        app = app.merge(relay_router);
    }

    app = app
        // Middleware
        .layer(remote_access_layer)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // Get bind address from env or use default
    let addr: SocketAddr = HostConfig::resolve_bind_address()?.parse()?;

    info!(%addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
