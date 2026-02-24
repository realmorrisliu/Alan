//! Builtin tool implementations for the Alan agent runtime.
//!
//! This crate provides the 7 core tools (read_file, write_file, edit_file,
//! bash, grep, glob, list_dir) as independent implementations of the
//! `Tool` trait defined in `alan-runtime`.

use alan_runtime::tools::{Sandbox, Tool, ToolContext, ToolRegistry, ToolResult};
use anyhow::{Result, anyhow};
use regex::RegexBuilder;
use serde_json::{Value, json};
use std::fs::FileType;
use std::path::Path;

// ============================================================================
// ReadFile
// ============================================================================

/// read_file - Read a file's contents
pub struct ReadFileTool {
    sandbox: Sandbox,
}

impl ReadFileTool {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self {
            sandbox: Sandbox::new(workspace),
        }
    }
}

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read a file's contents. For images, returns metadata."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Start reading from this line (1-indexed)",
                    "minimum": 1
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read",
                    "minimum": 1,
                    "maximum": 1000
                }
            }
        })
    }

    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult {
        let sandbox = self.sandbox.clone();
        let path = ctx.resolve_path(args["path"].as_str().unwrap_or(""));
        let offset = args["offset"].as_u64().unwrap_or(1) as usize;
        let limit = args["limit"].as_u64().unwrap_or(1000) as usize;

        Box::pin(async move {
            // Check if it's an image
            if is_image(&path) {
                let content = sandbox.read(&path).await?;
                return Ok(json!({
                    "type": "image",
                    "path": path.to_string_lossy(),
                    "size_bytes": content.len(),
                    "mime_type": detect_mime(&path)
                }));
            }

            // Read as text
            let content = sandbox.read_string(&path).await?;
            let lines: Vec<&str> = content.lines().collect();

            let start = offset.saturating_sub(1);
            let end = (start + limit).min(lines.len());

            let selected: Vec<&str> = if start < lines.len() {
                lines[start..end].to_vec()
            } else {
                Vec::new()
            };

            Ok(json!({
                "type": "text",
                "path": path.to_string_lossy(),
                "content": selected.join("\n"),
                "total_lines": lines.len(),
                "start_line": start + 1,
                "end_line": end,
                "truncated": lines.len() > limit
            }))
        })
    }

    fn capability(&self, _args: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Read
    }
}

// ============================================================================
// WriteFile
// ============================================================================

/// write_file - Write content to a file
pub struct WriteFileTool {
    sandbox: Sandbox,
}

impl WriteFileTool {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self {
            sandbox: Sandbox::new(workspace),
        }
    }
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates parent directories if needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "content"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write"
                }
            }
        })
    }

    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult {
        let sandbox = self.sandbox.clone();
        let path = ctx.resolve_path(args["path"].as_str().unwrap_or(""));
        let content = args["content"].as_str().unwrap_or("").to_string();

        Box::pin(async move {
            sandbox.write(&path, content.as_bytes()).await?;
            Ok(json!({
                "success": true,
                "path": path.to_string_lossy(),
                "bytes_written": content.len()
            }))
        })
    }

    fn capability(&self, _args: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Write
    }
}

// ============================================================================
// EditFile
// ============================================================================

/// edit_file - Edit a file using search/replace
pub struct EditFileTool {
    sandbox: Sandbox,
}

impl EditFileTool {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self {
            sandbox: Sandbox::new(workspace),
        }
    }
}

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing search text with replacement text."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "old_string", "new_string"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file"
                },
                "old_string": {
                    "type": "string",
                    "description": "Text to search for (exact match)"
                },
                "new_string": {
                    "type": "string",
                    "description": "Text to replace with"
                }
            }
        })
    }

    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult {
        let sandbox = self.sandbox.clone();
        let path = ctx.resolve_path(args["path"].as_str().unwrap_or(""));
        let old_string = args["old_string"].as_str().unwrap_or("").to_string();
        let new_string = args["new_string"].as_str().unwrap_or("").to_string();

        Box::pin(async move {
            let content = sandbox.read_string(&path).await?;

            if !content.contains(&old_string) {
                return Err(anyhow!(
                    "Search text not found in file: '{}...'",
                    &old_string[..old_string.len().min(50)]
                ));
            }

            let new_content = content.replacen(&old_string, &new_string, 1);
            sandbox.write(&path, new_content.as_bytes()).await?;

            Ok(json!({
                "success": true,
                "path": path.to_string_lossy(),
                "replacements": 1
            }))
        })
    }

    fn capability(&self, _args: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Write
    }
}

// ============================================================================
// Bash
// ============================================================================

/// bash - Execute shell commands
pub struct BashTool {
    sandbox: Sandbox,
}

