//! Alan — AI Turing Machine CLI & daemon library.
//!
//! This crate provides both the CLI binary and daemon server functionality.

pub mod cli;
pub mod daemon;
pub mod registry;

pub use registry::{WorkspaceRegistry, WorkspaceEntry, generate_workspace_id};

// Re-export daemon components for advanced use
pub use daemon::{
    workspace_resolver::{WorkspaceResolver, ResolvedWorkspace},
    runtime_manager::{RuntimeManager, RuntimeManagerConfig, RuntimeInfo},
    session_store::{SessionStore, SessionBinding},
};
