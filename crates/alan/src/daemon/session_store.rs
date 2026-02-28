//! Session Store - persistence for `Session -> Workspace` bindings.
//!
//! Stores the mapping from session IDs to workspace paths so bindings can be
//! recovered after daemon restarts.
//!
//! Storage location: `~/.alan/sessions/<session_id>.json`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Session binding metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBinding {
    /// Session ID
    pub session_id: String,
    /// Workspace path
    pub workspace_path: PathBuf,
    /// Creation time
    pub created_at: String,
    /// Governance configuration
    #[serde(default)]
    pub governance: alan_protocol::GovernanceConfig,
    /// Per-session streaming mode override (None = runtime default/config).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming_mode: Option<alan_runtime::StreamingMode>,
    /// Per-session partial stream recovery override (None = runtime default/config).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial_stream_recovery_mode: Option<alan_runtime::PartialStreamRecoveryMode>,
    /// Rollout file path (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollout_path: Option<PathBuf>,
}

/// Session store
#[derive(Debug)]
pub struct SessionStore {
    storage_dir: PathBuf,
    /// In-memory cache
    cache: std::sync::RwLock<HashMap<String, SessionBinding>>,
}

impl SessionStore {
    /// Create a new `SessionStore`
    pub fn new() -> Result<Self> {
        let storage_dir = Self::default_storage_dir()?;
        std::fs::create_dir_all(&storage_dir)?;

        let cache = std::sync::RwLock::new(HashMap::new());
        let store = Self { storage_dir, cache };

        // Load all persisted sessions.
        store.load_all()?;

        Ok(store)
    }