impl BashTool {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self {
            sandbox: Sandbox::new(workspace),
        }
    }
}

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the workspace."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (max 300)",
                    "minimum": 1,
                    "maximum": 300,
                    "default": 60
                }
            }
        })
    }

    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult {
        let sandbox = self.sandbox.clone();
        let cwd = ctx.cwd.clone();
        let command = args["command"].as_str().unwrap_or("").to_string();
        let _timeout = args["timeout"].as_u64().unwrap_or(60);

        Box::pin(async move {
            let result = sandbox.exec(&command, &cwd).await?;

            Ok(json!({
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
                "success": result.exit_code == 0
            }))
        })
    }

    fn capability(&self, _args: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Network
    }

    fn timeout_secs(&self) -> usize {
        120 // Longer timeout for bash commands
    }
}

// ============================================================================
// Grep
// ============================================================================

/// grep - Search file contents
pub struct GrepTool {
    sandbox: Sandbox,
}

impl GrepTool {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self {
            sandbox: Sandbox::new(workspace),
        }
    }
}

impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for patterns in files using regex."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["pattern", "path"],
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Case sensitive search",
                    "default": false
                }
            }
        })
    }

    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult {
        let sandbox = self.sandbox.clone();
        let path = ctx.resolve_path(args["path"].as_str().unwrap_or(""));
        let pattern = args["pattern"].as_str().unwrap_or("").to_string();
        let case_sensitive = args["case_sensitive"].as_bool().unwrap_or(false);

        Box::pin(async move {
            let regex = RegexBuilder::new(&pattern)
                .case_insensitive(!case_sensitive)
                .build()
                .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

            let mut matches = Vec::new();

            if path.is_file() {
                let content = sandbox.read_string(&path).await?;
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        matches.push(json!({
                            "path": path.to_string_lossy(),
                            "line": line_num + 1,
                            "content": line
                        }));
                    }
                }
            } else if path.is_dir() {
                // Recursive search
                search_directory(&sandbox, &path, &regex, &mut matches).await?;
            }

            Ok(json!({
                "matches": matches,
                "total": matches.len()
            }))
        })
    }

    fn capability(&self, _args: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Read
    }
}

async fn search_directory(
    sandbox: &Sandbox,
    dir: &Path,
    regex: &regex::Regex,
    matches: &mut Vec<Value>,
) -> Result<()> {
    let entries = sandbox.list_dir(dir).await?;

    for entry in entries {
        let path = entry.path();
        let file_type: FileType = entry.file_type().await?;

        if file_type.is_dir() {
            // Skip hidden directories
            if let Some(name) = path.file_name()
                && name.to_string_lossy().starts_with('.')
            {
                continue;
            }
            Box::pin(search_directory(sandbox, &path, regex, matches)).await?;
        } else if file_type.is_file() {
            // Skip binary files
            if is_binary_file(&path) {
                continue;
            }

            if let Ok(content) = sandbox.read_string(&path).await {
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        matches.push(json!({
                            "path": path.to_string_lossy(),
                            "line": line_num + 1,
                            "content": line
                        }));
                    }
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// Glob
// ============================================================================

/// glob - Find files matching patterns
pub struct GlobTool {
    sandbox: Sandbox,
}

impl GlobTool {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self {
            sandbox: Sandbox::new(workspace),
        }
    }
}

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["pattern"],
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., '**/*.rs', 'src/*.txt')"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory (default: workspace root)",
                    "default": "."
                }
            }
        })
    }

    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult {
        let _sandbox = self.sandbox.clone();
        let base_path = if let Some(path) = args["path"].as_str() {
            ctx.resolve_path(path)
        } else {
            ctx.cwd.clone()
        };
        let pattern = args["pattern"].as_str().unwrap_or("").to_string();

        Box::pin(async move {
            let pattern_str = base_path.join(&pattern);
            let pattern_str = pattern_str.to_string_lossy();

            let mut matches = Vec::new();

            // Use glob crate for pattern matching
            for path in glob::glob(&pattern_str)?.flatten() {
                if path.is_file() {
                    matches.push(path.to_string_lossy().to_string());
                }
            }

            Ok(json!({
                "matches": matches,
                "total": matches.len()
            }))
        })
    }

    fn capability(&self, _args: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Read
    }
}

// ============================================================================
// ListDir
// ============================================================================

/// list_dir - List directory contents
pub struct ListDirTool {
    sandbox: Sandbox,
}

impl ListDirTool {
    pub fn new(workspace: std::path::PathBuf) -> Self {
        Self {
            sandbox: Sandbox::new(workspace),
        }
    }
}

impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List contents of a directory."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path (default: current directory)",
                    "default": "."
                }
            }
        })
    }

    fn execute(&self, args: Value, ctx: &ToolContext) -> ToolResult {
        let sandbox = self.sandbox.clone();
        let path = if let Some(p) = args["path"].as_str() {
            ctx.resolve_path(p)
        } else {
            ctx.cwd.clone()
        };

        Box::pin(async move {
            let entries = sandbox.list_dir(&path).await?;
            let mut items = Vec::new();

            for entry in entries {
                let file_type = entry.file_type().await?;
                let metadata = entry.metadata().await?;
                let name = entry.file_name().to_string_lossy().to_string();

                items.push(json!({
                    "name": name,
                    "type": if file_type.is_dir() { "directory" } else { "file" },
                    "size": metadata.len()
                }));
            }

            // Sort: directories first, then by name
            items.sort_by(|a, b| {
                let a_is_dir = a["type"] == "directory";
                let b_is_dir = b["type"] == "directory";
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a["name"].as_str().cmp(&b["name"].as_str()),
                }
            });

            Ok(json!({
                "path": path.to_string_lossy(),
                "entries": items,
                "total": items.len()
            }))
        })
    }

    fn capability(&self, _args: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Read
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn is_image(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp"
        )
    } else {
        false
    }
}

