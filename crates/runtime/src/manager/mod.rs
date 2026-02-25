//! Workspace state types — shared data structures for workspace lifecycle management.
//!
//! This module contains the **data types** for workspace state persistence.
//! The orchestration logic (WorkspaceManager, WorkspaceInstance) lives in the
//! `agentd` crate, as it is a hosting concern rather than a core runtime concern.

mod state;

pub use state::{
    PersistedLlmProvider, WorkspaceConfigState, WorkspaceInfo, WorkspaceState, WorkspaceStatus,
};
