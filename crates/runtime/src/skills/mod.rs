//! Skills framework for extending agent capabilities.
//!
//! Skills are directory-backed capability packages centered on a single
//! portable `SKILL.md`, with optional Alan-native sidecars and child-agent
//! exports.
//!
//! # Example Skill Structure
//!
//! ```text
//! my-skill/
//! ├── SKILL.md              # Required
//! ├── skill.yaml            # Optional Alan-native runtime metadata
//! ├── package.yaml          # Optional package-level runtime defaults
//! ├── scripts/              # Optional: executable code
//! ├── references/           # Optional: documentation
//! ├── assets/               # Optional: templates, resources
//! ├── evals/                # Optional: explicit authoring/eval manifests
//! ├── eval-viewer/          # Optional: static review/viewer assets
//! └── agents/               # Optional: package-local child-agent exports
//! ```
//!
//! # SKILL.md Format
//!
//! ```markdown
//! ---
//! name: skill-name
//! description: What this skill does and when to use it
//! metadata:
//!   short-description: Brief description
//!   tags: ["tag1", "tag2"]
//! ---
//!
//! # Instructions
//!
//! Step-by-step guidance for the agent...
//! ```
//!
//! Discovery is filesystem-based and deterministic. Runtime skill ids derive
//! from the package directory name, while `SKILL.md` stays canonical for
//! triggers, availability, and instructions. Activation comes from mount
//! defaults, explicit mentions / aliases, and declared keyword / pattern
//! triggers. Delegated skills render lightweight parent-side stubs and execute
//! through package-local child-agent exports when the runtime supports
//! `invoke_delegated_skill`.

mod capability_view;
mod injector;
mod loader;
pub mod registry;
pub mod types;

pub use injector::*;
pub use loader::*;
pub use registry::SkillsRegistry;
pub use types::*;

// ============================================================================
// Built-in package assets
// ============================================================================

use include_dir::{Dir, DirEntry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub(crate) const BUILTIN_MEMORY_PACKAGE_ID: &str = "builtin:alan-memory";
pub(crate) const BUILTIN_PLAN_PACKAGE_ID: &str = "builtin:alan-plan";
pub(crate) const BUILTIN_SHELL_CONTROL_PACKAGE_ID: &str = "builtin:alan-shell-control";
pub(crate) const BUILTIN_SKILL_CREATOR_PACKAGE_ID: &str = "builtin:alan-skill-creator";
pub(crate) const BUILTIN_WORKSPACE_MANAGER_PACKAGE_ID: &str = "builtin:alan-workspace-manager";

static MEMORY_PACKAGE_DIR: Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/skills/memory");
static PLAN_PACKAGE_DIR: Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/skills/plan");
static SHELL_CONTROL_PACKAGE_DIR: Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/skills/alan-shell-control");
static SKILL_CREATOR_PACKAGE_DIR: Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/skills/skill-creator");
static WORKSPACE_MANAGER_PACKAGE_DIR: Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/skills/workspace-manager");

#[derive(Clone, Copy)]
pub(crate) struct BuiltinPackageAsset {
    pub package_id: &'static str,
    pub skill_label: &'static str,
    pub default_mount_mode: Option<PackageMountMode>,
    pub dir: &'static Dir<'static>,
}

#[derive(Debug, Clone)]
pub(crate) struct MaterializedBuiltinPackage {
    pub root_dir: PathBuf,
    pub skill_path: PathBuf,
}

static MATERIALIZED_BUILTIN_PACKAGES: OnceLock<HashMap<&'static str, MaterializedBuiltinPackage>> =
    OnceLock::new();

pub(crate) const BUILTIN_PACKAGE_ASSETS: [BuiltinPackageAsset; 5] = [
    BuiltinPackageAsset {
        package_id: BUILTIN_MEMORY_PACKAGE_ID,
        skill_label: "memory",
        default_mount_mode: Some(PackageMountMode::AlwaysActive),
        dir: &MEMORY_PACKAGE_DIR,
    },
    BuiltinPackageAsset {
        package_id: BUILTIN_PLAN_PACKAGE_ID,
        skill_label: "plan",
        default_mount_mode: Some(PackageMountMode::AlwaysActive),
        dir: &PLAN_PACKAGE_DIR,
    },
    BuiltinPackageAsset {
        package_id: BUILTIN_SHELL_CONTROL_PACKAGE_ID,
        skill_label: "alan-shell-control",
        default_mount_mode: Some(PackageMountMode::AlwaysActive),
        dir: &SHELL_CONTROL_PACKAGE_DIR,
    },
    BuiltinPackageAsset {
        package_id: BUILTIN_SKILL_CREATOR_PACKAGE_ID,
        skill_label: "skill-creator",
        default_mount_mode: Some(PackageMountMode::Discoverable),
        dir: &SKILL_CREATOR_PACKAGE_DIR,
    },
    BuiltinPackageAsset {
        package_id: BUILTIN_WORKSPACE_MANAGER_PACKAGE_ID,
        skill_label: "workspace-manager",
        default_mount_mode: Some(PackageMountMode::AlwaysActive),
        dir: &WORKSPACE_MANAGER_PACKAGE_DIR,
    },
];

pub(crate) fn default_builtin_package_mounts() -> Vec<PackageMount> {
    BUILTIN_PACKAGE_ASSETS
        .iter()
        .filter_map(|asset| {
            asset.default_mount_mode.map(|mode| PackageMount {
                package_id: asset.package_id.to_string(),
                mode,
            })
        })
        .collect()
}

pub(crate) fn merge_package_mounts(
    base_mounts: &[PackageMount],
    overrides: &[PackageMount],
) -> Vec<PackageMount> {
    let mut merged = base_mounts.to_vec();

    for mount in overrides {
        if let Some(existing) = merged
            .iter_mut()
            .find(|existing| existing.package_id == mount.package_id)
        {
            *existing = mount.clone();
        } else {
            merged.push(mount.clone());
        }
    }

    merged
}

pub(crate) fn merge_builtin_package_mounts(package_mounts: &[PackageMount]) -> Vec<PackageMount> {
    merge_package_mounts(&default_builtin_package_mounts(), package_mounts)
}

pub(crate) fn builtin_skill_content(asset: &BuiltinPackageAsset) -> &'static str {
    asset
        .dir
        .get_file("SKILL.md")
        .unwrap_or_else(|| {
            panic!(
                "builtin package `{}` is missing SKILL.md",
                asset.skill_label
            )
        })
        .contents_utf8()
        .unwrap_or_else(|| {
            panic!(
                "builtin package `{}` SKILL.md is not valid UTF-8",
                asset.skill_label
            )
        })
}

