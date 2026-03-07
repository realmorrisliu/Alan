//! Configuration management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Memory configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub workspace_dir: Option<PathBuf>,
    pub strict_workspace: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            workspace_dir: None,
            strict_workspace: true,
        }
    }
}

/// Session durability configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DurabilityConfig {
    /// Fail startup instead of silently degrading to in-memory mode.
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    Gemini,
    Openai,
    OpenaiCompatible,
    AnthropicCompatible,
}

/// Runtime streaming behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamingMode {
    /// Use provider-native streaming when possible.
    #[default]
    Auto,
    /// Force streaming path.
    On,
    /// Force non-streaming path.
    Off,
}

/// Behavior when a streaming response is interrupted after visible output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PartialStreamRecoveryMode {
    /// Attempt one non-streaming continuation pass to recover from interruption.
    #[default]
    ContinueOnce,
    /// Keep partial output and do not attempt continuation.
    Off,
}

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // ========================================================================
    // LLM Provider Selection
    // ========================================================================
    /// Active LLM provider
    #[serde(default = "default_llm_provider")]
    pub llm_provider: LlmProvider,

    // ========================================================================
    // Gemini Configuration
    // ========================================================================
    /// GEMINI_PROJECT_ID
    #[serde(default)]
    pub gemini_project_id: Option<String>,

    /// GEMINI_LOCATION (default: us-central1)
    #[serde(default = "default_gemini_location")]
    pub gemini_location: String,

    /// GEMINI_MODEL (default: gemini-2.0-flash)
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,

    // ========================================================================
    // OpenAI Configuration
    // ========================================================================
    /// OPENAI_API_KEY
    #[serde(default)]
    pub openai_api_key: Option<String>,

    /// OPENAI_BASE_URL (default: <https://api.openai.com/v1>)
    #[serde(default = "default_openai_base_url")]
    pub openai_base_url: String,

    /// OPENAI_MODEL (default: gpt-4o)
    #[serde(default = "default_openai_model")]
    pub openai_model: String,

    // ========================================================================
    // OpenAI-compatible Configuration
    // ========================================================================
    /// OPENAI_COMPAT_API_KEY
    #[serde(default)]
    pub openai_compat_api_key: Option<String>,

    /// OPENAI_COMPAT_BASE_URL (default: <https://api.openai.com/v1>)
    #[serde(default = "default_openai_compat_base_url")]
    pub openai_compat_base_url: String,

    /// OPENAI_COMPAT_MODEL (default: gpt-4o)
    #[serde(default = "default_openai_compat_model")]
    pub openai_compat_model: String,

    // ========================================================================
    // Anthropic-compatible Configuration
    // ========================================================================
    /// ANTHROPIC_COMPAT_API_KEY
    #[serde(default)]
    pub anthropic_compat_api_key: Option<String>,

    /// ANTHROPIC_COMPAT_BASE_URL (default: <https://api.anthropic.com/v1>)
    #[serde(default = "default_anthropic_compat_base_url")]
    pub anthropic_compat_base_url: String,

    /// ANTHROPIC_COMPAT_MODEL (default: claude-3-5-sonnet-latest)
    #[serde(default = "default_anthropic_compat_model")]
    pub anthropic_compat_model: String,

    /// ANTHROPIC_COMPAT_CLIENT_NAME - Client name for usage tracking (e.g., "marco")
    #[serde(default)]
    pub anthropic_compat_client_name: Option<String>,

    /// ANTHROPIC_COMPAT_USER_AGENT - Custom User-Agent header
    #[serde(default)]
    pub anthropic_compat_user_agent: Option<String>,

    /// LLM request timeout in seconds
    #[serde(default = "default_llm_timeout_secs")]
    pub llm_request_timeout_secs: usize,

    /// Tool execution timeout in seconds
    #[serde(default = "default_tool_timeout_secs")]
    pub tool_timeout_secs: usize,

    /// Optional hard limit for tool-call loop iterations in a single turn.
    /// `None` means unlimited (default).
    #[serde(default)]
    pub max_tool_loops: Option<usize>,

    /// Consecutive identical tool-call guard.
    /// Set to 0 to disable this guard.
    #[serde(default = "default_tool_repeat_limit")]
    pub tool_repeat_limit: usize,

    // ========================================================================
    // Prompt Logging
    // ========================================================================
    /// Enable prompt snapshot logging for observability
    #[serde(default)]
    pub prompt_snapshot_enabled: bool,

    /// Max characters to include in prompt snapshots
    #[serde(default = "default_prompt_snapshot_max_chars")]
    pub prompt_snapshot_max_chars: usize,

    // ========================================================================
    // Thinking / Reasoning Controls
    // ========================================================================
    /// Budget tokens for provider-specific thinking/reasoning. None = disabled.
    #[serde(default)]
    pub thinking_budget_tokens: Option<u32>,

    /// Streaming strategy (`auto`/`on`/`off`).
    #[serde(default = "default_streaming_mode")]
    pub streaming_mode: StreamingMode,

    /// Recovery strategy when streaming is interrupted after visible output.
    #[serde(default = "default_partial_stream_recovery_mode")]
    pub partial_stream_recovery_mode: PartialStreamRecoveryMode,

    // ========================================================================
    // Memory Configuration
    // ========================================================================
    #[serde(default)]
    pub memory: MemoryConfig,

    // ========================================================================
    // Durability Configuration
    // ========================================================================
    #[serde(default)]
    pub durability: DurabilityConfig,
}

