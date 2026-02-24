//! Agent instance - wraps a running agent runtime.

use alan_runtime::manager::{AgentState, AgentStatus};
use alan_runtime::runtime::{AgentRuntimeConfig, AgentRuntimeController, AgentRuntimeHandle, spawn};
use alan_protocol::Event;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// A running or paused agent instance
pub struct AgentInstance {
    /// Agent state (persistent)
    pub state: Arc<RwLock<AgentState>>,
    /// Agent workspace directory
    pub workspace_dir: PathBuf,
    /// Runtime controller (None if paused)
    runtime_controller: Option<AgentRuntimeController>,
    /// Background task that updates in-memory activity timestamp from runtime events
    activity_task_handle: Option<JoinHandle<()>>,
    /// Base configuration for spawning
    runtime_config: AgentRuntimeConfig,
}

impl AgentInstance {
    /// Create a new agent instance (does not start runtime yet)
    pub fn new(
        agent_id: String,
        workspace_dir: PathBuf,
        runtime_config: AgentRuntimeConfig,
    ) -> Self {
        let state = AgentState::new(agent_id);

        Self {
            state: Arc::new(RwLock::new(state)),
            workspace_dir,
            runtime_controller: None,
            activity_task_handle: None,
            runtime_config,
        }
    }

    /// Load an existing agent instance from disk
    pub async fn load(
        workspace_dir: PathBuf,
        runtime_config: AgentRuntimeConfig,
    ) -> anyhow::Result<Self> {
        let state = AgentState::load(&workspace_dir)?;
        let state = Arc::new(RwLock::new(state));

        Ok(Self {
            state,
            workspace_dir,
            runtime_controller: None,
            activity_task_handle: None,
            runtime_config,
        })
    }

    /// Get agent ID (async)
    pub async fn id(&self) -> String {
        let state = self.state.read().await;
        state.id.clone()
    }

    /// Get current status (async)
    #[allow(dead_code)]
    pub async fn status(&self) -> AgentStatus {
        let state = self.state.read().await;
        state.status
    }

    /// Check if runtime is active
    pub fn is_running(&self) -> bool {
        self.runtime_controller
            .as_ref()
            .map(|controller| !controller.is_finished())
            .unwrap_or(false)
    }

    /// Start or resume the agent runtime
    pub async fn start(&mut self) -> anyhow::Result<()> {
        let agent_id = self.id().await;

        if self.is_running() {
            debug!(agent_id = %agent_id, "Agent runtime already running");
            return Ok(());
        }

        info!(agent_id = %agent_id, "Starting agent runtime");

        // Prepare runtime config with agent-specific paths
        let mut config = self.runtime_config.clone();
        config.agent_id = agent_id.clone();
        config.workspace_dir = Some(self.workspace_dir.clone());

        // Spawn the runtime
        let mut controller = match spawn(config) {
            Ok(controller) => controller,
            Err(err) => {
                self.set_error(&err.to_string()).await?;
                return Err(err);
            }
        };

        // Wait for startup readiness before marking as running.
        if let Err(err) = controller.wait_until_ready().await {
            controller.abort().await;
            self.set_error(&err.to_string()).await?;
            return Err(err);
        }

        self.spawn_activity_monitor(&controller.handle);
        self.runtime_controller = Some(controller);

        // Save state
        {
            let mut state = self.state.write().await;
            state.status = AgentStatus::Running;
            state.touch();
        }
        self.save_state().await?;

        Ok(())
    }

    /// Pause the agent (graceful shutdown)
    ///
    /// Shuts down the runtime gracefully. The shutdown itself has a 10s timeout
    /// and will abort if exceeded. Returns Ok only if shutdown succeeds.
    /// Returns Err if shutdown fails (timeout/abort), and status is set to Error.
    pub async fn pause(&mut self) -> anyhow::Result<()> {
        if self.runtime_controller.is_none() {
            return Ok(());
        }

        let agent_id = self.id().await;
        info!(agent_id = %agent_id, "Pausing agent runtime");
        self.stop_activity_monitor();

        // Take ownership of controller and shutdown gracefully
        // The shutdown() method handles its own timeout and abort logic
        let shutdown_result = if let Some(controller) = self.runtime_controller.take() {
            controller.shutdown().await
        } else {
            return Ok(());
        };

        // Set status based on shutdown result
        let new_status = match &shutdown_result {
            Ok(_) => {
                info!(agent_id = %agent_id, "Runtime paused successfully");
                AgentStatus::Paused
            }
            Err(err) => {
                warn!(agent_id = %agent_id, error = %err, "Runtime shutdown failed");
                AgentStatus::Error
            }
        };

        {
            let mut state = self.state.write().await;
            state.status = new_status;
            state.touch();
        }

        self.save_state().await?;
        shutdown_result
    }

    /// Get runtime handle (if running)
    pub fn handle(&self) -> Option<AgentRuntimeHandle> {
        if self.is_running() {
            self.runtime_controller.as_ref().map(|c| c.handle.clone())
        } else {
            None
        }
    }

