//! Skills injector for adding skill content to prompts.

use crate::skills::types::*;
use std::path::Path;

/// Extract skill mentions from user input.
/// Supports `$skill-name` or `$skill_name` format.
pub fn extract_mentions(input: &str) -> Vec<SkillId> {
    let mut mentions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '$' {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < chars.len() {
            let c = chars[j];
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                j += 1;
            } else {
                break;
            }
        }

        if j > i + 1 {
            let raw: String = chars[i + 1..j].iter().collect();
            let id = name_to_id(&raw);
            if seen.insert(id.clone()) {
                mentions.push(id);
            }
        }

        i = j;
    }

    mentions
}

/// Inject skill content into a prompt.
pub fn inject_skills(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut sections = Vec::new();

    for skill in skills {
        let envelope = ActiveSkillEnvelope::available(
            skill.metadata.clone(),
            SkillActivationReason::AlwaysActiveMount,
        );
        sections.push(inject_active_skill(skill, &envelope));
    }

    sections.join("\n\n")
}

/// Inject one active skill using the structured runtime envelope.
pub fn inject_active_skill(skill: &Skill, envelope: &ActiveSkillEnvelope) -> String {
    let runtime_context = format_active_skill_context(envelope);
    let resources = format_skill_resources(envelope);
    format!(
        r#"## Skill: {}

{runtime_context}

{}

{resources}

---"#,
        skill.metadata.name,
        skill.content,
        runtime_context = runtime_context,
        resources = resources
    )
}

fn format_active_skill_context(envelope: &ActiveSkillEnvelope) -> String {
    let mut lines = vec![
        "### Alan Runtime Context".to_string(),
        format!("skill_id: {}", envelope.metadata.id),
        format!(
            "package_id: {}",
            envelope.metadata.package_id.as_deref().unwrap_or("<none>")
        ),
        format!(
            "mount_mode: {}",
            render_mount_mode(envelope.metadata.mount_mode)
        ),
        format!("canonical_path: {}", envelope.metadata.path.display()),
        format!(
            "package_root: {}",
            render_optional_path(envelope.metadata.package_root())
        ),
        format!(
            "resource_root: {}",
            render_optional_path(envelope.metadata.resource_root())
        ),
        format!("availability: {}", envelope.availability.render_label()),
        format!(
            "activation_reason: {}",
            envelope.activation_reason.render_label()
        ),
    ];

    if envelope.metadata.resource_root().is_some() {
        lines.push(
            "Resolve relative skill resource references against `resource_root`.".to_string(),
        );
    }

    lines.join("\n")
}

fn format_skill_resources(envelope: &ActiveSkillEnvelope) -> String {
    let Some(skill_dir) = envelope
        .metadata
        .resource_root()
        .or_else(|| envelope.metadata.path.parent())
    else {
        return String::new();
    };

    let resources = load_skill_resources(skill_dir);
    if resources.scripts.is_empty()
        && resources.references.is_empty()
        && resources.assets.is_empty()
    {
        return String::new();
    }

    let mut lines = vec!["### Resources".to_string()];
    lines.push(format!("base: {}", skill_dir.display()));

    if !resources.scripts.is_empty() {
        let items: Vec<String> = resources
            .scripts
            .iter()
            .map(|p| render_relative_resource_path(skill_dir, p))
            .collect();
        lines.push(format!("- scripts: {}", items.join(", ")));
    }

    if !resources.references.is_empty() {
        let items: Vec<String> = resources
            .references
            .iter()
            .map(|p| render_relative_resource_path(skill_dir, p))
            .collect();
        lines.push(format!("- references: {}", items.join(", ")));
    }

    if !resources.assets.is_empty() {
        let items: Vec<String> = resources
            .assets
            .iter()
            .map(|p| render_relative_resource_path(skill_dir, p))
            .collect();
        lines.push(format!("- assets: {}", items.join(", ")));
    }

    lines.join("\n")
}

