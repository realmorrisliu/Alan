use crate::cli::load_agent_config_metadata_with_notice;
use crate::registry::normalize_workspace_root_path;
use alan_runtime::skills::{PackageMountMode, SkillMetadata, SkillsRegistry, list_skills};
use alan_runtime::{
    AgentRootKind, LoadedConfig, ResolvedAgentDefinition, WorkspaceRuntimeConfig,
    workspace_alan_dir,
};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub fn run_list_skills(workspace: Option<PathBuf>, agent_name: Option<&str>) -> Result<()> {
    let (_, registry) = resolve_registry(workspace, agent_name)?;
    print!("{}", list_skills(&registry));
    Ok(())
}

pub fn run_list_packages(workspace: Option<PathBuf>, agent_name: Option<&str>) -> Result<()> {
    let (resolved, registry) = resolve_registry(workspace, agent_name)?;
    println!("{}", render_packages(&resolved, &registry));
    Ok(())
}

fn resolve_registry(
    workspace: Option<PathBuf>,
    agent_name: Option<&str>,
) -> Result<(ResolvedAgentDefinition, SkillsRegistry)> {
    let loaded = load_agent_config_metadata_with_notice()?;
    resolve_registry_with_loaded_config(workspace, agent_name, loaded, None)
}

fn resolve_registry_with_loaded_config(
    workspace: Option<PathBuf>,
    agent_name: Option<&str>,
    loaded: LoadedConfig,
    home_paths: Option<alan_runtime::AlanHomePaths>,
) -> Result<(ResolvedAgentDefinition, SkillsRegistry)> {
    let workspace_root = resolve_workspace_root(workspace)?;
    let mut runtime_config = WorkspaceRuntimeConfig::from(loaded);
    runtime_config.workspace_root_dir = Some(workspace_root.clone());
    runtime_config.workspace_alan_dir = Some(workspace_alan_dir(&workspace_root));
    runtime_config.agent_name = agent_name.map(str::to_owned);
    runtime_config.agent_home_paths = home_paths;

    let resolved = ResolvedAgentDefinition::from_runtime_config(&runtime_config)?;
    let registry = SkillsRegistry::load_capability_view(&resolved.capability_view)?;
    Ok((resolved, registry))
}

fn resolve_workspace_root(workspace: Option<PathBuf>) -> Result<PathBuf> {
    let workspace = workspace.unwrap_or(
        std::env::current_dir()
            .context("Cannot determine current directory for skill inspection")?,
    );
    let canonical = std::fs::canonicalize(&workspace)
        .with_context(|| format!("Cannot resolve workspace path: {}", workspace.display()))?;
    Ok(normalize_workspace_root_path(&canonical))
}

fn render_packages(resolved: &ResolvedAgentDefinition, registry: &SkillsRegistry) -> String {
    let mut lines = vec![
        "Resolved Agent Roots".to_string(),
        "====================".to_string(),
        String::new(),
    ];

    if resolved.roots.roots().is_empty() {
        lines.push("No agent roots resolved.".to_string());
    } else {
        for root in resolved.roots.roots() {
            lines.push(format!(
                "[{}] {}",
                root_kind_label(&root.kind),
                root.root_dir.display()
            ));
        }
    }

    lines.extend([
        String::new(),
        "Resolved Packages".to_string(),
        "=================".to_string(),
        String::new(),
    ]);

    let mount_modes: HashMap<_, _> = resolved
        .capability_view
        .mounts
        .iter()
        .map(|mount| (mount.package_id.as_str(), mount.mode))
        .collect();
    let mut skills_by_package: BTreeMap<&str, Vec<&SkillMetadata>> = BTreeMap::new();
    for skill in registry.list_sorted() {
        if let Some(package_id) = skill.package_id.as_deref() {
            skills_by_package.entry(package_id).or_default().push(skill);
        }
    }

    let mut packages: Vec<_> = resolved.capability_view.packages.iter().collect();
    packages.sort_by(|left, right| {
        left.scope
            .priority()
            .cmp(&right.scope.priority())
            .then_with(|| left.id.cmp(&right.id))
    });

    for package in packages {
        let mount_label = mount_modes
            .get(package.id.as_str())
            .copied()
            .map(package_mount_mode_label)
            .unwrap_or("unmounted");
        let source_label = package
            .root_dir
            .as_deref()
            .map(render_path)
            .unwrap_or_else(|| "<embedded>".to_string());
        let skills_label = skills_by_package
            .get(package.id.as_str())
            .map(|skills| {
                skills
                    .iter()
                    .map(|skill| format!("${}", skill.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "not exposed".to_string());

        lines.push(format!(
            "[{}] {} ({})",
            scope_label(package.scope),
            package.id,
            mount_label
        ));
        lines.push(format!("         source: {}", source_label));
        lines.push(format!("         skills: {}", skills_label));
        lines.push(String::new());
    }

    lines.join("\n")
}

fn root_kind_label(kind: &AgentRootKind) -> &'static str {
    match kind {
        AgentRootKind::GlobalBase => "global-base",
        AgentRootKind::WorkspaceBase => "workspace-base",
        AgentRootKind::GlobalNamed(_) => "global-named",
        AgentRootKind::WorkspaceNamed(_) => "workspace-named",
    }
}

fn scope_label(scope: alan_runtime::skills::SkillScope) -> &'static str {
    match scope {
        alan_runtime::skills::SkillScope::Repo => "repo",
        alan_runtime::skills::SkillScope::User => "user",
        alan_runtime::skills::SkillScope::Builtin => "builtin",
    }
}

fn package_mount_mode_label(mode: PackageMountMode) -> &'static str {
    match mode {
        PackageMountMode::AlwaysActive => "always_active",
        PackageMountMode::Discoverable => "discoverable",
        PackageMountMode::ExplicitOnly => "explicit_only",
        PackageMountMode::Internal => "internal",
    }
}

fn render_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alan_runtime::{AlanHomePaths, Config};
    use std::fs;
    use tempfile::TempDir;

    fn create_skill(root: &Path, skill_name: &str, title: &str) {
        let skill_dir = root.join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                r#"---
name: {title}
description: {title} description
---

Body
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn render_packages_shows_roots_mounts_and_exposed_skills() {
        let temp = TempDir::new().unwrap();
        let home_paths = AlanHomePaths::from_home_dir(temp.path());
        let workspace_root = temp.path().join("workspace");
        let workspace_skills_dir = workspace_root.join(".alan/agent/skills");
        create_skill(&workspace_skills_dir, "repo-skill", "Repo Skill");

        let (resolved, registry) = resolve_registry_with_loaded_config(
            Some(workspace_root),
            None,
            LoadedConfig {
                config: Config::default(),
                path: None,
                source: alan_runtime::ConfigSourceKind::Default,
            },
            Some(home_paths),
        )
        .unwrap();

        let rendered = render_packages(&resolved, &registry);
        assert!(rendered.contains("Resolved Agent Roots"));
        assert!(rendered.contains("Resolved Packages"));
        assert!(rendered.contains("[builtin] builtin:alan-plan (always_active)"));
        assert!(rendered.contains("[repo] skill:repo-skill (discoverable)"));
        assert!(rendered.contains("skills: $repo-skill"));
    }

    #[test]
    fn resolve_workspace_root_defaults_to_current_directory() {
        let current = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
        assert_eq!(
            resolve_workspace_root(None).unwrap(),
            normalize_workspace_root_path(&current)
        );
    }
}
