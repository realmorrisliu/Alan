//! Configuration management.

use serde::Deserialize;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    Gemini,
    OpenaiCompatible,
    AnthropicCompatible,
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
    // Memory Configuration
    // ========================================================================
    #[serde(default)]
    pub memory: MemoryConfig,
}

fn default_llm_provider() -> LlmProvider {
    LlmProvider::Gemini
}

fn default_gemini_location() -> String {
    "us-central1".to_string()
}

fn default_gemini_model() -> String {
    "gemini-2.0-flash".to_string()
}

fn default_openai_compat_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_openai_compat_model() -> String {
    "gpt-4o".to_string()
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

impl Default for Config {
    fn default() -> Self {
        Self {
            llm_provider: default_llm_provider(),
            gemini_project_id: None,
            gemini_location: default_gemini_location(),
            gemini_model: default_gemini_model(),
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

            memory: MemoryConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // Check for config file first
        if let Some(config_path) = Self::config_file_path()
            && config_path.exists()
        {
            match Self::from_file(&config_path) {
                Ok(config) => {
                    tracing::info!(path = %config_path.display(), "Loaded configuration from file");
                    return config;
                }
                Err(e) => {
                    tracing::warn!(path = %config_path.display(), error = %e, "Failed to load config file, falling back to environment");
                }
            }
        }

        Self::from_env_only()
    }

    /// Load configuration from file (TOML format)
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the default config file path
    pub fn config_file_path() -> Option<std::path::PathBuf> {
        if let Ok(home) = std::env::var("HOME") {
            let path = std::path::PathBuf::from(home)
                .join(".alan")
                .join("config.toml");
            if path.exists() {
                return Some(path);
            }
        }

        // Also check for ALAN_CONFIG_PATH environment variable
        if let Ok(path_str) = std::env::var("ALAN_CONFIG_PATH") {
            return Some(std::path::PathBuf::from(path_str));
        }

        None
    }

    /// Load configuration from environment variables only (no file)
    pub fn from_env_only() -> Self {
        Self {
            llm_provider: env_llm_provider("LLM_PROVIDER", default_llm_provider()),

            gemini_project_id: std::env::var("GEMINI_PROJECT_ID").ok(),
            gemini_location: std::env::var("GEMINI_LOCATION")
                .unwrap_or_else(|_| default_gemini_location()),
            gemini_model: std::env::var("GEMINI_MODEL").unwrap_or_else(|_| default_gemini_model()),

            openai_compat_api_key: std::env::var("OPENAI_COMPAT_API_KEY").ok(),
            openai_compat_base_url: std::env::var("OPENAI_COMPAT_BASE_URL")
                .unwrap_or_else(|_| default_openai_compat_base_url()),
            openai_compat_model: std::env::var("OPENAI_COMPAT_MODEL")
                .unwrap_or_else(|_| default_openai_compat_model()),

            anthropic_compat_api_key: std::env::var("ANTHROPIC_COMPAT_API_KEY").ok(),
            anthropic_compat_base_url: std::env::var("ANTHROPIC_COMPAT_BASE_URL")
                .unwrap_or_else(|_| default_anthropic_compat_base_url()),
            anthropic_compat_model: std::env::var("ANTHROPIC_COMPAT_MODEL")
                .unwrap_or_else(|_| default_anthropic_compat_model()),
            anthropic_compat_client_name: std::env::var("ANTHROPIC_COMPAT_CLIENT_NAME").ok(),
            anthropic_compat_user_agent: std::env::var("ANTHROPIC_COMPAT_USER_AGENT").ok(),

            llm_request_timeout_secs: env_usize("LLM_TIMEOUT_SECS", default_llm_timeout_secs()),
            tool_timeout_secs: env_usize("TOOL_TIMEOUT_SECS", default_tool_timeout_secs()),
            max_tool_loops: env_optional_usize("MAX_TOOL_LOOPS"),
            tool_repeat_limit: env_usize("TOOL_REPEAT_LIMIT", default_tool_repeat_limit()),
            prompt_snapshot_enabled: env_bool("PROMPT_SNAPSHOT_ENABLED", false),
            prompt_snapshot_max_chars: env_usize(
                "PROMPT_SNAPSHOT_MAX_CHARS",
                default_prompt_snapshot_max_chars(),
            ),

            memory: MemoryConfig {
                enabled: env_bool("MEMORY_ENABLED", true),
                workspace_dir: env_path("MEMORY_WORKSPACE_DIR"),
                strict_workspace: env_bool("MEMORY_STRICT_WORKSPACE", true),
            },
        }
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

    pub fn has_openai_compatible_config(&self) -> bool {
        self.openai_compat_api_key.is_some()
    }

    pub fn has_anthropic_compatible_config(&self) -> bool {
        self.anthropic_compat_api_key.is_some()
    }

    pub fn has_llm_config(&self) -> bool {
        match self.llm_provider {
            LlmProvider::Gemini => self.has_gemini_config(),
            LlmProvider::OpenaiCompatible => self.has_openai_compatible_config(),
            LlmProvider::AnthropicCompatible => self.has_anthropic_compatible_config(),
        }
    }

    pub fn effective_model(&self) -> &str {
        match self.llm_provider {
            LlmProvider::Gemini => &self.gemini_model,
            LlmProvider::OpenaiCompatible => &self.openai_compat_model,
            LlmProvider::AnthropicCompatible => &self.anthropic_compat_model,
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
            LlmProvider::OpenaiCompatible => {
                let api_key = self.openai_compat_api_key.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("OpenAI-compatible provider requires OPENAI_COMPAT_API_KEY")
                })?;
                Ok(ProviderConfig::openai(api_key, &self.openai_compat_model)
                    .with_base_url(&self.openai_compat_base_url))
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

fn env_llm_provider(key: &str, default: LlmProvider) -> LlmProvider {
    match std::env::var(key).ok().as_deref() {
        Some("gemini") => LlmProvider::Gemini,
        Some("openai_compatible") => LlmProvider::OpenaiCompatible,
        Some("anthropic_compatible") => LlmProvider::AnthropicCompatible,
        Some(_) => default,
        None => default,
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_optional_usize(key: &str) -> Option<usize> {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key).ok().map(PathBuf::from)
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
        assert_eq!(config.llm_provider, LlmProvider::Gemini);
        assert_eq!(config.gemini_location, "us-central1");
        assert_eq!(config.gemini_model, "gemini-2.0-flash");
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
        // Memory config
        assert!(config.memory.enabled);
        assert!(config.memory.strict_workspace);
        assert!(config.memory.workspace_dir.is_none());
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

        let openai = Config::for_openai_compatible("k", None, Some("gpt-4.1"));
        assert_eq!(openai.effective_model(), "gpt-4.1");

        let anthropic = Config::for_anthropic_compatible("k", None, Some("claude-3-5-sonnet"));
        assert_eq!(anthropic.effective_model(), "claude-3-5-sonnet");
    }

    #[test]
    fn test_has_llm_config_without_api_key() {
        let mut config = Config {
            llm_provider: LlmProvider::OpenaiCompatible,
            openai_compat_api_key: None,
            ..Config::default()
        };
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
llm_provider = "openai_compatible"
openai_compat_api_key = "sk-test123"
openai_compat_model = "gpt-4"
llm_request_timeout_secs = 300
tool_timeout_secs = 60
"#;

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.llm_provider, LlmProvider::OpenaiCompatible);
        assert_eq!(config.openai_compat_api_key, Some("sk-test123".to_string()));
        assert_eq!(config.openai_compat_model, "gpt-4");
        assert_eq!(config.llm_request_timeout_secs, 300);
        assert_eq!(config.tool_timeout_secs, 60);
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
    fn test_config_from_file_full() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("full_config.toml");

        let toml_content = r#"
llm_provider = "anthropic_compatible"
gemini_project_id = "test-project"
gemini_location = "europe-west1"
gemini_model = "gemini-2.5-pro"
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

[memory]
enabled = false
strict_workspace = false
"#;

        std::fs::write(&config_path, toml_content).unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.llm_provider, LlmProvider::AnthropicCompatible);
        assert_eq!(config.gemini_project_id, Some("test-project".to_string()));
        assert_eq!(config.gemini_location, "europe-west1");
        assert_eq!(config.gemini_model, "gemini-2.5-pro");
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
        // Memory
        assert!(!config.memory.enabled);
        assert!(!config.memory.strict_workspace);
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
        let config = Config::for_openai_compatible("sk-test", None, Some("gpt-4o"));
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
            llm_provider: LlmProvider::OpenaiCompatible,
            openai_compat_api_key: None,
            ..Config::default()
        };
        let result = config.to_provider_config();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("OPENAI_COMPAT_API_KEY")
        );
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

    // Tests for environment variable parsing helpers
    #[test]
    fn test_env_llm_provider_parsing() {
        // We can only test the default case without env vars
        let result = env_llm_provider("NONEXISTENT_VAR_XYZ", LlmProvider::Gemini);
        assert_eq!(result, LlmProvider::Gemini);
    }

    #[test]
    fn test_env_bool_parsing() {
        // Test default
        assert!(env_bool("NONEXISTENT_BOOL_VAR", true));
        assert!(!env_bool("NONEXISTENT_BOOL_VAR", false));
    }

    #[test]
    fn test_env_usize_parsing() {
        // Test default
        assert_eq!(env_usize("NONEXISTENT_USIZE_VAR", 42), 42);
    }

    #[test]
    fn test_env_optional_usize_parsing() {
        // Test default (none)
        assert_eq!(env_optional_usize("NONEXISTENT_OPT_USIZE_VAR"), None);
    }

    #[test]
    fn test_env_path_parsing() {
        // Test default (none)
        assert_eq!(env_path("NONEXISTENT_PATH_VAR"), None);
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_llm_provider(), LlmProvider::Gemini);
        assert_eq!(default_gemini_location(), "us-central1");
        assert_eq!(default_gemini_model(), "gemini-2.0-flash");
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
    }
}
