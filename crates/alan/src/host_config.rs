use alan_runtime::AlanHomePaths;
use anyhow::Context;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8090";
const DEFAULT_DAEMON_URL: &str = "http://127.0.0.1:8090";

#[derive(Debug, Clone, Deserialize)]
pub struct HostConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_daemon_url")]
    pub daemon_url: String,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            daemon_url: default_daemon_url(),
        }
    }
}

impl HostConfig {
    pub fn load() -> anyhow::Result<Self> {
        Self::load_with_path(Self::host_file_path())
    }

    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path).with_context(|| {
            format!("failed to read host configuration file {}", path.display())
        })?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse host configuration file {}", path.display()))
    }

    pub fn host_file_path() -> Option<PathBuf> {
        AlanHomePaths::detect().map(|paths| paths.global_host_config_path)
    }

    #[cfg(test)]
    pub fn host_file_path_from_home(home: &Path) -> Option<PathBuf> {
        Some(AlanHomePaths::from_home_dir(home).global_host_config_path)
    }

    pub fn effective_bind_address(&self) -> String {
        std::env::var("BIND_ADDRESS").unwrap_or_else(|_| self.bind_address.clone())
    }

    pub fn effective_daemon_url(&self) -> String {
        std::env::var("ALAN_AGENTD_URL").unwrap_or_else(|_| self.daemon_url.clone())
    }

    fn load_with_path(path: Option<PathBuf>) -> anyhow::Result<Self> {
        if let Some(path) = path
            && path.exists()
        {
            return Self::from_file(&path);
        }

        Ok(Self::default())
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
    fn test_host_config_defaults_when_file_missing() {
        let config = HostConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0:8090");
        assert_eq!(config.daemon_url, "http://127.0.0.1:8090");
    }
}
