use crate::{
    AlanHomePaths, ConfigSourceKind, ResolvedAgentRoots,
    runtime::WorkspaceRuntimeConfig,
    skills::{
        PackageMount, ResolvedCapabilityView, ScopedPackageDir, SkillScope,
        default_builtin_package_mounts,
    },
    workspace_public_skills_dir,
};
use anyhow::Context;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Canonical resolved agent definition derived from runtime launch input.
#[derive(Debug, Clone)]
pub struct ResolvedAgentDefinition {
    pub agent_name: Option<String>,
    pub workspace_root_dir: Option<PathBuf>,
    pub workspace_alan_dir: Option<PathBuf>,
    pub roots: ResolvedAgentRoots,
    pub config_overlay_paths: Vec<PathBuf>,
    pub persona_dirs: Vec<PathBuf>,
    pub capability_view: ResolvedCapabilityView,
    pub default_policy_path: Option<PathBuf>,
    pub writable_root_dir: Option<PathBuf>,
    pub writable_persona_dir: Option<PathBuf>,
}

impl ResolvedAgentDefinition {
    pub fn from_runtime_config(config: &WorkspaceRuntimeConfig) -> anyhow::Result<Self> {
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
        let home_paths = config
            .agent_home_paths
            .clone()
            .or_else(AlanHomePaths::detect);
        let roots = ResolvedAgentRoots::with_home_paths(
            home_paths.clone(),
            workspace_root_dir.as_deref(),
            agent_name.as_deref(),
        );
        let config_overlay_paths = overlay_config_paths(&roots, config.core_config_source);
        let persona_dirs = if roots.is_empty() {
            legacy_persona_dirs_without_agent_roots(&config.agent_config.core_config)
        } else {
            roots.persona_dirs()
        };
        let package_dirs =
            package_dirs_for_roots(&roots, home_paths.as_ref(), workspace_root_dir.as_deref());
        let mut capability_view =
            ResolvedCapabilityView::from_package_dirs(package_dirs).with_default_mounts();
        capability_view.apply_mount_overrides(&default_builtin_package_mounts());
        capability_view.apply_mount_overrides(&config.agent_config.core_config.package_mounts);
        capability_view.apply_mount_overrides(&package_mount_overrides_for_roots(
            &roots,
            config.core_config_source,
        )?);

        Ok(Self {
            agent_name,
            workspace_root_dir,
            workspace_alan_dir,
            default_policy_path: roots.highest_precedence_policy_path(),
            writable_root_dir: roots.writable_root_dir(),
            writable_persona_dir: roots.writable_persona_dir(),
            roots,
            config_overlay_paths,
            persona_dirs,
            capability_view,
        })
    }
}

fn package_dirs_for_roots(
    roots: &ResolvedAgentRoots,
    home_paths: Option<&AlanHomePaths>,
    workspace_root_dir: Option<&Path>,
) -> Vec<ScopedPackageDir> {
    let mut package_dirs = Vec::new();

    for root in roots.roots() {
        match root.kind {
            crate::AgentRootKind::GlobalBase => {
                if let Some(home_paths) = home_paths {
                    package_dirs.push(ScopedPackageDir {
                        path: home_paths.global_public_skills_dir.clone(),
                        scope: SkillScope::User,
                    });
                }
                package_dirs.push(ScopedPackageDir {
                    path: root.skills_dir.clone(),
                    scope: SkillScope::User,
                });
            }
            crate::AgentRootKind::WorkspaceBase => {
                if let Some(workspace_root_dir) = workspace_root_dir {
                    package_dirs.push(ScopedPackageDir {
                        path: workspace_public_skills_dir(workspace_root_dir),
                        scope: SkillScope::Repo,
                    });
                }
                package_dirs.push(ScopedPackageDir {
                    path: root.skills_dir.clone(),
                    scope: SkillScope::Repo,
                });
            }
            crate::AgentRootKind::GlobalNamed(_) => {
                package_dirs.push(ScopedPackageDir {
                    path: root.skills_dir.clone(),
                    scope: SkillScope::User,
                });
            }
            crate::AgentRootKind::WorkspaceNamed(_) => {
                package_dirs.push(ScopedPackageDir {
                    path: root.skills_dir.clone(),
                    scope: SkillScope::Repo,
                });
            }
        }
    }

    package_dirs
}

#[derive(Debug, Default, Deserialize)]
struct PackageMountOverlayFile {
    #[serde(default)]
    package_mounts: Vec<PackageMount>,
}

