//! Prompt assembly logic.

use crate::config::Config;
use std::path::Path;
use tracing::warn;

use super::workspace::{ensure_workspace_bootstrap_files, load_workspace_bootstrap_files};

const DEFAULT_BOOTSTRAP_MAX_CHARS: usize = 6_000;
const BOOTSTRAP_HEAD_RATIO: f32 = 0.7;
const BOOTSTRAP_TAIL_RATIO: f32 = 0.2;

/// Build agent system prompt with proper assembly order:
/// 1. Runtime Base (hard constraints - cannot be overridden)
/// 2. Skills (modular capabilities loaded dynamically)
/// 3. Workspace Profile (persona files)
pub fn build_agent_system_prompt(config: &Config, domain_prompt: &str) -> String {
    build_agent_system_prompt_with_limit(config, domain_prompt, DEFAULT_BOOTSTRAP_MAX_CHARS, None)
}

pub fn build_agent_system_prompt_for_workspace(
    config: &Config,
    domain_prompt: &str,
    workspace_dir: Option<&Path>,
) -> String {
    build_agent_system_prompt_with_limit(
        config,
        domain_prompt,
        DEFAULT_BOOTSTRAP_MAX_CHARS,
        workspace_dir,
    )
}

fn build_agent_system_prompt_with_limit(
    _config: &Config,
    domain_prompt: &str,
    max_chars: usize,
    workspace_override: Option<&Path>,
) -> String {
    // Step 1: Runtime Base (hard constraints - always first, cannot be overridden)
    let mut prompt = super::RUNTIME_BASE_PROMPT.to_string();

    // Step 2: Skills (modular capabilities)
    // Skills are loaded dynamically based on context and user request
    // They provide specialized instructions for specific domains or tasks
    prompt.push_str("\n\n");
    prompt.push_str(domain_prompt.trim_end());

    // Step 3: Workspace Profile (persona files)
    let workspace_dir = if let Some(path) = workspace_override {
        if !path.exists() {
            return prompt;
        }
        path.to_path_buf()
    } else {
        match ensure_workspace_bootstrap_files(_config) {
            Ok(Some(path)) => path,
            Ok(None) => return prompt,
            Err(err) => {
                warn!(?err, "Failed to ensure workspace bootstrap files");
                return prompt;
            }
        }
    };

    let files = load_workspace_bootstrap_files(&workspace_dir);
    if files.is_empty() {
        return prompt;
    }

    prompt.push_str("\n\n## Workspace Persona Context\n");
    prompt.push_str(&format!("Workspace: {}\n", workspace_dir.display()));
    prompt.push_str(
        "The following workspace files define the persona, role, and operating style.\n",
    );

    for file in files {
        prompt.push_str(&format!("\n### {}\n", file.name));
        if file.missing {
            prompt.push_str(&format!("[MISSING] Expected at: {}\n", file.path.display()));
            continue;
        }
        let content = file.content.unwrap_or_default();
        let trimmed = trim_workspace_content(&content, file.name, max_chars);
        if trimmed.is_empty() {
            prompt.push_str("[EMPTY]\n");
        } else {
            prompt.push_str(trimmed.as_str());
            prompt.push('\n');
        }
    }

    prompt
}

fn trim_workspace_content(content: &str, file_name: &str, max_chars: usize) -> String {
    let trimmed = content.trim_end();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let head_chars = ((max_chars as f32) * BOOTSTRAP_HEAD_RATIO).floor() as usize;
    let tail_chars = ((max_chars as f32) * BOOTSTRAP_TAIL_RATIO).floor() as usize;

    let head = take_chars(trimmed, head_chars);
    let tail = take_last_chars(trimmed, tail_chars);
    let marker = format!(
        "\n[...truncated, read {} for full content...]\n...(truncated {}: kept {}+{} chars)...\n",
        file_name, file_name, head_chars, tail_chars
    );

    format!("{}{}{}", head, marker, tail)
}

fn take_chars(input: &str, count: usize) -> String {
    input.chars().take(count).collect()
}

fn take_last_chars(input: &str, count: usize) -> String {
    let chars: Vec<char> = input.chars().collect();
    let start = chars.len().saturating_sub(count);
    chars[start..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use std::fs;
    use tempfile::TempDir;

    fn test_config_with_workspace(temp_dir: &TempDir) -> Config {
        let mut config = Config::default();
        config.memory.workspace_dir = Some(temp_dir.path().to_path_buf());
        config.memory.strict_workspace = false;
        config
    }

    #[test]
    fn test_build_agent_system_prompt_includes_runtime_base() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config_with_workspace(&temp_dir);

        let prompt = build_agent_system_prompt(&config, "Domain Prompt");

        // Should include runtime base
        assert!(prompt.contains("Runtime Base Constraints"));
        assert!(prompt.contains("No Self-Modification"));
        // Should include domain
        assert!(prompt.contains("Domain Prompt")); // Skills content
    }

    #[test]
    fn test_build_agent_system_prompt_injects_workspace_context() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config_with_workspace(&temp_dir);

        let prompt = build_agent_system_prompt(&config, "Domain Prompt");

        assert!(prompt.contains("Workspace Persona Context"));
        assert!(prompt.contains("### SOUL.md"));
        assert!(temp_dir.path().join("SOUL.md").exists());
        assert!(temp_dir.path().join("ROLE.md").exists());
    }

    #[test]
    fn test_build_agent_system_prompt_uses_existing_workspace_file_content() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path()).unwrap();
        fs::write(
            temp_dir.path().join("SOUL.md"),
            "custom persona instructions",
        )
        .unwrap();

        let config = test_config_with_workspace(&temp_dir);
        let prompt = build_agent_system_prompt(&config, "Domain Prompt");

        assert!(prompt.contains("custom persona instructions"));
    }

    #[test]
    fn test_build_agent_system_prompt_truncates_large_workspace_content() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config_with_workspace(&temp_dir);
        fs::create_dir_all(temp_dir.path()).unwrap();
        let large = "a".repeat(2000);
        fs::write(temp_dir.path().join("ROLE.md"), large).unwrap();

        let prompt = build_agent_system_prompt_with_limit(&config, "Domain Prompt", 120, None);

        assert!(prompt.contains("[...truncated, read ROLE.md for full content...]"));
    }

    #[test]
    fn test_prompt_assembly_order() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path()).unwrap();
        fs::write(temp_dir.path().join("SOUL.md"), "SOUL content").unwrap();

        let config = test_config_with_workspace(&temp_dir);
        let prompt = build_agent_system_prompt(&config, "DOMAIN content");

        // Runtime base should come first
        let runtime_base_pos = prompt.find("Runtime Base Constraints").unwrap();
        let skills_pos = prompt.find("DOMAIN content").unwrap();
        let workspace_pos = prompt.find("Workspace Persona Context").unwrap();

        assert!(
            runtime_base_pos < skills_pos,
            "Runtime base should come before skills"
        );
        assert!(
            skills_pos < workspace_pos,
            "Skills should come before workspace"
        );
    }
}