fn detect_mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("bmp") => "image/bmp",
        _ => "application/octet-stream",
    }
}

fn is_binary_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(
            ext.as_str(),
            "exe"
                | "dll"
                | "so"
                | "dylib"
                | "bin"
                | "o"
                | "a"
                | "zip"
                | "tar"
                | "gz"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "webp"
                | "mp3"
                | "mp4"
                | "pdf"
        )
    } else {
        false
    }
}

// ============================================================================
// Factory
// ============================================================================

/// Create all 7 core tools with the given workspace
pub fn create_core_tools(workspace: std::path::PathBuf) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ReadFileTool::new(workspace.clone())),
        Box::new(WriteFileTool::new(workspace.clone())),
        Box::new(EditFileTool::new(workspace.clone())),
        Box::new(BashTool::new(workspace.clone())),
        Box::new(GrepTool::new(workspace.clone())),
        Box::new(GlobTool::new(workspace.clone())),
        Box::new(ListDirTool::new(workspace.clone())),
    ]
}

/// Create a ToolRegistry with all 7 core tools pre-registered
pub fn create_tool_registry_with_core_tools(workspace: std::path::PathBuf) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    for tool in create_core_tools(workspace) {
        registry.register_boxed(tool);
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use alan_runtime::Config;
    use alan_runtime::tools::ToolContext;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_tool() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();

        // Create test file
        tokio::fs::write(workspace.join("test.txt"), "line1\nline2\nline3\n")
            .await
            .unwrap();

        let tool = ReadFileTool::new(workspace.clone());
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(workspace.clone(), workspace.join("tmp"), config);

        let args = json!({"path": "test.txt"});
        let result = tool.execute(args, &ctx).await.unwrap();

        assert_eq!(result["type"], "text");
        assert!(result["content"].as_str().unwrap().contains("line1"));
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();

        let write_tool = WriteFileTool::new(workspace.clone());
        let read_tool = ReadFileTool::new(workspace.clone());
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(workspace.clone(), workspace.join("tmp"), config);

        // Write
        let write_args = json!({"path": "output.txt", "content": "Hello World"});
        let write_result = write_tool.execute(write_args, &ctx).await.unwrap();
        assert!(write_result["success"].as_bool().unwrap());

        // Read back
        let read_args = json!({"path": "output.txt"});
        let read_result = read_tool.execute(read_args, &ctx).await.unwrap();
        assert_eq!(read_result["content"], "Hello World");
    }

    #[tokio::test]
    async fn test_edit_file() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();

        // Create file
        tokio::fs::write(workspace.join("edit.txt"), "Hello World")
            .await
            .unwrap();

        let edit_tool = EditFileTool::new(workspace.clone());
        let read_tool = ReadFileTool::new(workspace.clone());
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(workspace.clone(), workspace.join("tmp"), config);

        // Edit
        let edit_args = json!({"path": "edit.txt", "old_string": "World", "new_string": "Rust"});
        let edit_result = edit_tool.execute(edit_args, &ctx).await.unwrap();
        assert!(edit_result["success"].as_bool().unwrap());

        // Verify
        let read_args = json!({"path": "edit.txt"});
        let read_result = read_tool.execute(read_args, &ctx).await.unwrap();
        assert_eq!(read_result["content"], "Hello Rust");
    }

    #[tokio::test]
    async fn test_bash_tool() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();

        let tool = BashTool::new(workspace.clone());
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(workspace.clone(), workspace.join("tmp"), config);

        let args = json!({"command": "echo test_output"});
        let result = tool.execute(args, &ctx).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["stdout"].as_str().unwrap().contains("test_output"));
    }

    #[tokio::test]
    async fn test_grep_tool() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();

        // Create test file
        tokio::fs::write(
            workspace.join("search.txt"),
            "hello world\nfoo bar\nhello rust",
        )
        .await
        .unwrap();

        let tool = GrepTool::new(workspace.clone());
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(workspace.clone(), workspace.join("tmp"), config);

        let args = json!({"pattern": "hello", "path": "search.txt"});
        let result = tool.execute(args, &ctx).await.unwrap();

        assert_eq!(result["total"], 2);
    }

    #[tokio::test]
    async fn test_list_dir_tool() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();

        // Create some files
        tokio::fs::write(workspace.join("file1.txt"), "")
            .await
            .unwrap();
        tokio::fs::create_dir(workspace.join("dir1")).await.unwrap();

        let tool = ListDirTool::new(workspace.clone());
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(workspace.clone(), workspace.join("tmp"), config);

        let args = json!({"path": "."});
        let result = tool.execute(args, &ctx).await.unwrap();

        assert_eq!(result["total"], 2);
    }
}
