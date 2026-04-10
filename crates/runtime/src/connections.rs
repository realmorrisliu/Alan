use crate::config::{Config, LlmProvider};
use crate::paths::AlanHomePaths;
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

const CONNECTIONS_VERSION: u32 = 1;
const CHATGPT_AUTH_BACKEND: &str = "alan_home_auth_json";
const SECRET_STORE_BACKEND: &str = "alan_home_secret_store";
const AMBIENT_BACKEND: &str = "ambient";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    ManagedOauth,
    SecretString,
    AmbientCloudAuth,
}

impl CredentialKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ManagedOauth => "managed_oauth",
            Self::SecretString => "secret_string",
            Self::AmbientCloudAuth => "ambient_cloud_auth",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConnectionCredential {
    pub kind: CredentialKind,
    pub provider_family: LlmProvider,
    pub label: String,
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConnectionProfile {
    pub provider: LlmProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
    #[serde(default = "default_profile_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_profile_timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default = "default_profile_source")]
    pub source: String,
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConnectionsFile {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub credentials: BTreeMap<String, ConnectionCredential>,
    #[serde(default)]
    pub profiles: BTreeMap<String, ConnectionProfile>,
}

impl Default for ConnectionsFile {
    fn default() -> Self {
        Self {
            version: CONNECTIONS_VERSION,
            default_profile: None,
            credentials: BTreeMap::new(),
            profiles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConnectionProfile {
    pub profile_id: String,
    pub provider: LlmProvider,
    pub credential_id: Option<String>,
    pub credential_kind: CredentialKind,
    pub settings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderDescriptor {
    pub provider_id: LlmProvider,
    pub display_name: &'static str,
    pub credential_kind: CredentialKind,
    pub supports_browser_login: bool,
    pub supports_device_login: bool,
    pub supports_secret_entry: bool,
    pub supports_logout: bool,
    pub supports_test: bool,
    pub required_settings: &'static [&'static str],
    pub optional_settings: &'static [&'static str],
    pub default_settings: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone)]
pub struct SecretStore {
    root_dir: PathBuf,
}

impl SecretStore {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn from_home_paths(home_paths: &AlanHomePaths) -> Self {
        Self::new(home_paths.global_credentials_dir.clone())
    }

    pub fn load(&self, credential_id: &str) -> anyhow::Result<Option<String>> {
        let path = self.secret_path(credential_id)?;
        match std::fs::read_to_string(&path) {
            Ok(value) => Ok(Some(value.trim().to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error)
                .with_context(|| format!("failed to read credential secret {}", path.display())),
        }
    }

    pub fn save(&self, credential_id: &str, secret: &str) -> anyhow::Result<()> {
        let path = self.secret_path(credential_id)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create credentials directory {}",
                    parent.display()
                )
            })?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;

            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .mode(0o600)
                .open(&path)
                .with_context(|| format!("failed to open credential secret {}", path.display()))?;
            use std::io::Write;
            file.write_all(secret.as_bytes())
                .with_context(|| format!("failed to write credential secret {}", path.display()))?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&path, secret)
                .with_context(|| format!("failed to write credential secret {}", path.display()))?;
        }
        Ok(())
    }

    pub fn delete(&self, credential_id: &str) -> anyhow::Result<bool> {
        let path = self.secret_path(credential_id)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error)
                .with_context(|| format!("failed to remove credential secret {}", path.display())),
        }
    }

    fn secret_path(&self, credential_id: &str) -> anyhow::Result<PathBuf> {
        let credential_id = validated_identifier_component("credential id", credential_id)?;
        let mut digest = Sha256::new();
        digest.update(credential_id.as_bytes());
        let digest = digest.finalize();
        let mut file_name = String::with_capacity((digest.len() * 2) + ".secret".len());
        for byte in digest {
            use std::fmt::Write as _;
            let _ = write!(&mut file_name, "{byte:02x}");
        }
        file_name.push_str(".secret");
        Ok(self.root_dir.join(file_name))
    }
}

impl ConnectionsFile {
    pub fn detect() -> anyhow::Result<(Self, Option<PathBuf>)> {
        let Some(home_paths) = AlanHomePaths::detect() else {
            return Ok((Self::default(), None));
        };
        Self::load_from_home_paths(&home_paths)
    }