fn default_llm_provider() -> LlmProvider {
    LlmProvider::Openai
}

fn default_openai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_openai_model() -> String {
    "gpt-4o".to_string()
}

fn default_gemini_location() -> String {
    "us-central1".to_string()
}

fn default_gemini_model() -> String {
    "gemini-2.0-flash".to_string()
}

fn default_openai_compat_base_url() -> String {
    default_openai_base_url()
}

fn default_openai_compat_model() -> String {
    default_openai_model()
}

fn default_anthropic_compat_base_url() -> String {
    "https://api.anthropic.com/v1".to_string()
}

fn default_anthropic_compat_model() -> String {
    "claude-3-5-sonnet-latest".to_string()
}

fn default_llm_timeout_secs() -> usize {
    180
}

fn default_tool_timeout_secs() -> usize {
    30
}

fn default_prompt_snapshot_max_chars() -> usize {
    8000
}

fn default_tool_repeat_limit() -> usize {
    4
}

fn default_streaming_mode() -> StreamingMode {
    StreamingMode::Auto
}

fn default_partial_stream_recovery_mode() -> PartialStreamRecoveryMode {
    PartialStreamRecoveryMode::ContinueOnce
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm_provider: default_llm_provider(),
            gemini_project_id: None,
            gemini_location: default_gemini_location(),
            gemini_model: default_gemini_model(),
            openai_api_key: None,
            openai_base_url: default_openai_base_url(),
            openai_model: default_openai_model(),
            openai_compat_api_key: None,
            openai_compat_base_url: default_openai_compat_base_url(),
            openai_compat_model: default_openai_compat_model(),
            anthropic_compat_api_key: None,
            anthropic_compat_base_url: default_anthropic_compat_base_url(),
            anthropic_compat_model: default_anthropic_compat_model(),
            anthropic_compat_client_name: None,
            anthropic_compat_user_agent: None,
            llm_request_timeout_secs: default_llm_timeout_secs(),
            tool_timeout_secs: default_tool_timeout_secs(),
            max_tool_loops: None,
            tool_repeat_limit: default_tool_repeat_limit(),
            prompt_snapshot_enabled: false,
            prompt_snapshot_max_chars: default_prompt_snapshot_max_chars(),
            thinking_budget_tokens: None,
            streaming_mode: default_streaming_mode(),
            partial_stream_recovery_mode: default_partial_stream_recovery_mode(),

            memory: MemoryConfig::default(),
            durability: DurabilityConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from config file (~/.config/alan/config.toml or ALAN_CONFIG_PATH).
    /// Falls back to defaults if no config file is found.
    pub fn load() -> Self {
        Self::load_with_paths(
            Self::env_override_config_path(),
            Self::home_config_file_path(),
        )
    }

    /// Load configuration from file (TOML format)
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the config file path.
    /// Resolution order:
    /// 1. `ALAN_CONFIG_PATH` override
    /// 2. `~/.config/alan/config.toml`
    pub fn config_file_path() -> Option<std::path::PathBuf> {
        Self::resolve_config_file_path(
            Self::env_override_config_path(),
            Self::home_config_file_path(),
        )
    }

    fn env_override_config_path() -> Option<std::path::PathBuf> {
        std::env::var("ALAN_CONFIG_PATH")
            .ok()
            .map(std::path::PathBuf::from)
    }

    fn home_config_file_path() -> Option<std::path::PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Self::home_config_file_path_from_home(std::path::Path::new(&home))
    }

    fn home_config_file_path_from_home(home: &std::path::Path) -> Option<std::path::PathBuf> {
        Some(
            std::path::PathBuf::from(home)
                .join(".config")
                .join("alan")
                .join("config.toml"),
        )
    }

    fn resolve_config_file_path(
        override_path: Option<std::path::PathBuf>,
        home_path: Option<std::path::PathBuf>,
    ) -> Option<std::path::PathBuf> {
        if let Some(path) = override_path {
            return Some(path);
        }

        if let Some(path) = home_path
            && path.exists()
        {
            return Some(path);
        }

        None
    }

    fn load_with_paths(
        override_path: Option<std::path::PathBuf>,
        home_path: Option<std::path::PathBuf>,
    ) -> Self {
        if let Some(config_path) = override_path {
            match Self::from_file(&config_path) {
                Ok(config) => {
                    tracing::info!(path = %config_path.display(), "Loaded configuration from file");
                    return config;
                }
                Err(e) => {
                    tracing::warn!(
                        path = %config_path.display(),
                        error = %e,
                        "Failed to load config file from ALAN_CONFIG_PATH, falling back to home config/defaults"
                    );
                }
            }
        }

        if let Some(config_path) = home_path
            && config_path.exists()
        {
            match Self::from_file(&config_path) {
                Ok(config) => {
                    tracing::info!(path = %config_path.display(), "Loaded configuration from file");
                    return config;
                }
                Err(e) => {
                    tracing::warn!(path = %config_path.display(), error = %e, "Failed to load config file, using defaults");
                }
            }
        }

        Self::default()
    }

    pub fn for_gemini(project_id: &str, location: Option<&str>, model: Option<&str>) -> Self {
        Self {
            llm_provider: LlmProvider::Gemini,
            gemini_project_id: Some(project_id.to_string()),
            gemini_location: location
                .map(ToString::to_string)
                .unwrap_or_else(default_gemini_location),
            gemini_model: model
                .map(ToString::to_string)
                .unwrap_or_else(default_gemini_model),
            ..Self::default()
        }
    }

    pub fn for_openai(api_key: &str, base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            llm_provider: LlmProvider::Openai,
            openai_api_key: Some(api_key.to_string()),
            openai_base_url: base_url
                .map(ToString::to_string)
                .unwrap_or_else(default_openai_base_url),
            openai_model: model
                .map(ToString::to_string)
                .unwrap_or_else(default_openai_model),
            ..Self::default()
        }
    }

    pub fn for_openai_compatible(
        api_key: &str,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Self {
        Self {
            llm_provider: LlmProvider::OpenaiCompatible,
            openai_compat_api_key: Some(api_key.to_string()),
            openai_compat_base_url: base_url
                .map(ToString::to_string)
                .unwrap_or_else(default_openai_compat_base_url),
            openai_compat_model: model
                .map(ToString::to_string)
                .unwrap_or_else(default_openai_compat_model),
            ..Self::default()
        }
    }

    pub fn for_anthropic_compatible(
        api_key: &str,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Self {
        Self {
            llm_provider: LlmProvider::AnthropicCompatible,
            anthropic_compat_api_key: Some(api_key.to_string()),
            anthropic_compat_base_url: base_url
                .map(ToString::to_string)
                .unwrap_or_else(default_anthropic_compat_base_url),
            anthropic_compat_model: model
                .map(ToString::to_string)
                .unwrap_or_else(default_anthropic_compat_model),
            ..Self::default()
        }
    }

    pub fn has_gemini_config(&self) -> bool {
        self.gemini_project_id.is_some()
    }

    pub fn has_openai_config(&self) -> bool {
        self.openai_api_key.is_some() || self.openai_compat_api_key.is_some()
    }

    pub fn has_openai_compatible_config(&self) -> bool {
        self.openai_compat_api_key.is_some()
    }

    pub fn has_anthropic_compatible_config(&self) -> bool {
        self.anthropic_compat_api_key.is_some()
    }

    pub fn has_llm_config(&self) -> bool {
        match self.llm_provider {
            LlmProvider::Gemini => self.has_gemini_config(),
            LlmProvider::Openai => self.has_openai_config(),
            LlmProvider::OpenaiCompatible => self.has_openai_compatible_config(),
            LlmProvider::AnthropicCompatible => self.has_anthropic_compatible_config(),
        }
    }

    pub fn effective_model(&self) -> &str {
        match self.llm_provider {
            LlmProvider::Gemini => &self.gemini_model,
            LlmProvider::Openai => self.resolved_openai_model(),
            LlmProvider::OpenaiCompatible => &self.openai_compat_model,
            LlmProvider::AnthropicCompatible => &self.anthropic_compat_model,
        }
    }

    fn use_openai_compat_fallback(&self) -> bool {
        self.openai_api_key.is_none() && self.openai_compat_api_key.is_some()
    }

    fn resolved_openai_api_key(&self) -> Option<&String> {
        self.openai_api_key
            .as_ref()
            .or(self.openai_compat_api_key.as_ref())
    }

    fn resolved_openai_base_url(&self) -> &str {
        if self.use_openai_compat_fallback() && self.openai_base_url == default_openai_base_url() {
            &self.openai_compat_base_url
        } else {
            &self.openai_base_url
        }
    }

    fn resolved_openai_model(&self) -> &str {
        if self.use_openai_compat_fallback() && self.openai_model == default_openai_model() {
            &self.openai_compat_model
        } else {
            &self.openai_model
        }
    }

    /// Convert to LLM provider configuration
    pub fn to_provider_config(&self) -> anyhow::Result<crate::llm::ProviderConfig> {
        use crate::llm::factory::ProviderConfig;

        match self.llm_provider {
            LlmProvider::Gemini => {
                let project_id = self
                    .gemini_project_id
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Gemini requires GEMINI_PROJECT_ID"))?;
                Ok(ProviderConfig::gemini(project_id, &self.gemini_model)
                    .with_location(&self.gemini_location))
            }
            LlmProvider::Openai => {
                let api_key = self.resolved_openai_api_key().ok_or_else(|| {
                    anyhow::anyhow!(
                        "OpenAI provider requires OPENAI_API_KEY (or legacy OPENAI_COMPAT_API_KEY)"
                    )
                })?;
                Ok(
                    ProviderConfig::openai(api_key, self.resolved_openai_model())
                        .with_base_url(self.resolved_openai_base_url()),
                )
            }
            LlmProvider::OpenaiCompatible => {
                let api_key = self.openai_compat_api_key.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("OpenAI-compatible provider requires OPENAI_COMPAT_API_KEY")
                })?;
                Ok(
                    ProviderConfig::openai_compatible(api_key, &self.openai_compat_model)
                        .with_base_url(&self.openai_compat_base_url),
                )
            }
            LlmProvider::AnthropicCompatible => {
                let api_key = self.anthropic_compat_api_key.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Anthropic-compatible provider requires ANTHROPIC_COMPAT_API_KEY"
                    )
                })?;
                let mut config = ProviderConfig::anthropic(api_key, &self.anthropic_compat_model)
                    .with_base_url(&self.anthropic_compat_base_url);

                if let Some(client_name) = &self.anthropic_compat_client_name {
                    config = config.with_client_name(client_name);
                }
                if let Some(user_agent) = &self.anthropic_compat_user_agent {
                    config = config.with_user_agent(user_agent);
                }

                Ok(config)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.llm_provider, LlmProvider::Openai);
        assert_eq!(config.gemini_location, "us-central1");
        assert_eq!(config.gemini_model, "gemini-2.0-flash");
        assert_eq!(config.openai_base_url, "https://api.openai.com/v1");
        assert_eq!(config.openai_model, "gpt-4o");
        assert_eq!(config.openai_compat_base_url, "https://api.openai.com/v1");
        assert_eq!(config.openai_compat_model, "gpt-4o");
        assert_eq!(
            config.anthropic_compat_base_url,
            "https://api.anthropic.com/v1"
        );
        assert_eq!(config.anthropic_compat_model, "claude-3-5-sonnet-latest");
        assert_eq!(config.llm_request_timeout_secs, 180);
        assert_eq!(config.tool_timeout_secs, 30);
        assert_eq!(config.tool_repeat_limit, 4);
        assert_eq!(config.prompt_snapshot_max_chars, 8000);
        assert!(!config.prompt_snapshot_enabled);
        assert!(config.max_tool_loops.is_none());
        assert_eq!(config.streaming_mode, StreamingMode::Auto);
        assert_eq!(
            config.partial_stream_recovery_mode,
            PartialStreamRecoveryMode::ContinueOnce
        );
        // Memory config
        assert!(config.memory.enabled);
        assert!(config.memory.strict_workspace);
        assert!(config.memory.workspace_dir.is_none());
        assert!(!config.durability.required);
    }

    #[test]
    fn test_config_for_gemini() {
        let config = Config::for_gemini("project", Some("europe-west1"), Some("gemini-2.5-pro"));
        assert_eq!(config.llm_provider, LlmProvider::Gemini);
        assert_eq!(config.gemini_project_id, Some("project".to_string()));
        assert_eq!(config.gemini_location, "europe-west1");
        assert_eq!(config.gemini_model, "gemini-2.5-pro");
        assert!(config.has_gemini_config());
        assert!(config.has_llm_config());
    }

    #[test]
    fn test_config_for_gemini_defaults() {
        let config = Config::for_gemini("project", None, None);
        assert_eq!(config.gemini_location, "us-central1");
        assert_eq!(config.gemini_model, "gemini-2.0-flash");
    }

    #[test]
    fn test_config_for_openai() {
        let config = Config::for_openai(
            "sk-test",
            Some("https://api.openai.com/v1"),
            Some("gpt-4.1"),
        );
        assert_eq!(config.llm_provider, LlmProvider::Openai);
        assert_eq!(config.openai_api_key, Some("sk-test".to_string()));
        assert_eq!(config.openai_model, "gpt-4.1");
        assert!(config.has_openai_config());
        assert!(config.has_llm_config());
    }

    #[test]
    fn test_config_for_openai_defaults() {
        let config = Config::for_openai("sk-test", None, None);
        assert_eq!(config.openai_base_url, "https://api.openai.com/v1");
        assert_eq!(config.openai_model, "gpt-4o");
    }

    #[test]
    fn test_config_for_openai_compatible() {
        let config = Config::for_openai_compatible(
            "sk-test",
            Some("https://api.openai.com/v1"),
            Some("gpt-4.1"),
        );
        assert_eq!(config.llm_provider, LlmProvider::OpenaiCompatible);
        assert_eq!(config.openai_compat_api_key, Some("sk-test".to_string()));
        assert_eq!(config.openai_compat_model, "gpt-4.1");
        assert!(config.has_openai_compatible_config());
        assert!(config.has_llm_config());
    }

    #[test]
    fn test_config_for_openai_compatible_defaults() {
        let config = Config::for_openai_compatible("sk-test", None, None);
        assert_eq!(config.openai_compat_base_url, "https://api.openai.com/v1");
        assert_eq!(config.openai_compat_model, "gpt-4o");
    }

    #[test]
    fn test_config_for_anthropic_compatible() {
        let config = Config::for_anthropic_compatible(
            "ak-test",
            Some("https://api.anthropic.com/v1"),
            Some("claude-sonnet-4-5"),
        );
        assert_eq!(config.llm_provider, LlmProvider::AnthropicCompatible);
        assert_eq!(config.anthropic_compat_api_key, Some("ak-test".to_string()));
        assert_eq!(config.anthropic_compat_model, "claude-sonnet-4-5");
        assert!(config.has_anthropic_compatible_config());
        assert!(config.has_llm_config());
    }

    #[test]
    fn test_config_for_anthropic_compatible_with_options() {
        let config = Config {
            llm_provider: LlmProvider::AnthropicCompatible,
            anthropic_compat_api_key: Some("key".to_string()),
            anthropic_compat_base_url: "https://api.anthropic.com/v1".to_string(),
            anthropic_compat_model: "claude-3".to_string(),
            anthropic_compat_client_name: Some("test-client".to_string()),
            anthropic_compat_user_agent: Some("test-agent/1.0".to_string()),
            ..Config::default()
        };
        assert_eq!(
            config.anthropic_compat_client_name,
            Some("test-client".to_string())
        );
        assert_eq!(
            config.anthropic_compat_user_agent,
            Some("test-agent/1.0".to_string())
        );
    }

    #[test]
    fn test_config_for_anthropic_compatible_defaults() {
        let config = Config::for_anthropic_compatible("ak-test", None, None);
        assert_eq!(
            config.anthropic_compat_base_url,
            "https://api.anthropic.com/v1"
        );
        assert_eq!(config.anthropic_compat_model, "claude-3-5-sonnet-latest");
    }

    #[test]
    fn test_effective_model() {
        let gemini = Config::for_gemini("project", None, Some("gemini-2.5-pro"));
        assert_eq!(gemini.effective_model(), "gemini-2.5-pro");

        let openai = Config::for_openai("k", None, Some("gpt-4.1"));
        assert_eq!(openai.effective_model(), "gpt-4.1");

        let openai_compatible = Config::for_openai_compatible("k", None, Some("gpt-4.1-mini"));
        assert_eq!(openai_compatible.effective_model(), "gpt-4.1-mini");

        let anthropic = Config::for_anthropic_compatible("k", None, Some("claude-3-5-sonnet"));
        assert_eq!(anthropic.effective_model(), "claude-3-5-sonnet");
    }

    #[test]
    fn test_has_llm_config_without_api_key() {
        let mut config = Config {
            llm_provider: LlmProvider::Openai,
            openai_api_key: None,
            openai_compat_api_key: None,
            ..Config::default()
        };
        assert!(!config.has_openai_config());
        assert!(!config.has_llm_config());

        config.llm_provider = LlmProvider::OpenaiCompatible;
        assert!(!config.has_openai_compatible_config());
        assert!(!config.has_llm_config());

        config.llm_provider = LlmProvider::AnthropicCompatible;
        config.anthropic_compat_api_key = None;
        assert!(!config.has_anthropic_compatible_config());
        assert!(!config.has_llm_config());

        config.llm_provider = LlmProvider::Gemini;
        config.gemini_project_id = None;
        assert!(!config.has_gemini_config());
    }

    #[test]
    fn test_llm_provider_deserialization() {
        let gemini: LlmProvider = serde_json::from_str("\"gemini\"").unwrap();
        assert_eq!(gemini, LlmProvider::Gemini);

        let openai: LlmProvider = serde_json::from_str("\"openai\"").unwrap();
        assert_eq!(openai, LlmProvider::Openai);

        let openai: LlmProvider = serde_json::from_str("\"openai_compatible\"").unwrap();
        assert_eq!(openai, LlmProvider::OpenaiCompatible);

        let anthropic: LlmProvider = serde_json::from_str("\"anthropic_compatible\"").unwrap();
        assert_eq!(anthropic, LlmProvider::AnthropicCompatible);
    }

    #[test]
    fn test_config_from_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("test_config.toml");

        let toml_content = r#"
