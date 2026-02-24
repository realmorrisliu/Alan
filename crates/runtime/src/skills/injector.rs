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
        let resources = format_skill_resources(&skill.metadata.path);
        sections.push(format!(
            r#"## Skill: {}

{}

{resources}

---"#,
            skill.metadata.name,
            skill.content,
            resources = resources
        ));
    }

    sections.join("\n\n")
}

fn format_skill_resources(skill_path: &Path) -> String {
    let Some(skill_dir) = skill_path.parent() else {
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

    if !resources.scripts.is_empty() {
        let items: Vec<String> = resources
            .scripts
            .iter()
            .filter_map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        lines.push(format!("- scripts: {}", items.join(", ")));
    }

    if !resources.references.is_empty() {
        let items: Vec<String> = resources
            .references
            .iter()
            .filter_map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        lines.push(format!("- references: {}", items.join(", ")));
    }

    if !resources.assets.is_empty() {
        let items: Vec<String> = resources
            .assets
            .iter()
            .filter_map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        lines.push(format!("- assets: {}", items.join(", ")));
    }

    lines.join("\n")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mentions() {
        assert_eq!(
            extract_mentions("Use $test-skill for testing"),
            vec!["test-skill"]
        );

        assert_eq!(
            extract_mentions("Run $my_skill please"),
            vec!["my-skill"]
        );

        // Multiple mentions
        let mentions = extract_mentions("$skill-a and $skill-b");
        assert_eq!(mentions, vec!["skill-a", "skill-b"]);

        // With punctuation at end
        assert_eq!(
            extract_mentions("Use $skill-name."),
            vec!["skill-name"]
        );

        // No mentions
        assert!(extract_mentions("Plain text without mentions").is_empty());
    }

    #[test]
    fn test_inject_skills() {
        let skill = Skill {
            metadata: SkillMetadata {
                id: "test".to_string(),
                name: "Test Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/tmp/test/SKILL.md"),
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
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
                name: "Evaluation".to_string(),
                description: "Eval".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/tmp/eval/SKILL.md"),
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
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
                name: "Skill A".to_string(),
                description: "Does A".to_string(),
                short_description: Some("Short A".to_string()),
                path: std::path::PathBuf::from("/a/SKILL.md"),
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
            },
            SkillMetadata {
                id: "skill-b".to_string(),
                name: "Skill B".to_string(),
                description: "Does B".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/b/SKILL.md"),
                scope: SkillScope::Repo,
                tags: vec![],
                capabilities: None,
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
}