    pub fn load_from_home_paths(
        home_paths: &AlanHomePaths,
    ) -> anyhow::Result<(Self, Option<PathBuf>)> {
        let path = &home_paths.global_connections_path;
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let parsed: Self = toml::from_str(&content).with_context(|| {
                    format!("failed to parse connections file {}", path.display())
                })?;
                if parsed.version != CONNECTIONS_VERSION {
                    anyhow::bail!(
                        "unsupported connections file version {} in {}",
                        parsed.version,
                        path.display()
                    );
                }
                Ok((parsed, Some(path.to_path_buf())))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok((Self::default(), Some(path.to_path_buf())))
            }
            Err(error) => Err(error)
                .with_context(|| format!("failed to read connections file {}", path.display())),
        }
    }

    pub fn save_to_home_paths(&self, home_paths: &AlanHomePaths) -> anyhow::Result<()> {
        let path = &home_paths.global_connections_path;
        if self.version != CONNECTIONS_VERSION {
            anyhow::bail!("unsupported connections file version {}", self.version);
        }
        let rendered = toml::to_string_pretty(self)
            .context("failed to encode connections.toml while saving")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create connections directory {}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(path, rendered)
            .with_context(|| format!("failed to write connections file {}", path.display()))?;
        Ok(())
    }

    pub fn profile_descriptor(provider: LlmProvider) -> &'static ProviderDescriptor {
        provider_catalog()
            .iter()
            .find(|descriptor| descriptor.provider_id == provider)
            .expect("provider descriptor missing")
    }

    pub fn resolve_profile(
        &self,
        profile_id: Option<&str>,
    ) -> anyhow::Result<ResolvedConnectionProfile> {
        let selected_profile_id = profile_id
            .map(str::to_owned)
            .or_else(|| self.default_profile.clone())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No connection profile selected. Set connection_profile in agent.toml or default_profile in connections.toml."
                )
            })?;
        let profile = self
            .profiles
            .get(&selected_profile_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown connection profile `{selected_profile_id}`"))?;
        let descriptor = Self::profile_descriptor(profile.provider);
        let normalized = normalize_profile_settings(profile.provider, &profile.settings);
        validate_profile_settings(profile.provider, &normalized)?;

        let credential_kind = if let Some(credential_id) = profile.credential_id.as_deref() {
            let credential = self.credentials.get(credential_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Profile `{selected_profile_id}` references unknown credential `{credential_id}`"
                )
            })?;
            if credential.provider_family != profile.provider {
                anyhow::bail!(
                    "Profile `{selected_profile_id}` uses provider `{}` but credential `{credential_id}` is bound to `{}`",
                    profile.provider.as_str(),
                    credential.provider_family.as_str(),
                );
            }
            if credential.kind != descriptor.credential_kind {
                anyhow::bail!(
                    "Profile `{selected_profile_id}` uses credential kind `{}` but provider `{}` requires `{}`",
                    credential.kind.as_str(),
                    profile.provider.as_str(),
                    descriptor.credential_kind.as_str(),
                );
            }
            credential.kind
        } else {
            if descriptor.credential_kind != CredentialKind::AmbientCloudAuth {
                anyhow::bail!(
                    "Profile `{selected_profile_id}` requires a credential for provider `{}`",
                    profile.provider.as_str()
                );
            }
            CredentialKind::AmbientCloudAuth
        };

        Ok(ResolvedConnectionProfile {
            profile_id: selected_profile_id,
            provider: profile.provider,
            credential_id: profile.credential_id.clone(),
            credential_kind,
            settings: normalized,
        })
    }

    pub fn apply_profile_to_config(
        &self,
        profile_id: Option<&str>,
        secret_store: &SecretStore,
        config: &mut Config,
    ) -> anyhow::Result<ResolvedConnectionProfile> {
        let resolved = self.resolve_profile(profile_id)?;
        apply_resolved_profile_to_config(&resolved, secret_store, config)?;
        Ok(resolved)
    }
}

