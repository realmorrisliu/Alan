//! Daemon module — HTTP/WS server hosting workspace runtimes.
//!
//! This module contains the `agentd` server logic, now invoked via `alan daemon start`.

pub mod auth_control;
pub mod auth_routes;
pub mod manager;
pub mod relay;
pub mod remote_control;
pub mod routes;
pub mod runtime_manager;
pub mod scheduler;
pub mod server;
pub mod session_store;
pub mod state;
pub mod task_store;
pub mod websocket;
pub mod workspace_resolver;
