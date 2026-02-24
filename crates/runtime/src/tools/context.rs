//! Tool execution context for dependency injection.

use crate::config::Config;
use std::path::PathBuf;
use std::sync::Arc;

/// Context provided to tools during execution.
/// Contains all dependencies needed by tools.
pub struct ToolContext {
    /// Working directory for the tool
    pub cwd: PathBuf,
    /// Scratch directory for temporary files
    pub scratch_dir: PathBuf,
    /// Global configuration
    pub config: Arc<Config>,
}

impl ToolContext {
    /// Create a new tool context
    pub fn new(cwd: PathBuf, scratch_dir: PathBuf, config: Arc<Config>) -> Self {
        Self {
            cwd,
            scratch_dir,
            config,
        }
    }

    /// Resolve a path relative to working directory
    pub fn resolve_path(&self, path: impl AsRef<std::path::Path>) -> PathBuf {
        if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_context_resolve_path() {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(
            PathBuf::from("/workspace"),
            PathBuf::from("/tmp/scratch"),
            config,
        );

        // Relative path
        assert_eq!(
            ctx.resolve_path("file.txt"),
            PathBuf::from("/workspace/file.txt")
        );

        // Absolute path
        assert_eq!(
            ctx.resolve_path("/absolute/file.txt"),
            PathBuf::from("/absolute/file.txt")
        );
    }
}
