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

Be concise, structured, and focused on helping the next LLM seamlessly continue the work.
"#;

/// Memory tools usage instructions
pub const MEMORY_PROMPT: &str = r#"## Memory Tools

You have access to durable, local memory files for persisting important information across conversations.

### Available Memory Tools

1. **memory_search** - Search for information in memory (keyword matching)
2. **memory_get** - Read specific lines from a memory file
3. **memory_write** - Write memory notes (defaults to today's daily log)

### When to Use Memory

**Always write:**
- User preferences (contact methods, communication style)
- Important decisions or requirements
- Key facts about the user's project or needs
- Commitments or promises made

**Always search:**
- User refers to "last time", "before", "remember"
- User asks about previous decisions or discussions
- Context seems to reference earlier conversation

If you're unsure whether prior context exists, run `memory_search` once before answering.

Memory files are stored in `memory/YYYY-MM-DD.md` and `MEMORY.md` under the memory workspace."#;
