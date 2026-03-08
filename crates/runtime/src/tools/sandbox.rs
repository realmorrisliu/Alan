//! Simple workspace-only sandbox.
//!
//! This sandbox only enforces that all operations happen within
//! the workspace directory. No OS-level sandboxing (Landlock/Seatbelt).

use anyhow::{Result, anyhow};
use regex::Regex;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const SANDBOX_BACKEND_WORKSPACE_PATH_GUARD: &str = "workspace_path_guard";
const PROTECTED_SUBPATHS: [&str; 3] = [".git", ".alan", ".agents"];

/// Execution result from sandbox
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Simple workspace-only sandbox
#[derive(Clone)]
pub struct Sandbox {
    workspace_root: PathBuf,
}

impl Sandbox {
    /// Create a new sandbox restricted to the given workspace
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Name of the active sandbox backend.
    pub fn backend_name(&self) -> &'static str {
        Self::backend_name_static()
    }

    /// Name of the built-in workspace path guard backend.
    pub const fn backend_name_static() -> &'static str {
        SANDBOX_BACKEND_WORKSPACE_PATH_GUARD
    }

    /// Check if a path is within the workspace
    pub fn is_in_workspace(&self, path: &Path) -> bool {
        // Try to get absolute path
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Get canonical workspace (may fail if doesn't exist)
        let canonical_workspace = self
            .canonicalize(&self.workspace_root)
            .unwrap_or_else(|_| dunce::simplified(&self.workspace_root).to_path_buf());

        // For existing paths, use canonical path
        if absolute_path.exists() {
            let canonical_path = self
                .canonicalize(&absolute_path)
                .unwrap_or_else(|_| dunce::simplified(&absolute_path).to_path_buf());
            return canonical_path.starts_with(&canonical_workspace);
        }

        // For new files, check that all existing parent directories are within workspace
        let mut current = absolute_path.parent();
        while let Some(parent) = current {
            if parent.exists() {
                let canonical_parent = self
                    .canonicalize(parent)
                    .unwrap_or_else(|_| dunce::simplified(parent).to_path_buf());
                return canonical_parent.starts_with(&canonical_workspace);
            }
            current = parent.parent();
        }

        // If no parent exists, check if the path itself starts with workspace
        dunce::simplified(&absolute_path)
            .to_string_lossy()
            .starts_with(&canonical_workspace.to_string_lossy().to_string())
    }

    /// Read a file within the workspace
    pub async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        if !self.is_in_workspace(path) {
            return Err(anyhow!(
                "Path outside workspace: {} (workspace: {})",
                path.display(),
                self.workspace_root.display()
            ));
        }

        tokio::fs::read(path)
            .await
            .map_err(|e| anyhow!("Failed to read file: {}", e))
    }

    /// Read file as string
    pub async fn read_string(&self, path: &Path) -> Result<String> {
        let bytes = self.read(path).await?;
        String::from_utf8(bytes).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
    }

    /// Write a file within the workspace
    pub async fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        if !self.is_in_workspace(path) {
            return Err(anyhow!(
                "Path outside workspace: {} (workspace: {})",
                path.display(),
                self.workspace_root.display()
            ));
        }
        self.ensure_path_not_protected(path, "write")?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| anyhow!("Failed to write file: {}", e))
    }

    /// Execute a command within the workspace
    pub async fn exec(&self, cmd: &str, cwd: &Path) -> Result<ExecResult> {
        self.exec_with_timeout_and_capability(cmd, cwd, None, None)
            .await
    }

    /// Execute a command within the workspace with an optional timeout.
    pub async fn exec_with_timeout(
        &self,
        cmd: &str,
        cwd: &Path,
        timeout: Option<Duration>,
    ) -> Result<ExecResult> {
        self.exec_with_timeout_and_capability(cmd, cwd, timeout, None)
            .await
    }

    /// Execute a command within the workspace with capability-aware path checks.
    pub async fn exec_with_timeout_and_capability(
        &self,
        cmd: &str,
        cwd: &Path,
        timeout: Option<Duration>,
        capability: Option<alan_protocol::ToolCapability>,
    ) -> Result<ExecResult> {
        if !self.is_in_workspace(cwd) {
            return Err(anyhow!(
                "Working directory outside workspace: {} (workspace: {})",
                cwd.display(),
                self.workspace_root.display()
            ));
        }
        if should_protect_process_paths(capability) {
            self.ensure_path_not_protected(cwd, "process cwd")?;
        }

        self.validate_command_paths(cmd, cwd, should_protect_process_paths(capability))?;

        let mut command = tokio::process::Command::new("sh");
        command.arg("-c").arg(cmd).current_dir(cwd);
        let output = if let Some(limit) = timeout {
            match tokio::time::timeout(limit, command.output()).await {
                Ok(result) => result.map_err(|e| anyhow!("Failed to execute command: {}", e))?,
                Err(_) => {
                    return Err(anyhow!(
                        "Command execution timed out after {}s",
                        limit.as_secs()
                    ));
                }
            }
        } else {
            command
                .output()
                .await
                .map_err(|e| anyhow!("Failed to execute command: {}", e))?
        };

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// List directory contents
    pub async fn list_dir(&self, path: &Path) -> Result<Vec<tokio::fs::DirEntry>> {
        if !self.is_in_workspace(path) {
            return Err(anyhow!(
                "Path outside workspace: {} (workspace: {})",
                path.display(),
                self.workspace_root.display()
            ));
        }

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry);
        }
        Ok(entries)
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(dunce::canonicalize(path)?)
    }

    fn validate_command_paths(
        &self,
        cmd: &str,
        cwd: &Path,
        block_protected_subpaths: bool,
    ) -> Result<()> {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        for raw_token in trimmed.split_whitespace() {
            let mut token = raw_token.trim_matches(|c: char| {
                matches!(
                    c,
                    '"' | '\'' | '`' | ';' | '|' | '&' | '(' | ')' | '{' | '}'
                )
            });

            if token.is_empty() || token.starts_with('-') {
                continue;
            }

            if token.starts_with("~/")
                || token == "~"
                || token.contains("$HOME")
                || token.contains("${HOME}")
            {
                return Err(anyhow!(
                    "Command references HOME paths outside workspace: {}",
                    token
                ));
            }

            while let Some(stripped) = token
                .strip_prefix("2>")
                .or_else(|| token.strip_prefix("1>"))
                .or_else(|| token.strip_prefix(">>"))
                .or_else(|| token.strip_prefix('>'))
                .or_else(|| token.strip_prefix('<'))
            {
                token = stripped;
            }

            if token.is_empty() || token.contains("://") {
                continue;
            }

            if looks_like_path_token(token) {
                let candidate = if Path::new(token).is_absolute() {
                    PathBuf::from(token)
                } else {
                    cwd.join(token)
                };
                if candidate.is_absolute() && is_allowed_absolute_command_path(&candidate) {
                    continue;
                }
                if !self.is_in_workspace(&candidate) {
                    return Err(anyhow!(
                        "Command references path outside workspace: {}",
                        token
                    ));
                }
                if block_protected_subpaths {
                    self.ensure_path_not_protected(&candidate, "process path reference")?;
                }
            }
        }

        let regex = Regex::new(r"/[A-Za-z0-9._/-]+").expect("absolute-path regex is valid");
        for matched in regex.find_iter(trimmed) {
            let start = matched.start();
            if start > 0 {
                let prev = trimmed.as_bytes()[start - 1];
                if prev == b':'
                    || prev == b'.'
                    || prev == b'/'
                    || prev == b'_'
                    || prev == b'-'
                    || prev.is_ascii_alphanumeric()
                {
                    // Skip URL fragments and path segments within relative paths or identifiers.
                    continue;
                }
            }
            let literal = matched.as_str();
            if is_allowed_absolute_command_path(Path::new(literal)) {
                continue;
            }
            if !self.is_in_workspace(Path::new(literal)) {
                return Err(anyhow!(
                    "Command contains absolute path outside workspace: {}",
                    literal
                ));
            }
            if block_protected_subpaths {
                self.ensure_path_not_protected(Path::new(literal), "process path reference")?;
            }
        }

        Ok(())
    }

    fn ensure_path_not_protected(&self, path: &Path, action: &str) -> Result<()> {
        if let Some(component) = self.protected_subpath_component(path) {
            return Err(anyhow!(
                "Sandbox backend {} blocks {} under protected subpath {}: {}",
                self.backend_name(),
                action,
                component,
                path.display()
            ));
        }
        Ok(())
    }

    fn protected_subpath_component(&self, path: &Path) -> Option<&'static str> {
        let relative = if path.is_absolute() {
            if let Ok(relative) = path.strip_prefix(&self.workspace_root) {
                relative.to_path_buf()
            } else {
                let canonical_workspace = self
                    .canonicalize(&self.workspace_root)
                    .unwrap_or_else(|_| lexically_normalize_path(&self.workspace_root));
                let normalized_path = self.normalized_path(path);
                normalized_path
                    .strip_prefix(&canonical_workspace)
                    .ok()?
                    .to_path_buf()
            }
        } else {
            lexically_normalize_path(path)
        };
        relative.components().find_map(|component| match component {
            Component::Normal(name) => {
                let candidate = name.to_str()?;
                PROTECTED_SUBPATHS
                    .iter()
                    .copied()
                    .find(|protected| *protected == candidate)
            }
            _ => None,
        })
    }

    fn normalized_path(&self, path: &Path) -> PathBuf {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };
        if absolute_path.exists() {
            self.canonicalize(&absolute_path)
                .unwrap_or_else(|_| lexically_normalize_path(&absolute_path))
        } else {
            lexically_normalize_path(&absolute_path)
        }
    }
}

