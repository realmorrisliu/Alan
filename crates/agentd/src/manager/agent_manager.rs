//! Agent manager - manages multiple agent instances.

use super::instance::AgentInstance;
use alan_runtime::manager::{AgentInfo, AgentState, AgentStatus};
use alan_runtime::runtime::{AgentRuntimeConfig, AgentRuntimeHandle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for the agent manager
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Base directory for all agent data
    pub base_dir: PathBuf,
    /// Maximum number of concurrently running agent runtimes in this process
    pub max_instances: usize,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            base_dir: default_agents_dir(),
            max_instances: 10,
        }
    }
}

impl ManagerConfig {
    /// Create config with custom base directory
    #[allow(dead_code)]
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            max_instances: 10,
        }
    }
}

/// Manages multiple agent instances
pub struct AgentManager {
    config: ManagerConfig,
    /// Active instances (may be running or paused) - wrapped in Arc<RwLock> for shared access
    instances: Arc<RwLock<HashMap<String, Arc<RwLock<AgentInstance>>>>>,
    /// Base runtime config template for creating/loading agents
    /// This ensures consistent configuration across agent lifecycle
    base_runtime_config: AgentRuntimeConfig,
    /// Serializes runtime starts so max_instances is enforced consistently
    start_lock: Arc<Mutex<()>>,
}

