//! Simple workspace-only sandbox.
//!
//! This sandbox only enforces that all operations happen within
//! the workspace directory. No OS-level sandboxing (Landlock/Seatbelt).

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

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
        if !self.is_in_workspace(cwd) {
            return Err(anyhow!(
                "Working directory outside workspace: {} (workspace: {})",
                cwd.display(),
                self.workspace_root.display()
            ));
        }

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

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
}