pub fn provider_catalog() -> &'static [ProviderDescriptor] {
    const CHATGPT_REQUIRED: &[&str] = &["base_url", "model"];
    const CHATGPT_OPTIONAL: &[&str] = &["account_id"];
    const CHATGPT_DEFAULTS: &[(&str, &str)] = &[
        ("base_url", "https://chatgpt.com/backend-api/codex"),
        ("model", "gpt-5.3-codex"),
        ("account_id", ""),
    ];

    const OPENAI_REQUIRED: &[&str] = &["base_url", "model"];
    const OPENAI_DEFAULTS: &[(&str, &str)] = &[
        ("base_url", "https://api.openai.com/v1"),
        ("model", "gpt-5.4"),
    ];

    const OPENAI_COMPAT_DEFAULTS: &[(&str, &str)] = &[
        ("base_url", "https://api.openai.com/v1"),
        ("model", "qwen3.5-plus"),
    ];

    const ANTHROPIC_REQUIRED: &[&str] = &["base_url", "model"];
    const ANTHROPIC_OPTIONAL: &[&str] = &["client_name", "user_agent"];
    const ANTHROPIC_DEFAULTS: &[(&str, &str)] = &[
        ("base_url", "https://api.anthropic.com/v1"),
        ("model", "claude-3-5-sonnet-latest"),
        ("client_name", ""),
        ("user_agent", ""),
    ];

    const GEMINI_REQUIRED: &[&str] = &["project_id", "location", "model"];
    const GEMINI_DEFAULTS: &[(&str, &str)] = &[
        ("project_id", ""),
        ("location", "us-central1"),
        ("model", "gemini-2.0-flash"),
    ];

    static CATALOG: std::sync::OnceLock<Vec<ProviderDescriptor>> = std::sync::OnceLock::new();
    CATALOG
        .get_or_init(|| {
            vec![
                ProviderDescriptor {
                    provider_id: LlmProvider::Chatgpt,
                    display_name: "ChatGPT / Codex",
                    credential_kind: CredentialKind::ManagedOauth,
                    supports_browser_login: true,
                    supports_device_login: true,
                    supports_secret_entry: false,
                    supports_logout: true,
                    supports_test: true,
                    required_settings: CHATGPT_REQUIRED,
                    optional_settings: CHATGPT_OPTIONAL,
                    default_settings: CHATGPT_DEFAULTS,
                },
                ProviderDescriptor {
                    provider_id: LlmProvider::OpenAiResponses,
                    display_name: "OpenAI Responses API",
                    credential_kind: CredentialKind::SecretString,
                    supports_browser_login: false,
                    supports_device_login: false,
                    supports_secret_entry: true,
                    supports_logout: false,
                    supports_test: true,
                    required_settings: OPENAI_REQUIRED,
                    optional_settings: &[],
                    default_settings: OPENAI_DEFAULTS,
                },
                ProviderDescriptor {
                    provider_id: LlmProvider::OpenAiChatCompletions,
                    display_name: "OpenAI Chat Completions API",
                    credential_kind: CredentialKind::SecretString,
                    supports_browser_login: false,
                    supports_device_login: false,
                    supports_secret_entry: true,
                    supports_logout: false,
                    supports_test: true,
                    required_settings: OPENAI_REQUIRED,
                    optional_settings: &[],
                    default_settings: OPENAI_DEFAULTS,
                },
                ProviderDescriptor {
                    provider_id: LlmProvider::OpenAiChatCompletionsCompatible,
                    display_name: "OpenAI Chat Completions API-compatible",
                    credential_kind: CredentialKind::SecretString,
                    supports_browser_login: false,
                    supports_device_login: false,
                    supports_secret_entry: true,
                    supports_logout: false,
                    supports_test: true,
                    required_settings: OPENAI_REQUIRED,
                    optional_settings: &[],
                    default_settings: OPENAI_COMPAT_DEFAULTS,
                },
                ProviderDescriptor {
                    provider_id: LlmProvider::AnthropicMessages,
                    display_name: "Anthropic Messages API",
                    credential_kind: CredentialKind::SecretString,
                    supports_browser_login: false,
                    supports_device_login: false,
                    supports_secret_entry: true,
                    supports_logout: false,
                    supports_test: true,
                    required_settings: ANTHROPIC_REQUIRED,
                    optional_settings: ANTHROPIC_OPTIONAL,
                    default_settings: ANTHROPIC_DEFAULTS,
                },
                ProviderDescriptor {
                    provider_id: LlmProvider::GoogleGeminiGenerateContent,
                    display_name: "Google Gemini GenerateContent API",
                    credential_kind: CredentialKind::AmbientCloudAuth,
                    supports_browser_login: false,
                    supports_device_login: false,
                    supports_secret_entry: false,
                    supports_logout: false,
                    supports_test: true,
                    required_settings: GEMINI_REQUIRED,
                    optional_settings: &[],
                    default_settings: GEMINI_DEFAULTS,
                },
            ]
        })
        .as_slice()
}

