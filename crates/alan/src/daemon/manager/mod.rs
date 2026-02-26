//! Workspace manager module - multi-workspace isolation.
//!
//! This module provides the infrastructure for managing multiple independent
//! workspace instances, each with its own isolated workspace, memory, and sessions.

pub mod instance;
pub mod workspace_manager;

pub use workspace_manager::{ManagerConfig, WorkspaceManager};