llm_provider = "openai"
openai_api_key = "sk-test123"
openai_model = "gpt-4"
llm_request_timeout_secs = 300
tool_timeout_secs = 60
streaming_mode = "off"
partial_stream_recovery_mode = "off"
"#;

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.llm_provider, LlmProvider::Openai);
        assert_eq!(config.openai_api_key, Some("sk-test123".to_string()));
        assert_eq!(config.openai_model, "gpt-4");
        assert_eq!(config.llm_request_timeout_secs, 300);
        assert_eq!(config.tool_timeout_secs, 60);
        assert_eq!(config.streaming_mode, StreamingMode::Off);
        assert_eq!(
            config.partial_stream_recovery_mode,
            PartialStreamRecoveryMode::Off
        );
    }

    #[test]
    fn test_config_from_file_not_found() {
        let result = Config::from_file(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_file_invalid_toml() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("invalid.toml");

        std::fs::write(&config_path, "not valid toml {{").unwrap();

        let result = Config::from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_file_path_prefers_alan_config_path_env() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let home_config = Config::home_config_file_path_from_home(&home).unwrap();
        std::fs::create_dir_all(home_config.parent().unwrap()).unwrap();
        std::fs::write(&home_config, "llm_provider = \"gemini\"\n").unwrap();

        let override_path = temp.path().join("override.toml");
        std::fs::write(&override_path, "llm_provider = \"gemini\"\n").unwrap();

        let resolved =
            Config::resolve_config_file_path(Some(override_path.clone()), Some(home_config))
                .unwrap();
        assert_eq!(resolved, override_path);
    }

    #[test]
    fn test_config_file_path_uses_home_config_dir() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let home_config = Config::home_config_file_path_from_home(&home).unwrap();
        std::fs::create_dir_all(home_config.parent().unwrap()).unwrap();
        std::fs::write(&home_config, "llm_provider = \"gemini\"\n").unwrap();

        let resolved = Config::resolve_config_file_path(None, Some(home_config.clone())).unwrap();
        assert_eq!(resolved, home_config);
    }

    #[test]
    fn test_load_falls_back_to_home_when_alan_config_path_missing() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let home_config = Config::home_config_file_path_from_home(&home).unwrap();
        std::fs::create_dir_all(home_config.parent().unwrap()).unwrap();
        std::fs::write(&home_config, "llm_provider = \"openai\"\n").unwrap();

        let missing_override = temp.path().join("missing-override.toml");
        let loaded = Config::load_with_paths(Some(missing_override), Some(home_config));
        assert_eq!(loaded.llm_provider, LlmProvider::Openai);
    }

    #[test]
    fn test_config_from_file_full() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("full_config.toml");

        let toml_content = r#"
