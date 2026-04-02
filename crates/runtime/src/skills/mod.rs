//! Skills framework for extending agent capabilities.
//!
//! Skills are modular, reusable capabilities defined in Markdown files.
//! Each skill consists of:
//! - SKILL.md with YAML frontmatter (metadata) and instructions (body)
//! - Optional scripts/, references/, assets/ directories
//!
//! # Example Skill Structure
//!
//! ```text
//! my-skill/
//! ├── SKILL.md              # Required
//! ├── scripts/              # Optional: executable code
//! ├── references/           # Optional: documentation
//! └── assets/               # Optional: templates, resources
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
//! # Usage
//!
//! Skills can be triggered:
//! 1. Explicitly: `$skill-name` in user input
//! 2. Implicitly: LLM selects based on description matching

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
// Built-in portable skill assets
// ============================================================================

pub(crate) const BUILTIN_MEMORY_PACKAGE_ID: &str = "builtin:alan-memory";
pub(crate) const BUILTIN_PLAN_PACKAGE_ID: &str = "builtin:alan-plan";
pub(crate) const BUILTIN_SHELL_CONTROL_PACKAGE_ID: &str = "builtin:alan-shell-control";
pub(crate) const BUILTIN_WORKSPACE_MANAGER_PACKAGE_ID: &str = "builtin:alan-workspace-manager";

/// Built-in portable skill: persistent memory across sessions
pub(crate) const MEMORY_SKILL_MD: &str = include_str!("../../skills/memory/SKILL.md");

/// Built-in portable skill: structured execution plans for complex tasks
pub(crate) const PLAN_SKILL_MD: &str = include_str!("../../skills/plan/SKILL.md");

/// Built-in portable skill: shell control for the native Alan terminal app
pub(crate) const SHELL_CONTROL_SKILL_MD: &str =
    include_str!("../../skills/alan-shell-control/SKILL.md");

/// Built-in portable skill: workspace management via alan CLI
pub(crate) const WORKSPACE_MANAGER_SKILL_MD: &str =
    include_str!("../../skills/workspace-manager/SKILL.md");

#[derive(Clone, Copy)]
pub(crate) struct BuiltinPackageAsset {
    pub package_id: &'static str,
    pub skill_label: &'static str,
    pub content: &'static str,
}

pub(crate) const BUILTIN_PACKAGE_ASSETS: [BuiltinPackageAsset; 4] = [
    BuiltinPackageAsset {
        package_id: BUILTIN_MEMORY_PACKAGE_ID,
        skill_label: "memory",
        content: MEMORY_SKILL_MD,
    },
    BuiltinPackageAsset {
        package_id: BUILTIN_PLAN_PACKAGE_ID,
        skill_label: "plan",
        content: PLAN_SKILL_MD,
    },
    BuiltinPackageAsset {
        package_id: BUILTIN_SHELL_CONTROL_PACKAGE_ID,
        skill_label: "alan-shell-control",
        content: SHELL_CONTROL_SKILL_MD,
    },
    BuiltinPackageAsset {
        package_id: BUILTIN_WORKSPACE_MANAGER_PACKAGE_ID,
        skill_label: "workspace-manager",
        content: WORKSPACE_MANAGER_SKILL_MD,
    },
];

pub(crate) fn default_builtin_package_mounts() -> Vec<PackageMount> {
    vec![
        PackageMount {
            package_id: BUILTIN_MEMORY_PACKAGE_ID.to_string(),
            mode: PackageMountMode::AlwaysActive,
        },
        PackageMount {
            package_id: BUILTIN_PLAN_PACKAGE_ID.to_string(),
            mode: PackageMountMode::AlwaysActive,
        },
        PackageMount {
            package_id: BUILTIN_SHELL_CONTROL_PACKAGE_ID.to_string(),
            mode: PackageMountMode::Discoverable,
        },
        PackageMount {
            package_id: BUILTIN_WORKSPACE_MANAGER_PACKAGE_ID.to_string(),
            mode: PackageMountMode::AlwaysActive,
        },
    ]
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

/// List all available skills in a user-friendly format.
pub fn list_skills(registry: &SkillsRegistry, host_capabilities: &SkillHostCapabilities) -> String {
    let skills = registry.list_sorted();
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

        lines.push(format!("{} ${} - {}", scope_str, skill.id, skill.name));

        let desc = skill
            .short_description
            .as_ref()
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
            lines.push(format!("{} ${} - {}", scope_str, skill.id, skill.name));
            let desc = skill
                .short_description
                .as_ref()
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
                package_skill_ids,
                child_agent_exports,
            } => Some(format!(
                "ambiguous package shape; package skills={}; child agents={}",
                render_csv_or_none(package_skill_ids),
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
            "diagnostic: ambiguous package shape; package skills=skill-creator; child agents=creator, grader"
        ));
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

        for package_id in [
            BUILTIN_MEMORY_PACKAGE_ID,
            BUILTIN_PLAN_PACKAGE_ID,
            BUILTIN_WORKSPACE_MANAGER_PACKAGE_ID,
        ] {
            assert!(setup_catalog.contains(&format!("package = \"{package_id}\"")));
        }

        let always_active_count = setup_catalog.matches("mode = \"always_active\"").count();
        assert!(always_active_count >= BUILTIN_PACKAGE_ASSETS.len());
    }
}