    /// Reconcile persisted status if the runtime task exited unexpectedly.
    pub async fn reconcile_runtime_state(&mut self) -> anyhow::Result<()> {
        let Some(controller) = self.runtime_controller.as_ref() else {
            return Ok(());
        };
        if !controller.is_finished() {
            return Ok(());
        }

        let agent_id = self.id().await;
        warn!(
            agent_id = %agent_id,
            "Runtime task already exited; marking instance as error and clearing controller"
        );

        self.runtime_controller.take();
        self.stop_activity_monitor();
        {
            let mut state = self.state.write().await;
            state.status = AgentStatus::Error;
            state.touch();
        }
        self.save_state().await?;
        Ok(())
    }

    /// Save state to disk
    async fn save_state(&self) -> anyhow::Result<()> {
        let state = self.state.read().await;
        state.save(&self.workspace_dir)?;
        Ok(())
    }

    fn spawn_activity_monitor(&mut self, handle: &AgentRuntimeHandle) {
        self.stop_activity_monitor();

        let mut rx = handle.event_sender.subscribe();
        let state = Arc::clone(&self.state);
        self.activity_task_handle = Some(tokio::spawn(async move {
            loop {
                let runtime_event = match rx.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                };

                if !counts_as_activity_event(&runtime_event.event) {
                    continue;
                }

                {
                    let mut guard = state.write().await;
                    guard.touch();
                }
            }
        }));
    }

    fn stop_activity_monitor(&mut self) {
        if let Some(handle) = self.activity_task_handle.take() {
            handle.abort();
        }
    }

    /// Update status and save
    pub async fn set_status(&self, status: AgentStatus) -> anyhow::Result<()> {
        {
            let mut state = self.state.write().await;
            state.status = status;
            state.touch();
        }
        self.save_state().await?;
        Ok(())
    }

    /// Set error status
    pub async fn set_error(&self, error: &str) -> anyhow::Result<()> {
        let agent_id = self.id().await;
        warn!(agent_id = %agent_id, error = %error, "Agent error");
        self.set_status(AgentStatus::Error).await?;
        Ok(())
    }
}

fn counts_as_activity_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Thinking { .. }
            | Event::ThinkingComplete {}
            | Event::StructuredUserInputRequested { .. }
            | Event::ConfirmationRequired { .. }
            | Event::ToolCallStarted { .. }
            | Event::ToolCallCompleted { .. }
            | Event::TaskCompleted { .. }
            | Event::ContextCompacted {}
            | Event::PlanUpdated { .. }
            | Event::SessionRolledBack { .. }
            | Event::Error { .. }
            | Event::SkillsLoaded { .. }
            | Event::DynamicToolsRegistered { .. }
            | Event::DynamicToolCallRequested { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alan_runtime::Config;
    use tempfile::TempDir;

    fn test_runtime_config() -> AgentRuntimeConfig {
        AgentRuntimeConfig::from(Config::default())
    }

    #[tokio::test]
    async fn test_agent_instance_new() {
        let temp = TempDir::new().unwrap();
        let config = test_runtime_config();

        let instance =
            AgentInstance::new("test-agent".to_string(), temp.path().to_path_buf(), config);

        assert_eq!(instance.id().await, "test-agent");
        assert_eq!(instance.status().await, AgentStatus::Idle);
        assert!(!instance.is_running());
    }

    #[tokio::test]
    async fn test_agent_instance_state_management() {
        let temp = TempDir::new().unwrap();
        let config = test_runtime_config();

        let instance =
            AgentInstance::new("test-agent".to_string(), temp.path().to_path_buf(), config);

        // Initially not running
        assert!(!instance.is_running());
        assert_eq!(instance.status().await, AgentStatus::Idle);

        // Manually set status to simulate start (without spawning runtime)
        instance.set_status(AgentStatus::Running).await.unwrap();
        assert_eq!(instance.status().await, AgentStatus::Running);

        // Pause (just changes status, no runtime to stop)
        instance.set_status(AgentStatus::Paused).await.unwrap();
        assert_eq!(instance.status().await, AgentStatus::Paused);

        // Error status
        instance.set_error("test error").await.unwrap();
        assert_eq!(instance.status().await, AgentStatus::Error);
    }

    #[tokio::test]
    async fn test_agent_instance_save_load() {
        let temp = TempDir::new().unwrap();
        let config = test_runtime_config();

        // Create an instance and set status
        let instance = AgentInstance::new(
            "test-agent".to_string(),
            temp.path().to_path_buf(),
            config.clone(),
        );

        // Set status and save
        instance.set_status(AgentStatus::Running).await.unwrap();

        // Load the instance back
        let loaded = AgentInstance::load(temp.path().to_path_buf(), config)
            .await
            .unwrap();

        assert_eq!(loaded.id().await, "test-agent");
        // Status should be restored
        assert_eq!(loaded.status().await, AgentStatus::Running);
    }
}
