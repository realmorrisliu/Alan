//! Agent Runtime — the execution engine for the AI Turing Machine.
//!
//! Drives the agent loop: receive input → LLM generation → tool execution → state transition.

mod agent_loop;
mod engine;
mod loop_guard;
mod submission_handlers;
mod tool_orchestrator;
mod tool_policy;
mod turn_driver;
mod turn_executor;
mod turn_state;
mod turn_support;
mod virtual_tools;

pub use engine::{
    AgentConfig, RuntimeController, RuntimeEventEnvelope, RuntimeHandle, WorkspaceRuntimeConfig,
    spawn, spawn_with_llm_client, spawn_with_llm_client_and_tools, spawn_with_tool_registry,
};

// Re-export agent loop types for internal use
pub(crate) use agent_loop::RuntimeLoopState;
pub(crate) use turn_state::TurnState;

/// Configuration for the agent runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of tool loops per turn (0 = unlimited)
    pub max_tool_loops: usize,
    /// Maximum consecutive repeats of the same tool call
    pub tool_repeat_limit: usize,
    /// LLM request timeout in seconds
    pub llm_request_timeout_secs: u64,
    /// Temperature for generation
    pub temperature: f32,
    /// Max tokens for generation
    pub max_tokens: u32,
    /// Whether to enable prompt snapshots for debugging
    pub prompt_snapshot_enabled: bool,
    /// Maximum characters for prompt snapshots
    pub prompt_snapshot_max_chars: usize,
    /// Context compaction trigger threshold (number of messages)
    pub compaction_trigger_messages: usize,
    /// Number of recent messages to keep after compaction
    pub compaction_keep_last: usize,
    /// Tool approval behavior
    pub approval_policy: alan_protocol::ApprovalPolicy,
    /// Coarse sandbox mode for tool execution policy
    pub sandbox_mode: alan_protocol::SandboxMode,
    /// Budget tokens for provider-specific thinking/reasoning. None = disabled.
    pub thinking_budget_tokens: Option<u32>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_tool_loops: 0, // Unlimited by default
            tool_repeat_limit: 4,
            llm_request_timeout_secs: 180,
            temperature: 0.3,
            max_tokens: 2048,
            prompt_snapshot_enabled: false,
            prompt_snapshot_max_chars: 8000,
            compaction_trigger_messages: 60,
            compaction_keep_last: 20,
            approval_policy: alan_protocol::ApprovalPolicy::default(),
            sandbox_mode: alan_protocol::SandboxMode::default(),
            thinking_budget_tokens: None,
        }
    }
}

impl From<&crate::config::Config> for RuntimeConfig {
    fn from(config: &crate::config::Config) -> Self {
        Self {
            max_tool_loops: config.max_tool_loops.unwrap_or(0),
            tool_repeat_limit: config.tool_repeat_limit,
            llm_request_timeout_secs: config.llm_request_timeout_secs as u64,
            temperature: 0.3,
            max_tokens: 2048,
            prompt_snapshot_enabled: config.prompt_snapshot_enabled,
            prompt_snapshot_max_chars: config.prompt_snapshot_max_chars,
            compaction_trigger_messages: 60,
            compaction_keep_last: 20,
            approval_policy: alan_protocol::ApprovalPolicy::default(),
            sandbox_mode: alan_protocol::SandboxMode::default(),
            thinking_budget_tokens: config.thinking_budget_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert_eq!(config.max_tool_loops, 0); // Unlimited
        assert_eq!(config.tool_repeat_limit, 4);
        assert_eq!(config.llm_request_timeout_secs, 180);
        assert_eq!(config.temperature, 0.3);
        assert_eq!(config.max_tokens, 2048);
        assert!(!config.prompt_snapshot_enabled);
        assert_eq!(config.prompt_snapshot_max_chars, 8000);
        assert_eq!(config.compaction_trigger_messages, 60);
        assert_eq!(config.compaction_keep_last, 20);
    }

    #[test]
    fn test_runtime_config_clone() {
        let config = RuntimeConfig::default();
        let cloned = config.clone();
        assert_eq!(config.max_tool_loops, cloned.max_tool_loops);
        assert_eq!(config.tool_repeat_limit, cloned.tool_repeat_limit);
    }

    #[test]
    fn test_runtime_config_debug() {
        let config = RuntimeConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("RuntimeConfig"));
        assert!(debug_str.contains("max_tool_loops"));
    }

    #[test]
    fn test_runtime_config_from_core_config() {
        let core_config = crate::config::Config::default();
        let runtime_config = RuntimeConfig::from(&core_config);

        assert_eq!(
            runtime_config.tool_repeat_limit,
            core_config.tool_repeat_limit
        );
        assert_eq!(
            runtime_config.llm_request_timeout_secs,
            core_config.llm_request_timeout_secs as u64
        );
        assert_eq!(
            runtime_config.prompt_snapshot_enabled,
            core_config.prompt_snapshot_enabled
        );
        assert_eq!(
            runtime_config.prompt_snapshot_max_chars,
            core_config.prompt_snapshot_max_chars
        );
    }
}