pub(crate) fn materialized_builtin_package(
    asset: &BuiltinPackageAsset,
) -> Option<MaterializedBuiltinPackage> {
    MATERIALIZED_BUILTIN_PACKAGES
        .get_or_init(materialize_builtin_packages)
        .get(asset.package_id)
        .cloned()
}

fn materialize_builtin_packages() -> HashMap<&'static str, MaterializedBuiltinPackage> {
    let mut packages = HashMap::new();
    let base_dir = std::env::temp_dir()
        .join("alan")
        .join("builtin-skill-packages")
        .join(env!("CARGO_PKG_VERSION"))
        .join(std::process::id().to_string());

    for asset in BUILTIN_PACKAGE_ASSETS {
        let root_dir = base_dir.join(asset.skill_label);
        match materialize_builtin_package_dir(asset.dir, &root_dir) {
            Ok(()) => {
                let canonical_root =
                    std::fs::canonicalize(&root_dir).unwrap_or_else(|_| root_dir.clone());
                packages.insert(
                    asset.package_id,
                    MaterializedBuiltinPackage {
                        skill_path: canonical_root.join("SKILL.md"),
                        root_dir: canonical_root,
                    },
                );
            }
            Err(err) => {
                tracing::warn!(
                    package_id = asset.package_id,
                    skill_label = asset.skill_label,
                    error = %err,
                    "Failed to materialize builtin skill package; falling back to embedded SKILL.md-only view"
                );
            }
        }
    }

    packages
}

fn materialize_builtin_package_dir(
    dir: &Dir<'static>,
    destination_root: &Path,
) -> std::io::Result<()> {
    match std::fs::remove_dir_all(destination_root) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    std::fs::create_dir_all(destination_root)?;
    write_embedded_dir_entries(dir.path(), dir.entries(), destination_root)
}

