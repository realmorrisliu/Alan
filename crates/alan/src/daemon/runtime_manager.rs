//! Runtime Manager — 直接管理 RuntimeController 实例。
//!
//! 替代旧的 WorkspaceManager，移除 WorkspaceInstance 中间层，
//! 直接管理 session -> runtime 的映射。

use alan_runtime::runtime::{
    RuntimeController, RuntimeHandle, WorkspaceRuntimeConfig, spawn_with_tool_registry,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 运行中的 runtime 条目
struct RuntimeEntry {
    /// 关联的 session ID
    #[allow(dead_code)]
    session_id: String,
    /// Workspace 路径
    #[allow(dead_code)]
    workspace_path: PathBuf,
    /// Runtime 控制器
    controller: RuntimeController,
    /// 创建时间
    #[allow(dead_code)]
    created_at: Instant,
    /// 最后活动时间
    #[allow(dead_code)]
    last_activity: Instant,
}

/// Runtime 管理器配置
#[derive(Debug, Clone)]
pub struct RuntimeManagerConfig {
    /// 最大并发 runtime 数量
    pub max_concurrent_runtimes: usize,
    /// 全局配置模板
    pub runtime_config_template: WorkspaceRuntimeConfig,
}

impl Default for RuntimeManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_runtimes: 10,
            runtime_config_template: WorkspaceRuntimeConfig::default(),
        }
    }
}

/// Runtime 管理器
///
/// 直接管理 Session -> RuntimeController 的映射，
/// 不再经过 WorkspaceInstance 中间层。
pub struct RuntimeManager {
    config: RuntimeManagerConfig,
    /// session_id -> RuntimeEntry
    runtimes: RwLock<HashMap<String, RuntimeEntry>>,
}

impl RuntimeManager {
    /// 创建新的 RuntimeManager
    pub fn new(config: RuntimeManagerConfig) -> Self {
        Self {
            config,
            runtimes: RwLock::new(HashMap::new()),
        }
    }

