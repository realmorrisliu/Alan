//! Workspace bootstrap file management for prompt assembly.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::debug;

pub const DEFAULT_AGENTS_FILENAME: &str = "AGENTS.md";
pub const DEFAULT_SOUL_FILENAME: &str = "SOUL.md";
pub const DEFAULT_ROLE_FILENAME: &str = "ROLE.md";
pub const DEFAULT_USER_FILENAME: &str = "USER.md";
pub const DEFAULT_TOOLS_FILENAME: &str = "TOOLS.md";
pub const DEFAULT_HEARTBEAT_FILENAME: &str = "HEARTBEAT.md";
pub const DEFAULT_BOOTSTRAP_FILENAME: &str = "BOOTSTRAP.md";
pub(crate) const WORKSPACE_PERSONA_MAX_CHARS: usize = 6_000;

const BOOTSTRAP_HEAD_RATIO: f32 = 0.7;
const BOOTSTRAP_TAIL_RATIO: f32 = 0.2;

#[derive(Debug, Clone)]
struct WorkspaceTemplate {
    name: &'static str,
    content: &'static str,
}

const REQUIRED_WORKSPACE_TEMPLATES: [WorkspaceTemplate; 6] = [
    WorkspaceTemplate {
        name: DEFAULT_AGENTS_FILENAME,
        content: include_str!("../../prompts/persona/AGENTS.md"),
    },
    WorkspaceTemplate {
        name: DEFAULT_SOUL_FILENAME,
        content: include_str!("../../prompts/persona/SOUL.md"),
    },
    WorkspaceTemplate {
        name: DEFAULT_ROLE_FILENAME,
        content: include_str!("../../prompts/persona/ROLE.md"),
    },
    WorkspaceTemplate {
        name: DEFAULT_USER_FILENAME,
        content: include_str!("../../prompts/persona/USER.md"),
    },
    WorkspaceTemplate {
        name: DEFAULT_TOOLS_FILENAME,
        content: include_str!("../../prompts/persona/TOOLS.md"),
    },
    WorkspaceTemplate {
        name: DEFAULT_HEARTBEAT_FILENAME,
        content: include_str!("../../prompts/persona/HEARTBEAT.md"),
    },
];

const OPTIONAL_BOOTSTRAP_TEMPLATE: WorkspaceTemplate = WorkspaceTemplate {
    name: DEFAULT_BOOTSTRAP_FILENAME,
    content: include_str!("../../prompts/persona/BOOTSTRAP.md"),
};

#[derive(Debug, Clone)]
pub struct WorkspaceBootstrapFile {
    pub name: &'static str,
    pub path: PathBuf,
    pub content: Option<String>,
    pub missing: bool,
}

pub fn ensure_workspace_bootstrap_files_at(workspace_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(workspace_dir)?;

    let required_paths: Vec<PathBuf> = REQUIRED_WORKSPACE_TEMPLATES
        .iter()
        .map(|template| workspace_dir.join(template.name))
        .collect();
    let is_brand_new_workspace = required_paths.iter().all(|path| !path.exists());

    for template in REQUIRED_WORKSPACE_TEMPLATES {
        let path = workspace_dir.join(template.name);
        write_file_if_missing(&path, template.content)?;
    }

    if is_brand_new_workspace {
        let bootstrap_path = workspace_dir.join(OPTIONAL_BOOTSTRAP_TEMPLATE.name);
        write_file_if_missing(&bootstrap_path, OPTIONAL_BOOTSTRAP_TEMPLATE.content)?;
    }

    Ok(())
}

#[allow(dead_code)]
pub fn load_workspace_bootstrap_files(workspace_dir: &Path) -> Vec<WorkspaceBootstrapFile> {
    load_workspace_bootstrap_files_from_dirs(&[workspace_dir.to_path_buf()])
}

pub fn load_workspace_bootstrap_files_from_dirs(
    workspace_dirs: &[PathBuf],
) -> Vec<WorkspaceBootstrapFile> {
    let mut files = Vec::new();
    for template in REQUIRED_WORKSPACE_TEMPLATES {
        files.push(read_workspace_file_from_dirs(
            workspace_dirs,
            template.name,
            /* optional */ false,
        ));
    }

    if overlay_workspace_file_path(workspace_dirs, OPTIONAL_BOOTSTRAP_TEMPLATE.name).is_some() {
        files.push(read_workspace_file_from_dirs(
            workspace_dirs,
            OPTIONAL_BOOTSTRAP_TEMPLATE.name,
            /* optional */ true,
        ));
    }

    files
}

#[allow(dead_code)]
pub(crate) fn workspace_persona_tracked_paths(workspace_dir: &Path) -> Vec<PathBuf> {
    workspace_persona_tracked_paths_from_dirs(&[workspace_dir.to_path_buf()])
}

pub(crate) fn workspace_persona_tracked_paths_from_dirs(
    workspace_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let mut tracked = Vec::new();
    for workspace_dir in workspace_dirs {
        tracked.push(workspace_dir.join(DEFAULT_AGENTS_FILENAME));
        tracked.push(workspace_dir.join(DEFAULT_SOUL_FILENAME));
        tracked.push(workspace_dir.join(DEFAULT_ROLE_FILENAME));
        tracked.push(workspace_dir.join(DEFAULT_USER_FILENAME));
        tracked.push(workspace_dir.join(DEFAULT_TOOLS_FILENAME));
        tracked.push(workspace_dir.join(DEFAULT_HEARTBEAT_FILENAME));
        tracked.push(workspace_dir.join(DEFAULT_BOOTSTRAP_FILENAME));
    }
    tracked.sort();
    tracked.dedup();
    tracked
}

