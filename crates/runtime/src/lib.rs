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

mod config;
mod approval;
mod llm;
mod retry;
mod rollout;
mod session;
pub mod tape;

pub mod manager;
pub mod prompts;
pub mod runtime;
pub mod skills;
pub mod tools;

pub use config::Config;
pub use approval::{
    PendingConfirmation, PendingDynamicToolCall, PendingStructuredInputRequest,
    ToolApprovalCacheKey, ToolApprovalDecision,
};
pub use llm::{
    GenerationRequest, GenerationResponse, LlmClient, TokenUsage, ToolCall, ToolDefinition,
};
pub use manager::{WorkspaceConfigState, WorkspaceInfo, WorkspaceState, WorkspaceStatus, PersistedLlmProvider};
pub use prompts::PromptLoader;
pub use rollout::{
    CheckpointRecord, EventRecord, MessageRecord, RolloutItem, RolloutRecorder, SessionMeta,
    ToolCallRecord,
};
pub use runtime::{AgentConfig, WorkspaceRuntimeConfig, RuntimeController, RuntimeHandle, spawn};
pub use session::Session;
pub use tools::ToolRegistry;
