//! Workspace registry — persistent workspace registration.
//!
//! Maintains a `registry.json` at `~/.alan/registry.json` that tracks
//! all known workspaces across the filesystem. Each workspace is identified
//! by its canonical path, with a short hash ID and user-friendly alias.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// The workspace registry, stored as `~/.alan/registry.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRegistry {
    pub version: u32,
    pub workspaces: Vec<WorkspaceEntry>,
}

/// A single registered workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    /// Short hash ID derived from canonical path (6 hex chars)
    pub id: String,
    /// Canonical absolute path to the workspace root
    pub path: PathBuf,
    /// User-friendly alias (defaults to directory name)
    pub alias: String,
    /// ISO 8601 timestamp
    pub created_at: String,
}

impl WorkspaceRegistry {
    /// Default registry file path.
    pub fn registry_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".alan").join("registry.json"))
    }

    /// Load registry from disk, creating an empty one if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::registry_path()?;
        if !path.exists() {
            return Ok(Self {
                version: 1,
                workspaces: Vec::new(),
            });
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read registry: {}", path.display()))?;
        let registry: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse registry: {}", path.display()))?;
        Ok(registry)
    }

    /// Save registry to disk atomically.
    pub fn save(&self) -> Result<()> {
        let path = Self::registry_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;

        // Write atomically: write to temp file, then rename
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &content)
            .with_context(|| format!("Failed to write registry: {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path)
            .with_context(|| format!("Failed to rename registry: {}", path.display()))?;
        Ok(())
    }

    /// Register a workspace. Returns the created entry.
    ///
    /// Fails if the path is already registered or the alias conflicts.
    pub fn register(
        &mut self,
        workspace_path: &Path,
        alias: Option<String>,
    ) -> Result<WorkspaceEntry> {
        let canonical = fs::canonicalize(workspace_path)
            .with_context(|| format!("Cannot resolve path: {}", workspace_path.display()))?;

        // Check for duplicate path
        if self.workspaces.iter().any(|w| w.path == canonical) {
            bail!("Workspace already registered: {}", canonical.display());
        }

        let id = generate_workspace_id(&canonical);
        let alias = alias.unwrap_or_else(|| default_alias(&canonical));

        // Check for duplicate alias
        if self.workspaces.iter().any(|w| w.alias == alias) {
            bail!(
                "Alias '{}' already in use. Use --name to specify a different alias.",
                alias
            );
        }

        let entry = WorkspaceEntry {
            id,
            path: canonical,
            alias,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.workspaces.push(entry.clone());
        Ok(entry)
    }

    /// Unregister a workspace by alias, ID, or path.
    pub fn unregister(&mut self, query: &str) -> Result<WorkspaceEntry> {
        let idx = self
            .find_index(query)
            .with_context(|| format!("Workspace not found: '{}'", query))?;
        Ok(self.workspaces.remove(idx))
    }

    /// Find a workspace entry by alias, short ID, or path.
    pub fn find(&self, query: &str) -> Option<&WorkspaceEntry> {
        let idx = self.find_index(query)?;
        self.workspaces.get(idx)
    }

    /// Find the index of a workspace by alias, short ID, or path.
    fn find_index(&self, query: &str) -> Option<usize> {
        // Try alias first (exact match)
        if let Some(idx) = self.workspaces.iter().position(|w| w.alias == query) {
            return Some(idx);
        }

        // Try short ID
        if let Some(idx) = self.workspaces.iter().position(|w| w.id == query) {
            return Some(idx);
        }

        // Try path (resolve to canonical for comparison)
        if let Ok(canonical) = fs::canonicalize(query)
            && let Some(idx) = self.workspaces.iter().position(|w| w.path == canonical)
        {
            return Some(idx);
        }

        // Try path as-is (might match stored path directly)
        let query_path = PathBuf::from(query);
        self.workspaces.iter().position(|w| w.path == query_path)
    }

    /// List all registered workspaces.
    pub fn list(&self) -> &[WorkspaceEntry] {
        &self.workspaces
    }
}

/// Generate a short workspace ID from the canonical path.
fn generate_workspace_id(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let hash = hasher.finalize();
    hash.iter().take(3).map(|b| format!("{:02x}", b)).collect()
}

/// Derive a default alias from a directory path.
fn default_alias(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "workspace".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_registry(_tmp: &TempDir) -> WorkspaceRegistry {
        // Override the registry path for testing by creating registry manually
        WorkspaceRegistry {
            version: 1,
            workspaces: Vec::new(),
        }
    }

    #[test]
    fn test_generate_workspace_id() {
        let path = PathBuf::from("/Users/test/my-project");
        let id = generate_workspace_id(&path);
        assert_eq!(id.len(), 6);
        // Same path always produces same ID
        assert_eq!(id, generate_workspace_id(&path));
        // Different path produces different ID
        let other = generate_workspace_id(&PathBuf::from("/Users/test/other"));
        assert_ne!(id, other);
    }

    #[test]
    fn test_default_alias() {
        assert_eq!(
            default_alias(Path::new("/Users/test/my-project")),
            "my-project"
        );
        assert_eq!(default_alias(Path::new("/Users/test/.alan")), ".alan");
    }

    #[test]
    fn test_register_and_find() {
        let tmp = TempDir::new().unwrap();
        let mut registry = setup_test_registry(&tmp);

        // Create a workspace dir
        let ws_dir = tmp.path().join("my-workspace");
        std::fs::create_dir_all(&ws_dir).unwrap();

        let entry = registry
            .register(&ws_dir, Some("test-ws".to_string()))
            .unwrap();
        assert_eq!(entry.alias, "test-ws");
        assert_eq!(entry.id.len(), 6);

        // Find by alias
        assert!(registry.find("test-ws").is_some());
        // Find by ID
        assert!(registry.find(&entry.id).is_some());
        // Find by path
        assert!(registry.find(ws_dir.to_str().unwrap()).is_some());
    }

    #[test]
    fn test_register_duplicate_path_fails() {
        let tmp = TempDir::new().unwrap();
        let mut registry = setup_test_registry(&tmp);

        let ws_dir = tmp.path().join("dup");
        std::fs::create_dir_all(&ws_dir).unwrap();

        registry
            .register(&ws_dir, Some("first".to_string()))
            .unwrap();
        let err = registry.register(&ws_dir, Some("second".to_string()));
        assert!(err.is_err());
    }

    #[test]
    fn test_register_duplicate_alias_fails() {
        let tmp = TempDir::new().unwrap();
        let mut registry = setup_test_registry(&tmp);

        let ws1 = tmp.path().join("ws1");
        let ws2 = tmp.path().join("ws2");
        std::fs::create_dir_all(&ws1).unwrap();
        std::fs::create_dir_all(&ws2).unwrap();

        registry
            .register(&ws1, Some("same-name".to_string()))
            .unwrap();
        let err = registry.register(&ws2, Some("same-name".to_string()));
        assert!(err.is_err());
    }

    #[test]
    fn test_unregister() {
        let tmp = TempDir::new().unwrap();
        let mut registry = setup_test_registry(&tmp);

        let ws_dir = tmp.path().join("to-remove");
        std::fs::create_dir_all(&ws_dir).unwrap();

        registry
            .register(&ws_dir, Some("removable".to_string()))
            .unwrap();
        assert_eq!(registry.list().len(), 1);

        let removed = registry.unregister("removable").unwrap();
        assert_eq!(removed.alias, "removable");
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mut registry = setup_test_registry(&tmp);

        let ws_dir = tmp.path().join("roundtrip");
        std::fs::create_dir_all(&ws_dir).unwrap();

        registry
            .register(&ws_dir, Some("rt-test".to_string()))
            .unwrap();

        let json = serde_json::to_string_pretty(&registry).unwrap();
        let loaded: WorkspaceRegistry = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.workspaces.len(), 1);
        assert_eq!(loaded.workspaces[0].alias, "rt-test");
    }
}
