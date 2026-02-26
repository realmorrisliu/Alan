//! Session Store — Session 与 Workspace 绑定持久化。
//!
//! 存储 session 到 workspace 路径的映射，支持 daemon 重启后的恢复。
//!
//! 存储位置: ~/.alan/sessions/<session_id>.json

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Session 绑定信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBinding {
    /// Session ID
    pub session_id: String,
    /// Workspace 路径
    pub workspace_path: PathBuf,
    /// 创建时间
    pub created_at: String,
    /// Approval policy
    #[serde(default)]
    pub approval_policy: alan_protocol::ApprovalPolicy,
    /// Sandbox mode
    #[serde(default)]
    pub sandbox_mode: alan_protocol::SandboxMode,
    /// Rollout 文件路径（如果存在）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollout_path: Option<PathBuf>,
}

/// Session 存储
#[derive(Debug)]
pub struct SessionStore {
    storage_dir: PathBuf,
    /// 内存缓存
    cache: std::sync::RwLock<HashMap<String, SessionBinding>>,
}

impl SessionStore {
    /// 创建新的 SessionStore
    pub fn new() -> Result<Self> {
        let storage_dir = Self::default_storage_dir()?;
        std::fs::create_dir_all(&storage_dir)?;

        let cache = std::sync::RwLock::new(HashMap::new());
        let store = Self { storage_dir, cache };

        // 加载所有持久化的 session
        store.load_all()?;

        Ok(store)
    }

    /// 使用指定的存储目录创建（用于测试）
    #[cfg(test)]
    pub fn with_dir(storage_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_dir)?;
        Ok(Self {
            storage_dir,
            cache: std::sync::RwLock::new(HashMap::new()),
        })
    }

    /// 默认存储目录
    fn default_storage_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".alan").join("sessions"))
    }

    /// 获取 session 文件路径
    fn session_file_path(&self, session_id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", session_id))
    }

    /// 保存 session 绑定
    pub fn save(&self, binding: SessionBinding) -> Result<()> {
        let session_id = binding.session_id.clone();
        let path = self.session_file_path(&session_id);

        // 序列化并写入
        let content = serde_json::to_string_pretty(&binding)?;
        std::fs::write(&path, content)?;

        // 更新缓存
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(session_id.clone(), binding);
        }

        debug!(%session_id, path = %path.display(), "Saved session binding");
        Ok(())
    }

    /// 加载指定 session
    #[allow(dead_code)]
    pub fn load(&self, session_id: &str) -> Option<SessionBinding> {
        // 先查缓存
        if let Ok(cache) = self.cache.read()
            && let Some(binding) = cache.get(session_id)
        {
            return Some(binding.clone());
        }

        // 从磁盘加载
        let path = self.session_file_path(session_id);
        if !path.exists() {
            return None;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<SessionBinding>(&content) {
                    Ok(binding) => {
                        // 更新缓存
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

    /// 删除 session 绑定
    pub fn remove(&self, session_id: &str) -> Result<()> {
        let path = self.session_file_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        // 更新缓存
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(session_id);
        }

        debug!(%session_id, "Removed session binding");
        Ok(())
    }

    /// 列出所有 session
    pub fn list_all(&self) -> Vec<SessionBinding> {
        // 刷新缓存
        let _ = self.load_all();

        if let Ok(cache) = self.cache.read() {
            cache.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 列出活跃 session（绑定存在且 workspace 路径有效）
    pub fn list_active(&self) -> Vec<SessionBinding> {
        self.list_all()
            .into_iter()
            .filter(|b| b.workspace_path.exists())
            .collect()
    }

    /// 检查 session 是否存在
    #[allow(dead_code)]
    pub fn exists(&self, session_id: &str) -> bool {
        // 先查缓存
        if let Ok(cache) = self.cache.read()
            && cache.contains_key(session_id)
        {
            return true;
        }

        // 查文件
        self.session_file_path(session_id).exists()
    }

    /// 更新 rollout 路径
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

    /// 获取 workspace 路径
    #[allow(dead_code)]
    pub fn get_workspace_path(&self, session_id: &str) -> Option<PathBuf> {
        self.load(session_id).map(|b| b.workspace_path)
    }

    /// 加载所有 session 到缓存
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

    /// 清理无效的 session（workspace 路径不存在）
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

        // 初始为空
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
            approval_policy: alan_protocol::ApprovalPolicy::OnRequest,
            sandbox_mode: alan_protocol::SandboxMode::WorkspaceWrite,
            rollout_path: None,
        };

        store.save(binding.clone()).unwrap();

        // 应该能加载回来
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
            approval_policy: alan_protocol::ApprovalPolicy::default(),
            sandbox_mode: alan_protocol::SandboxMode::default(),
            rollout_path: None,
        };

        store.save(binding).unwrap();
        assert!(store.exists("exists-session"));
    }

    #[test]
    fn test_remove() {
        let temp = TempDir::new().unwrap();
        let store = SessionStore::with_dir(temp.path().to_path_buf()).unwrap();

        let binding = SessionBinding {
            session_id: "to-remove".to_string(),
            workspace_path: PathBuf::from("/tmp/test"),
            created_at: chrono::Utc::now().to_rfc3339(),
            approval_policy: alan_protocol::ApprovalPolicy::default(),
            sandbox_mode: alan_protocol::SandboxMode::default(),
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

        // 创建多个 session
        for i in 0..3 {
            let binding = SessionBinding {
                session_id: format!("session-{}", i),
                workspace_path: PathBuf::from(format!("/tmp/ws-{}", i)),
                created_at: chrono::Utc::now().to_rfc3339(),
                approval_policy: alan_protocol::ApprovalPolicy::default(),
                sandbox_mode: alan_protocol::SandboxMode::default(),
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
            approval_policy: alan_protocol::ApprovalPolicy::default(),
            sandbox_mode: alan_protocol::SandboxMode::default(),
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
            approval_policy: alan_protocol::ApprovalPolicy::default(),
            sandbox_mode: alan_protocol::SandboxMode::default(),
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

        // 第一个 store 实例保存数据
        {
            let store = SessionStore::with_dir(storage_dir.clone()).unwrap();
            let binding = SessionBinding {
                session_id: "persistent".to_string(),
                workspace_path: PathBuf::from("/tmp/persistent-ws"),
                created_at: chrono::Utc::now().to_rfc3339(),
                approval_policy: alan_protocol::ApprovalPolicy::default(),
                sandbox_mode: alan_protocol::SandboxMode::default(),
                rollout_path: None,
            };
            store.save(binding).unwrap();
        }

        // 第二个 store 实例应该能加载数据
        {
            let store = SessionStore::with_dir(storage_dir).unwrap();
            let loaded = store.load("persistent").unwrap();
            assert_eq!(loaded.session_id, "persistent");
            assert_eq!(loaded.workspace_path, PathBuf::from("/tmp/persistent-ws"));
        }
    }
}
