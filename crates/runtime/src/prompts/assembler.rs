//! Prompt assembly logic.

use crate::config::Config;
use std::path::Path;

/// Build agent system prompt with proper assembly order:
/// 1. Runtime Base (hard constraints - cannot be overridden)
/// 2. System Prompt (default identity and behavior)
/// 3. Domain Prompt (skills/domain overlays loaded dynamically)
/// 4. Workspace Profile (persona files)
pub fn build_agent_system_prompt(config: &Config, domain_prompt: &str) -> String {
    let workspace_persona_dirs = resolve_workspace_persona_dirs_for_workspace(config, None);
    build_agent_system_prompt_internal(domain_prompt, &workspace_persona_dirs)
}

pub fn build_agent_system_prompt_for_workspace(
    config: &Config,
    domain_prompt: &str,
    workspace_dir: Option<&Path>,
) -> String {
    let workspace_persona_dirs =
        resolve_workspace_persona_dirs_for_workspace(config, workspace_dir);
    build_agent_system_prompt_internal(domain_prompt, &workspace_persona_dirs)
}

#[allow(dead_code)]
pub(crate) fn resolve_workspace_persona_dir_for_workspace(
    config: &Config,
    workspace_dir: Option<&Path>,
) -> Option<std::path::PathBuf> {
    resolve_workspace_persona_dirs_for_workspace(config, workspace_dir)
        .into_iter()
        .last()
}

pub(crate) fn resolve_workspace_persona_dirs_for_workspace(
    config: &Config,
    workspace_dir: Option<&Path>,
) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(global_dir) = global_agent_persona_dir() {
        dirs.push(global_dir);
    }
    if let Some(workspace_dir) = workspace_dir {
        dirs.push(resolve_workspace_agent_persona_dir(workspace_dir));
        return dirs;
    }
    if let Some(config_dir) = resolve_workspace_persona_dir_from_config(config)
        && !dirs.contains(&config_dir)
    {
        dirs.push(config_dir);
    }
    dirs
}

pub(crate) fn build_agent_system_prompt_with_workspace_context(
    domain_prompt: &str,
    workspace_context: Option<&str>,
) -> String {
    let mut prompt = String::new();

    append_prompt_section(&mut prompt, super::RUNTIME_BASE_PROMPT);
    append_prompt_section(&mut prompt, super::SYSTEM_PROMPT);
    append_prompt_section(&mut prompt, domain_prompt);
    if let Some(workspace_context) = workspace_context {
        append_prompt_section(&mut prompt, workspace_context);
    }

    prompt
}

fn build_agent_system_prompt_internal(
    domain_prompt: &str,
    workspace_persona_dirs: &[std::path::PathBuf],
) -> String {
    let workspace_context = if workspace_persona_dirs.iter().any(|path| path.exists()) {
        Some(super::workspace::render_workspace_persona_context_from_dirs(workspace_persona_dirs))
    } else {
        None
    };
    build_agent_system_prompt_with_workspace_context(domain_prompt, workspace_context.as_deref())
}

fn append_prompt_section(prompt: &mut String, section: &str) {
    let trimmed = section.trim();
    if trimmed.is_empty() {
        return;
    }

    if !prompt.is_empty() {
        prompt.push_str("\n\n");
    }
    prompt.push_str(trimmed);
}

fn resolve_workspace_persona_dir_from_config(config: &Config) -> Option<std::path::PathBuf> {
    if let Some(path) = config.memory.workspace_dir.clone() {
        let is_memory_dir = path
            .file_name()
            .map(|name| name == std::ffi::OsStr::new("memory"))
            .unwrap_or(false);
        if is_memory_dir {
            return path
                .parent()
                .map(|parent| parent.join("agent").join("persona"));
        }
        return Some(path);
    }

    if cfg!(test) {
        return None;
    }

    global_agent_persona_dir()
}

fn global_agent_persona_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|home| home.join(".alan").join("agent").join("persona"))
}

fn resolve_workspace_agent_persona_dir(workspace_dir: &Path) -> std::path::PathBuf {
    let is_alan_dir = workspace_dir
        .file_name()
        .map(|name| name == std::ffi::OsStr::new(".alan"))
        .unwrap_or(false);
    if is_alan_dir {
        workspace_dir.join("agent").join("persona")
    } else {
        workspace_dir.join(".alan").join("agent").join("persona")
    }
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

        assert!(prompt.contains("Runtime Base Constraints"));
        assert!(prompt.contains("No Self-Modification"));
        assert!(prompt.contains("Alan System Prompt"));
        assert!(prompt.contains("Domain Prompt"));
    }

    #[test]
    fn test_build_agent_system_prompt_injects_workspace_context() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config_with_workspace(&temp_dir);
        crate::prompts::ensure_workspace_bootstrap_files_at(temp_dir.path()).unwrap();

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
        crate::prompts::ensure_workspace_bootstrap_files_at(temp_dir.path()).unwrap();
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
        crate::prompts::ensure_workspace_bootstrap_files_at(temp_dir.path()).unwrap();
        let large = "a".repeat(10_000);
        fs::write(temp_dir.path().join("ROLE.md"), large).unwrap();

        let workspace_context = crate::prompts::render_workspace_persona_context(temp_dir.path());
        let prompt = build_agent_system_prompt_with_workspace_context(
            "Domain Prompt",
            Some(&workspace_context),
        );

        assert!(prompt.contains("[...truncated, read ROLE.md for full content...]"));
    }

    #[test]
    fn test_prompt_assembly_order() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path()).unwrap();
        crate::prompts::ensure_workspace_bootstrap_files_at(temp_dir.path()).unwrap();
        fs::write(temp_dir.path().join("SOUL.md"), "SOUL content").unwrap();

        let config = test_config_with_workspace(&temp_dir);
        let prompt = build_agent_system_prompt(&config, "DOMAIN content");

        let runtime_base_pos = prompt.find("Runtime Base Constraints").unwrap();
        let system_pos = prompt.find("Alan System Prompt").unwrap();
        let skills_pos = prompt.find("DOMAIN content").unwrap();
        let workspace_pos = prompt.find("Workspace Persona Context").unwrap();

        assert!(runtime_base_pos < system_pos);
        assert!(system_pos < skills_pos);
        assert!(skills_pos < workspace_pos);
    }

    #[test]
    fn test_build_agent_system_prompt_does_not_create_workspace_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config_with_workspace(&temp_dir);

        let prompt = build_agent_system_prompt(&config, "Domain Prompt");

        assert!(!prompt.contains("Workspace Persona Context"));
        assert!(!temp_dir.path().join("SOUL.md").exists());
        assert!(!temp_dir.path().join("ROLE.md").exists());
    }
}
