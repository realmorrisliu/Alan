//! Workspace bootstrap file management for prompt assembly.

use crate::config::Config;
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

pub fn ensure_workspace_bootstrap_files(config: &Config) -> io::Result<Option<PathBuf>> {
    let Some(workspace_persona_dir) = resolve_workspace_persona_dir(config) else {
        return Ok(None);
    };
    ensure_workspace_bootstrap_files_at(&workspace_persona_dir)?;
    Ok(Some(workspace_persona_dir))
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

pub fn load_workspace_bootstrap_files(workspace_dir: &Path) -> Vec<WorkspaceBootstrapFile> {
    let mut files = Vec::new();
    for template in REQUIRED_WORKSPACE_TEMPLATES {
        files.push(read_workspace_file(
            workspace_dir,
            template.name,
            /* optional */ false,
        ));
    }

    let optional_bootstrap_path = workspace_dir.join(OPTIONAL_BOOTSTRAP_TEMPLATE.name);
    if optional_bootstrap_path.exists() {
        files.push(read_workspace_file(
            workspace_dir,
            OPTIONAL_BOOTSTRAP_TEMPLATE.name,
            /* optional */ true,
        ));
    }

    files
}

fn resolve_workspace_persona_dir(config: &Config) -> Option<PathBuf> {
    if let Some(path) = config.memory.workspace_dir.clone() {
        let is_memory_dir = path
            .file_name()
            .map(|name| name == std::ffi::OsStr::new("memory"))
            .unwrap_or(false);
        if is_memory_dir {
            return path.parent().map(|parent| parent.join("persona"));
        }
        // Backward-friendly fallback for callers that already pass persona dir directly.
        return Some(path);
    }

    // Avoid writing to the real home directory for tests unless explicitly configured.
    if cfg!(test) {
        return None;
    }

    Some(default_workspace_alan_dir().join("persona"))
}

fn default_workspace_alan_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".alan")
    } else {
        PathBuf::from(".alan")
    }
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

fn read_workspace_file(
    workspace_dir: &Path,
    name: &'static str,
    optional: bool,
) -> WorkspaceBootstrapFile {
    let path = workspace_dir.join(name);

    match fs::read_to_string(&path) {
        Ok(content) => WorkspaceBootstrapFile {
            name,
            path,
            content: Some(content),
            missing: false,
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound && optional => WorkspaceBootstrapFile {
            name,
            path,
            content: None,
            missing: true,
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => WorkspaceBootstrapFile {
            name,
            path,
            content: None,
            missing: true,
        },
        Err(err) => WorkspaceBootstrapFile {
            name,
            path,
            content: Some(format!("[ERROR] Failed to read file: {}", err)),
            missing: false,
        },
    }
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
