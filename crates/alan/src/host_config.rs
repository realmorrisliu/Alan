use alan_runtime::AlanHomePaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8090";
pub(crate) const DEFAULT_DAEMON_URL: &str = "http://127.0.0.1:8090";

#[derive(Debug, Clone, Serialize)]
pub struct HostConfig {
    pub bind_address: String,
    pub daemon_url: String,
}

impl Default for HostConfig {
    fn default() -> Self {
        let bind_address = default_bind_address();
        Self {
            daemon_url: Self::local_daemon_url_for_bind_address(&bind_address),
            bind_address,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawHostConfig {
    #[serde(default = "default_bind_address")]
    bind_address: String,
    #[serde(default)]
    daemon_url: Option<String>,
}

impl HostConfig {
    pub fn load() -> Result<Self> {
        Self::load_with_path(Self::host_file_path())
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).with_context(|| {
            format!("failed to read host configuration file {}", path.display())
        })?;
        let raw: RawHostConfig = toml::from_str(&content).with_context(|| {
            format!("failed to parse host configuration file {}", path.display())
        })?;
        Ok(Self::from_raw(raw))
    }

    pub fn host_file_path() -> Option<PathBuf> {
        AlanHomePaths::detect().map(|paths| paths.global_host_config_path)
    }

    #[cfg(test)]
    pub fn host_file_path_from_home(home: &Path) -> Option<PathBuf> {
        Some(AlanHomePaths::from_home_dir(home).global_host_config_path)
    }

    pub fn resolve_bind_address() -> Result<String> {
        Self::resolve_bind_address_from(std::env::var("BIND_ADDRESS").ok(), Self::load())
    }

    pub fn resolve_bind_address_best_effort() -> String {
        Self::resolve_bind_address_best_effort_from(
            std::env::var("BIND_ADDRESS").ok(),
            Self::load(),
        )
    }

    pub fn resolve_daemon_url_best_effort() -> String {
        Self::resolve_daemon_url_best_effort_from(
            std::env::var("ALAN_AGENTD_URL").ok(),
            Self::load(),
        )
    }

    pub(crate) fn local_daemon_url_for_bind_address(bind_address: &str) -> String {
        let port = bind_address
            .rsplit(':')
            .next()
            .and_then(|raw| raw.parse::<u16>().ok())
            .unwrap_or(8090);
        format!("http://127.0.0.1:{port}")
    }

    fn load_with_path(path: Option<PathBuf>) -> Result<Self> {
        if let Some(path) = path
            && path.exists()
        {
            return Self::from_file(&path);
        }

        Ok(Self::default())
    }

    fn from_raw(raw: RawHostConfig) -> Self {
        let bind_address = raw.bind_address;
        let daemon_url = raw
            .daemon_url
            .unwrap_or_else(|| Self::local_daemon_url_for_bind_address(&bind_address));
        Self {
            bind_address,
            daemon_url,
        }
    }

    fn resolve_bind_address_from(
        env_override: Option<String>,
        config: Result<Self>,
    ) -> Result<String> {
        match env_override {
            Some(bind_address) => Ok(bind_address),
            None => config.map(|config| config.bind_address),
        }
    }

    fn resolve_bind_address_best_effort_from(
        env_override: Option<String>,
        config: Result<Self>,
    ) -> String {
        Self::resolve_bind_address_from(env_override, config)
            .unwrap_or_else(|_| default_bind_address())
    }

    fn resolve_daemon_url_best_effort_from(
        env_override: Option<String>,
        config: Result<Self>,
    ) -> String {
        match env_override {
            Some(daemon_url) => daemon_url,
            None => config
                .map(|config| config.daemon_url)
                .unwrap_or_else(|_| default_daemon_url()),
        }
    }
}

fn default_bind_address() -> String {
    DEFAULT_BIND_ADDRESS.to_string()
}

fn default_daemon_url() -> String {
    DEFAULT_DAEMON_URL.to_string()
}

#[cfg(test)]
mod tests {
    use super::HostConfig;
    use anyhow::anyhow;
    use tempfile::TempDir;

    #[test]
    fn test_host_file_path_from_home_uses_alan_home_root() {
        let path = HostConfig::host_file_path_from_home(std::path::Path::new("/tmp/demo")).unwrap();
        assert_eq!(path, std::path::Path::new("/tmp/demo/.alan/host.toml"));
    }

    #[test]
    fn test_host_config_from_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("host.toml");
        std::fs::write(
            &path,
            r#"
bind_address = "127.0.0.1:9000"
daemon_url = "http://127.0.0.1:9000"
"#,
        )
        .unwrap();

        let config = HostConfig::from_file(&path).unwrap();
        assert_eq!(config.bind_address, "127.0.0.1:9000");
        assert_eq!(config.daemon_url, "http://127.0.0.1:9000");
    }

    #[test]
    fn test_host_config_from_file_derives_daemon_url_from_bind_address() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("host.toml");
        std::fs::write(&path, "bind_address = \"127.0.0.1:9123\"\n").unwrap();

        let config = HostConfig::from_file(&path).unwrap();
        assert_eq!(config.bind_address, "127.0.0.1:9123");
        assert_eq!(config.daemon_url, "http://127.0.0.1:9123");
    }

    #[test]
    fn test_host_config_defaults_when_file_missing() {
        let config = HostConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:8090");
        assert_eq!(config.daemon_url, "http://127.0.0.1:8090");
    }

    #[test]
    fn test_resolve_daemon_url_best_effort_prefers_env_on_load_error() {
        let resolved = HostConfig::resolve_daemon_url_best_effort_from(
            Some("http://127.0.0.1:9999".to_string()),
            Err(anyhow!("broken host config")),
        );
        assert_eq!(resolved, "http://127.0.0.1:9999");
    }

    #[test]
    fn test_resolve_bind_address_best_effort_prefers_env_on_load_error() {
        let resolved = HostConfig::resolve_bind_address_best_effort_from(
            Some("127.0.0.1:9999".to_string()),
            Err(anyhow!("broken host config")),
        );
        assert_eq!(resolved, "127.0.0.1:9999");
    }

    #[test]
    fn test_resolve_bind_address_prefers_env_before_load() {
        let resolved = HostConfig::resolve_bind_address_from(
            Some("127.0.0.1:9999".to_string()),
            Err(anyhow!("broken host config")),
        )
        .unwrap();
        assert_eq!(resolved, "127.0.0.1:9999");
    }
}
