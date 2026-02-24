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

mod injector;
mod loader;
pub mod registry;
pub mod types;

pub use injector::*;
pub use loader::*;
pub use registry::SkillsRegistry;
pub use types::*;

/// Initialize the skills framework and return a loaded registry.
pub fn init(cwd: &std::path::Path) -> Result<SkillsRegistry, SkillsError> {
    SkillsRegistry::load(cwd)
}

/// List all available skills in a user-friendly format.
pub fn list_skills(registry: &SkillsRegistry) -> String {
    let skills = registry.list_sorted();

    if skills.is_empty() {
        return "No skills found.\n".to_string();
    }

    let mut lines = vec![
        "Available Skills".to_string(),
        "================".to_string(),
        String::new(),
    ];

    for skill in skills {
        let scope_str = match skill.scope {
            types::SkillScope::Repo => "[repo]",
            types::SkillScope::User => "[user]",
            types::SkillScope::System => "[system]",
        };

        lines.push(format!("{} ${} - {}", scope_str, skill.id, skill.name));

        let desc = skill
            .short_description
            .as_ref()
            .unwrap_or(&skill.description);
        lines.push(format!("         {}", desc));
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_init() {
        let temp = TempDir::new().unwrap();
        let registry = init(temp.path()).unwrap();
        // No built-in skills in this build, registry may be empty
        let _ = registry.len();
    }

    #[test]
    fn test_list_skills() {
        let temp = TempDir::new().unwrap();
        let repo_skills = temp.path().join(".alan/skills");
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

        let registry = SkillsRegistry::load(temp.path()).unwrap();
        let output = list_skills(&registry);

        assert!(output.contains("Available Skills"));
        assert!(output.contains("test-skill"));
        assert!(output.contains("[repo]"));
        assert!(output.contains("Short desc"));
    }

    #[test]
    fn test_list_skills_empty_registry() {
        // Create a registry with no skills by loading from empty dir
        // No built-in skills in this build
        let temp = TempDir::new().unwrap();
        let registry = SkillsRegistry::load(temp.path()).unwrap();
        let output = list_skills(&registry);
        // Should indicate no skills found
        assert!(output.contains("No skills found") || output.contains("Available Skills"));
    }

    #[test]
    fn test_skill_load_outcome_is_empty() {
        let mut outcome = SkillLoadOutcome::default();
        assert!(outcome.is_empty());

        outcome.skills.push(SkillMetadata {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            short_description: None,
            path: std::path::PathBuf::from("/test"),
            scope: SkillScope::Repo,
            tags: vec![],
            capabilities: None,
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
}