pub fn default_profile_source() -> String {
    "managed".to_string()
}

pub fn default_profile_timestamp() -> DateTime<Utc> {
    Utc::now()
}

pub fn normalize_profile_settings(
    provider: LlmProvider,
    settings: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let descriptor = ConnectionsFile::profile_descriptor(provider);
    let mut normalized = BTreeMap::new();
    for (key, value) in descriptor.default_settings {
        normalized.insert((*key).to_string(), (*value).to_string());
    }
    for (key, value) in settings {
        normalized.insert(key.clone(), value.clone());
    }
    normalized
}

pub fn validate_profile_settings(
    provider: LlmProvider,
    settings: &BTreeMap<String, String>,
) -> anyhow::Result<()> {
    let descriptor = ConnectionsFile::profile_descriptor(provider);
    for key in descriptor.required_settings {
        let value = settings
            .get(*key)
            .map(|value| value.trim())
            .unwrap_or_default();
        if value.is_empty() {
            anyhow::bail!(
                "Provider `{}` requires setting `{}`",
                provider.as_str(),
                key
            );
        }
    }
    let allowed_keys: std::collections::BTreeSet<&str> = descriptor
        .required_settings
        .iter()
        .chain(descriptor.optional_settings.iter())
        .copied()
        .collect();
    for key in settings.keys() {
        if !allowed_keys.contains(key.as_str()) {
            anyhow::bail!(
                "Provider `{}` does not support setting `{}`",
                provider.as_str(),
                key
            );
        }
    }
    Ok(())
}

pub fn default_credential_backend(kind: CredentialKind) -> &'static str {
    match kind {
        CredentialKind::ManagedOauth => CHATGPT_AUTH_BACKEND,
        CredentialKind::SecretString => SECRET_STORE_BACKEND,
        CredentialKind::AmbientCloudAuth => AMBIENT_BACKEND,
    }
}

pub fn sanitize_identifier(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut sanitized = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            return None;
        }
    }
    Some(sanitized)
}

fn validated_identifier_component<'a>(label: &str, value: &'a str) -> anyhow::Result<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        anyhow::bail!("invalid {label} `{value}`");
    }
    Ok(trimmed)
}