fn write_embedded_dir_entries(
    base_path: &Path,
    entries: &[DirEntry<'static>],
    destination_root: &Path,
) -> std::io::Result<()> {
    for entry in entries {
        match entry {
            DirEntry::Dir(dir) => {
                let relative = dir
                    .path()
                    .strip_prefix(base_path)
                    .unwrap_or_else(|_| dir.path());
                let target_dir = destination_root.join(relative);
                std::fs::create_dir_all(&target_dir)?;
                write_embedded_dir_entries(base_path, dir.entries(), destination_root)?;
            }
            DirEntry::File(file) => {
                let relative = file
                    .path()
                    .strip_prefix(base_path)
                    .unwrap_or_else(|_| file.path());
                let target_file = destination_root.join(relative);
                if let Some(parent) = target_file.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target_file, file.contents())?;
                set_builtin_file_permissions(&target_file)?;
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn set_builtin_file_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let in_scripts_dir = path
        .components()
        .any(|component| component.as_os_str() == std::ffi::OsStr::new("scripts"));
    if !in_scripts_dir {
        return Ok(());
    }

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_builtin_file_permissions(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// List all available skills in a user-friendly format.
pub fn list_skills(registry: &SkillsRegistry, host_capabilities: &SkillHostCapabilities) -> String {
    let skills: Vec<_> = registry
        .list_sorted()
        .into_iter()
        .filter(|skill| skill.mount_mode.is_catalog_visible())
        .collect();
    if skills.is_empty() {
        return "No skills found.\n".to_string();
    }

    let mut available = Vec::new();
    let mut unavailable = Vec::new();
    for skill in skills {
        let issues = skill_availability_issues(skill, host_capabilities);
        if issues.is_empty() {
            available.push(skill);
        } else {
            unavailable.push((skill, issues));
        }
    }

    let mut lines = vec![
        "Available Skills".to_string(),
        "================".to_string(),
        String::new(),
    ];

    for skill in available {
        let scope_str = match skill.scope {
            types::SkillScope::Repo => "[repo]",
            types::SkillScope::User => "[user]",
            types::SkillScope::Builtin => "[builtin]",
        };

        lines.push(format!(
            "{} ${} - {}",
            scope_str,
            skill.id,
            skill.display_name()
        ));

        let desc = skill
            .effective_short_description()
            .unwrap_or(&skill.description);
        lines.push(format!("         {}", desc));
        lines.extend(render_skill_execution_lines(skill));
        lines.push(String::new());
    }

    if !unavailable.is_empty() {
        lines.extend([
            "Unavailable Skills".to_string(),
            "==================".to_string(),
            String::new(),
        ]);

        for (skill, issues) in unavailable {
            let scope_str = match skill.scope {
                types::SkillScope::Repo => "[repo]",
                types::SkillScope::User => "[user]",
                types::SkillScope::Builtin => "[builtin]",
            };
            lines.push(format!(
                "{} ${} - {}",
                scope_str,
                skill.id,
                skill.display_name()
            ));
            let desc = skill
                .effective_short_description()
                .unwrap_or(&skill.description);
            lines.push(format!("         {}", desc));
            lines.extend(render_skill_execution_lines(skill));
            lines.push(format!(
                "         unavailable: {}",
                format_skill_availability_issues(&issues)
            ));
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

fn render_skill_execution_lines(skill: &SkillMetadata) -> Vec<String> {
    let mut lines = vec![format!(
        "         execution: {}",
        skill.execution.render_label()
    )];
    if let Some(diagnostic) = render_skill_execution_diagnostic(&skill.execution) {
        lines.push(format!("         diagnostic: {diagnostic}"));
    }
    lines
}

fn render_skill_execution_diagnostic(execution: &ResolvedSkillExecution) -> Option<String> {
    match execution {
        ResolvedSkillExecution::Unresolved { reason } => match reason {
            SkillExecutionUnresolvedReason::NotResolved => None,
            SkillExecutionUnresolvedReason::MissingChildAgentExports => Some(
                "delegated execution was requested but the package exports no child agents"
                    .to_string(),
            ),
            SkillExecutionUnresolvedReason::DelegateTargetNotFound {
                target,
                available_targets,
            } => Some(format!(
                "delegate target '{target}' was not found (available: {})",
                render_csv_or_none(available_targets)
            )),
            SkillExecutionUnresolvedReason::AmbiguousPackageShape {
                skill_id,
                child_agent_exports,
            } => Some(format!(
                "ambiguous package shape; skill={skill_id}; child agents={}",
                render_csv_or_none(child_agent_exports)
            )),
        },
        _ => None,
    }
}

fn render_csv_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_list_skills() {
        let temp = TempDir::new().unwrap();
        let repo_skills = temp.path().join("skills");
        std::fs::create_dir_all(&repo_skills).unwrap();

        // Create a test skill
        let skill_dir = repo_skills.join("test-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(
            file,
            r#"---
name: Test Skill
description: A test skill for testing
metadata:
  short-description: Short desc
---

Body
"#
        )
        .unwrap();

        let registry = SkillsRegistry::load_package_dirs(&[ScopedPackageDir {
            path: repo_skills,
            scope: SkillScope::Repo,
        }])
        .unwrap();
        let output = list_skills(&registry, &SkillHostCapabilities::default());

        assert!(output.contains("Available Skills"));
        assert!(output.contains("test-skill"));
        assert!(output.contains("[repo]"));
        assert!(output.contains("Short desc"));
        assert!(output.contains("execution: inline(no_child_agent_exports)"));
    }

    #[test]
    fn test_list_skills_empty_registry() {
        let registry = SkillsRegistry::default();
        let output = list_skills(&registry, &SkillHostCapabilities::default());
        assert!(output.contains("No skills found"));
    }

    #[test]
    fn test_list_skills_surfaces_delegated_execution_and_diagnostics() {
        let temp = TempDir::new().unwrap();
        let repo_skills = temp.path().join("skills");
        std::fs::create_dir_all(&repo_skills).unwrap();

        let delegated_skill_dir = repo_skills.join("repo-review");
        std::fs::create_dir_all(delegated_skill_dir.join("agents/repo-review")).unwrap();
        std::fs::write(
            delegated_skill_dir.join("agents/repo-review/agent.toml"),
            "openai_responses_model = \"gpt-5.4\"\n",
        )
        .unwrap();
        let mut delegated_skill =
            std::fs::File::create(delegated_skill_dir.join("SKILL.md")).unwrap();
        writeln!(
            delegated_skill,
            r#"---
name: Repo Review
description: Review the repo
---

Body
"#
        )
        .unwrap();

        let ambiguous_skill_dir = repo_skills.join("skill-creator");
        std::fs::create_dir_all(ambiguous_skill_dir.join("agents/creator")).unwrap();
        std::fs::create_dir_all(ambiguous_skill_dir.join("agents/grader")).unwrap();
        std::fs::write(
            ambiguous_skill_dir.join("agents/creator/agent.toml"),
            "openai_responses_model = \"gpt-5.4\"\n",
        )
        .unwrap();
        std::fs::write(
            ambiguous_skill_dir.join("agents/grader/agent.toml"),
            "openai_responses_model = \"gpt-5.4\"\n",
        )
        .unwrap();
        let mut ambiguous_skill =
            std::fs::File::create(ambiguous_skill_dir.join("SKILL.md")).unwrap();
        writeln!(
            ambiguous_skill,
            r#"---
name: Skill Creator
description: Create skills
---

Body
"#
        )
        .unwrap();

        let registry = SkillsRegistry::load_package_dirs(&[ScopedPackageDir {
            path: repo_skills,
            scope: SkillScope::Repo,
        }])
        .unwrap();
        let output = list_skills(&registry, &SkillHostCapabilities::default());

        assert!(output.contains(
            "execution: delegate(target=repo-review, source=same_name_skill_and_child_agent)"
        ));
        assert!(output.contains("execution: unresolved(ambiguous_package_shape)"));
        assert!(output.contains(
            "diagnostic: ambiguous package shape; skill=skill-creator; child agents=creator, grader"
        ));
    }

    #[test]
    fn test_list_skills_hides_explicit_only_skills_from_catalog_output() {
        let mut capability_view =
            ResolvedCapabilityView::from_package_dirs(Vec::new()).with_default_mounts();
        capability_view.apply_mount_overrides(&[PackageMount {
            package_id: "builtin:alan-memory".to_string(),
            mode: PackageMountMode::ExplicitOnly,
        }]);

        let registry = SkillsRegistry::load_capability_view(&capability_view).unwrap();
        let output = list_skills(&registry, &SkillHostCapabilities::default());

        assert!(!output.contains("$memory"));
        assert!(output.contains("$plan"));
    }

    #[test]
    fn test_skill_load_outcome_is_empty() {
        let mut outcome = SkillLoadOutcome::default();
        assert!(outcome.is_empty());

        outcome.skills.push(SkillMetadata {
            id: "test".to_string(),
            package_id: None,
            name: "Test".to_string(),
            description: "Test".to_string(),
            short_description: None,
            path: std::path::PathBuf::from("/test"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::Repo,
            tags: vec![],
            capabilities: None,
            compatibility: Default::default(),
            source: SkillContentSource::File(std::path::PathBuf::from("/test")),
            mount_mode: PackageMountMode::Discoverable,
            alan_metadata: Default::default(),
            compatible_metadata: Default::default(),
            execution: Default::default(),
        });
        assert!(!outcome.is_empty());
    }

    #[test]
    fn test_skill_error_display() {
        let error = SkillError {
            path: std::path::PathBuf::from("/test/skill.md"),
            message: "Test error message".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Test error message"));
        assert!(debug_str.contains("/test/skill.md"));
    }

    #[test]
    fn test_builtin_package_contract_matches_tui_setup_catalog() {
        let setup_catalog = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../clients/tui/src/setup-catalog.ts"),
        )
        .unwrap();

        let always_active_builtin_packages = [
            BUILTIN_MEMORY_PACKAGE_ID,
            BUILTIN_PLAN_PACKAGE_ID,
            BUILTIN_SHELL_CONTROL_PACKAGE_ID,
            BUILTIN_WORKSPACE_MANAGER_PACKAGE_ID,
        ];

        for package_id in always_active_builtin_packages {
            assert!(setup_catalog.contains(&format!("package = \"{package_id}\"")));
        }

        let always_active_count = setup_catalog.matches("mode = \"always_active\"").count();
        assert!(always_active_count >= always_active_builtin_packages.len());
    }
}
