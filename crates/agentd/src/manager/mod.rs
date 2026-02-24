//! Agent manager module - multi-agent workspace isolation.
//!
//! This module provides the infrastructure for managing multiple independent
//! agent instances, each with its own isolated workspace, memory, and sessions.

pub mod instance;
pub mod agent_manager;

pub use agent_manager::{AgentManager, ManagerConfig};
