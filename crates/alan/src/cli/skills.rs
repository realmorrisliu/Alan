use crate::cli::load_agent_config_metadata_with_notice;
use crate::registry::normalize_workspace_root_path;
use alan_runtime::skills::{
    PackageMountMode, SkillHostCapabilities, SkillMetadata, SkillsRegistry,
    format_skill_availability_issues, list_skills, skill_availability_issues,
};
use alan_runtime::{
    AgentRootKind, LoadedConfig, ResolvedAgentDefinition, ToolRegistry, WorkspaceRuntimeConfig,
    workspace_alan_dir,
};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn run_list_skills(workspace: Option<PathBuf>, agent_name: Option<&str>) -> Result<()> {
    let (_, registry, host_capabilities) = resolve_registry(workspace, agent_name)?;
    print!("{}", list_skills(&registry, &host_capabilities));
    Ok(())
}

pub fn run_list_packages(workspace: Option<PathBuf>, agent_name: Option<&str>) -> Result<()> {
    let (resolved, registry, host_capabilities) = resolve_registry(workspace, agent_name)?;
    println!(
        "{}",
        render_packages(&resolved, &registry, &host_capabilities)
    );
    Ok(())
}

fn resolve_registry(
    workspace: Option<PathBuf>,
    agent_name: Option<&str>,
) -> Result<(
    ResolvedAgentDefinition,
    SkillsRegistry,
    SkillHostCapabilities,
)> {
    let loaded = load_agent_config_metadata_with_notice()?;
    resolve_registry_with_loaded_config(workspace, agent_name, loaded, None)
}

fn resolve_registry_with_loaded_config(
    workspace: Option<PathBuf>,
    agent_name: Option<&str>,
    loaded: LoadedConfig,
    home_paths: Option<alan_runtime::AlanHomePaths>,
) -> Result<(
    ResolvedAgentDefinition,
    SkillsRegistry,
    SkillHostCapabilities,
)> {
    let (workspace_root, workspace_alan_dir) = resolve_workspace_context(workspace)?;
    let mut runtime_config = WorkspaceRuntimeConfig::from(loaded.clone());
    runtime_config.workspace_root_dir = Some(workspace_root.clone());
    runtime_config.workspace_alan_dir = Some(workspace_alan_dir);
    runtime_config.agent_name = agent_name.map(str::to_owned);
    runtime_config.agent_home_paths = home_paths;

    let resolved = ResolvedAgentDefinition::from_runtime_config(&runtime_config)?;
    let registry = SkillsRegistry::load_capability_view(&resolved.capability_view)?;
    let host_capabilities = resolve_host_capabilities(&loaded.config, &resolved)?;
    Ok((resolved, registry, host_capabilities))
}

fn canonicalize_workspace_input(workspace: Option<PathBuf>) -> Result<PathBuf> {
    let workspace = workspace.unwrap_or(
        std::env::current_dir()
            .context("Cannot determine current directory for skill inspection")?,
    );
    std::fs::canonicalize(&workspace)
        .with_context(|| format!("Cannot resolve workspace path: {}", workspace.display()))
}

#[cfg(test)]
fn resolve_workspace_root(workspace: Option<PathBuf>) -> Result<PathBuf> {
    let canonical = canonicalize_workspace_input(workspace)?;
    Ok(normalize_workspace_root_path(&canonical))
}

fn resolve_workspace_context(workspace: Option<PathBuf>) -> Result<(PathBuf, PathBuf)> {
    let canonical = canonicalize_workspace_input(workspace)?;
    let workspace_root = normalize_workspace_root_path(&canonical);
    let workspace_alan_dir = if canonical
        .file_name()
        .map(|name| name == std::ffi::OsStr::new(".alan"))
        .unwrap_or(false)
    {
        canonical
    } else {
        workspace_alan_dir(&workspace_root)
    };
    Ok((workspace_root, workspace_alan_dir))
}

