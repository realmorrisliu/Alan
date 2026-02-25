//! Agent state persistence and management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// LLM provider type for persistence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedLlmProvider {
    Gemini,
    OpenaiCompatible,
    AnthropicCompatible,
}

/// Status of an agent instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    /// Agent is idle and waiting for input
    Idle,
    /// Agent is actively processing
    Running,
    /// Agent is paused (resources released but state preserved)
    Paused,
    /// Agent encountered an error
    Error,
    /// Agent is being destroyed
    Destroying,
}

impl std::fmt::Display for WorkspaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceStatus::Idle => write!(f, "idle"),
            WorkspaceStatus::Running => write!(f, "running"),
            WorkspaceStatus::Paused => write!(f, "paused"),
            WorkspaceStatus::Error => write!(f, "error"),
            WorkspaceStatus::Destroying => write!(f, "destroying"),
        }
    }
}

/// Persistent state for an agent instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    /// Unique agent identifier
    pub id: String,
    /// Current status
    pub status: WorkspaceStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_active: DateTime<Utc>,
    /// Current session ID (if any)
    pub current_session_id: Option<String>,
    /// Agent configuration overrides
    pub config: WorkspaceConfigState,
}

/// Configuration state for an agent
///
/// These fields are persisted to state.json so that agent behavior
/// remains consistent across restarts.
///
/// Note: Fields using `Option` type allow distinguishing between "not set" (None)
/// and "explicitly set to 0" (Some(0)), which is important for values like
/// `tool_repeat_limit` where 0 means "disable protection".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfigState {
    // Runtime behavior settings
    /// Maximum tool loops per turn (Some(0) = unlimited, None = use default)
    ///
    /// Note: Runtime semantics are 0 = unlimited, but we use Option for persistence
    /// to distinguish "explicitly set to 0" from "not set".
    pub max_tool_loops: Option<usize>,
    /// Tool repeat limit (Some(0) = disable protection, None = use default)
    pub tool_repeat_limit: Option<usize>,
    /// LLM request timeout in seconds (Some(0) = no timeout, None = use default)
    pub llm_timeout_secs: Option<usize>,
    /// Tool execution timeout in seconds (Some(0) = no ToolRegistry timeout, None = use default)
    ///
    /// Note: Setting this to 0 disables the ToolRegistry-level timeout wrapper
    /// and built-in Firecrawl HTTP timeouts. Custom tools may still enforce
    /// their own internal timeouts.
    pub tool_timeout_secs: Option<usize>,

    // LLM provider settings (persisted for consistency)
    /// LLM provider type
    pub llm_provider: Option<PersistedLlmProvider>,
    /// Model name (provider-specific)
    pub llm_model: Option<String>,
    /// Temperature for generation
    pub temperature: Option<f32>,
    /// Max tokens for generation
    pub max_tokens: Option<u32>,
    /// Tool approval policy
    pub approval_policy: Option<alan_protocol::ApprovalPolicy>,
    /// Coarse sandbox mode
    pub sandbox_mode: Option<alan_protocol::SandboxMode>,
}

