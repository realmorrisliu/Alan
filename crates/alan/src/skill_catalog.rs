use alan_runtime::skills::{
    CapabilityPackage, CapabilityPackageResources, CompatibleSkillMetadata, PackageMountMode,
    ResolvedSkillExecution, SkillHostCapabilities, SkillMetadata, SkillRemediation, SkillsRegistry,
    skill_availability_issues, skill_remediation,
};
use alan_runtime::{Config, ResolvedAgentDefinition, ToolRegistry, WorkspaceRuntimeConfig};
use alan_tools::create_core_tools;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct SkillCatalogContext {
    pub resolved: ResolvedAgentDefinition,
    pub registry: SkillsRegistry,
    pub host_capabilities: SkillHostCapabilities,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillCatalogTarget {
    pub workspace_dir: Option<PathBuf>,
    pub agent_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCatalogSnapshot {
    pub cursor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_alan_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writable_root_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writable_config_path: Option<PathBuf>,
    pub packages: Vec<SkillCatalogPackageSnapshot>,
    pub skills: Vec<SkillCatalogSkillSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCatalogPackageSnapshot {
    pub id: String,
    pub scope: alan_runtime::skills::SkillScope,
    pub mount_mode: PackageMountMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<PathBuf>,
    pub exports: SkillCatalogPackageExportsSnapshot,
    pub skill_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCatalogPackageExportsSnapshot {
    pub child_agents: Vec<SkillCatalogChildAgentExportSnapshot>,
    pub resources: SkillCatalogResourceExportsSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCatalogChildAgentExportSnapshot {
    pub name: String,
    pub root_dir: PathBuf,
    pub handle: alan_protocol::SpawnTarget,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillCatalogResourceExportsSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scripts_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewers_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCatalogSkillSnapshot {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_root: Option<PathBuf>,
    pub scope: alan_runtime::skills::SkillScope,
    pub mount_mode: PackageMountMode,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "CompatibleSkillMetadata::is_empty")]
    pub compatible_metadata: CompatibleSkillMetadata,
    pub execution: ResolvedSkillExecution,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub availability_issues: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<SkillRemediation>,
}

pub fn resolve_skill_catalog_context(
    runtime_config: &WorkspaceRuntimeConfig,
) -> Result<SkillCatalogContext> {
    let resolved = ResolvedAgentDefinition::from_runtime_config(runtime_config)?;
    let registry = SkillsRegistry::load_capability_view(&resolved.capability_view)?;
    let host_capabilities =
        resolve_skill_host_capabilities(&runtime_config.agent_config.core_config, &resolved)?;
    Ok(SkillCatalogContext {
        resolved,
        registry,
        host_capabilities,
    })
}

pub fn resolve_skill_host_capabilities(
    base_config: &Config,
    resolved: &ResolvedAgentDefinition,
) -> Result<SkillHostCapabilities> {
    let mut core_config = base_config.clone();
    if !resolved.config_overlay_paths.is_empty() {
        core_config = core_config.with_agent_root_overlays(&resolved.config_overlay_paths)?;
    }
    let mut tools = ToolRegistry::with_config(Arc::new(core_config));
    if let Some(workspace_root) = resolved.workspace_root_dir.as_ref() {
        for tool in create_core_tools(workspace_root.clone()) {
            tools.register_boxed(tool);
        }
    }
    Ok(
        SkillHostCapabilities::with_tools(tools.list_tools().into_iter().map(str::to_string))
            .with_process_env()
            .with_runtime_defaults(),
    )
}

pub fn build_skill_catalog_snapshot(context: &SkillCatalogContext) -> Result<SkillCatalogSnapshot> {
    let mut skill_ids_by_package: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut skills = Vec::new();

    for skill in context.registry.list_sorted().into_iter().cloned() {
        if let Some(package_id) = skill.package_id.as_deref() {
            skill_ids_by_package
                .entry(package_id.to_string())
                .or_default()
                .push(skill.id.clone());
        }
        skills.push(build_skill_snapshot(&skill, &context.host_capabilities));
    }

    for skill_ids in skill_ids_by_package.values_mut() {
        skill_ids.sort();
        skill_ids.dedup();
    }

    let mount_modes: HashMap<_, _> = context
        .resolved
        .capability_view
        .mounts
        .iter()
        .map(|mount| (mount.package_id.as_str(), mount.mode))
        .collect();

    let mut packages: Vec<&CapabilityPackage> =
        context.resolved.capability_view.packages.iter().collect();
    packages.sort_by(|left, right| {
        left.scope
            .priority()
            .cmp(&right.scope.priority())
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut package_snapshots = Vec::with_capacity(packages.len());
    for package in packages {
        let mount_mode = mount_modes
            .get(package.id.as_str())
            .copied()
            .unwrap_or(PackageMountMode::Discoverable);
        package_snapshots.push(SkillCatalogPackageSnapshot {
            id: package.id.clone(),
            scope: package.scope,
            mount_mode,
            root_dir: package.root_dir.clone(),
            exports: build_package_exports_snapshot(package),
            skill_ids: skill_ids_by_package
                .get(package.id.as_str())
                .cloned()
                .unwrap_or_default(),
        });
    }

    let writable_config_path = context
        .resolved
        .writable_root_dir
        .as_ref()
        .map(|root| root.join("agent.toml"));

    let mut snapshot = SkillCatalogSnapshot {
        cursor: String::new(),
        workspace_root_dir: context.resolved.workspace_root_dir.clone(),
        workspace_alan_dir: context.resolved.workspace_alan_dir.clone(),
        agent_name: context.resolved.agent_name.clone(),
        writable_root_dir: context.resolved.writable_root_dir.clone(),
        writable_config_path,
        packages: package_snapshots,
        skills,
    };
    snapshot.cursor = compute_skill_catalog_cursor(&snapshot)?;
    Ok(snapshot)
}

pub fn write_package_mount_override(
    config_path: &Path,
    package_id: &str,
    mode: Option<PackageMountMode>,
) -> Result<()> {
    let package_id = package_id.trim();
    if package_id.is_empty() {
        bail!("package_id must not be empty");
    }

    let mut root = load_config_table(config_path)?;
    let mut package_mounts = match root.remove("package_mounts") {
        Some(toml::Value::Array(entries)) => entries
            .into_iter()
            .filter(|entry| !package_mount_entry_matches(entry, package_id))
            .collect::<Vec<_>>(),
        Some(other) => {
            bail!(
                "Invalid package_mounts in {}: expected array, found {}",
                config_path.display(),
                type_name_for_toml_value(&other)
            );
        }
        None => Vec::new(),
    };

    if let Some(mode) = mode {
        package_mounts.push(toml::Value::Table(package_mount_entry(package_id, mode)));
    }

    if !package_mounts.is_empty() {
        root.insert(
            "package_mounts".to_string(),
            toml::Value::Array(package_mounts),
        );
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create config directory for skill mount override: {}",
                parent.display()
            )
        })?;
    }

    if root.is_empty() {
        if config_path.exists() {
            std::fs::remove_file(config_path).with_context(|| {
                format!(
                    "Failed to remove empty agent config after clearing mount override: {}",
                    config_path.display()
                )
            })?;
        }
        return Ok(());
    }

    let rendered = toml::to_string_pretty(&toml::Value::Table(root))
        .context("Failed to encode agent.toml while writing skill mount override")?;
    write_atomically(config_path, &rendered)?;
    Ok(())
}

fn write_atomically(path: &Path, content: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("Config path has no parent directory: {}", path.display()))?;
    let tmp_path = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agent.toml"),
        std::process::id()
    ));
    let mut tmp_file = std::fs::File::create(&tmp_path)
        .with_context(|| format!("Failed to create temp config file: {}", tmp_path.display()))?;
    tmp_file
        .write_all(content.as_bytes())
        .with_context(|| format!("Failed to write temp config file: {}", tmp_path.display()))?;
    if let Ok(metadata) = std::fs::metadata(path) {
        tmp_file
            .set_permissions(metadata.permissions())
            .with_context(|| {
                format!(
                    "Failed to preserve existing permissions on temp config file: {}",
                    tmp_path.display()
                )
            })?;
    }
    tmp_file
        .sync_all()
        .with_context(|| format!("Failed to sync temp config file: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "Failed to atomically replace skill mount config {} -> {}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn build_skill_snapshot(
    skill: &SkillMetadata,
    host_capabilities: &SkillHostCapabilities,
) -> SkillCatalogSkillSnapshot {
    let issues = skill_availability_issues(skill, host_capabilities);
    let remediation = skill_remediation(skill, host_capabilities);
    SkillCatalogSkillSnapshot {
        id: skill.id.clone(),
        package_id: skill.package_id.clone(),
        name: skill.name.clone(),
        description: skill.description.clone(),
        short_description: skill.short_description.clone(),
        path: skill.path.clone(),
        package_root: skill.package_root.clone(),
        resource_root: skill.resource_root.clone(),
        scope: skill.scope,
        mount_mode: skill.mount_mode,
        tags: skill.tags.clone(),
        compatible_metadata: skill.compatible_metadata.clone(),
        execution: skill.execution.clone(),
        available: issues.is_empty(),
        availability_issues: issues.iter().map(ToString::to_string).collect::<Vec<_>>(),
        remediation,
    }
}

fn build_package_exports_snapshot(
    package: &CapabilityPackage,
) -> SkillCatalogPackageExportsSnapshot {
    SkillCatalogPackageExportsSnapshot {
        child_agents: package
            .exports
            .child_agents
            .iter()
            .map(|export| SkillCatalogChildAgentExportSnapshot {
                name: export.name.clone(),
                root_dir: export.root_dir.clone(),
                handle: export.handle.clone(),
            })
            .collect(),
        resources: build_resource_snapshot(&package.exports.resources),
    }
}

fn build_resource_snapshot(
    resources: &CapabilityPackageResources,
) -> SkillCatalogResourceExportsSnapshot {
    SkillCatalogResourceExportsSnapshot {
        scripts_dir: resources.scripts_dir.clone(),
        references_dir: resources.references_dir.clone(),
        assets_dir: resources.assets_dir.clone(),
        viewers_dir: resources.viewers_dir.clone(),
    }
}

fn compute_skill_catalog_cursor(snapshot: &SkillCatalogSnapshot) -> Result<String> {
    let mut cursor_input = snapshot.clone();
    cursor_input.cursor.clear();
    let encoded = serde_json::to_vec(&cursor_input)
        .context("Failed to serialize skill catalog snapshot for cursor computation")?;
    let digest = Sha256::digest(encoded);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn load_config_table(config_path: &Path) -> Result<toml::Table> {
    if !config_path.exists() {
        return Ok(toml::Table::new());
    }

    let raw = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    if raw.trim().is_empty() {
        return Ok(toml::Table::new());
    }
    let value: toml::Value = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;
    match value {
        toml::Value::Table(table) => Ok(table),
        other => bail!(
            "Invalid agent config {}: expected table, found {}",
            config_path.display(),
            type_name_for_toml_value(&other)
        ),
    }
}

fn package_mount_entry(package_id: &str, mode: PackageMountMode) -> toml::Table {
    let mut entry = toml::Table::new();
    entry.insert(
        "package".to_string(),
        toml::Value::String(package_id.to_string()),
    );
    entry.insert(
        "mode".to_string(),
        toml::Value::String(package_mount_mode_label(mode).to_string()),
    );
    entry
}

fn package_mount_entry_matches(entry: &toml::Value, package_id: &str) -> bool {
    entry
        .as_table()
        .and_then(|table| {
            table
                .get("package")
                .or_else(|| table.get("package_id"))
                .and_then(toml::Value::as_str)
        })
        .map(|value| value == package_id)
        .unwrap_or(false)
}

fn package_mount_mode_label(mode: PackageMountMode) -> &'static str {
    match mode {
        PackageMountMode::AlwaysActive => "always_active",
        PackageMountMode::Discoverable => "discoverable",
        PackageMountMode::ExplicitOnly => "explicit_only",
        PackageMountMode::Internal => "internal",
    }
}

fn type_name_for_toml_value(value: &toml::Value) -> &'static str {
    match value {
        toml::Value::String(_) => "string",
        toml::Value::Integer(_) => "integer",
        toml::Value::Float(_) => "float",
        toml::Value::Boolean(_) => "boolean",
        toml::Value::Datetime(_) => "datetime",
        toml::Value::Array(_) => "array",
        toml::Value::Table(_) => "table",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alan_runtime::{AlanHomePaths, Config};
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn create_skill(root: &Path, skill_name: &str, frontmatter: &str) {
        let skill_dir = root.join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), frontmatter).unwrap();
    }

    #[test]
    fn build_skill_catalog_snapshot_includes_packages_skills_and_cursor() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("workspace");
        let workspace_alan_dir = workspace_root.join(".alan");
        let skills_dir = workspace_alan_dir.join("agent/skills");
        let review_dir = skills_dir.join("repo-review");
        fs::create_dir_all(review_dir.join("agents/repo-review")).unwrap();
        fs::create_dir_all(review_dir.join("agents")).unwrap();
        create_skill(
            &skills_dir,
            "repo-review",
            r#"---
name: Repo Review
description: Review the current diff
---

Body
"#,
        );
        fs::write(
            review_dir.join("agents/openai.yaml"),
            r#"
interface:
  display_name: "Repository Review"
  short_description: "Review the current diff"
  icon_small: "./assets/review-small.svg"
"#,
        )
        .unwrap();

        let mut runtime_config = WorkspaceRuntimeConfig::from(Config::default());
        runtime_config.workspace_root_dir = Some(workspace_root);
        runtime_config.workspace_alan_dir = Some(workspace_alan_dir);
        runtime_config.agent_home_paths = Some(AlanHomePaths::from_home_dir(temp.path()));

        let context = resolve_skill_catalog_context(&runtime_config).unwrap();
        let snapshot = build_skill_catalog_snapshot(&context).unwrap();

        assert!(!snapshot.cursor.is_empty());
        assert!(
            snapshot
                .packages
                .iter()
                .any(|package| package.id == "skill:repo-review")
        );
        let skill = snapshot
            .skills
            .iter()
            .find(|skill| skill.id == "repo-review")
            .unwrap();
        assert_eq!(
            skill.execution,
            ResolvedSkillExecution::Delegate {
                target: "repo-review".to_string(),
                source:
                    alan_runtime::skills::SkillExecutionResolutionSource::SameNameSkillAndChildAgent,
            }
        );
        assert_eq!(
            skill.compatible_metadata.interface.display_name.as_deref(),
            Some("Repository Review")
        );
        assert_eq!(
            skill
                .compatible_metadata
                .interface
                .short_description
                .as_deref(),
            Some("Review the current diff")
        );
        let expected_icon_small = std::fs::canonicalize(review_dir.join("assets/review-small.svg"))
            .unwrap_or_else(|_| {
                std::fs::canonicalize(&review_dir)
                    .unwrap()
                    .join("assets/review-small.svg")
            });
        assert_eq!(
            skill.compatible_metadata.interface.icon_small.as_deref(),
            Some(expected_icon_small.as_path())
        );
    }

    #[test]
    fn write_package_mount_override_adds_updates_and_removes_entry() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("agent.toml");

        write_package_mount_override(
            &config_path,
            "skill:repo-review",
            Some(PackageMountMode::AlwaysActive),
        )
        .unwrap();
        let first = fs::read_to_string(&config_path).unwrap();
        assert!(first.contains("package = \"skill:repo-review\""));
        assert!(first.contains("mode = \"always_active\""));

        write_package_mount_override(
            &config_path,
            "skill:repo-review",
            Some(PackageMountMode::ExplicitOnly),
        )
        .unwrap();
        let second = fs::read_to_string(&config_path).unwrap();
        assert!(!second.contains("always_active"));
        assert!(second.contains("mode = \"explicit_only\""));

        write_package_mount_override(&config_path, "skill:repo-review", None).unwrap();
        assert!(!config_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_package_mount_override_preserves_existing_file_permissions() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("agent.toml");
        fs::write(&config_path, "llm_provider = \"openai_responses\"\n").unwrap();
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600)).unwrap();

        write_package_mount_override(
            &config_path,
            "skill:repo-review",
            Some(PackageMountMode::AlwaysActive),
        )
        .unwrap();

        let mode = std::fs::metadata(&config_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