fn looks_like_path_token(token: &str) -> bool {
    token.starts_with('/')
        || token.starts_with("./")
        || token.starts_with("../")
        || token == "."
        || token == ".."
        || token.contains('/')
}

fn is_allowed_absolute_command_path(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some("/dev/null" | "/dev/stdin" | "/dev/stdout" | "/dev/stderr")
    )
}

fn should_protect_process_paths(capability: Option<alan_protocol::ToolCapability>) -> bool {
    !matches!(capability, Some(alan_protocol::ToolCapability::Read))
}

fn lexically_normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sandbox_read_write() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        // Write a file
        let file_path = temp.path().join("test.txt");
        sandbox.write(&file_path, b"hello world").await.unwrap();

        // Read it back
        let content = sandbox.read_string(&file_path).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_sandbox_blocks_outside_workspace() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        // Try to read outside workspace
        let outside_path = PathBuf::from("/etc/passwd");
        let result = sandbox.read(&outside_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sandbox_exec() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox.exec("echo hello", temp.path()).await.unwrap();
        assert_eq!(result.stdout.trim(), "hello");
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_outside_workspace_path_reference() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox.exec("cat /etc/passwd", temp.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_workspace_relative_paths() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let file = temp.path().join("in_workspace.txt");
        tokio::fs::write(&file, "ok").await.unwrap();

        let result = sandbox.exec("cat ./in_workspace.txt", temp.path()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().stdout.trim(), "ok");
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_dev_null_redirection() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox.exec("echo ok > /dev/null", temp.path()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().exit_code, 0);
    }

    #[tokio::test]
    async fn test_sandbox_blocks_write_to_protected_subpath() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".git/config");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();

        let result = sandbox.write(&protected, b"[core]\n").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }

    #[tokio::test]
    async fn test_sandbox_allows_read_from_protected_subpath() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".alan/policy.yaml");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&protected, "rules: []\n").await.unwrap();

        let result = sandbox.read_string(&protected).await;
        assert_eq!(result.unwrap(), "rules: []\n");
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_mutating_command_for_protected_path() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".alan/config.toml");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "touch .alan/config.toml",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .alan")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_read_only_command_for_protected_path() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".git/HEAD");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&protected, "ref: refs/heads/main\n")
            .await
            .unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "cat .git/HEAD",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().stdout.trim(), "ref: refs/heads/main");
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_mutating_cwd_inside_protected_subpath() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".agents");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "touch state.txt",
                &protected_dir,
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .agents")
        );
    }
}