    /// Create with a specific storage directory (for tests)
    #[cfg(test)]
    pub fn with_dir(storage_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_dir)?;
        Ok(Self {
            storage_dir,
            cache: std::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Default storage directory
    fn default_storage_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".alan").join("sessions"))
    }

    /// Get the session file path
    fn session_file_path(&self, session_id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", session_id))
    }

    /// Save a session binding
    pub fn save(&self, binding: SessionBinding) -> Result<()> {
        let session_id = binding.session_id.clone();
        let path = self.session_file_path(&session_id);

        // Serialize and write.
        let content = serde_json::to_string_pretty(&binding)?;
        std::fs::write(&path, content)?;

        // Update cache.
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(session_id.clone(), binding);
        }

        debug!(%session_id, path = %path.display(), "Saved session binding");
        Ok(())
    }

    /// Load a specific session
    #[allow(dead_code)]
    pub fn load(&self, session_id: &str) -> Option<SessionBinding> {
        // Check cache first.
        if let Ok(cache) = self.cache.read()
            && let Some(binding) = cache.get(session_id)
        {
            return Some(binding.clone());
        }

        // Load from disk.
        let path = self.session_file_path(session_id);
        if !path.exists() {
            return None;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<SessionBinding>(&content) {
                    Ok(binding) => {
                        // Update cache.
                        if let Ok(mut cache) = self.cache.write() {
                            cache.insert(session_id.to_string(), binding.clone());
                        }
                        Some(binding)
                    }
                    Err(err) => {
                        warn!(%session_id, error = %err, "Failed to parse session binding");
                        None
                    }
                }
            }
            Err(err) => {
                warn!(%session_id, error = %err, "Failed to read session binding");
                None
            }
        }
    }

    /// Remove a session binding
    pub fn remove(&self, session_id: &str) -> Result<()> {
        let path = self.session_file_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        // Update cache.
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(session_id);
        }

        debug!(%session_id, "Removed session binding");
        Ok(())
    }

    /// List all sessions
    pub fn list_all(&self) -> Vec<SessionBinding> {
        // Refresh cache.
        let _ = self.load_all();

        if let Ok(cache) = self.cache.read() {
            cache.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// List active sessions (binding exists and workspace path is valid)
    pub fn list_active(&self) -> Vec<SessionBinding> {
        self.list_all()
            .into_iter()
            .filter(|b| b.workspace_path.exists())
            .collect()
    }

    /// Check whether a session exists
    #[allow(dead_code)]
    pub fn exists(&self, session_id: &str) -> bool {
        // Check cache first.
        if let Ok(cache) = self.cache.read()
            && cache.contains_key(session_id)
        {
            return true;
        }

        // Check file on disk.
        self.session_file_path(session_id).exists()
    }

    /// Update rollout path
    pub fn update_rollout_path(
        &self,
        session_id: &str,
        rollout_path: Option<PathBuf>,
    ) -> Result<()> {
        if let Some(mut binding) = self.load(session_id) {
            binding.rollout_path = rollout_path;
            self.save(binding)?;
        }
        Ok(())
    }

    /// Get workspace path
    #[allow(dead_code)]
    pub fn get_workspace_path(&self, session_id: &str) -> Option<PathBuf> {
        self.load(session_id).map(|b| b.workspace_path)
    }

    /// Load all sessions into cache
    fn load_all(&self) -> Result<()> {
        let entries = std::fs::read_dir(&self.storage_dir)?;
        let mut bindings = HashMap::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let session_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            if let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(binding) = serde_json::from_str::<SessionBinding>(&content)
            {
                bindings.insert(session_id, binding);
            }
        }

        if let Ok(mut cache) = self.cache.write() {
            *cache = bindings;
        }

        info!(
            count = self.cache.read().map(|c| c.len()).unwrap_or(0),
            "Loaded session bindings"
        );
        Ok(())
    }

    /// Remove stale sessions (workspace path does not exist)
    #[allow(dead_code)]
    pub fn cleanup_stale(&self) -> usize {
        let all = self.list_all();
        let mut removed = 0;

        for binding in all {
            if !binding.workspace_path.exists() {
                if let Err(err) = self.remove(&binding.session_id) {
                    warn!(session_id = %binding.session_id, error = %err, "Failed to remove stale session");
                } else {
                    info!(session_id = %binding.session_id, "Removed stale session binding");
                    removed += 1;
                }
            }
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_store_new() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        // Initially empty.
        assert!(store.list_all().is_empty());
        assert!(!store.exists("test"));
    }

    #[test]
    fn test_save_and_load() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        let binding = SessionBinding {
            session_id: "test-session".to_string(),
            workspace_path: PathBuf::from("/tmp/test-workspace"),
            created_at: chrono::Utc::now().to_rfc3339(),
            governance: alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Conservative,
                policy_path: None,
            },
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            rollout_path: None,
        };

        store.save(binding.clone()).unwrap();

        // Should be loadable again.
        let loaded = store.load("test-session").unwrap();
        assert_eq!(loaded.session_id, binding.session_id);
        assert_eq!(loaded.workspace_path, binding.workspace_path);
    }

    #[test]
    fn test_exists() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        assert!(!store.exists("nonexistent"));

        let binding = SessionBinding {
            session_id: "exists-session".to_string(),
            workspace_path: PathBuf::from("/tmp/test"),
            created_at: chrono::Utc::now().to_rfc3339(),
            governance: alan_protocol::GovernanceConfig::default(),
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            rollout_path: None,
        };

        store.save(binding).unwrap();
        assert!(store.exists("exists-session"));
    }

    #[test]
    fn test_save_and_load_streaming_mode() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        let binding = SessionBinding {
            session_id: "streaming-mode-session".to_string(),
            workspace_path: PathBuf::from("/tmp/test-streaming"),
            created_at: chrono::Utc::now().to_rfc3339(),
            governance: alan_protocol::GovernanceConfig::default(),
            streaming_mode: Some(alan_runtime::StreamingMode::Off),
            partial_stream_recovery_mode: Some(alan_runtime::PartialStreamRecoveryMode::Off),
            rollout_path: None,
        };

        store.save(binding).unwrap();
        let loaded = store.load("streaming-mode-session").unwrap();
        assert_eq!(
            loaded.streaming_mode,
            Some(alan_runtime::StreamingMode::Off)
        );
    }

    #[test]
    fn test_remove() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        let binding = SessionBinding {
            session_id: "to-remove".to_string(),
            workspace_path: PathBuf::from("/tmp/test"),
            created_at: chrono::Utc::now().to_rfc3339(),
            governance: alan_protocol::GovernanceConfig::default(),
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            rollout_path: None,
        };

        store.save(binding).unwrap();
        assert!(store.exists("to-remove"));

        store.remove("to-remove").unwrap();
        assert!(!store.exists("to-remove"));
        assert!(store.load("to-remove").is_none());
    }

    #[test]
    fn test_list_all() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        // Create multiple sessions.
        for i in 0..3 {
            let binding = SessionBinding {
                session_id: format!("session-{}", i),
                workspace_path: PathBuf::from(format!("/tmp/ws-{}", i)),
                created_at: chrono::Utc::now().to_rfc3339(),
                governance: alan_protocol::GovernanceConfig::default(),
                streaming_mode: None,
                partial_stream_recovery_mode: None,
                rollout_path: None,
            };
            store.save(binding).unwrap();
        }

        let all = store.list_all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_workspace_path() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        let workspace_path = PathBuf::from("/tmp/my-workspace");
        let binding = SessionBinding {
            session_id: "ws-test".to_string(),
            workspace_path: workspace_path.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            governance: alan_protocol::GovernanceConfig::default(),
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            rollout_path: None,
        };

        store.save(binding).unwrap();

        let retrieved = store.get_workspace_path("ws-test").unwrap();
        assert_eq!(retrieved, workspace_path);
    }

    #[test]
    fn test_update_rollout_path() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        let binding = SessionBinding {
            session_id: "rollout-test".to_string(),
            workspace_path: PathBuf::from("/tmp/ws"),
            created_at: chrono::Utc::now().to_rfc3339(),
            governance: alan_protocol::GovernanceConfig::default(),
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            rollout_path: None,
        };

        store.save(binding).unwrap();

        let new_rollout = Some(PathBuf::from("/tmp/rollout.jsonl"));
        store
            .update_rollout_path("rollout-test", new_rollout.clone())
            .unwrap();

        let loaded = store.load("rollout-test").unwrap();
        assert_eq!(loaded.rollout_path, new_rollout);
    }

    #[test]
    fn test_load_nonexistent() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        assert!(store.load("nonexistent").is_none());
        assert!(store.get_workspace_path("nonexistent").is_none());
    }

    #[test]
    fn test_persistence() {
        let temp = TempDir::new().unwrap();
        let storage_dir = temp.path().to_path_buf();

        // The first store instance saves data.
        {
            let store = SessionStore::with_dir(storage_dir.clone()).unwrap();
            let binding = SessionBinding {
                session_id: "persistent".to_string(),
                workspace_path: PathBuf::from("/tmp/persistent-ws"),
                created_at: chrono::Utc::now().to_rfc3339(),
                governance: alan_protocol::GovernanceConfig::default(),
                streaming_mode: None,
                partial_stream_recovery_mode: None,
                rollout_path: None,
            };
            store.save(binding).unwrap();
        }

        // The second store instance should load persisted data.
        {
            let store = SessionStore::with_dir(storage_dir).unwrap();
            let loaded = store.load("persistent").unwrap();
            assert_eq!(loaded.session_id, "persistent");
            assert_eq!(loaded.workspace_path, PathBuf::from("/tmp/persistent-ws"));
        }
    }
}