    /// 使用默认配置创建
    #[allow(dead_code, clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(RuntimeManagerConfig::default())
    }

    /// 使用模板配置创建
    pub fn with_template(template: WorkspaceRuntimeConfig) -> Self {
        let config = RuntimeManagerConfig {
            runtime_config_template: template,
            ..Default::default()
        };
        Self::new(config)
    }

    /// 启动一个新的 runtime
    ///
    /// # Arguments
    /// * `session_id` - Session ID
    /// * `workspace_path` - Workspace 路径（用于 session 存储）
    /// * `resume_rollout_path` - 可选的 rollout 恢复路径
    ///
    /// # Returns
    /// * `Ok(RuntimeHandle)` - Runtime 启动成功
    /// * `Err(...)` - 启动失败或超过最大并发数
    pub async fn start_runtime(
        &self,
        session_id: String,
        workspace_path: PathBuf,
        resume_rollout_path: Option<PathBuf>,
    ) -> anyhow::Result<RuntimeHandle> {
        // 检查是否已存在
        {
            let runtimes = self.runtimes.read().await;
            if let Some(entry) = runtimes.get(&session_id)
                && !entry.controller.is_finished()
            {
                debug!(%session_id, "Runtime already exists and running");
                return Ok(entry.controller.handle.clone());
            }
        }

        // 检查并发限制
        let current_count = self.runtime_count().await;
        if current_count >= self.config.max_concurrent_runtimes {
            anyhow::bail!(
                "Maximum number of concurrent runtimes ({}) reached",
                self.config.max_concurrent_runtimes
            );
        }

        info!(%session_id, path = %workspace_path.display(), "Starting runtime");

        // 构建 runtime 配置
        let mut runtime_config = self.config.runtime_config_template.clone();
        runtime_config.workspace_id = session_id.clone();
        runtime_config.workspace_dir = Some(workspace_path.clone());
        runtime_config.resume_rollout_path = resume_rollout_path;

        let mut tools = alan_runtime::tools::ToolRegistry::with_config(Arc::new(
            runtime_config.agent_config.core_config.clone(),
        ));
        for tool in alan_tools::create_core_tools(workspace_path.clone()) {
            tools.register_boxed(tool);
        }

        // 启动 runtime
        let mut controller = spawn_with_tool_registry(runtime_config, tools)?;

        // 等待启动完成
        if let Err(err) = controller.wait_until_ready().await {
            controller.abort().await;
            return Err(err);
        }

        let handle = controller.handle.clone();

        // 存储 runtime 条目
        let entry = RuntimeEntry {
            session_id: session_id.clone(),
            workspace_path,
            controller,
            created_at: Instant::now(),
            last_activity: Instant::now(),
        };

        let mut runtimes = self.runtimes.write().await;
        runtimes.insert(session_id.clone(), entry);

        info!(%session_id, "Runtime started successfully");
        Ok(handle)
    }

    /// 获取 runtime handle
    ///
    /// 如果 runtime 不存在或已停止，返回错误
    pub async fn get_handle(&self, session_id: &str) -> anyhow::Result<RuntimeHandle> {
        let runtimes = self.runtimes.read().await;
        match runtimes.get(session_id) {
            Some(entry) => {
                if entry.controller.is_finished() {
                    anyhow::bail!("Runtime for session {} has stopped", session_id);
                }
                Ok(entry.controller.handle.clone())
            }
            None => anyhow::bail!("Runtime for session {} not found", session_id),
        }
    }

    /// 停止 runtime
    ///
    /// 停止一个特定的 runtime
    #[allow(dead_code)]
    pub async fn stop_runtime(&self, session_id: &str) -> anyhow::Result<()> {
        let controller = {
            let mut runtimes = self.runtimes.write().await;
            match runtimes.remove(session_id) {
                Some(entry) => {
                    info!(%session_id, "Stopping runtime");
                    entry.controller
                }
                None => {
                    debug!(%session_id, "Runtime not found, already stopped?");
                    return Ok(());
                }
            }
        };

        // 在锁外执行关闭
        match controller.shutdown().await {
            Ok(()) => {
                info!(%session_id, "Runtime stopped gracefully");
                Ok(())
            }
            Err(err) => {
                error!(%session_id, error = %err, "Runtime shutdown failed");
                Err(err)
            }
        }
    }

    /// 强制停止 runtime
    ///
    /// 立即中止，不等待优雅关闭
    #[allow(dead_code)]
    pub async fn abort_runtime(&self, session_id: &str) {
        let controller = {
            let mut runtimes = self.runtimes.write().await;
            runtimes.remove(session_id)
        };

        if let Some(entry) = controller {
            info!(%session_id, "Aborting runtime");
            entry.controller.abort().await;
        }
    }

    /// 检查 runtime 是否正在运行
    pub async fn is_running(&self, session_id: &str) -> bool {
        let runtimes = self.runtimes.read().await;
        match runtimes.get(session_id) {
            Some(entry) => !entry.controller.is_finished(),
            None => false,
        }
    }

    /// 获取当前 runtime 数量
    pub async fn runtime_count(&self) -> usize {
        // 清理已停止的 runtime
        self.cleanup_finished().await;
        self.runtimes.read().await.len()
    }

    /// 停止所有 runtime
    #[allow(dead_code)]
    pub async fn stop_all(&self) {
        let controllers: Vec<_> = {
            let mut runtimes = self.runtimes.write().await;
            runtimes
                .drain()
                .map(|(_, entry)| entry.controller)
                .collect()
        };

        info!(count = controllers.len(), "Stopping all runtimes");

        for controller in controllers {
            // 并发关闭所有 runtime
            let _ = controller.shutdown().await;
        }
    }

    /// 获取 runtime 的 workspace 路径
    #[allow(dead_code)]
    pub async fn get_workspace_path(&self, session_id: &str) -> Option<PathBuf> {
        let runtimes = self.runtimes.read().await;
        runtimes.get(session_id).map(|e| e.workspace_path.clone())
    }

    /// 更新最后活动时间
    #[allow(dead_code)]
    pub async fn touch(&self, session_id: &str) {
        let mut runtimes = self.runtimes.write().await;
        if let Some(entry) = runtimes.get_mut(session_id) {
            entry.last_activity = Instant::now();
        }
    }

    /// 获取 runtime 信息
    #[allow(dead_code)]
    pub async fn get_runtime_info(&self, session_id: &str) -> Option<RuntimeInfo> {
        let runtimes = self.runtimes.read().await;
        runtimes.get(session_id).map(|e| RuntimeInfo {
            session_id: e.session_id.clone(),
            workspace_path: e.workspace_path.clone(),
            created_at: e.created_at,
            last_activity: e.last_activity,
            is_running: !e.controller.is_finished(),
        })
    }

    /// 列出所有运行中的 runtimes
    #[allow(dead_code)]
    pub async fn list_runtimes(&self) -> Vec<RuntimeInfo> {
        self.cleanup_finished().await;
        let runtimes = self.runtimes.read().await;
        runtimes
            .values()
            .map(|e| RuntimeInfo {
                session_id: e.session_id.clone(),
                workspace_path: e.workspace_path.clone(),
                created_at: e.created_at,
                last_activity: e.last_activity,
                is_running: !e.controller.is_finished(),
            })
            .collect()
    }

    /// 清理已停止的 runtime
    async fn cleanup_finished(&self) {
        let to_remove: Vec<String> = {
            let runtimes = self.runtimes.read().await;
            runtimes
                .iter()
                .filter(|(_, entry)| entry.controller.is_finished())
                .map(|(id, _)| id.clone())
                .collect()
        };

        if !to_remove.is_empty() {
            let mut runtimes = self.runtimes.write().await;
            for id in to_remove {
                warn!(session_id = %id, "Removing finished runtime from registry");
                runtimes.remove(&id);
            }
        }
    }
}

