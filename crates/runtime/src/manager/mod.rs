//! Agent state types — shared data structures for agent lifecycle management.
//!
//! This module contains the **data types** for agent state persistence.
//! The orchestration logic (AgentManager, AgentInstance) lives in the
//! `agentd` crate, as it is a hosting concern rather than a core runtime concern.

mod state;

pub use state::{AgentConfigState, AgentInfo, AgentState, AgentStatus, PersistedLlmProvider};
