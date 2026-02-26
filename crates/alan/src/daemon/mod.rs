//! Daemon module — HTTP/WS server hosting workspace runtimes.
//!
//! This module contains the `agentd` server logic, now invoked via `alan daemon start`.

pub mod manager;
pub mod routes;
pub mod server;
pub mod state;
pub mod websocket;
