//! `alan workspace` — workspace management subcommands.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::registry::WorkspaceRegistry;

/// Shorten a path by replacing the home directory prefix with ~.
fn shorten_path(path: &Path, home: Option<&Path>) -> String {
    let path_display = path.display().to_string();

    if let Some(home_dir) = home
        && let Ok(stripped) = path.strip_prefix(home_dir)
    {
        return format!("~/{}", stripped.display());
    }

    path_display
}

/// List all registered workspaces.
pub fn list_workspaces() -> Result<()> {
    let registry = WorkspaceRegistry::load()?;
    let workspaces = registry.list();

    if workspaces.is_empty() {
        println!("No workspaces registered.");
        println!("Run `alan init` in a project directory to create one.");
        return Ok(());
    }

    // Print header
    println!("{:<10} {:<20} PATH", "ID", "ALIAS");
    println!("{}", "-".repeat(60));

    let home = dirs::home_dir();
    for ws in workspaces {
        let path_short = shorten_path(&ws.path, home.as_deref());
        println!("{:<10} {:<20} {}", ws.id, ws.alias, path_short);
    }

    println!("\n{} workspace(s)", workspaces.len());
    Ok(())
}

/// Register an existing workspace directory.
pub fn add_workspace(path: PathBuf, name: Option<String>) -> Result<()> {
    let canonical = std::fs::canonicalize(&path)
        .with_context(|| format!("Cannot resolve path: {}", path.display()))?;

    // Verify .alan directory exists
    if !canonical.join(".alan").exists() {
        anyhow::bail!(
            "Directory {} does not contain an .alan/ directory.\nRun `alan init --path {}` first.",
            canonical.display(),
            canonical.display(),
        );
    }

    let mut registry = WorkspaceRegistry::load()?;
    let entry = registry.register(&canonical, name)?;
    registry.save()?;

    println!("✅ Workspace registered: {}", canonical.display());
    println!("   Alias: {}", entry.alias);
    println!("   ID:    {}", entry.id);
    Ok(())
}

/// Unregister a workspace (does not delete files).
pub fn remove_workspace(workspace: &str) -> Result<()> {
    let mut registry = WorkspaceRegistry::load()?;
    let removed = registry.unregister(workspace)?;
    registry.save()?;

    println!("Workspace '{}' unregistered.", removed.alias);
    println!("Files at {} were not deleted.", removed.path.display());
    Ok(())
}

/// Show workspace details.
pub fn workspace_info(workspace: &str) -> Result<()> {
    let registry = WorkspaceRegistry::load()?;
    let entry = registry
        .find(workspace)
        .with_context(|| format!("Workspace not found: '{}'", workspace))?;

    println!("Workspace: {}", entry.alias);
    println!("  ID:         {}", entry.id);
    println!("  Path:       {}", entry.path.display());
    println!("  Created:    {}", entry.created_at);

    // Check if .alan directory exists
    let alan_dir = entry.path.join(".alan");
    if alan_dir.exists() {
        println!("  Status:     ✅ initialized");

        // Check for sessions
        let sessions_dir = alan_dir.join("sessions");
        if sessions_dir.exists() {
            let count = std::fs::read_dir(&sessions_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == "jsonl")
                                .unwrap_or(false)
                        })
                        .count()
                })
                .unwrap_or(0);
            println!("  Sessions:   {}", count);
        }
    } else {
        println!("  Status:     ⚠️  .alan/ directory missing");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_shorten_path_with_home_prefix() {
        let home = PathBuf::from("/Users/test");
        let path = PathBuf::from("/Users/test/projects/myapp");

        let result = shorten_path(&path, Some(&home));
        assert_eq!(result, "~/projects/myapp");
    }

    #[test]
    fn test_shorten_path_without_home_prefix() {
        let home = PathBuf::from("/Users/test");
        let path = PathBuf::from("/opt/projects/myapp");

        let result = shorten_path(&path, Some(&home));
        assert_eq!(result, "/opt/projects/myapp");
    }

    #[test]
    fn test_shorten_path_no_home() {
        let path = PathBuf::from("/Users/test/projects/myapp");

        let result = shorten_path(&path, None);
        assert_eq!(result, "/Users/test/projects/myapp");
    }

    #[test]
    fn test_shorten_path_exact_home() {
        let home = PathBuf::from("/Users/test");
        let path = PathBuf::from("/Users/test");

        let result = shorten_path(&path, Some(&home));
        assert_eq!(result, "~/");
    }

    #[test]
    fn test_shorten_path_with_trailing_slash() {
        let home = PathBuf::from("/Users/test/");
        let path = PathBuf::from("/Users/test/projects/myapp");

        let result = shorten_path(&path, Some(&home));
        // strip_prefix should still work correctly
        assert_eq!(result, "~/projects/myapp");
    }

    #[test]
    fn test_shorten_path_relative() {
        let home = PathBuf::from("/Users/test");
        let path = PathBuf::from("./relative/path");

        let result = shorten_path(&path, Some(&home));
        // Relative paths don't start with home, so should remain unchanged
        assert_eq!(result, "./relative/path");
    }
}