fn render_packages(
    resolved: &ResolvedAgentDefinition,
    registry: &SkillsRegistry,
    host_capabilities: &SkillHostCapabilities,
) -> String {
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
    let mut skills_by_package: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for skill in registry.list_sorted() {
        if let Some(package_id) = skill.package_id.as_deref() {
            skills_by_package
                .entry(package_id)
                .or_default()
                .push(render_skill_label(skill, host_capabilities));
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
            .map(|skills| skills.join(", "))
            .unwrap_or_else(|| "not exposed".to_string());

        lines.push(format!(
            "[{}] {} ({})",
            scope_label(package.scope),
            package.id,
            mount_label
        ));
        lines.push(format!("         source: {}", source_label));
        if let Some(exports_label) = render_package_exports(package) {
            lines.push(format!("         exports: {}", exports_label));
        }
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

fn resolve_host_capabilities(
    base_config: &alan_runtime::Config,
    resolved: &ResolvedAgentDefinition,
) -> Result<SkillHostCapabilities> {
    let mut core_config = base_config.clone();
    if !resolved.config_overlay_paths.is_empty() {
        core_config = core_config.with_agent_root_overlays(&resolved.config_overlay_paths)?;
    }
    let tools = ToolRegistry::with_config(Arc::new(core_config));
    Ok(SkillHostCapabilities::with_tools(
        tools.list_tools().into_iter().map(str::to_string),
    ))
}

fn render_skill_label(skill: &SkillMetadata, host_capabilities: &SkillHostCapabilities) -> String {
    let issues = skill_availability_issues(skill, host_capabilities);
    if issues.is_empty() {
        format!("${}", skill.id)
    } else {
        format!(
            "${} [unavailable: {}]",
            skill.id,
            format_skill_availability_issues(&issues)
        )
    }
}

fn render_package_exports(package: &alan_runtime::skills::CapabilityPackage) -> Option<String> {
    if package.exports.is_empty() {
        return None;
    }

    let mut exports = Vec::new();
    if !package.exports.child_agent_roots.is_empty() {
        exports.push(format!(
            "child_agents={}",
            package.exports.child_agent_roots.len()
        ));
    }

    let mut resources = Vec::new();
    if package.exports.resources.scripts_dir.is_some() {
        resources.push("scripts");
    }
    if package.exports.resources.references_dir.is_some() {
        resources.push("references");
    }
    if package.exports.resources.assets_dir.is_some() {
        resources.push("assets");
    }
    if package.exports.resources.viewers_dir.is_some() {
        resources.push("viewers");
    }
    if !resources.is_empty() {
        exports.push(format!("resources={}", resources.join("+")));
    }

    (!exports.is_empty()).then(|| exports.join(", "))
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

    fn create_skill_with_frontmatter(root: &Path, skill_name: &str, frontmatter: &str) {
        let skill_dir = root.join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), frontmatter).unwrap();
    }

    #[test]
    fn render_packages_shows_roots_mounts_and_exposed_skills() {
        let temp = TempDir::new().unwrap();
        let home_paths = AlanHomePaths::from_home_dir(temp.path());
        let workspace_root = temp.path().join("workspace");
        let workspace_skills_dir = workspace_root.join(".alan/agent/skills");
        create_skill(&workspace_skills_dir, "repo-skill", "Repo Skill");

        let (resolved, registry, host_capabilities) = resolve_registry_with_loaded_config(
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

        let rendered = render_packages(&resolved, &registry, &host_capabilities);
        assert!(rendered.contains("Resolved Agent Roots"));
        assert!(rendered.contains("Resolved Packages"));
        assert!(rendered.contains("[builtin] builtin:alan-plan (always_active)"));
        assert!(rendered.contains("[repo] skill:repo-skill (discoverable)"));
        assert!(rendered.contains("skills: $repo-skill"));
    }

    #[test]
    fn render_packages_shows_package_exports_and_unavailable_skills() {
        let temp = TempDir::new().unwrap();
        let home_paths = AlanHomePaths::from_home_dir(temp.path());
        let workspace_root = temp.path().join("workspace");
        let workspace_skills_dir = workspace_root.join(".alan/agent/skills");
        let skill_dir = workspace_skills_dir.join("tool-heavy");
        fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        fs::create_dir_all(skill_dir.join("agents/reviewer")).unwrap();
        create_skill_with_frontmatter(
            &workspace_skills_dir,
            "tool-heavy",
            r#"---
name: Tool Heavy
description: Needs extra tools
capabilities:
  required_tools: ["missing_tool"]
---

Body
"#,
        );

        let (resolved, registry, host_capabilities) = resolve_registry_with_loaded_config(
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

        let rendered = render_packages(&resolved, &registry, &host_capabilities);
        assert!(rendered.contains("exports: child_agents=1, resources=scripts"));
        assert!(
            rendered.contains(
                "skills: $tool-heavy [unavailable: missing required tools: missing_tool]"
            )
        );
    }

    #[test]
    fn resolve_workspace_root_defaults_to_current_directory() {
        let current = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
        assert_eq!(
            resolve_workspace_root(None).unwrap(),
            normalize_workspace_root_path(&current)
        );
    }

    #[test]
    fn resolve_workspace_context_keeps_explicit_alan_state_dir() {
        let temp = TempDir::new().unwrap();
        let default_workspace = temp.path().join(".alan");
        fs::create_dir_all(&default_workspace).unwrap();
        let canonical_workspace_root = std::fs::canonicalize(temp.path()).unwrap();
        let canonical_alan_dir = std::fs::canonicalize(&default_workspace).unwrap();

        let (workspace_root, alan_dir) =
            resolve_workspace_context(Some(default_workspace.clone())).unwrap();
        assert_eq!(workspace_root, canonical_workspace_root);
        assert_eq!(alan_dir, canonical_alan_dir);
    }
}