impl AgentManager {
    /// Create a new agent manager
    #[allow(dead_code)]
    pub fn new(config: ManagerConfig) -> Self {
        // Ensure base directory exists
        if let Err(e) = std::fs::create_dir_all(&config.base_dir) {
            warn!("Failed to create agents directory: {}", e);
        }

        Self {
            config,
            instances: Arc::new(RwLock::new(HashMap::new())),
            base_runtime_config: AgentRuntimeConfig::default(),
            start_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Create a new agent manager with custom runtime config template
    pub fn with_runtime_config(config: ManagerConfig, runtime_config: AgentRuntimeConfig) -> Self {
        // Ensure base directory exists
        if let Err(e) = std::fs::create_dir_all(&config.base_dir) {
            warn!("Failed to create agents directory: {}", e);
        }

        Self {
            config,
            instances: Arc::new(RwLock::new(HashMap::new())),
            base_runtime_config: runtime_config,
            start_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Create a new agent manager with default config
    #[allow(dead_code)]
    pub fn with_default_config() -> Self {
        Self::new(ManagerConfig::default())
    }

    /// Create a new agent instance
    pub async fn create(&self, runtime_config: AgentRuntimeConfig) -> anyhow::Result<String> {
        let agent_id = format!(
            "agent-{}",
            Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        info!(agent_id = %agent_id, "Creating new agent");

        // Create agent directory structure
        let agent_dir = self.agent_dir(&agent_id);
        Self::create_agent_directory(&agent_dir)?;

        // Ensure runtime config has agent_id and workspace_dir
        let mut runtime_config = runtime_config;
        runtime_config.agent_id = agent_id.clone();
        runtime_config.workspace_dir = Some(agent_dir.clone());

        // Create instance (not started yet)
        let instance =
            AgentInstance::new(agent_id.clone(), agent_dir.clone(), runtime_config.clone());

        // Apply runtime config to state and save
        {
            let mut state = instance.state.write().await;
            state.apply_runtime_config(&runtime_config);
            state.save(&agent_dir)?;
        }

        // Register instance
        {
            let mut instances = self.instances.write().await;
            instances.insert(agent_id.clone(), Arc::new(RwLock::new(instance)));
        }

        info!(agent_id = %agent_id, "Agent created successfully");
        Ok(agent_id)
    }

    /// Create a new agent and start it immediately
    pub async fn create_and_start(
        &self,
        runtime_config: AgentRuntimeConfig,
    ) -> anyhow::Result<String> {
        let agent_id = self.create(runtime_config).await?;
        self.start(&agent_id).await?;
        Ok(agent_id)
    }

    /// Get an agent instance (loads from disk if not in memory)
    /// Returns Arc<RwLock<AgentInstance>> for shared access
    pub async fn get(&self, agent_id: &str) -> anyhow::Result<Arc<RwLock<AgentInstance>>> {
        // Check if already loaded
        {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.get(agent_id) {
                let instance = Arc::clone(instance);
                drop(instances);
                if let Err(err) = self.reconcile_instance_liveness(&instance).await {
                    warn!(agent_id = %agent_id, error = %err, "Failed to reconcile instance liveness");
                }
                return Ok(instance);
            }
        }

        // Load from disk
        let agent_dir = self.agent_dir(agent_id);
        if !agent_dir.exists() {
            anyhow::bail!("Agent {} not found", agent_id);
        }

        debug!(agent_id = %agent_id, "Loading agent from disk");

        // Load state first to get the persisted config
        let state = AgentState::load(&agent_dir)?;

        // Create runtime config from base template, then apply persisted settings
        // This ensures provider/model/timeout settings are preserved across restarts
        let mut runtime_config = self.base_runtime_config.clone();
        runtime_config.agent_id = agent_id.to_string();
        runtime_config.workspace_dir = Some(agent_dir.clone());

        // Apply persisted runtime config settings
        runtime_config.apply_persisted_state(&state.config);

        let instance = AgentInstance::load(agent_dir, runtime_config).await?;
        let instance = Arc::new(RwLock::new(instance));

        // Cache in memory
        {
            let mut instances = self.instances.write().await;
            instances.insert(agent_id.to_string(), Arc::clone(&instance));
        }

        Ok(instance)
    }

    /// Get runtime handle for an agent (must be running)
    pub async fn get_handle(&self, agent_id: &str) -> anyhow::Result<AgentRuntimeHandle> {
        // Auto-start if paused/stopped, while enforcing max concurrent running runtimes.
        self.ensure_running(agent_id).await?;
        let instance_arc = self.get(agent_id).await?;
        let instance = instance_arc.write().await;
        instance
            .handle()
            .ok_or_else(|| anyhow::anyhow!("Agent {} runtime not available", agent_id))
    }

    /// Start a paused agent
    pub async fn start(&self, agent_id: &str) -> anyhow::Result<()> {
        self.ensure_running(agent_id).await
    }

    /// Pause a running agent
    #[allow(dead_code)]
    pub async fn pause(&self, agent_id: &str) -> anyhow::Result<()> {
        let instance = self.get(agent_id).await?;
        let mut instance = instance.write().await;
        instance.pause().await?;
        Ok(())
    }

    /// Destroy an agent (removes all data)
    ///
    /// First pauses the agent to ensure runtime is stopped, then removes data.
    /// Returns Ok if the agent doesn't exist (idempotent).
    /// Returns Err if runtime fails to stop (to prevent data corruption).
    pub async fn destroy(&self, agent_id: &str) -> anyhow::Result<()> {
        info!(agent_id = %agent_id, "Destroying agent");

        // Check if agent exists first (for idempotency)
        if !self.exists(agent_id) {
            return Ok(());
        }

        // Get the instance and pause it first to ensure runtime is stopped
        let pause_result = match self.get(agent_id).await {
            Ok(instance) => {
                let mut instance = instance.write().await;
                // Set status to Destroying before pausing
                if let Err(e) = instance.set_status(AgentStatus::Destroying).await {
                    warn!(agent_id = %agent_id, error = %e, "Failed to set Destroying status");
                }
                // Pause will gracefully shutdown the runtime
                instance.pause().await
            }
            Err(e) => {
                warn!(agent_id = %agent_id, error = %e, "Failed to get agent for destruction");
                Err(e)
            }
        };

        // If pause failed, don't continue with deletion to avoid data corruption
        if let Err(ref e) = pause_result {
            warn!(agent_id = %agent_id, error = %e, "Failed to pause agent, aborting destroy");
            return Err(anyhow::anyhow!(
                "Cannot destroy agent {}: failed to stop runtime. Error: {}",
                agent_id,
                e
            ));
        }

        // Now safe to remove from memory
        {
            let mut instances = self.instances.write().await;
            instances.remove(agent_id);
        }

        // Remove directory
        let agent_dir = self.agent_dir(agent_id);
        if agent_dir.exists() {
            std::fs::remove_dir_all(&agent_dir)?;
        }

        info!(agent_id = %agent_id, "Agent destroyed");
        Ok(())
    }

    /// List all agents
    pub async fn list(&self) -> Vec<AgentInfo> {
        let mut infos = Vec::new();

        // First, scan disk for all agents
        let entries = match std::fs::read_dir(&self.config.base_dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read agents directory: {}", e);
                return infos;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            if let Some(agent_id) = path.file_name().and_then(|n| n.to_str()) {
                match self.get_agent_info(agent_id, &path).await {
                    Ok(info) => infos.push(info),
                    Err(e) => debug!("Failed to load agent {}: {}", agent_id, e),
                }
            }
        }

        // Sort by creation time (newest first)
        infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        infos
    }

    /// Get info for a specific agent
    #[allow(dead_code)]
    pub async fn get_info(&self, agent_id: &str) -> anyhow::Result<AgentInfo> {
        let agent_dir = self.agent_dir(agent_id);
        if !agent_dir.exists() {
            anyhow::bail!("Agent {} not found", agent_id);
        }
        self.get_agent_info(agent_id, &agent_dir).await
    }

    /// Check if an agent exists
    pub fn exists(&self, agent_id: &str) -> bool {
        self.agent_dir(agent_id).exists()
    }

    /// Get the number of managed instances
    #[allow(dead_code)]
    pub async fn count(&self) -> usize {
        self.instances.read().await.len()
    }

    async fn ensure_running(&self, agent_id: &str) -> anyhow::Result<()> {
        let _start_guard = self.start_lock.lock().await;

        let target = self.get(agent_id).await?;
        {
            let mut instance = target.write().await;
            instance.reconcile_runtime_state().await?;
            if instance.is_running() {
                return Ok(());
            }
        }

        let running_count = self.count_running_instances().await;
        if running_count >= self.config.max_instances {
            anyhow::bail!(
                "Maximum number of concurrently running agents ({}) reached",
                self.config.max_instances
            );
        }

        let mut instance = target.write().await;
        instance.reconcile_runtime_state().await?;
        if instance.is_running() {
            return Ok(());
        }
        instance.start().await
    }

    async fn count_running_instances(&self) -> usize {
        let loaded_instances: Vec<_> = {
            let instances = self.instances.read().await;
            instances.values().cloned().collect()
        };

        let mut running = 0usize;
        for instance in loaded_instances {
            let mut guard = instance.write().await;
            if let Err(err) = guard.reconcile_runtime_state().await {
                warn!(error = %err, "Failed to reconcile instance while counting running instances");
            }
            if guard.is_running() {
                running += 1;
            }
        }
        running
    }

    async fn reconcile_instance_liveness(
        &self,
        instance: &Arc<RwLock<AgentInstance>>,
    ) -> anyhow::Result<()> {
        let mut instance = instance.write().await;
        instance.reconcile_runtime_state().await
    }

    /// Get agent directory path
    pub fn agent_dir(&self, agent_id: &str) -> PathBuf {
        self.config.base_dir.join(agent_id)
    }

    /// Create agent directory structure
    pub fn create_agent_directory(agent_dir: &PathBuf) -> anyhow::Result<()> {
        std::fs::create_dir_all(agent_dir)?;
        std::fs::create_dir_all(agent_dir.join("workspace"))?;
        std::fs::create_dir_all(agent_dir.join("memory"))?;
        std::fs::create_dir_all(agent_dir.join("sessions"))?;
        std::fs::create_dir_all(agent_dir.join("workspace/skills"))?;

        // Create empty MEMORY.md
        let memory_md = agent_dir.join("memory/MEMORY.md");
        std::fs::write(&memory_md, "")?;

        Ok(())
    }

    /// Get agent info from directory
    async fn get_agent_info(&self, agent_id: &str, agent_dir: &Path) -> anyhow::Result<AgentInfo> {
        let state = if let Some(instance) = {
            let instances = self.instances.read().await;
            instances.get(agent_id).cloned()
        } {
            if let Err(err) = self.reconcile_instance_liveness(&instance).await {
                warn!(agent_id = %agent_id, error = %err, "Failed to reconcile agent before reading info");
            }
            let instance_guard = instance.read().await;
            let state_guard = instance_guard.state.read().await;
            state_guard.clone()
        } else {
            AgentState::load(agent_dir)?
        };

        // Count sessions
        let session_count = std::fs::read_dir(agent_dir.join("sessions"))?
            .filter(|e| {
                e.as_ref()
                    .map(|entry| {
                        entry
                            .path()
                            .extension()
                            .map(|ext| ext == "jsonl")
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            })
            .count();

        Ok(AgentInfo {
            id: agent_id.to_string(),
            status: state.status,
            created_at: state.created_at,
            last_active: state.last_active,
            session_count,
        })
    }
}

fn default_agents_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".alan/agents")
    } else {
        PathBuf::from(".alan/agents")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alan_runtime::Config;
    use tempfile::TempDir;

    fn test_manager() -> (AgentManager, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = ManagerConfig::with_base_dir(temp.path().to_path_buf());
        let manager = AgentManager::new(config);
        (manager, temp)
    }

    fn test_runtime_config() -> AgentRuntimeConfig {
        AgentRuntimeConfig::from(Config::default())
    }

    #[tokio::test]
    async fn test_create_agent() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        assert!(agent_id.starts_with("agent-"));
        assert!(manager.exists(&agent_id));
    }

    #[tokio::test]
    async fn test_create_and_set_status() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        assert!(manager.exists(&agent_id));

        // Set status directly (without starting runtime)
        let instance = manager.get(&agent_id).await.unwrap();
        instance
            .write()
            .await
            .set_status(AgentStatus::Running)
            .await
            .unwrap();

        // Verify status
        let instance = instance.read().await;
        assert_eq!(instance.status().await, AgentStatus::Running);
    }

    #[tokio::test]
    async fn test_status_transitions() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        // Get instance
        let instance = manager.get(&agent_id).await.unwrap();

        // Idle -> Running
        instance
            .write()
            .await
            .set_status(AgentStatus::Running)
            .await
            .unwrap();
        let inst = instance.read().await;
        assert_eq!(inst.status().await, AgentStatus::Running);
        drop(inst);

        // Running -> Paused
        instance
            .write()
            .await
            .set_status(AgentStatus::Paused)
            .await
            .unwrap();
        let inst = instance.read().await;
        assert_eq!(inst.status().await, AgentStatus::Paused);
        drop(inst);

        // Paused -> Running
        instance
            .write()
            .await
            .set_status(AgentStatus::Running)
            .await
            .unwrap();
        let inst = instance.read().await;
        assert_eq!(inst.status().await, AgentStatus::Running);
    }

    #[tokio::test]
    async fn test_destroy() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();
        assert!(manager.exists(&agent_id));

        manager.destroy(&agent_id).await.unwrap();
        assert!(!manager.exists(&agent_id));
    }

    #[tokio::test]
    async fn test_list_agents() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        // Create a few agents
        let id1 = manager.create(runtime_config.clone()).await.unwrap();
        let id2 = manager.create(runtime_config.clone()).await.unwrap();

        let list = manager.list().await;
        assert_eq!(list.len(), 2);

        let ids: Vec<_> = list.iter().map(|a| &a.id).collect();
        assert!(ids.contains(&&id1));
        assert!(ids.contains(&&id2));
    }

