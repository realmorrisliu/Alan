//! Prompt management module.
//!
//! This module provides access to prompt templates that guide the agent's behavior.
//! Prompts are embedded at compile time for zero-cost runtime access.

mod assembler;
mod loader;
mod workspace;

pub use assembler::{build_agent_system_prompt, build_agent_system_prompt_for_workspace};
pub use loader::PromptLoader;
pub use workspace::ensure_workspace_bootstrap_files_at;

// ============================================================================
// Compile-time embedded prompts
// ============================================================================

/// Runtime base prompt - hard constraints that cannot be overridden
pub const RUNTIME_BASE_PROMPT: &str = include_str!("../../prompts/runtime_base.md");

/// Main system prompt defining the agent's role and behavior
pub const SYSTEM_PROMPT: &str = include_str!("../../prompts/system.md");

/// Compaction prompt for summarizing older conversation history
pub const COMPACT_PROMPT: &str = r#"You are performing a CONTEXT CHECKPOINT COMPACTION. Create a handoff summary for another LLM that will resume the task.

Include:
- Current progress and key decisions made
- Important context, constraints, or user preferences
- What remains to be done (clear next steps)
- Any critical data, examples, or references needed to continue
- If a previous compaction summary is included, integrate its key points into your new summary

Be concise, structured, and focused on helping the next LLM seamlessly continue the work.
"#;