#[allow(dead_code)]
pub(crate) fn render_workspace_persona_context(workspace_dir: &Path) -> String {
    render_workspace_persona_context_from_dirs(&[workspace_dir.to_path_buf()])
}

pub(crate) fn render_workspace_persona_context_from_dirs(workspace_dirs: &[PathBuf]) -> String {
    let files = load_workspace_bootstrap_files_from_dirs(workspace_dirs);
    if files.is_empty() || files.iter().all(|file| file.missing) {
        return String::new();
    }
    let workspace_label = workspace_dirs
        .last()
        .map(|dir| dir.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    let mut prompt = String::new();
    prompt.push_str("## Workspace Persona Context\n");
    prompt.push_str(&format!("Workspace: {workspace_label}\n"));
    prompt
        .push_str("The following workspace files define the persona, role, and operating style.\n");

    for file in files {
        prompt.push_str(&format!("\n### {}\n", file.name));
        if file.missing {
            prompt.push_str(&format!("[MISSING] Expected at: {}\n", file.path.display()));
            continue;
        }
        let content = file.content.unwrap_or_default();
        let trimmed = trim_workspace_content(&content, file.name, WORKSPACE_PERSONA_MAX_CHARS);
        if trimmed.is_empty() {
            prompt.push_str("[EMPTY]\n");
        } else {
            prompt.push_str(trimmed.as_str());
            prompt.push('\n');
        }
    }

    prompt
}

fn write_file_if_missing(path: &Path, content: &str) -> io::Result<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(content.as_bytes())?;
            debug!(path = %path.display(), "Created workspace bootstrap file");
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
    }
}

fn read_workspace_file_from_dirs(
    workspace_dirs: &[PathBuf],
    name: &'static str,
    optional: bool,
) -> WorkspaceBootstrapFile {
    let expected_path = workspace_dirs
        .last()
        .map(|dir| dir.join(name))
        .unwrap_or_else(|| PathBuf::from(name));

    if let Some(path) = overlay_workspace_file_path(workspace_dirs, name) {
        return match fs::read_to_string(&path) {
            Ok(content) => WorkspaceBootstrapFile {
                name,
                path,
                content: Some(content),
                missing: false,
            },
            Err(err) => WorkspaceBootstrapFile {
                name,
                path,
                content: Some(format!("[ERROR] Failed to read file: {}", err)),
                missing: false,
            },
        };
    }

    if optional {
        WorkspaceBootstrapFile {
            name,
            path: expected_path,
            content: None,
            missing: true,
        }
    } else {
        WorkspaceBootstrapFile {
            name,
            path: expected_path,
            content: None,
            missing: true,
        }
    }
}

fn overlay_workspace_file_path(workspace_dirs: &[PathBuf], name: &'static str) -> Option<PathBuf> {
    workspace_dirs
        .iter()
        .rev()
        .map(|dir| dir.join(name))
        .find(|path| path.exists())
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
    use tempfile::TempDir;

    #[test]
    fn test_ensure_workspace_bootstrap_files_creates_required_templates() {
        let temp_dir = TempDir::new().unwrap();
        ensure_workspace_bootstrap_files_at(temp_dir.path()).unwrap();

        for name in [
            DEFAULT_AGENTS_FILENAME,
            DEFAULT_SOUL_FILENAME,
            DEFAULT_ROLE_FILENAME,
            DEFAULT_USER_FILENAME,
            DEFAULT_TOOLS_FILENAME,
            DEFAULT_HEARTBEAT_FILENAME,
            DEFAULT_BOOTSTRAP_FILENAME,
        ] {
            assert!(
                temp_dir.path().join(name).exists(),
                "expected {} to be created",
                name
            );
        }
    }

    #[test]
    fn test_ensure_workspace_bootstrap_files_does_not_create_bootstrap_when_not_brand_new() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join(DEFAULT_AGENTS_FILENAME),
            "# Existing workspace",
        )
        .unwrap();

        ensure_workspace_bootstrap_files_at(temp_dir.path()).unwrap();

        assert!(!temp_dir.path().join(DEFAULT_BOOTSTRAP_FILENAME).exists());
        assert!(temp_dir.path().join(DEFAULT_SOUL_FILENAME).exists());
        assert!(temp_dir.path().join(DEFAULT_ROLE_FILENAME).exists());
    }

    #[test]
    fn test_ensure_workspace_bootstrap_files_preserves_existing_content() {
        let temp_dir = TempDir::new().unwrap();
        let soul_path = temp_dir.path().join(DEFAULT_SOUL_FILENAME);
        fs::write(&soul_path, "custom soul").unwrap();

        ensure_workspace_bootstrap_files_at(temp_dir.path()).unwrap();

        let content = fs::read_to_string(soul_path).unwrap();
        assert_eq!(content, "custom soul");
    }
}