fn package_mount_overrides_for_roots(
    roots: &ResolvedAgentRoots,
    base_source: ConfigSourceKind,
) -> anyhow::Result<Vec<PackageMount>> {
    let mut overrides = Vec::new();

    for root in roots.roots() {
        if matches!(
            (&root.kind, base_source),
            (
                crate::AgentRootKind::GlobalBase,
                ConfigSourceKind::GlobalAgentHome
            ) | (_, ConfigSourceKind::EnvOverride)
        ) {
            continue;
        }
        if !root.config_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&root.config_path).with_context(|| {
            format!(
                "failed to read configuration file {}",
                root.config_path.display()
            )
        })?;
        let overlay: PackageMountOverlayFile = toml::from_str(&content).with_context(|| {
            format!(
                "failed to parse configuration file {}",
                root.config_path.display()
            )
        })?;
        overrides.extend(overlay.package_mounts);
    }

    Ok(overrides)
}

fn overlay_config_paths(roots: &ResolvedAgentRoots, base_source: ConfigSourceKind) -> Vec<PathBuf> {
    roots
        .roots()
        .iter()
        .filter(|root| {
            !matches!(
                (&root.kind, base_source),
                (
                    crate::AgentRootKind::GlobalBase,
                    ConfigSourceKind::GlobalAgentHome
                ) | (_, ConfigSourceKind::EnvOverride)
            )
        })
        .map(|root| root.config_path.clone())
        .collect()
}

fn legacy_persona_dirs_without_agent_roots(config: &crate::config::Config) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(global_dir) = legacy_global_persona_dir() {
        dirs.push(global_dir);
    }
    if let Some(config_dir) = legacy_persona_dir_from_core_config(config)
        && !dirs.contains(&config_dir)
    {
        dirs.push(config_dir);
    }
    dirs
}

fn legacy_persona_dir_from_core_config(config: &crate::config::Config) -> Option<PathBuf> {
    if let Some(path) = config.memory.workspace_dir.clone() {
        let is_memory_dir = path
            .file_name()
            .map(|name| name == std::ffi::OsStr::new("memory"))
            .unwrap_or(false);
        if is_memory_dir {
            return path
                .parent()
                .map(crate::workspace_persona_dir_from_alan_dir);
        }
        return Some(path);
    }

    if cfg!(test) {
        return None;
    }

    legacy_global_persona_dir()
}