    #[tokio::test]
    async fn test_max_instances() {
        let temp = TempDir::new().unwrap();
        let config = ManagerConfig {
            base_dir: temp.path().to_path_buf(),
            max_instances: 2,
        };
        let manager = AgentManager::new(config);
        let runtime_config = test_runtime_config();

        // Create up to max
        manager.create(runtime_config.clone()).await.unwrap();
        manager.create(runtime_config.clone()).await.unwrap();

        // Creation is allowed beyond the concurrent-running limit.
        let result = manager.create(runtime_config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_respects_max_running_instances_limit() {
        let temp = TempDir::new().unwrap();
        let config = ManagerConfig {
            base_dir: temp.path().to_path_buf(),
            max_instances: 0,
        };
        let manager = AgentManager::new(config);
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        let result = manager.start(&agent_id).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("concurrently running agents")
        );
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let (manager, _temp) = test_manager();

        let result = manager.get("nonexistent-agent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_info_nonexistent() {
        let (manager, _temp) = test_manager();

        let result = manager.get_info("nonexistent-agent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_info_existing() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        let info = manager.get_info(&agent_id).await.unwrap();
        assert_eq!(info.id, agent_id);
    }

    #[tokio::test]
    async fn test_count() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        assert_eq!(manager.count().await, 0);

        let _id1 = manager.create(runtime_config.clone()).await.unwrap();
        assert_eq!(manager.count().await, 1);

        let _id2 = manager.create(runtime_config.clone()).await.unwrap();
        assert_eq!(manager.count().await, 2);
    }

    #[tokio::test]
    async fn test_exists() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        assert!(manager.exists(&agent_id));
        assert!(!manager.exists("nonexistent-agent"));
    }

    #[tokio::test]
    async fn test_create_and_start() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let _ = manager.create_and_start(runtime_config).await;

        // create_and_start() creates first, then starts.
        // Even if start fails (e.g. missing LLM config), the instance should exist.
        assert_eq!(manager.count().await, 1);
    }