llm_provider = "anthropic_compatible"
gemini_project_id = "test-project"
gemini_location = "europe-west1"
gemini_model = "gemini-2.5-pro"
openai_api_key = "sk-openai-official"
openai_base_url = "https://api.openai.com/v1"
openai_model = "gpt-4.1"
openai_compat_api_key = "sk-openai"
openai_compat_base_url = "https://api.openai.com/v1"
openai_compat_model = "gpt-4o"
anthropic_compat_api_key = "sk-anthropic"
anthropic_compat_base_url = "https://api.anthropic.com/v1"
anthropic_compat_model = "claude-3-5-sonnet-latest"
anthropic_compat_client_name = "test-client"
anthropic_compat_user_agent = "test-agent/1.0"
llm_request_timeout_secs = 240
tool_timeout_secs = 45
max_tool_loops = 10
tool_repeat_limit = 5
prompt_snapshot_enabled = true
prompt_snapshot_max_chars = 10000
streaming_mode = "on"
partial_stream_recovery_mode = "continue_once"

[memory]
enabled = false
strict_workspace = false

[durability]
required = true
"#;

        std::fs::write(&config_path, toml_content).unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.llm_provider, LlmProvider::AnthropicCompatible);
        assert_eq!(config.gemini_project_id, Some("test-project".to_string()));
        assert_eq!(config.gemini_location, "europe-west1");
        assert_eq!(config.gemini_model, "gemini-2.5-pro");
        assert_eq!(
            config.openai_api_key,
            Some("sk-openai-official".to_string())
        );
        assert_eq!(config.openai_compat_api_key, Some("sk-openai".to_string()));
        assert_eq!(
            config.anthropic_compat_api_key,
            Some("sk-anthropic".to_string())
        );
        assert_eq!(
            config.anthropic_compat_client_name,
            Some("test-client".to_string())
        );
        assert_eq!(
            config.anthropic_compat_user_agent,
            Some("test-agent/1.0".to_string())
        );
        assert_eq!(config.llm_request_timeout_secs, 240);
        assert_eq!(config.tool_timeout_secs, 45);
        assert_eq!(config.max_tool_loops, Some(10));
        assert_eq!(config.tool_repeat_limit, 5);
        assert!(config.prompt_snapshot_enabled);
        assert_eq!(config.prompt_snapshot_max_chars, 10000);
        assert_eq!(config.streaming_mode, StreamingMode::On);
        assert_eq!(
            config.partial_stream_recovery_mode,
            PartialStreamRecoveryMode::ContinueOnce
        );
        // Memory
        assert!(!config.memory.enabled);
        assert!(!config.memory.strict_workspace);
        assert!(config.durability.required);
    }

    #[test]
    fn test_memory_config_default() {
        let memory = MemoryConfig::default();
        assert!(memory.enabled);
        assert!(memory.strict_workspace);
        assert!(memory.workspace_dir.is_none());
    }

    #[test]
    fn test_memory_config_deserialization() {
        let toml_content = r#"
enabled = false
strict_workspace = false
workspace_dir = "/custom/path"
"#;
        let memory: MemoryConfig = toml::from_str(toml_content).unwrap();
        assert!(!memory.enabled);
        assert!(!memory.strict_workspace);
        assert_eq!(memory.workspace_dir, Some(PathBuf::from("/custom/path")));
    }

    #[test]
    fn test_durability_config_deserialization() {
        let toml_content = r#"
[durability]
required = true
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert!(config.durability.required);
    }

    #[test]
    fn test_to_provider_config_gemini() {
        let config = Config::for_gemini("my-project", None, Some("gemini-2.0-flash"));
        let provider_config = config.to_provider_config().unwrap();
        // Verify it creates the right config type
        assert_eq!(
            provider_config.provider_type,
            alan_llm::factory::ProviderType::Gemini
        );
        assert_eq!(provider_config.project_id, Some("my-project".to_string()));
        assert_eq!(provider_config.model, "gemini-2.0-flash");
    }

    #[test]
    fn test_to_provider_config_gemini_missing_project() {
        let config = Config {
            llm_provider: LlmProvider::Gemini,
            gemini_project_id: None,
            ..Config::default()
        };
        let result = config.to_provider_config();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("GEMINI_PROJECT_ID")
        );
    }

    #[test]
    fn test_to_provider_config_openai() {
        let config = Config::for_openai("sk-test", None, Some("gpt-4o"));
        let provider_config = config.to_provider_config().unwrap();
        assert_eq!(
            provider_config.provider_type,
            alan_llm::factory::ProviderType::OpenAi
        );
        assert_eq!(provider_config.api_key, Some("sk-test".to_string()));
        assert_eq!(provider_config.model, "gpt-4o");
    }

    #[test]
    fn test_to_provider_config_openai_missing_key() {
        let config = Config {
            llm_provider: LlmProvider::Openai,
            openai_api_key: None,
            openai_compat_api_key: None,
            ..Config::default()
        };
        let result = config.to_provider_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("OPENAI_API_KEY"));
    }

    #[test]
    fn test_to_provider_config_openai_compatible() {
        let config = Config::for_openai_compatible("sk-test", None, Some("gpt-4o-mini"));
        let provider_config = config.to_provider_config().unwrap();
        assert_eq!(
            provider_config.provider_type,
            alan_llm::factory::ProviderType::OpenAiCompatible
        );
        assert_eq!(provider_config.api_key, Some("sk-test".to_string()));
        assert_eq!(provider_config.model, "gpt-4o-mini");
    }

    #[test]
    fn test_to_provider_config_openai_uses_legacy_compat_key_as_fallback() {
        let config = Config {
            llm_provider: LlmProvider::Openai,
            openai_api_key: None,
            openai_compat_api_key: Some("sk-legacy".to_string()),
            openai_compat_base_url: "https://proxy.example/v1".to_string(),
            openai_compat_model: "gpt-4.1-legacy".to_string(),
            ..Config::default()
        };
        let provider_config = config.to_provider_config().unwrap();
        assert_eq!(
            provider_config.provider_type,
            alan_llm::factory::ProviderType::OpenAi
        );
        assert_eq!(provider_config.api_key, Some("sk-legacy".to_string()));
        assert_eq!(
            provider_config.base_url.as_deref(),
            Some("https://proxy.example/v1")
        );
        assert_eq!(provider_config.model, "gpt-4.1-legacy");
    }

    #[test]
    fn test_to_provider_config_anthropic() {
        let config = Config::for_anthropic_compatible("sk-test", None, Some("claude-3"));
        let provider_config = config.to_provider_config().unwrap();
        assert_eq!(
            provider_config.provider_type,
            alan_llm::factory::ProviderType::Anthropic
        );
        assert_eq!(provider_config.api_key, Some("sk-test".to_string()));
        assert_eq!(provider_config.model, "claude-3");
    }

    #[test]
    fn test_to_provider_config_anthropic_with_options() {
        let config = Config {
            llm_provider: LlmProvider::AnthropicCompatible,
            anthropic_compat_api_key: Some("key".to_string()),
            anthropic_compat_base_url: "https://custom.api.com".to_string(),
            anthropic_compat_model: "claude-3".to_string(),
            anthropic_compat_client_name: Some("test-client".to_string()),
            anthropic_compat_user_agent: Some("test-agent/1.0".to_string()),
            ..Config::default()
        };
        let provider_config = config.to_provider_config().unwrap();
        assert_eq!(
            provider_config.base_url,
            Some("https://custom.api.com".to_string())
        );
        assert_eq!(provider_config.client_name, Some("test-client".to_string()));
        assert_eq!(
            provider_config.user_agent,
            Some("test-agent/1.0".to_string())
        );
    }

    #[test]
    fn test_to_provider_config_anthropic_missing_key() {
        let config = Config {
            llm_provider: LlmProvider::AnthropicCompatible,
            anthropic_compat_api_key: None,
            ..Config::default()
        };
        let result = config.to_provider_config();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("ANTHROPIC_COMPAT_API_KEY")
        );
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_llm_provider(), LlmProvider::Openai);
        assert_eq!(default_gemini_location(), "us-central1");
        assert_eq!(default_gemini_model(), "gemini-2.0-flash");
        assert_eq!(default_openai_base_url(), "https://api.openai.com/v1");
        assert_eq!(default_openai_model(), "gpt-4o");
        assert_eq!(
            default_openai_compat_base_url(),
            "https://api.openai.com/v1"
        );
        assert_eq!(default_openai_compat_model(), "gpt-4o");
        assert_eq!(
            default_anthropic_compat_base_url(),
            "https://api.anthropic.com/v1"
        );
        assert_eq!(default_anthropic_compat_model(), "claude-3-5-sonnet-latest");
        assert_eq!(default_llm_timeout_secs(), 180);
        assert_eq!(default_tool_timeout_secs(), 30);
        assert_eq!(default_prompt_snapshot_max_chars(), 8000);
        assert_eq!(default_tool_repeat_limit(), 4);
        assert_eq!(default_streaming_mode(), StreamingMode::Auto);
        assert_eq!(
            default_partial_stream_recovery_mode(),
            PartialStreamRecoveryMode::ContinueOnce
        );
    }
}
