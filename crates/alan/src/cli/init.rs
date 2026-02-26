//! `alan init` — initialize a directory as a workspace.

use anyhow::{Context, Result};
use std::path::Path;
use std::path::PathBuf;

use crate::registry::WorkspaceRegistry;

/// Run the `alan init` command.
pub fn run_init(path: Option<PathBuf>, name: Option<String>, silent: bool) -> Result<()> {
    let target_path = resolve_target_path(path)?;
    init_workspace(&target_path, name, silent)
}

/// Resolve the target path from optional input path.
fn resolve_target_path(path: Option<PathBuf>) -> Result<PathBuf> {
    match path {
        Some(p) => {
            std::fs::create_dir_all(&p)
                .with_context(|| format!("Cannot create directory: {}", p.display()))?;
            std::fs::canonicalize(&p)
                .with_context(|| format!("Cannot resolve path: {}", p.display()))
        }
        None => std::env::current_dir().context("Cannot determine current directory"),
    }
}

/// Initialize the workspace directory structure.
/// Returns true if the .alan directory was newly created.
pub fn create_alan_directory(alan_dir: &Path) -> Result<bool> {
    if alan_dir.exists() {
        return Ok(false);
    }

    std::fs::create_dir_all(alan_dir.join("context/skills"))?;
    std::fs::create_dir_all(alan_dir.join("sessions"))?;
    std::fs::create_dir_all(alan_dir.join("memory"))?;

    // Create MEMORY.md
    std::fs::write(alan_dir.join("memory/MEMORY.md"), "# Memory\n")?;

    Ok(true)
}

/// Initialize a workspace at the given path.
fn init_workspace(target_path: &Path, name: Option<String>, silent: bool) -> Result<()> {
    let alan_dir = target_path.join(".alan");

    // Create .alan directory structure if it doesn't already exist
    let _created = create_alan_directory(&alan_dir)?;

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

    let entry = registry.register(target_path, name)?;
    registry.save()?;

    if !silent {
        println!("✅ Workspace initialized: {}", target_path.display());
        println!("   Alias: {}", entry.alias);
        println!("   ID:    {}", entry.id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_target_path_with_none() {
        // Should return current directory
        let result = resolve_target_path(None);
        assert!(result.is_ok());
        // Current directory should exist
        assert!(result.unwrap().exists());
    }

    #[test]
    fn test_resolve_target_path_with_existing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("existing");
        std::fs::create_dir_all(&path).unwrap();

        let result = resolve_target_path(Some(path.clone())).unwrap();
        assert!(result.exists());
    }

    #[test]
    fn test_resolve_target_path_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new/nested/dir");

        assert!(!path.exists());
        let result = resolve_target_path(Some(path.clone())).unwrap();
        assert!(result.exists());
    }

    #[test]
    fn test_create_alan_directory_new() {
        let tmp = TempDir::new().unwrap();
        let alan_dir = tmp.path().join(".alan");

        assert!(!alan_dir.exists());
        let created = create_alan_directory(&alan_dir).unwrap();

        assert!(created);
        assert!(alan_dir.exists());
        assert!(alan_dir.join("context/skills").exists());
        assert!(alan_dir.join("sessions").exists());
        assert!(alan_dir.join("memory").exists());
        assert!(alan_dir.join("memory/MEMORY.md").exists());
    }

    #[test]
    fn test_create_alan_directory_existing() {
        let tmp = TempDir::new().unwrap();
        let alan_dir = tmp.path().join(".alan");

        // Create manually first
        std::fs::create_dir_all(&alan_dir).unwrap();

        let created = create_alan_directory(&alan_dir).unwrap();
        assert!(!created);
    }

    #[test]
    fn test_create_alan_directory_preserves_existing_content() {
        let tmp = TempDir::new().unwrap();
        let alan_dir = tmp.path().join(".alan");

        // Create manually with custom content
        std::fs::create_dir_all(alan_dir.join("custom")).unwrap();
        std::fs::write(alan_dir.join("custom/file.txt"), "hello").unwrap();

        let created = create_alan_directory(&alan_dir).unwrap();
        assert!(!created);

        // Custom content should still exist
        assert!(alan_dir.join("custom/file.txt").exists());
        let content = std::fs::read_to_string(alan_dir.join("custom/file.txt")).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_memory_md_content() {
        let tmp = TempDir::new().unwrap();
        let alan_dir = tmp.path().join(".alan");

        create_alan_directory(&alan_dir).unwrap();

        let memory_content = std::fs::read_to_string(alan_dir.join("memory/MEMORY.md")).unwrap();
        assert_eq!(memory_content, "# Memory\n");
    }
}