    #[tokio::test]
    async fn test_start_existing() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        let _ = manager.start(&agent_id).await;
        let instance = manager.get(&agent_id).await.unwrap();
        let status = instance.read().await.status().await;
        assert!(matches!(status, AgentStatus::Running | AgentStatus::Error));
    }

    #[tokio::test]
    async fn test_pause_existing() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        let result = manager.pause(&agent_id).await;
        assert!(result.is_ok());
        let instance = manager.get(&agent_id).await.unwrap();
        assert_eq!(instance.read().await.status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_get_handle_loads_instance_from_disk() {
        let temp = TempDir::new().unwrap();
        let config = ManagerConfig::with_base_dir(temp.path().to_path_buf());
        let manager1 = AgentManager::new(config.clone());
        let runtime_config = test_runtime_config();

        let agent_id = manager1.create(runtime_config).await.unwrap();
        assert_eq!(manager1.count().await, 1);
        drop(manager1);

        let manager2 = AgentManager::new(config);
        assert_eq!(manager2.count().await, 0);

        // Startup may fail due missing LLM config; we only care that get_handle()
        // no longer fails early with "not found" when agent exists on disk.
        let _ = manager2.get_handle(&agent_id).await;
        assert_eq!(manager2.count().await, 1);
    }

    #[tokio::test]
    async fn test_manager_config_default() {
        let config = ManagerConfig::default();
        assert!(config.base_dir.to_string_lossy().contains(".alan"));
        assert_eq!(config.max_instances, 10);
    }

    #[tokio::test]
    async fn test_manager_config_with_base_dir() {
        let temp = TempDir::new().unwrap();
        let config = ManagerConfig::with_base_dir(temp.path().to_path_buf());
        assert_eq!(config.base_dir, temp.path());
        assert_eq!(config.max_instances, 10);
    }

    #[tokio::test]
    async fn test_agent_manager_with_default_config() {
        let temp = TempDir::new().unwrap();
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", temp.path());
            std::env::set_var("ALAN_WORKSPACE_DIR", temp.path());
        }

        let manager = AgentManager::with_default_config();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        assert!(manager.exists(&agent_id));
        unsafe {
            std::env::remove_var("ALAN_WORKSPACE_DIR");
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[tokio::test]
    async fn test_list_empty_directory() {
        let (manager, _temp) = test_manager();
        let list = manager.list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_destroy_nonexistent() {
        let (manager, _temp) = test_manager();
        // Destroying non-existent agent should not fail (idempotent)
        let result = manager.destroy("nonexistent-agent").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_agents_dir() {
        let dir = default_agents_dir();
        assert!(
            dir.to_string_lossy().contains(".alan/agents")
                || dir.to_string_lossy().contains(".alan\\agents")
        );
    }

    #[tokio::test]
    async fn test_create_agent_directory_structure() {
        let temp = TempDir::new().unwrap();
        let agent_dir = temp.path().join("test-agent");

        AgentManager::create_agent_directory(&agent_dir).unwrap();

        assert!(agent_dir.exists());
        assert!(agent_dir.join("workspace").exists());
        assert!(agent_dir.join("memory").exists());
        assert!(agent_dir.join("sessions").exists());
        assert!(agent_dir.join("workspace/skills").exists());
        assert!(agent_dir.join("memory/MEMORY.md").exists());
    }

    #[tokio::test]
    async fn test_get_agent_info_with_sessions() {
        let (manager, _temp) = test_manager();
        let runtime_config = test_runtime_config();

        let agent_id = manager.create(runtime_config).await.unwrap();

        // Create a mock session file
        let agent_dir = manager.agent_dir(&agent_id);
        let sessions_dir = agent_dir.join("sessions");
        std::fs::write(sessions_dir.join("test-session.jsonl"), "").unwrap();

        let info = manager.get_info(&agent_id).await.unwrap();
        assert_eq!(info.session_count, 1);
    }
}
