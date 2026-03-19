use crate::{
    ConfigSourceKind, ResolvedAgentRoots, prompts,
    runtime::WorkspaceRuntimeConfig,
    skills::{ScopedSkillDir, SkillScope},
};
use std::path::{Path, PathBuf};

/// Canonical resolved agent definition derived from runtime launch input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAgentDefinition {
    pub agent_name: Option<String>,
    pub workspace_root_dir: Option<PathBuf>,
    pub workspace_alan_dir: Option<PathBuf>,
    pub roots: ResolvedAgentRoots,
    pub config_overlay_paths: Vec<PathBuf>,
    pub persona_dirs: Vec<PathBuf>,
    pub skill_dirs: Vec<ScopedSkillDir>,
    pub default_policy_path: Option<PathBuf>,
    pub writable_root_dir: Option<PathBuf>,
    pub writable_persona_dir: Option<PathBuf>,
}

impl ResolvedAgentDefinition {
    pub fn from_runtime_config(config: &WorkspaceRuntimeConfig) -> Self {
        let workspace_alan_dir = config.workspace_alan_dir.clone().or_else(|| {
            infer_workspace_alan_dir_from_memory_dir(
                config
                    .agent_config
                    .core_config
                    .memory
                    .workspace_dir
                    .as_deref(),
            )
        });
        let workspace_root_dir = config
            .workspace_root_dir
            .clone()
            .or_else(|| infer_workspace_root_from_alan_dir(workspace_alan_dir.as_deref()));
        let agent_name =
            crate::normalize_agent_name(config.agent_name.as_deref()).map(str::to_owned);
        let roots =
            ResolvedAgentRoots::for_workspace(workspace_root_dir.as_deref(), agent_name.as_deref());
        let config_overlay_paths = overlay_config_paths(&roots, config.core_config_source);
        let persona_dirs = if roots.is_empty() {
            prompts::resolve_workspace_persona_dirs_for_workspace(
                &config.agent_config.core_config,
                None,
            )
        } else {
            roots.persona_dirs()
        };
        let skill_dirs = roots
            .roots()
            .iter()
            .map(|root| ScopedSkillDir {
                path: root.skills_dir.clone(),
                scope: match root.kind {
                    crate::AgentRootKind::GlobalBase | crate::AgentRootKind::GlobalNamed(_) => {
                        SkillScope::User
                    }
                    crate::AgentRootKind::WorkspaceBase
                    | crate::AgentRootKind::WorkspaceNamed(_) => SkillScope::Repo,
                },
            })
            .collect();

        Self {
            agent_name,
            workspace_root_dir,
            workspace_alan_dir,
            default_policy_path: roots.highest_precedence_policy_path(),
            writable_root_dir: roots.writable_root_dir(),
            writable_persona_dir: roots.writable_persona_dir(),
            roots,
            config_overlay_paths,
            persona_dirs,
            skill_dirs,
        }
    }
}

fn overlay_config_paths(roots: &ResolvedAgentRoots, base_source: ConfigSourceKind) -> Vec<PathBuf> {
    roots
        .roots()
        .iter()
        .filter(|root| match (&root.kind, base_source) {
            (crate::AgentRootKind::GlobalBase, ConfigSourceKind::GlobalAgentHome) => false,
            (_, ConfigSourceKind::EnvOverride) => false,
            _ => true,
        })
        .map(|root| root.config_path.clone())
        .collect()
}

fn infer_workspace_alan_dir_from_memory_dir(memory_dir: Option<&Path>) -> Option<PathBuf> {
    let memory_dir = memory_dir?;
    let is_memory_dir = memory_dir
        .file_name()
        .map(|name| name == std::ffi::OsStr::new("memory"))
        .unwrap_or(false);
    if !is_memory_dir {
        return None;
    }

    let alan_dir = memory_dir.parent()?;
    let is_alan_dir = alan_dir
        .file_name()
        .map(|name| name == std::ffi::OsStr::new(".alan"))
        .unwrap_or(false);
    is_alan_dir.then(|| alan_dir.to_path_buf())
}

fn infer_workspace_root_from_alan_dir(alan_dir: Option<&Path>) -> Option<PathBuf> {
    let alan_dir = alan_dir?;
    let is_alan_dir = alan_dir
        .file_name()
        .map(|name| name == std::ffi::OsStr::new(".alan"))
        .unwrap_or(false);
    if !is_alan_dir {
        return None;
    }

    alan_dir.parent().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AlanHomePaths, Config};
    use tempfile::TempDir;

    #[test]
    fn resolved_agent_definition_uses_named_agent_overlay_order() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        let workspace_alan_dir = workspace_root.join(".alan");
        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.workspace_root_dir = Some(workspace_root.clone());
        config.workspace_alan_dir = Some(workspace_alan_dir.clone());
        config.agent_name = Some("coder".to_string());

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config);
        let home_paths = AlanHomePaths::detect().unwrap();

        assert_eq!(
            resolved.config_overlay_paths,
            vec![
                home_paths.global_agent_config_path,
                workspace_alan_dir.join("agent/agent.toml"),
                home_paths.global_named_agents_dir.join("coder/agent.toml"),
                workspace_alan_dir.join("agents/coder/agent.toml"),
            ]
        );
        assert_eq!(resolved.agent_name.as_deref(), Some("coder"));
        assert_eq!(
            resolved.writable_root_dir,
            Some(workspace_alan_dir.join("agents/coder"))
        );
    }

    #[test]
    fn resolved_agent_definition_skips_global_base_overlay_for_global_home_source() {
        let workspace_root = TempDir::new().unwrap();
        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.workspace_root_dir = Some(workspace_root.path().to_path_buf());
        config.workspace_alan_dir = Some(workspace_root.path().join(".alan"));
        config.core_config_source = ConfigSourceKind::GlobalAgentHome;

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config);

        assert_eq!(
            resolved.config_overlay_paths,
            vec![workspace_root.path().join(".alan/agent/agent.toml")]
        );
    }

    #[test]
    fn resolved_agent_definition_infers_workspace_paths_from_memory_dir() {
        let workspace_root = TempDir::new().unwrap();
        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.agent_config.core_config.memory.workspace_dir =
            Some(workspace_root.path().join(".alan/memory"));

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config);

        assert_eq!(
            resolved.workspace_alan_dir,
            Some(workspace_root.path().join(".alan"))
        );
        assert_eq!(
            resolved.workspace_root_dir,
            Some(workspace_root.path().to_path_buf())
        );
        assert_eq!(
            resolved.writable_root_dir,
            Some(workspace_root.path().join(".alan/agent"))
        );
    }
}
