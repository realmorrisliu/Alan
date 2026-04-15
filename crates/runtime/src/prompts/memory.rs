//! Workspace memory bootstrap file management for prompt assembly.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::debug;

pub const MEMORY_USER_FILENAME: &str = "USER.md";
pub const WORKSPACE_MEMORY_FILENAME: &str = "MEMORY.md";
pub const MEMORY_HANDOFFS_DIRNAME: &str = "handoffs";
pub const MEMORY_LATEST_FILENAME: &str = "LATEST.md";
pub const MEMORY_DAILY_DIRNAME: &str = "daily";
pub const MEMORY_SESSIONS_DIRNAME: &str = "sessions";
pub const MEMORY_WORKING_DIRNAME: &str = "working";
pub const MEMORY_TOPICS_DIRNAME: &str = "topics";
pub const MEMORY_INBOX_DIRNAME: &str = "inbox";
pub(crate) const WORKSPACE_MEMORY_MAX_CHARS: usize = 2_000;

const BOOTSTRAP_HEAD_RATIO: f32 = 0.7;
const BOOTSTRAP_TAIL_RATIO: f32 = 0.2;
const DEFAULT_USER_MEMORY_CONTENT: &str = "# User Memory\n";
const DEFAULT_WORKSPACE_MEMORY_CONTENT: &str = "# Memory\n";
const DEFAULT_LATEST_HANDOFF_CONTENT: &str = "# Latest Handoff\n";

#[derive(Debug, Clone)]
struct MemoryFileTemplate {
    relative_path: &'static [&'static str],
    content: &'static str,
}

const REQUIRED_MEMORY_DIRS: [&str; 6] = [
    MEMORY_HANDOFFS_DIRNAME,
    MEMORY_DAILY_DIRNAME,
    MEMORY_SESSIONS_DIRNAME,
    MEMORY_WORKING_DIRNAME,
    MEMORY_TOPICS_DIRNAME,
    MEMORY_INBOX_DIRNAME,
];

const REQUIRED_MEMORY_FILES: [MemoryFileTemplate; 3] = [
    MemoryFileTemplate {
        relative_path: &[MEMORY_USER_FILENAME],
        content: DEFAULT_USER_MEMORY_CONTENT,
    },
    MemoryFileTemplate {
        relative_path: &[WORKSPACE_MEMORY_FILENAME],
        content: DEFAULT_WORKSPACE_MEMORY_CONTENT,
    },
    MemoryFileTemplate {
        relative_path: &[MEMORY_HANDOFFS_DIRNAME, MEMORY_LATEST_FILENAME],
        content: DEFAULT_LATEST_HANDOFF_CONTENT,
    },
];

#[derive(Debug, Clone)]
pub struct WorkspaceMemoryBootstrapFile {
    pub label: String,
    pub path: PathBuf,
    pub write_path: PathBuf,
    pub content: Option<String>,
    pub missing: bool,
}

pub fn ensure_workspace_memory_layout_at(memory_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(memory_dir)?;

    for dir_name in REQUIRED_MEMORY_DIRS {
        fs::create_dir_all(memory_dir.join(dir_name))?;
    }

    for template in REQUIRED_MEMORY_FILES {
        let path = join_relative_path(memory_dir, template.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_file_if_missing(&path, template.content)?;
    }

    Ok(())
}

pub fn load_workspace_memory_bootstrap_files(
    memory_dir: &Path,
) -> Vec<WorkspaceMemoryBootstrapFile> {
    let mut files = vec![
        read_memory_file(
            MEMORY_USER_FILENAME.to_string(),
            memory_dir.join(MEMORY_USER_FILENAME),
        ),
        read_memory_file(
            WORKSPACE_MEMORY_FILENAME.to_string(),
            memory_dir.join(WORKSPACE_MEMORY_FILENAME),
        ),
        read_memory_file(
            format!("{MEMORY_HANDOFFS_DIRNAME}/{MEMORY_LATEST_FILENAME}"),
            memory_dir
                .join(MEMORY_HANDOFFS_DIRNAME)
                .join(MEMORY_LATEST_FILENAME),
        ),
    ];

    if let Some(latest_daily_note) = newest_daily_note_path(memory_dir) {
        let label = latest_daily_note
            .strip_prefix(memory_dir)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| latest_daily_note.display().to_string());
        files.push(read_memory_file(label, latest_daily_note));
    }

    files
}

pub(crate) fn workspace_memory_tracked_paths(memory_dir: &Path) -> Vec<PathBuf> {
    let mut tracked = vec![
        memory_dir.join(MEMORY_USER_FILENAME),
        memory_dir.join(WORKSPACE_MEMORY_FILENAME),
        memory_dir
            .join(MEMORY_HANDOFFS_DIRNAME)
            .join(MEMORY_LATEST_FILENAME),
        memory_dir.join(MEMORY_DAILY_DIRNAME),
    ];

    if let Some(latest_daily_note) = newest_daily_note_path(memory_dir) {
        tracked.push(latest_daily_note);
    }

    tracked.sort();
    tracked.dedup();
    tracked
}

