//! `alan workspace` — workspace management subcommands.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::registry::WorkspaceRegistry;

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

    for ws in workspaces {
        let path_display = ws.path.display().to_string();
        // Shorten home directory to ~
        let path_short = if let Some(home) = dirs::home_dir() {
            if let Ok(stripped) = ws.path.strip_prefix(&home) {
                format!("~/{}", stripped.display())
            } else {
                path_display
            }
        } else {
            path_display
        };
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
