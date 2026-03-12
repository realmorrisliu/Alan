//! Alan Core — the AI Turing Machine runtime.
//!
//! This crate implements a generic agent runtime modeled as a Turing machine:
//! - **Tape**: `tape::Tape` — manages conversation context
//! - **State**: `Session` — holds tape, tools, skills, and runtime config
//! - **Transition**: The agent loop drives LLM generation and tool execution
//! - **Persistence**: `RolloutRecorder` — checkpoints every state transition
//!
//! The core is intentionally agnostic of LLM providers, tool implementations,
//! hosting concerns, and domain-specific behavior. It defines interfaces
//! (`Tool` trait, `ToolRegistry`) that outer crates implement.

mod approval;
mod config;
mod llm;
mod models;
mod policy;
mod retry;
mod rollout;
mod session;
pub mod tape;
pub mod terminology;

pub mod manager;
pub mod prompts;
pub mod runtime;
pub mod skills;
pub mod tools;

pub use config::{Config, PartialStreamRecoveryMode, StreamingMode};
pub use llm::{
    GenerationRequest, GenerationResponse, LlmClient, LlmProjection, TokenUsage, ToolCall,
    ToolDefinition,
};
pub use manager::{
    PersistedLlmProvider, WorkspaceConfigState, WorkspaceInfo, WorkspaceState, WorkspaceStatus,
};
pub use models::{ModelCatalog, ModelInfo};
pub use policy::{PolicyAction, PolicyDecision, PolicyEngine, PolicyProfile, PolicyRule};
pub use prompts::PromptLoader;
pub use rollout::{
    CheckpointRecord, EventRecord, MessageRecord, RolloutItem, RolloutRecorder, SessionMeta,
    ToolCallRecord, session_storage_key,
};
pub use runtime::{
    AgentConfig, RuntimeController, RuntimeEventEnvelope, RuntimeHandle, WorkspaceRuntimeConfig,
    spawn, spawn_with_llm_client,
};
pub use session::{ROLLBACK_NON_DURABLE_WARNING, Session};
pub use terminology::{
    TerminologyFileKind, TerminologyMigration, migrate_config_toml, migrate_model_overlay_toml,
    migrate_workspace_state_json, migration_command_hint,
};
pub use tools::ToolRegistry;