fn legacy_global_persona_dir() -> Option<PathBuf> {
    crate::AlanHomePaths::detect().map(|paths| paths.global_agent_root_dir.join("persona"))
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
    use crate::{AlanHomePaths, Config, skills::PackageMountMode};
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_skill(root_dir: &Path, skill_dir_name: &str, skill_name: &str) {
        let skill_dir = root_dir.join("skills").join(skill_dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                r#"---
name: {skill_name}
description: test skill
---

Body
"#
            ),
        )
        .unwrap();
    }

    fn create_public_skill(root_dir: &Path, skill_dir_name: &str, skill_name: &str) {
        let skill_dir = root_dir.join(".agents/skills").join(skill_dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                r#"---
name: {skill_name}
description: public test skill
---

Body
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn resolved_agent_definition_uses_named_agent_overlay_order() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        let workspace_alan_dir = workspace_root.join(".alan");
        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.workspace_root_dir = Some(workspace_root.clone());
        config.workspace_alan_dir = Some(workspace_alan_dir.clone());
        config.agent_name = Some("coder".to_string());

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config).unwrap();
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

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config).unwrap();

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

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config).unwrap();

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

    #[test]
    fn legacy_persona_dirs_without_agent_roots_prefer_workspace_persona_from_memory_dir() {
        let workspace_root = TempDir::new().unwrap();
        let mut config = Config::default();
        config.memory.workspace_dir = Some(workspace_root.path().join(".alan/memory"));

        let persona_dirs = legacy_persona_dirs_without_agent_roots(&config);

        assert_eq!(
            persona_dirs.last(),
            Some(&workspace_root.path().join(".alan/agent/persona"))
        );
    }

    #[test]
    fn resolved_agent_definition_assigns_default_mount_modes() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        let workspace_agent_root = workspace_root.join(".alan/agent");
        create_test_skill(&workspace_agent_root, "test-skill", "Test Skill");

        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.workspace_root_dir = Some(workspace_root.clone());
        config.workspace_alan_dir = Some(workspace_root.join(".alan"));
        config.agent_home_paths = Some(AlanHomePaths::from_home_dir(temp.path()));

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config).unwrap();

        assert!(
            resolved
                .capability_view
                .mounts
                .iter()
                .any(|mount| mount.package_id == "builtin:alan-plan"
                    && mount.mode == PackageMountMode::AlwaysActive)
        );
        assert!(
            resolved
                .capability_view
                .mounts
                .iter()
                .any(|mount| mount.package_id == "skill:test-skill"
                    && mount.mode == PackageMountMode::Discoverable)
        );
    }

    #[test]
    fn resolved_agent_definition_applies_package_mount_overrides_in_overlay_order() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let workspace_root = temp.path().join("workspace");
        let home_paths = AlanHomePaths::from_home_dir(&home);
        let global_root = home_paths.global_agent_root_dir.clone();
        let workspace_agent_root = workspace_root.join(".alan/agent");
        let global_named_root = home_paths.global_named_agents_dir.join("coder");
        let workspace_named_root = workspace_root.join(".alan/agents/coder");

        create_test_skill(&workspace_agent_root, "test-skill", "Test Skill");
        std::fs::create_dir_all(&global_root).unwrap();
        std::fs::create_dir_all(&workspace_named_root).unwrap();
        std::fs::create_dir_all(&global_named_root).unwrap();

        std::fs::write(
            global_root.join("agent.toml"),
            r#"
[[package_mounts]]
package = "builtin:alan-plan"
mode = "discoverable"

[[package_mounts]]
package = "skill:test-skill"
mode = "explicit_only"
"#,
        )
        .unwrap();
        std::fs::write(
            workspace_named_root.join("agent.toml"),
            r#"
[[package_mounts]]
package = "skill:test-skill"
mode = "internal"
"#,
        )
        .unwrap();

        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.workspace_root_dir = Some(workspace_root.clone());
        config.workspace_alan_dir = Some(workspace_root.join(".alan"));
        config.agent_name = Some("coder".to_string());
        config.agent_home_paths = Some(home_paths);

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config).unwrap();

        let mounts = &resolved.capability_view.mounts;
        assert_eq!(
            mounts
                .iter()
                .find(|mount| mount.package_id == "builtin:alan-plan")
                .unwrap()
                .mode,
            PackageMountMode::Discoverable
        );
        assert_eq!(
            mounts
                .iter()
                .find(|mount| mount.package_id == "skill:test-skill")
                .unwrap()
                .mode,
            PackageMountMode::Internal
        );
    }

    #[test]
    fn resolved_agent_definition_honors_env_override_package_mounts_without_root_parsing() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let workspace_root = temp.path().join("workspace");
        let home_paths = AlanHomePaths::from_home_dir(&home);
        let global_root = home_paths.global_agent_root_dir.clone();

        std::fs::create_dir_all(&global_root).unwrap();
        std::fs::write(
            global_root.join("agent.toml"),
            "[[package_mounts]]\npackage = ",
        )
        .unwrap();

        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.workspace_root_dir = Some(workspace_root.clone());
        config.workspace_alan_dir = Some(workspace_root.join(".alan"));
        config.agent_home_paths = Some(home_paths);
        config.core_config_source = ConfigSourceKind::EnvOverride;
        config.agent_config.core_config.package_mounts = vec![PackageMount {
            package_id: "builtin:alan-plan".to_string(),
            mode: PackageMountMode::ExplicitOnly,
        }];

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config).unwrap();
        let mount = resolved
            .capability_view
            .mounts
            .iter()
            .find(|mount| mount.package_id == "builtin:alan-plan")
            .unwrap();
        assert_eq!(mount.mode, PackageMountMode::ExplicitOnly);
    }

    #[test]
    fn resolved_agent_definition_discovers_public_skill_directories() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let workspace_root = temp.path().join("workspace");
        let home_paths = AlanHomePaths::from_home_dir(&home);

        create_public_skill(&home, "global-public-skill", "Global Public Skill");
        create_public_skill(
            &workspace_root,
            "workspace-public-skill",
            "Workspace Public Skill",
        );

        let mut config = WorkspaceRuntimeConfig::from(Config::default());
        config.workspace_root_dir = Some(workspace_root.clone());
        config.workspace_alan_dir = Some(workspace_root.join(".alan"));
        config.agent_home_paths = Some(home_paths.clone());

        let resolved = ResolvedAgentDefinition::from_runtime_config(&config).unwrap();

        assert!(resolved.capability_view.package_dirs.iter().any(|dir| {
            dir.path == home_paths.global_public_skills_dir && dir.scope == SkillScope::User
        }));
        assert!(resolved.capability_view.package_dirs.iter().any(|dir| {
            dir.path == workspace_public_skills_dir(&workspace_root)
                && dir.scope == SkillScope::Repo
        }));
        assert!(
            resolved
                .capability_view
                .packages
                .iter()
                .any(|package| package.id == "skill:global-public-skill")
        );
        assert!(
            resolved
                .capability_view
                .packages
                .iter()
                .any(|package| package.id == "skill:workspace-public-skill")
        );
    }
}