impl WorkspaceState {
    /// Create a new agent state
    pub fn new(id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            status: WorkspaceStatus::Idle,
            created_at: now,
            last_active: now,
            current_session_id: None,
            config: WorkspaceConfigState::default(),
        }
    }

    /// Update last active timestamp
    pub fn touch(&mut self) {
        self.last_active = Utc::now();
    }

    /// Apply runtime configuration to persist agent behavior settings
    pub fn apply_runtime_config(&mut self, runtime_config: &crate::runtime::WorkspaceRuntimeConfig) {
        use crate::config::LlmProvider;

        // Persist runtime behavior settings
        // Use Some() to wrap values so we can distinguish "not set" from "set to 0"
        self.config.max_tool_loops = Some(runtime_config.agent_config.runtime_config.max_tool_loops);
        self.config.tool_repeat_limit = Some(runtime_config.agent_config.runtime_config.tool_repeat_limit);
        self.config.llm_timeout_secs =
            Some(runtime_config.agent_config.runtime_config.llm_request_timeout_secs as usize);
        self.config.tool_timeout_secs = Some(runtime_config.agent_config.core_config.tool_timeout_secs);
        self.config.temperature = Some(runtime_config.agent_config.runtime_config.temperature);
        self.config.max_tokens = Some(runtime_config.agent_config.runtime_config.max_tokens);
        self.config.approval_policy = Some(runtime_config.agent_config.runtime_config.approval_policy);
        self.config.sandbox_mode = Some(runtime_config.agent_config.runtime_config.sandbox_mode);

        // Persist LLM provider and model for consistency across restarts
        self.config.llm_provider = Some(match runtime_config.agent_config.core_config.llm_provider {
            LlmProvider::Gemini => PersistedLlmProvider::Gemini,
            LlmProvider::OpenaiCompatible => PersistedLlmProvider::OpenaiCompatible,
            LlmProvider::AnthropicCompatible => PersistedLlmProvider::AnthropicCompatible,
        });
        self.config.llm_model = Some(runtime_config.agent_config.core_config.effective_model().to_string());
    }

    /// Get the path to the state file within an agent directory
    pub fn state_file_path(agent_dir: &Path) -> PathBuf {
        agent_dir.join("state.json")
    }

    /// Load agent state from directory
    pub fn load(agent_dir: &Path) -> anyhow::Result<Self> {
        let path = Self::state_file_path(agent_dir);
        let content = std::fs::read_to_string(&path)?;
        let state: WorkspaceState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Save agent state to directory
    pub fn save(&self, agent_dir: &Path) -> anyhow::Result<()> {
        let path = Self::state_file_path(agent_dir);
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

/// Summary information about an agent (for listing)
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub session_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_agent_state_new() {
        let state = WorkspaceState::new("test-agent".to_string());
        assert_eq!(state.id, "test-agent");
        assert_eq!(state.status, WorkspaceStatus::Idle);
        assert!(state.current_session_id.is_none());
    }

    #[test]
    fn test_agent_state_touch() {
        let mut state = WorkspaceState::new("test".to_string());
        let before = state.last_active;
        std::thread::sleep(std::time::Duration::from_millis(10));
        state.touch();
        assert!(state.last_active > before);
    }

    #[test]
    fn test_agent_state_save_and_load() {
        let temp = TempDir::new().unwrap();
        let state = WorkspaceState::new("test-agent".to_string());

        state.save(temp.path()).unwrap();

        let loaded = WorkspaceState::load(temp.path()).unwrap();
        assert_eq!(loaded.id, state.id);
        assert_eq!(loaded.status, state.status);
    }

    #[test]
    fn test_agent_status_display() {
        assert_eq!(WorkspaceStatus::Idle.to_string(), "idle");
        assert_eq!(WorkspaceStatus::Running.to_string(), "running");
        assert_eq!(WorkspaceStatus::Paused.to_string(), "paused");
        assert_eq!(WorkspaceStatus::Error.to_string(), "error");
        assert_eq!(WorkspaceStatus::Destroying.to_string(), "destroying");
    }

    #[test]
    fn test_apply_runtime_config_uses_runtime_policy_fields() {
        let mut state = WorkspaceState::new("test-agent".to_string());
        let mut runtime_config = crate::runtime::WorkspaceRuntimeConfig::default();
        runtime_config.agent_config.runtime_config.approval_policy = alan_protocol::ApprovalPolicy::Never;
        runtime_config.agent_config.runtime_config.sandbox_mode = alan_protocol::SandboxMode::DangerFullAccess;

        state.apply_runtime_config(&runtime_config);

        assert_eq!(
            state.config.approval_policy,
            Some(alan_protocol::ApprovalPolicy::Never)
        );
        assert_eq!(
            state.config.sandbox_mode,
            Some(alan_protocol::SandboxMode::DangerFullAccess)
        );
    }
}
