use std::path::{Path, PathBuf};

/// Canonical Alan home paths derived from a user home directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlanHomePaths {
    pub home_dir: PathBuf,
    pub alan_home_dir: PathBuf,
    pub global_agent_root_dir: PathBuf,
    pub global_named_agents_dir: PathBuf,
    pub global_agent_config_path: PathBuf,
    pub global_host_config_path: PathBuf,
    pub global_models_path: PathBuf,
}

impl AlanHomePaths {
    /// Resolve Alan home paths from the current user's home directory.
    pub fn detect() -> Option<Self> {
        dirs::home_dir().map(|home| Self::from_home_dir(&home))
    }

    /// Resolve Alan home paths from an explicit home directory.
    pub fn from_home_dir(home_dir: &Path) -> Self {
        let home_dir = home_dir.to_path_buf();
        let alan_home_dir = home_dir.join(".alan");
        let global_agent_root_dir = alan_home_dir.join("agent");
        let global_named_agents_dir = alan_home_dir.join("agents");

        Self {
            home_dir: home_dir.clone(),
            alan_home_dir: alan_home_dir.clone(),
            global_agent_root_dir: global_agent_root_dir.clone(),
            global_named_agents_dir,
            global_agent_config_path: global_agent_root_dir.join("agent.toml"),
            global_host_config_path: alan_home_dir.join("host.toml"),
            global_models_path: alan_home_dir.join("models.toml"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AlanHomePaths;
    use std::path::Path;

    #[test]
    fn test_from_home_dir_builds_expected_layout() {
        let paths = AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"));

        assert_eq!(paths.alan_home_dir, Path::new("/tmp/demo-home/.alan"));
        assert_eq!(
            paths.global_agent_root_dir,
            Path::new("/tmp/demo-home/.alan/agent")
        );
        assert_eq!(
            paths.global_named_agents_dir,
            Path::new("/tmp/demo-home/.alan/agents")
        );
        assert_eq!(
            paths.global_agent_config_path,
            Path::new("/tmp/demo-home/.alan/agent/agent.toml")
        );
        assert_eq!(
            paths.global_host_config_path,
            Path::new("/tmp/demo-home/.alan/host.toml")
        );
        assert_eq!(
            paths.global_models_path,
            Path::new("/tmp/demo-home/.alan/models.toml")
        );
    }
}