/// Runtime 信息摘要
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RuntimeInfo {
    pub session_id: String,
    pub workspace_path: PathBuf,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_runtime_manager_new() {
        let manager = RuntimeManager::default();
        assert_eq!(manager.config.max_concurrent_runtimes, 10);
    }

    #[test]
    fn test_runtime_manager_with_config() {
        let config = RuntimeManagerConfig {
            max_concurrent_runtimes: 5,
            ..Default::default()
        };
        let manager = RuntimeManager::new(config);
        assert_eq!(manager.config.max_concurrent_runtimes, 5);
    }

    #[tokio::test]
    async fn test_runtime_count_empty() {
        let manager = RuntimeManager::default();
        assert_eq!(manager.runtime_count().await, 0);
    }

    #[tokio::test]
    async fn test_is_running_nonexistent() {
        let manager = RuntimeManager::default();
        assert!(!manager.is_running("nonexistent").await);
    }

    #[tokio::test]
    async fn test_get_handle_nonexistent() {
        let manager = RuntimeManager::default();
        let result = manager.get_handle("nonexistent").await;
        match result {
            Err(e) => {
                let err_msg = format!("{}", e);
                assert!(err_msg.contains("not found"));
            }
            Ok(_) => panic!("Expected error for nonexistent session"),
        }
    }

    #[tokio::test]
    async fn test_stop_runtime_nonexistent() {
        let manager = RuntimeManager::default();
        // 应该静默成功（幂等）
        let result = manager.stop_runtime("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_workspace_path() {
        let _temp = TempDir::new().unwrap();
        let manager = RuntimeManager::default();

        // 未启动时返回 None
        assert!(manager.get_workspace_path("test-session").await.is_none());
    }

    #[tokio::test]
    async fn test_touch_nonexistent() {
        let manager = RuntimeManager::default();
        // 应该静默成功
        manager.touch("nonexistent").await;
    }

    #[tokio::test]
    async fn test_list_runtimes_empty() {
        let manager = RuntimeManager::default();
        let list = manager.list_runtimes().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_runtime_info_nonexistent() {
        let manager = RuntimeManager::default();
        assert!(manager.get_runtime_info("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_limit() {
        let config = RuntimeManagerConfig {
            max_concurrent_runtimes: 0, // 设置为 0 以便测试限制
            ..Default::default()
        };
        let manager = RuntimeManager::new(config);

        let temp = TempDir::new().unwrap();
        let result = manager
            .start_runtime("test-session".to_string(), temp.path().to_path_buf(), None)
            .await;

        match result {
            Err(e) => {
                let err_msg = format!("{}", e);
                assert!(err_msg.contains("Maximum number of concurrent runtimes"));
            }
            Ok(_) => panic!("Expected error for concurrent limit"),
        }
    }

    #[tokio::test]
    async fn test_stop_all_empty() {
        let manager = RuntimeManager::default();
        // 应该静默成功
        manager.stop_all().await;
    }
}
