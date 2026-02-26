//! `alan init` — initialize a directory as a workspace.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::registry::WorkspaceRegistry;

/// Run the `alan init` command.
pub fn run_init(path: Option<PathBuf>, name: Option<String>, silent: bool) -> Result<()> {
    let target_path = match path {
        Some(p) => {
            std::fs::create_dir_all(&p)
                .with_context(|| format!("Cannot create directory: {}", p.display()))?;
            std::fs::canonicalize(&p)
                .with_context(|| format!("Cannot resolve path: {}", p.display()))?
        }
        None => std::env::current_dir().context("Cannot determine current directory")?,
    };

    let alan_dir = target_path.join(".alan");

    // Create .alan directory structure if it doesn't already exist
    if !alan_dir.exists() {
        std::fs::create_dir_all(alan_dir.join("context/skills"))?;
        std::fs::create_dir_all(alan_dir.join("sessions"))?;
        std::fs::create_dir_all(alan_dir.join("memory"))?;

        // Create MEMORY.md
        std::fs::write(alan_dir.join("memory/MEMORY.md"), "# Memory\n")?;
    }

    // Register in the workspace registry
    let mut registry = WorkspaceRegistry::load()?;

    // Check if already registered
    if let Some(existing) = registry.find(target_path.to_str().unwrap_or("")) {
        if !silent {
            println!("Workspace already initialized at {}", target_path.display());
            println!("  Alias: {}", existing.alias);
            println!("  ID:    {}", existing.id);
        }
        return Ok(());
    }

    let entry = registry.register(&target_path, name)?;
    registry.save()?;

    if !silent {
        println!("✅ Workspace initialized: {}", target_path.display());
        println!("   Alias: {}", entry.alias);
        println!("   ID:    {}", entry.id);
    }

    Ok(())
}