pub(crate) fn render_workspace_memory_context(memory_dir: &Path) -> String {
    let files = load_workspace_memory_bootstrap_files(memory_dir);
    if files.is_empty() {
        return String::new();
    }

    let mut prompt = String::new();
    prompt.push_str("## Workspace Memory Bootstrap\n");
    prompt.push_str(&format!(
        "Writable Memory Directory: {}\n",
        memory_dir.display()
    ));
    prompt.push_str(
        "The following pure-text memory surfaces are already injected into this prompt.\n",
    );
    prompt.push_str(
        "Prefer them before spending tool calls to rediscover stable identity, workspace memory, or recent cross-session continuity.\n",
    );
    prompt.push_str(
        "Do not re-read them with tools by default; only inspect the on-disk files when you need exact verification or you are editing them.\n",
    );
    prompt.push_str(
        "Paths below are exact on-disk targets. When you persist stable memory, edit the exact `Write updates to:` path shown for that file.\n",
    );

    for file in files {
        prompt.push_str(&format!("\n### {}\n", file.label));
        if file.missing {
            prompt.push_str(&format!(
                "[MISSING] Expected memory file not found: {}\n",
                file.path.display()
            ));
            prompt.push_str(&format!(
                "Write updates to: {}\n",
                file.write_path.display()
            ));
            continue;
        }

        prompt.push_str(&format!("Resolved from: {}\n", file.path.display()));
        prompt.push_str(&format!(
            "Write updates to: {}\n",
            file.write_path.display()
        ));
        let content = file.content.unwrap_or_default();
        let trimmed = trim_memory_content(&content, &file.label, WORKSPACE_MEMORY_MAX_CHARS);
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
            debug!(path = %path.display(), "Created workspace memory bootstrap file");
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
    }
}

fn join_relative_path(base_dir: &Path, relative_path: &[&str]) -> PathBuf {
    relative_path
        .iter()
        .fold(base_dir.to_path_buf(), |path, segment| path.join(segment))
}

fn read_memory_file(label: String, path: PathBuf) -> WorkspaceMemoryBootstrapFile {
    if path.exists() {
        return match fs::read_to_string(&path) {
            Ok(content) => WorkspaceMemoryBootstrapFile {
                label,
                path: path.clone(),
                write_path: path,
                content: Some(content),
                missing: false,
            },
            Err(err) => WorkspaceMemoryBootstrapFile {
                label,
                path: path.clone(),
                write_path: path,
                content: Some(format!("[ERROR] Failed to read file: {}", err)),
                missing: false,
            },
        };
    }

    WorkspaceMemoryBootstrapFile {
        label,
        path: path.clone(),
        write_path: path,
        content: None,
        missing: true,
    }
}

fn newest_daily_note_path(memory_dir: &Path) -> Option<PathBuf> {
    let daily_dir = memory_dir.join(MEMORY_DAILY_DIRNAME);
    let mut files: Vec<PathBuf> = fs::read_dir(daily_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == std::ffi::OsStr::new("md"))
        })
        .collect();
    files.sort();
    files.pop()
}

fn trim_memory_content(content: &str, label: &str, max_chars: usize) -> String {
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
        label, label, head_chars, tail_chars
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
    fn test_ensure_workspace_memory_layout_creates_required_structure() {
        let temp_dir = TempDir::new().unwrap();
        let memory_dir = temp_dir.path().join("memory");

        ensure_workspace_memory_layout_at(&memory_dir).unwrap();

        assert!(memory_dir.join(MEMORY_USER_FILENAME).exists());
        assert!(memory_dir.join(WORKSPACE_MEMORY_FILENAME).exists());
        assert!(
            memory_dir
                .join(MEMORY_HANDOFFS_DIRNAME)
                .join(MEMORY_LATEST_FILENAME)
                .exists()
        );
        assert!(memory_dir.join(MEMORY_DAILY_DIRNAME).exists());
        assert!(memory_dir.join(MEMORY_SESSIONS_DIRNAME).exists());
        assert!(memory_dir.join(MEMORY_WORKING_DIRNAME).exists());
        assert!(memory_dir.join(MEMORY_TOPICS_DIRNAME).exists());
        assert!(memory_dir.join(MEMORY_INBOX_DIRNAME).exists());
    }

    #[test]
    fn test_render_workspace_memory_context_adds_runtime_guidance() {
        let temp_dir = TempDir::new().unwrap();
        let memory_dir = temp_dir.path().join("memory");
        ensure_workspace_memory_layout_at(&memory_dir).unwrap();

        let prompt = render_workspace_memory_context(&memory_dir);

        assert!(prompt.contains("Workspace Memory Bootstrap"));
        assert!(prompt.contains("already injected into this prompt"));
        assert!(prompt.contains("Do not re-read them with tools by default"));
        assert!(prompt.contains("Write updates to:"));
        assert!(prompt.contains("Resolved from:"));
    }

    #[test]
    fn test_render_workspace_memory_context_includes_latest_daily_note() {
        let temp_dir = TempDir::new().unwrap();
        let memory_dir = temp_dir.path().join("memory");
        ensure_workspace_memory_layout_at(&memory_dir).unwrap();
        fs::write(
            memory_dir.join(MEMORY_DAILY_DIRNAME).join("2026-04-14.md"),
            "# 2026-04-14\nolder",
        )
        .unwrap();
        fs::write(
            memory_dir.join(MEMORY_DAILY_DIRNAME).join("2026-04-15.md"),
            "# 2026-04-15\nnewer",
        )
        .unwrap();

        let prompt = render_workspace_memory_context(&memory_dir);

        assert!(prompt.contains("daily/2026-04-15.md"));
        assert!(prompt.contains("newer"));
        assert!(!prompt.contains("daily/2026-04-14.md"));
    }
}