fn apply_resolved_profile_to_config(
    resolved: &ResolvedConnectionProfile,
    secret_store: &SecretStore,
    config: &mut Config,
) -> anyhow::Result<()> {
    config.reset_internal_provider_config();
    config.connection_profile = Some(resolved.profile_id.clone());
    match resolved.provider {
        LlmProvider::Chatgpt => {
            config.llm_provider = LlmProvider::Chatgpt;
            config.chatgpt_base_url = resolved.settings["base_url"].clone();
            config.chatgpt_model = resolved.settings["model"].clone();
            let account_id = resolved.settings["account_id"].trim().to_string();
            config.chatgpt_account_id = if account_id.is_empty() {
                None
            } else {
                Some(account_id)
            };
        }
        LlmProvider::OpenAiResponses => {
            let credential_id = resolved.credential_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!("Profile `{}` is missing a credential", resolved.profile_id)
            })?;
            let api_key = secret_store.load(credential_id)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Credential `{credential_id}` for profile `{}` is missing a secret",
                    resolved.profile_id
                )
            })?;
            config.llm_provider = LlmProvider::OpenAiResponses;
            config.openai_responses_api_key = Some(api_key);
            config.openai_responses_base_url = resolved.settings["base_url"].clone();
            config.openai_responses_model = resolved.settings["model"].clone();
        }
        LlmProvider::OpenAiChatCompletions => {
            let credential_id = resolved.credential_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!("Profile `{}` is missing a credential", resolved.profile_id)
            })?;
            let api_key = secret_store.load(credential_id)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Credential `{credential_id}` for profile `{}` is missing a secret",
                    resolved.profile_id
                )
            })?;
            config.llm_provider = LlmProvider::OpenAiChatCompletions;
            config.openai_chat_completions_api_key = Some(api_key);
            config.openai_chat_completions_base_url = resolved.settings["base_url"].clone();
            config.openai_chat_completions_model = resolved.settings["model"].clone();
        }
        LlmProvider::OpenAiChatCompletionsCompatible => {
            let credential_id = resolved.credential_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!("Profile `{}` is missing a credential", resolved.profile_id)
            })?;
            let api_key = secret_store.load(credential_id)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Credential `{credential_id}` for profile `{}` is missing a secret",
                    resolved.profile_id
                )
            })?;
            config.llm_provider = LlmProvider::OpenAiChatCompletionsCompatible;
            config.openai_chat_completions_compatible_api_key = Some(api_key);
            config.openai_chat_completions_compatible_base_url =
                resolved.settings["base_url"].clone();
            config.openai_chat_completions_compatible_model = resolved.settings["model"].clone();
        }
        LlmProvider::AnthropicMessages => {
            let credential_id = resolved.credential_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!("Profile `{}` is missing a credential", resolved.profile_id)
            })?;
            let api_key = secret_store.load(credential_id)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Credential `{credential_id}` for profile `{}` is missing a secret",
                    resolved.profile_id
                )
            })?;
            config.llm_provider = LlmProvider::AnthropicMessages;
            config.anthropic_messages_api_key = Some(api_key);
            config.anthropic_messages_base_url = resolved.settings["base_url"].clone();
            config.anthropic_messages_model = resolved.settings["model"].clone();
            config.anthropic_messages_client_name = resolved
                .settings
                .get("client_name")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            config.anthropic_messages_user_agent = resolved
                .settings
                .get("user_agent")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
        }
        LlmProvider::GoogleGeminiGenerateContent => {
            config.llm_provider = LlmProvider::GoogleGeminiGenerateContent;
            config.google_gemini_generate_content_project_id =
                Some(resolved.settings["project_id"].clone());
            config.google_gemini_generate_content_location = resolved.settings["location"].clone();
            config.google_gemini_generate_content_model = resolved.settings["model"].clone();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn normalize_profile_settings_applies_defaults() {
        let settings = BTreeMap::from([("model".to_string(), "gpt-5".to_string())]);
        let normalized = normalize_profile_settings(LlmProvider::OpenAiResponses, &settings);
        assert_eq!(
            normalized.get("base_url").map(String::as_str),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(normalized.get("model").map(String::as_str), Some("gpt-5"));
    }

    #[test]
    fn secret_store_round_trips_secret() {
        let temp = TempDir::new().unwrap();
        let store = SecretStore::new(temp.path().to_path_buf());
        store.save("kimi", "sk-test").unwrap();
        assert_eq!(store.load("kimi").unwrap().as_deref(), Some("sk-test"));
        assert!(store.delete("kimi").unwrap());
        assert_eq!(store.load("kimi").unwrap(), None);
    }

    #[test]
    fn resolve_profile_uses_default_profile() {
        let file = ConnectionsFile {
            default_profile: Some("chatgpt-main".to_string()),
            credentials: BTreeMap::from([(
                "chatgpt".to_string(),
                ConnectionCredential {
                    kind: CredentialKind::ManagedOauth,
                    provider_family: LlmProvider::Chatgpt,
                    label: "ChatGPT login".to_string(),
                    backend: CHATGPT_AUTH_BACKEND.to_string(),
                },
            )]),
            profiles: BTreeMap::from([(
                "chatgpt-main".to_string(),
                ConnectionProfile {
                    provider: LlmProvider::Chatgpt,
                    label: Some("ChatGPT".to_string()),
                    credential_id: Some("chatgpt".to_string()),
                    created_at: default_profile_timestamp(),
                    updated_at: default_profile_timestamp(),
                    source: default_profile_source(),
                    settings: BTreeMap::new(),
                },
            )]),
            ..ConnectionsFile::default()
        };

        let resolved = file.resolve_profile(None).unwrap();
        assert_eq!(resolved.profile_id, "chatgpt-main");
        assert_eq!(
            resolved.settings.get("model").map(String::as_str),
            Some("gpt-5.3-codex")
        );
    }
}
