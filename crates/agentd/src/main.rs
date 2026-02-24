//! Alan agent daemon (`agentd`) - persistent service host for agent runtimes.

mod manager;
mod routes;
mod state;
mod websocket;

use alan_runtime::Config;
use anyhow::Result;
use axum::{
    Router,
    routing::{delete, get, post},
};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                .add_directive("core=debug".parse().unwrap())
                .add_directive("agentd=debug".parse().unwrap()),
        )
        .init();

    info!("Starting agentd (Alan agent daemon)");

    // Load configuration
    let config = Config::from_env();

    // Create app state
    let state = AppState::new(config);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(routes::health))
        // API routes
        .route(
            "/api/v1/sessions",
            post(routes::create_session).get(routes::list_sessions),
        )
        .route("/api/v1/sessions/{id}", get(routes::get_session))
        .route("/api/v1/sessions/{id}/read", get(routes::read_session))
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
            "/api/v1/sessions/{id}/submit",
            post(routes::submit_operation),
        )
        .route("/api/v1/sessions/{id}/events", get(routes::stream_events))
        .route("/api/v1/sessions/{id}/ws", get(websocket::ws_handler))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Get bind address from env or use default
    let addr: SocketAddr = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:8090".to_string())
        .parse()?;

    info!(%addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