fn render_optional_path(path: Option<&Path>) -> String {
    path.map(|value| value.display().to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

fn render_mount_mode(mode: PackageMountMode) -> &'static str {
    match mode {
        PackageMountMode::AlwaysActive => "always_active",
        PackageMountMode::Discoverable => "discoverable",
        PackageMountMode::ExplicitOnly => "explicit_only",
        PackageMountMode::Internal => "internal",
    }
}

fn render_relative_resource_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .ok()
        .map(|relative| relative.display().to_string())
        .filter(|relative| !relative.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

/// Build a prompt with injected skills.
pub fn build_prompt_with_skills(user_input: &str, skills: &[Skill]) -> String {
    if skills.is_empty() {
        return user_input.to_string();
    }

    let skill_context = inject_skills(skills);

    format!(
        r#"{skill_context}

## User Request

{user_input}"#
    )
}

/// Render a list of available skills for the system prompt.
/// This helps the LLM know which skills are available for automatic triggering.
pub fn render_skills_list(skills: &[SkillMetadata]) -> Option<String> {
    if skills.is_empty() {
        return None;
    }

    let mut lines = vec![
        "## Available Skills".to_string(),
        "You have access to the following skills. When appropriate, you can use them by referencing their trigger word ($skill-name) or let the user know they are available:".to_string(),
        String::new(),
    ];

    for skill in skills {
        let desc = skill
            .short_description
            .as_ref()
            .unwrap_or(&skill.description);
        lines.push(format!("- **{}** (${}): {}", skill.name, skill.id, desc));
    }

    lines.push(String::new());
    lines.push(
        "When you decide to use a skill, announce it briefly and follow its instructions."
            .to_string(),
    );

    Some(lines.join("\n"))
}

/// Render a skill not found message.
pub fn render_skill_not_found(mention: &str, available: &[SkillMetadata]) -> String {
    let mut msg = format!("Skill '${}' not found. ", mention);

    // Suggest similar skills
    let similar: Vec<_> = available
        .iter()
        .filter(|s| s.id.contains(mention) || mention.contains(&s.id))
        .take(3)
        .collect();

    if !similar.is_empty() {
        msg.push_str("Did you mean: ");
        let names: Vec<_> = similar.iter().map(|s| format!("${}", s.id)).collect();
        msg.push_str(&names.join(", "));
        msg.push('?');
    } else {
        msg.push_str("Use `/skills` to see available skills.");
    }

    msg
}

/// Render a skill unavailable message with concrete host/runtime requirements.
pub fn render_skill_unavailable(mention: &str, reasons: &str) -> String {
    format!("Skill '${mention}' is unavailable in this runtime: {reasons}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mentions() {
        assert_eq!(
            extract_mentions("Use $test-skill for testing"),
            vec!["test-skill"]
        );

        assert_eq!(extract_mentions("Run $my_skill please"), vec!["my-skill"]);

        // Multiple mentions
        let mentions = extract_mentions("$skill-a and $skill-b");
        assert_eq!(mentions, vec!["skill-a", "skill-b"]);

        // With punctuation at end
        assert_eq!(extract_mentions("Use $skill-name."), vec!["skill-name"]);

        // No mentions
        assert!(extract_mentions("Plain text without mentions").is_empty());
    }

    #[test]
    fn test_inject_skills() {
        let skill = Skill {
            metadata: SkillMetadata {
                id: "test".to_string(),
                package_id: None,
                name: "Test Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/tmp/test/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/tmp/test/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
            content: "# Instructions\n\nDo this and that.".to_string(),
            frontmatter: SkillFrontmatter {
                name: "Test Skill".to_string(),
                description: "A test".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let injected = inject_skills(&[skill]);
        assert!(injected.contains("## Skill: Test Skill"));
        assert!(injected.contains("# Instructions"));
    }

    #[test]
    fn test_build_prompt_with_skills() {
        let skill = Skill {
            metadata: SkillMetadata {
                id: "eval".to_string(),
                package_id: None,
                name: "Evaluation".to_string(),
                description: "Eval".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/tmp/eval/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/tmp/eval/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
            content: "Follow these steps.".to_string(),
            frontmatter: SkillFrontmatter {
                name: "Evaluation".to_string(),
                description: "Eval".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let prompt = build_prompt_with_skills("Evaluate this", &[skill]);
        assert!(prompt.contains("Follow these steps"));
        assert!(prompt.contains("Evaluate this"));
    }

    #[test]
    fn test_render_skills_list() {
        let skills = vec![
            SkillMetadata {
                id: "skill-a".to_string(),
                package_id: None,
                name: "Skill A".to_string(),
                description: "Does A".to_string(),
                short_description: Some("Short A".to_string()),
                path: std::path::PathBuf::from("/a/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/a/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
            SkillMetadata {
                id: "skill-b".to_string(),
                package_id: None,
                name: "Skill B".to_string(),
                description: "Does B".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/b/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::Repo,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/b/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
        ];

        let list = render_skills_list(&skills).unwrap();
        assert!(list.contains("**Skill A** ($skill-a): Short A"));
        assert!(list.contains("**Skill B** ($skill-b): Does B"));
        assert!(list.contains("Available Skills"));
    }

    #[test]
    fn test_extract_mentions_edge_cases() {
        // Empty input
        assert!(extract_mentions("").is_empty());

        // Only $ sign
        assert!(extract_mentions("$").is_empty());

        // $ at end
        assert!(extract_mentions("text $").is_empty());

        // Duplicate mentions (should dedupe)
        assert_eq!(extract_mentions("$skill-a and $skill-a"), vec!["skill-a"]);

        // Underscore separator (converted to hyphen)
        assert_eq!(extract_mentions("$skill_name"), vec!["skill-name"]);
    }

    #[test]
    fn test_extract_mentions_multiple_same_and_different() {
        // Multiple skills with duplicates in various positions
        let mentions = extract_mentions("$skill-a $skill-b $skill-a $skill-c $skill-b");
        assert_eq!(mentions, vec!["skill-a", "skill-b", "skill-c"]);
    }

    #[test]
    fn test_extract_mentions_with_numbers() {
        assert_eq!(extract_mentions("Use $skill-123"), vec!["skill-123"]);
        assert_eq!(extract_mentions("$test-v2.0"), vec!["test-v2"]);
    }

    #[test]
    fn test_inject_skills_empty() {
        let result = inject_skills(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_inject_skills_with_resources() {
        let temp = tempfile::tempdir().unwrap();
        let skill_dir = temp.path().join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::create_dir(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir(skill_dir.join("references")).unwrap();
        std::fs::create_dir(skill_dir.join("assets")).unwrap();

        // Create resource files
        std::fs::write(skill_dir.join("scripts/test.sh"), "#!/bin/bash").unwrap();
        std::fs::write(skill_dir.join("references/ref.md"), "# Reference").unwrap();
        std::fs::write(skill_dir.join("assets/logo.png"), "fake png").unwrap();

        let skill = Skill {
            metadata: SkillMetadata {
                id: "test-res".to_string(),
                package_id: None,
                name: "Test Resource Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: skill_dir.join("SKILL.md"),
                package_root: Some(skill_dir.clone()),
                resource_root: Some(skill_dir.clone()),
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(skill_dir.join("SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
            content: "Instructions".to_string(),
            frontmatter: SkillFrontmatter {
                name: "Test Resource Skill".to_string(),
                description: "A test".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let injected = inject_skills(&[skill]);
        assert!(injected.contains("## Skill: Test Resource Skill"));
        assert!(injected.contains("### Alan Runtime Context"));
        assert!(injected.contains("### Resources"));
        assert!(injected.contains("scripts: scripts/test.sh"));
        assert!(injected.contains("references: references/ref.md"));
        assert!(injected.contains("assets: assets/logo.png"));
    }

    #[test]
    fn test_inject_skills_no_parent_path() {
        // Test the edge case where skill path has no parent
        let skill = Skill {
            metadata: SkillMetadata {
                id: "no-parent".to_string(),
                package_id: None,
                name: "No Parent".to_string(),
                description: "Test".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("SKILL.md"), // No parent
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
            content: "Content".to_string(),
            frontmatter: SkillFrontmatter {
                name: "No Parent".to_string(),
                description: "Test".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let injected = inject_skills(&[skill]);
        assert!(injected.contains("## Skill: No Parent"));
        // Should not panic and should not have Resources section
        assert!(!injected.contains("### Resources"));
    }

    #[test]
    fn test_build_prompt_with_skills_empty() {
        let prompt = build_prompt_with_skills("Just user input", &[]);
        assert_eq!(prompt, "Just user input");
    }

    #[test]
    fn test_render_skills_list_empty() {
        assert!(render_skills_list(&[]).is_none());
    }

    #[test]
    fn test_render_skill_not_found_with_similar() {
        let available = vec![
            SkillMetadata {
                id: "test-skill".to_string(),
                package_id: None,
                name: "Test Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/test/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/test/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
            SkillMetadata {
                id: "testing".to_string(),
                package_id: None,
                name: "Testing".to_string(),
                description: "Testing skill".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/testing/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/testing/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
            },
        ];

        let msg = render_skill_not_found("test", &available);
        assert!(msg.contains("Skill '$test' not found"));
        assert!(msg.contains("Did you mean:"));
        assert!(msg.contains("$test-skill"));
    }

    #[test]
    fn test_render_skill_not_found_no_similar() {
        let available = vec![SkillMetadata {
            id: "other".to_string(),
            package_id: None,
            name: "Other".to_string(),
            description: "Other skill".to_string(),
            short_description: None,
            path: std::path::PathBuf::from("/other/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::User,
            tags: vec![],
            capabilities: None,
            compatibility: Default::default(),
            source: SkillContentSource::File(std::path::PathBuf::from("/other/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
        }];

        let msg = render_skill_not_found("xyz", &available);
        assert!(msg.contains("Skill '$xyz' not found"));
        assert!(msg.contains("Use `/skills` to see available skills"));
        assert!(!msg.contains("Did you mean:"));
    }

    #[test]
    fn test_render_skill_not_found_partial_match() {
        // Test when the mention contains the skill id
        let available = vec![SkillMetadata {
            id: "rust".to_string(),
            package_id: None,
            name: "Rust".to_string(),
            description: "Rust skill".to_string(),
            short_description: None,
            path: std::path::PathBuf::from("/rust/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::User,
            tags: vec![],
            capabilities: None,
            compatibility: Default::default(),
            source: SkillContentSource::File(std::path::PathBuf::from("/rust/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
        }];

        // "rustacean" contains "rust"
        let msg = render_skill_not_found("rustacean", &available);
        assert!(msg.contains("Did you mean:"));
        assert!(msg.contains("$rust"));
    }
}
