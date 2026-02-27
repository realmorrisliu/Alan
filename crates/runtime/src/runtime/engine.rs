//! Agent Runtime - Core execution engine.

use super::agent_loop::handle_submission_with_cancel;
use super::turn_driver::{
    TurnInputBroker, drive_turn_submission_with_cancel, is_turn_inband_submission,
    should_drive_turn_submission,
};
use super::turn_state::TurnState;
use super::{RuntimeConfig, RuntimeLoopState};
use crate::{llm::LlmClient, session::Session};
use alan_protocol::{Event, Submission};
use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// Queues for managing submissions.
///
/// There are two submission queues in the agent runtime:
/// Requeue leftover inband submissions from turn state and broker to the outer queue.
async fn requeue_leftover_inband_submissions(
    broker: &TurnInputBroker,
    turn_state: &mut TurnState,
    queued_submissions: &mut VecDeque<Submission>,
) -> usize {
    let broker_drained = broker.drain().await;
    let turn_drained = turn_state.drain_buffered_inband_submissions();
    let count = broker_drained.len() + turn_drained.len();
    queued_submissions.extend(turn_drained);
    queued_submissions.extend(broker_drained);
    count
}

/// 1. The `outer_queue` - cross-turn queue for submissions that are not in the active turn.
/// 2. The `active_turn_broker` - channel for in-turn submissions during active turn execution.
#[derive(Default)]
struct RuntimeSubmissionQueues {
    /// Cross-turn queue for submissions.
    outer_queue: VecDeque<Submission>,
    /// The broker that queues in-turn submissions.
    active_turn_broker: TurnInputBroker,
}

impl RuntimeSubmissionQueues {
    fn pop_outer(&mut self) -> Option<Submission> {
        self.outer_queue.pop_front()
    }

    fn push_outer(&mut self, submission: Submission) {
        self.outer_queue.push_back(submission);
    }

    async fn requeue_active_turn_leftovers(&mut self, turn_state: &mut TurnState) -> usize {
        requeue_leftover_inband_submissions(
            &self.active_turn_broker,
            turn_state,
            &mut self.outer_queue,
        )
        .await
    }
}

/// Internal runtime event metadata preserved when forwarding events to hosts.
#[derive(Debug, Clone)]
pub struct RuntimeEventEnvelope {
    /// Submission id that produced this event, if any.
    pub submission_id: Option<String>,
    /// Actual protocol event payload.
    pub event: Event,
}

#[derive(Debug, Clone, Default)]
struct SubmissionEventContext {
    current_submission_id: Arc<Mutex<Option<String>>>,
}

impl SubmissionEventContext {
    fn set_submission_id(&self, submission_id: impl Into<String>) {
        if let Ok(mut guard) = self.current_submission_id.lock() {
            *guard = Some(submission_id.into());
        }
    }

    fn get_submission_id(&self) -> Option<String> {
        self.current_submission_id
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }
}

/// Handle for communicating with an agent runtime
#[derive(Clone)]
pub struct RuntimeHandle {
    pub submission_tx: mpsc::Sender<Submission>,
    /// Broadcast sender for events - create a receiver by calling subscribe()
    pub event_sender: tokio::sync::broadcast::Sender<RuntimeEventEnvelope>,
    /// Shutdown signal sender for graceful shutdown
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl RuntimeHandle {
    /// Request graceful shutdown of the runtime
    pub async fn shutdown(&self) -> Result<()> {
        if let Some(ref tx) = self.shutdown_tx {
            tx.send(()).await.map_err(|_| {
                anyhow::anyhow!("Failed to send shutdown signal - runtime may already be stopped")
            })?;
            info!("Shutdown signal sent to runtime");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Shutdown channel not available"))
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub core_config: crate::config::Config,
    pub runtime_config: RuntimeConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        let approval_policy = alan_protocol::ApprovalPolicy::default();
        let sandbox_mode = alan_protocol::SandboxMode::default();
        let runtime_config = RuntimeConfig {
            approval_policy,
            sandbox_mode,
            ..RuntimeConfig::default()
        };
        Self {
            core_config: crate::config::Config::default(),
            runtime_config,
        }
    }
}

impl From<crate::config::Config> for AgentConfig {
    fn from(config: crate::config::Config) -> Self {
        let approval_policy = alan_protocol::ApprovalPolicy::default();
        let sandbox_mode = alan_protocol::SandboxMode::default();
        let runtime_config = RuntimeConfig {
            approval_policy,
            sandbox_mode,
            ..RuntimeConfig::from(&config)
        };
        Self {
            core_config: config,
            runtime_config,
        }
    }
}

impl AgentConfig {
    /// Apply persisted configuration state to this agent config
    ///
    /// This is called when loading a workspace from disk to restore its
    /// original behavior settings (provider, model, timeouts, etc.)
    pub fn apply_persisted_state(&mut self, persisted: &crate::manager::WorkspaceConfigState) {
        use crate::config::LlmProvider;
        use crate::manager::PersistedLlmProvider;

        // Restore runtime behavior settings
        // All fields are Option<T> to distinguish "not set" from "set to 0"
        if let Some(max_tool_loops) = persisted.max_tool_loops {
            self.runtime_config.max_tool_loops = max_tool_loops;
        }
        if let Some(tool_repeat_limit) = persisted.tool_repeat_limit {
            self.runtime_config.tool_repeat_limit = tool_repeat_limit;
        }
        if let Some(llm_timeout_secs) = persisted.llm_timeout_secs {
            self.runtime_config.llm_request_timeout_secs = llm_timeout_secs as u64;
            self.core_config.llm_request_timeout_secs = llm_timeout_secs;
        }
        if let Some(tool_timeout_secs) = persisted.tool_timeout_secs {
            self.core_config.tool_timeout_secs = tool_timeout_secs;
        }
        if let Some(temp) = persisted.temperature {
            self.runtime_config.temperature = temp;
        }
        if let Some(max_tokens) = persisted.max_tokens {
            self.runtime_config.max_tokens = max_tokens;
        }
        if let Some(approval_policy) = persisted.approval_policy {
            self.runtime_config.approval_policy = approval_policy;
        }
        if let Some(sandbox_mode) = persisted.sandbox_mode {
            self.runtime_config.sandbox_mode = sandbox_mode;
        }

        // Restore LLM provider and model
        if let Some(provider) = persisted.llm_provider {
            self.core_config.llm_provider = match provider {
                PersistedLlmProvider::Gemini => LlmProvider::Gemini,
                PersistedLlmProvider::OpenaiCompatible => LlmProvider::OpenaiCompatible,
                PersistedLlmProvider::AnthropicCompatible => LlmProvider::AnthropicCompatible,
            };
        }

        // Restore model based on provider
        if let Some(ref model) = persisted.llm_model {
            match self.core_config.llm_provider {
                LlmProvider::Gemini => self.core_config.gemini_model = model.clone(),
                LlmProvider::OpenaiCompatible => {
                    self.core_config.openai_compat_model = model.clone()
                }
                LlmProvider::AnthropicCompatible => {
                    self.core_config.anthropic_compat_model = model.clone()
                }
            }
        }
    }
}

/// Combined config for spawning a runtime within a workspace
#[derive(Debug, Clone)]
pub struct WorkspaceRuntimeConfig {
    /// Agent capabilities (reusable across workspaces)
    pub agent_config: AgentConfig,
    /// Workspace identifier
    pub workspace_id: String,
    /// Workspace directory for persona, memory, and sessions
    pub workspace_dir: Option<std::path::PathBuf>,
    /// Optional rollout path to resume/fork from when starting this runtime
    pub resume_rollout_path: Option<std::path::PathBuf>,
}

impl Default for WorkspaceRuntimeConfig {
    fn default() -> Self {
        Self {
            agent_config: AgentConfig::default(),
            workspace_id: format!(
                "workspace-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            ),
            workspace_dir: None,
            resume_rollout_path: None,
        }
    }
}

impl From<crate::config::Config> for WorkspaceRuntimeConfig {
    fn from(config: crate::config::Config) -> Self {
        Self {
            agent_config: AgentConfig::from(config),
            workspace_id: format!(
                "workspace-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            ),
            workspace_dir: None,
            resume_rollout_path: None,
        }
    }
}

impl WorkspaceRuntimeConfig {
    /// Apply persisted configuration state (delegates to agent_config)
    pub fn apply_persisted_state(&mut self, persisted: &crate::manager::WorkspaceConfigState) {
        self.agent_config.apply_persisted_state(persisted);
    }
}

/// Runtime controller for managing a spawned agent runtime
pub struct RuntimeController {
    /// Handle for communicating with the runtime
    pub handle: RuntimeHandle,
    /// Join handle for the main runtime task (Option to allow take on abort)
    task_handle: Option<JoinHandle<()>>,
    /// Join handle for the event forwarding task
    event_task_handle: Option<JoinHandle<()>>,
    /// Runtime readiness channel
    ready_rx: Option<oneshot::Receiver<std::result::Result<(), String>>>,
}

impl RuntimeController {
    /// Returns true if the runtime task has already exited.
    pub fn is_finished(&self) -> bool {
        self.task_handle
            .as_ref()
            .map(tokio::task::JoinHandle::is_finished)
            .unwrap_or(true)
    }

    /// Wait until the runtime has completed startup.
    pub async fn wait_until_ready(&mut self) -> Result<()> {
        let Some(ready_rx) = self.ready_rx.take() else {
            return Ok(());
        };

        match ready_rx.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(message)) => Err(anyhow::anyhow!(message)),
            Err(_) => Err(anyhow::anyhow!(
                "Runtime stopped before signaling startup readiness"
            )),
        }
    }

    /// Shutdown the runtime gracefully and wait for it to complete
    ///
    /// First sends shutdown signal, then waits up to 10s for graceful shutdown.
    /// If timeout occurs, the task is explicitly aborted and awaited to ensure
    /// the runtime is truly stopped.
    pub async fn shutdown(mut self) -> Result<()> {
        // No longer need readiness signal once shutdown starts.
        self.ready_rx.take();

        // Send shutdown signal
        if let Some(ref tx) = self.handle.shutdown_tx
            && tx.send(()).await.is_err()
        {
            warn!("Shutdown channel closed - runtime may already be stopped");
        }

        // Close submission channel to stop accepting new work
        drop(self.handle.submission_tx);

        // Wait for the main task to complete with timeout
        let timeout = tokio::time::Duration::from_secs(10);

        // Use &mut handle so we don't consume it on timeout
        let result = if let Some(ref mut handle) = self.task_handle {
            match tokio::time::timeout(timeout, &mut *handle).await {
                Ok(Ok(())) => {
                    info!("Runtime task completed gracefully");
                    Ok(())
                }
                Ok(Err(e)) => {
                    // Task panicked
                    Err(anyhow::anyhow!("Runtime task panicked: {}", e))
                }
                Err(_) => {
                    // Timeout - explicitly abort the task
                    warn!("Runtime shutdown timeout, aborting task");
                    handle.abort();
                    // Wait for the aborted task to complete
                    match tokio::time::timeout(Duration::from_secs(5), handle).await {
                        Ok(_) => {
                            info!("Runtime task aborted successfully");
                            Ok(())
                        }
                        Err(_) => Err(anyhow::anyhow!("Runtime shutdown timeout and abort failed")),
                    }
                }
            }
        } else {
            Err(anyhow::anyhow!("Task handle not available"))
        };

        // Always abort event task
        if let Some(ref mut handle) = self.event_task_handle {
            handle.abort();
            // Wait a bit for the event task to actually stop
            let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
        }

        result
    }

    /// Abort the runtime immediately without waiting for graceful shutdown
    ///
    /// This takes ownership of the task handles and aborts them immediately.
    /// Use this when you need to guarantee the runtime stops.
    pub async fn abort(mut self) {
        // No longer need readiness signal once abort starts.
        self.ready_rx.take();

        // Send shutdown signal first (best effort)
        if let Some(ref tx) = self.handle.shutdown_tx {
            let _ = tx.try_send(());
        }

        // Take and abort both task handles
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            // Wait for the task to actually stop
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        if let Some(handle) = self.event_task_handle.take() {
            handle.abort();
            let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
        }
    }
}

/// Spawn a new agent runtime and return handles for communication
pub fn spawn(config: WorkspaceRuntimeConfig) -> Result<RuntimeController> {
    let mut core_config = config.agent_config.core_config.clone();
    if let Some(ws_dir) = config.workspace_dir.as_ref() {
        core_config.memory.workspace_dir = Some(ws_dir.join("memory"));
    }

    let llm_client = LlmClient::from_core_config(&core_config)
        .context("Failed to create LLM client for runtime")?;

    spawn_with_llm_client(config, llm_client)
}

/// Spawn a new agent runtime with an externally-provided LLM client.
///
/// This is useful for testing with a mock LLM provider.
pub fn spawn_with_llm_client(
    config: WorkspaceRuntimeConfig,
    llm_client: LlmClient,
) -> Result<RuntimeController> {
    let (sub_tx, mut sub_rx) = mpsc::channel::<Submission>(32);
    let (evt_tx, mut evt_rx) = mpsc::channel::<RuntimeEventEnvelope>(256);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let (ready_tx, ready_rx) = oneshot::channel::<std::result::Result<(), String>>();

    let workspace_dir = config.workspace_dir.clone();

    let mut core_config = config.agent_config.core_config.clone();
    if let Some(ws_dir) = workspace_dir.as_ref() {
        core_config.memory.workspace_dir = Some(ws_dir.join("memory"));
    }

    let tools = crate::tools::ToolRegistry::with_config(Arc::new(core_config.clone()));

    let runtime_config = config.agent_config.runtime_config.clone();
    let session_dir = workspace_dir.as_ref().map(|dir| dir.join("sessions"));
    let resume_rollout_path = config.resume_rollout_path.clone();

    // Spawn the main runtime task
    let task_handle = tokio::spawn(async move {
        let model = config
            .agent_config
            .core_config
            .effective_model()
            .to_string();
        let session = if let Some(path) = resume_rollout_path.as_ref() {
            if let Some(dir) = session_dir.as_ref() {
                match Session::load_from_rollout_in_dir(path, &model, dir).await {
                    Ok(session) => session,
                    Err(err) => {
                        warn!(error = %err, path = %path.display(), "Failed to load session from rollout; creating fresh persistent session");
                        Session::new_with_recorder_in_dir(&model, dir)
                            .await
                            .unwrap_or_else(|create_err| {
                                warn!(error = %create_err, "Failed to create persistent session after resume fallback; using in-memory session");
                                Session::new()
                            })
                    }
                }
            } else {
                Session::load_from_rollout(path, &model)
                    .await
                    .unwrap_or_else(|err| {
                        warn!(error = %err, path = %path.display(), "Failed to load session from rollout; creating fresh session");
                        Session::new()
                    })
            }
        } else if let Some(dir) = session_dir.as_ref() {
            Session::new_with_recorder_in_dir(&model, dir)
                .await
                .unwrap_or_else(|err| {
                    warn!(error = %err, "Failed to create persistent session; using in-memory session");
                    Session::new()
                })
        } else {
            Session::new_with_recorder(&model)
                .await
                .unwrap_or_else(|err| {
                    warn!(error = %err, "Failed to create persistent session; using in-memory session");
                    Session::new()
                })
        };

        // Build agent loop state
        let mut state = RuntimeLoopState {
            workspace_id: config.workspace_id.clone(),
            session,
            llm_client,
            tools,
            core_config,
            runtime_config,
            turn_state: super::TurnState::default(),
        };

        info!(session_id = %state.session.id, "Agent runtime started");
        let _ = ready_tx.send(Ok(()));

        // Main event loop with graceful shutdown support and interruptible submissions.
        let mut submissions_closed = false;
        let mut shutdown_requested = false;

        let mut queues = RuntimeSubmissionQueues::default();

        'runtime: loop {
            if shutdown_requested {
                info!(session_id = %state.session.id, "Shutdown signal received, stopping runtime");
                break;
            }

            let submission = if let Some(submission) = queues.pop_outer() {
                Some(submission)
            } else if submissions_closed {
                None
            } else {
                tokio::select! {
                    submission = sub_rx.recv() => submission,
                    _ = shutdown_rx.recv() => {
                        shutdown_requested = true;
                        None
                    }
                }
            };

            let Some(submission) = submission else {
                if shutdown_requested || submissions_closed {
                    break;
                }
                continue;
            };

            debug!(?submission.id, "Received submission");
            let drive_as_turn_submission = should_drive_turn_submission(&submission.op);
            let submission_event_ctx = SubmissionEventContext::default();
            submission_event_ctx.set_submission_id(submission.id.clone());

            // Fast path: no cancellation token needed unless we may run a long turn.
            let cancel = CancellationToken::new();

            // Create emitter closure
            let event_tx_clone = evt_tx.clone();
            let submission_event_ctx_for_emit = submission_event_ctx.clone();
            let mut emit = |event: Event| {
                let tx = event_tx_clone.clone();
                let submission_id = submission_event_ctx_for_emit.get_submission_id();
                async move {
                    let _ = tx
                        .send(RuntimeEventEnvelope {
                            submission_id,
                            event,
                        })
                        .await;
                }
            };

            // Allow interrupt/shutdown to be handled while the current submission is running.
            let broker_for_submission = queues.active_turn_broker.clone();
            let submission_event_ctx_for_turn = submission_event_ctx.clone();
            let mut set_active_submission_id = |submission_id: &str| {
                submission_event_ctx_for_turn.set_submission_id(submission_id.to_string());
            };
            let mut submission_fut: std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<()>> + Send + '_>,
            > = if drive_as_turn_submission {
                Box::pin(drive_turn_submission_with_cancel(
                    &mut state,
                    submission,
                    &broker_for_submission,
                    &mut emit,
                    &mut set_active_submission_id,
                    &cancel,
                ))
            } else {
                Box::pin(handle_submission_with_cancel(
                    &mut state, submission, &mut emit, &cancel,
                ))
            };

            loop {
                tokio::select! {
                    result = &mut submission_fut => {
                        drop(submission_fut);
                        if drive_as_turn_submission {
                            let _ = queues
                                .requeue_active_turn_leftovers(&mut state.turn_state)
                                .await;
                        }
                        if let Err(e) = result {
                            let error_msg = format!("Error handling submission: {}", e);
                            error!(error = %error_msg);
                            let _ = evt_tx
                                .send(RuntimeEventEnvelope {
                                    submission_id: submission_event_ctx.get_submission_id(),
                                    event: Event::Error {
                                        message: error_msg,
                                        recoverable: true,
                                    },
                                })
                                .await;
                        }
                        break;
                    }
                    incoming = sub_rx.recv(), if !submissions_closed => {
                        match incoming {
                            Some(incoming) => {
                                if matches!(incoming.op, alan_protocol::Op::Interrupt) {
                                    cancel.cancel();
                                } else if drive_as_turn_submission
                                    && is_turn_inband_submission(&incoming.op)
                                {
                                    if !queues.active_turn_broker.push(incoming.clone()).await {
                                        queues.push_outer(incoming);
                                    }
                                } else {
                                    queues.push_outer(incoming);
                                }
                            }
                            None => {
                                submissions_closed = true;
                                cancel.cancel();
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        shutdown_requested = true;
                        cancel.cancel();
                    }
                }

                if shutdown_requested {
                    // Wait for the current submission to unwind after cancellation
                    // before breaking the runtime loop.
                    continue;
                }
            }

            if shutdown_requested {
                break 'runtime;
            }
        }

        info!(session_id = %state.session.id, "Agent runtime stopped");
        state.session.flush().await;
    });

    // Spawn a task to forward events to a broadcast channel
    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<RuntimeEventEnvelope>(256);
    let broadcast_tx_clone = broadcast_tx.clone();
    let event_task_handle = tokio::spawn(async move {
        while let Some(runtime_event) = evt_rx.recv().await {
            let _ = broadcast_tx_clone.send(runtime_event);
        }
    });

    Ok(RuntimeController {
        handle: RuntimeHandle {
            submission_tx: sub_tx,
            event_sender: broadcast_tx,
            shutdown_tx: Some(shutdown_tx),
        },
        task_handle: Some(task_handle),
        event_task_handle: Some(event_task_handle),
        ready_rx: Some(ready_rx),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alan_protocol::Op;
    use tempfile::TempDir;

    #[test]
    fn test_agent_runtime_config_default() {
        let config = WorkspaceRuntimeConfig::default();
        assert!(config.workspace_id.starts_with("workspace-"));
        assert!(config.workspace_dir.is_none());
    }

    #[test]
    fn test_agent_runtime_config_from_core_config() {
        let core_config = crate::config::Config::default();
        let runtime_config = WorkspaceRuntimeConfig::from(core_config.clone());

        assert!(runtime_config.workspace_id.starts_with("workspace-"));
        assert_eq!(runtime_config.workspace_dir, None);
    }

    #[test]
    fn test_agent_runtime_config_clone() {
        let config = WorkspaceRuntimeConfig::default();
        let cloned = config.clone();
        assert_eq!(config.workspace_id, cloned.workspace_id);
    }

    #[test]
    fn test_agent_runtime_config_debug() {
        let config = WorkspaceRuntimeConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("WorkspaceRuntimeConfig"));
        assert!(debug_str.contains("workspace_id"));
    }

    #[test]
    fn test_agent_runtime_handle_clone() {
        let (sub_tx, _sub_rx) = mpsc::channel(10);
        let (evt_tx, _) = mpsc::channel::<RuntimeEventEnvelope>(10);

        let handle = RuntimeHandle {
            submission_tx: sub_tx,
            event_sender: tokio::sync::broadcast::channel(10).0,
            shutdown_tx: None,
        };

        let cloned = handle.clone();
        // Both handles should share the same channels
        drop(cloned);
        drop(handle);
        drop(evt_tx); // Clean up
    }

    #[test]
    fn test_agent_runtime_handle_fields() {
        let (sub_tx, _sub_rx) = mpsc::channel::<Submission>(10);

        let handle = RuntimeHandle {
            submission_tx: sub_tx,
            event_sender: tokio::sync::broadcast::channel(10).0,
            shutdown_tx: None,
        };

        // Verify handle can be created
        assert!(!handle.submission_tx.is_closed());
    }

    #[test]
    fn test_submission_event_context_tracks_latest_submission_id() {
        let ctx = SubmissionEventContext::default();
        assert_eq!(ctx.get_submission_id(), None);

        ctx.set_submission_id("sub-1");
        assert_eq!(ctx.get_submission_id().as_deref(), Some("sub-1"));

        ctx.set_submission_id("sub-2");
        assert_eq!(ctx.get_submission_id().as_deref(), Some("sub-2"));
    }

    #[tokio::test]
    async fn test_agent_runtime_handle_shutdown_without_channel() {
        let (sub_tx, _sub_rx) = mpsc::channel::<Submission>(10);
        let handle = RuntimeHandle {
            submission_tx: sub_tx,
            event_sender: tokio::sync::broadcast::channel(10).0,
            shutdown_tx: None,
        };

        let result = handle.shutdown().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_persisted_state_some_zero_values() {
        // Regression test: ensure Some(0) values are correctly restored
        // and not treated as "not set" (which would use defaults instead)
        use crate::manager::WorkspaceConfigState;

        let base_config = WorkspaceRuntimeConfig::default();
        let mut restored_config = base_config.clone();

        // Create persisted state with explicit 0 values
        let persisted = WorkspaceConfigState {
            max_tool_loops: Some(0),    // 0 = unlimited
            tool_repeat_limit: Some(0), // 0 = disable protection
            llm_timeout_secs: Some(0),  // 0 = no timeout
            tool_timeout_secs: Some(0), // 0 = no timeout
            llm_provider: None,
            llm_model: None,
            temperature: None,
            max_tokens: None,
            approval_policy: None,
            sandbox_mode: None,
        };

        restored_config.apply_persisted_state(&persisted);

        // Verify Some(0) values were restored (not skipped)
        assert_eq!(
            restored_config.agent_config.runtime_config.max_tool_loops, 0,
            "max_tool_loops Some(0) should be restored"
        );
        assert_eq!(
            restored_config
                .agent_config
                .runtime_config
                .tool_repeat_limit,
            0,
            "tool_repeat_limit Some(0) should be restored"
        );
        assert_eq!(
            restored_config
                .agent_config
                .runtime_config
                .llm_request_timeout_secs,
            0,
            "llm_timeout_secs Some(0) should be restored"
        );
        assert_eq!(
            restored_config.agent_config.core_config.tool_timeout_secs, 0,
            "tool_timeout_secs Some(0) should be restored"
        );
    }

    #[test]
    fn test_apply_persisted_state_none_uses_base() {
        // Test that None values fall back to base config defaults
        use crate::manager::WorkspaceConfigState;

        let base_config = WorkspaceRuntimeConfig::default();
        let mut restored_config = base_config.clone();

        // Create persisted state with None values
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: None,
            llm_model: None,
            temperature: None,
            max_tokens: None,
            approval_policy: None,
            sandbox_mode: None,
        };

        restored_config.apply_persisted_state(&persisted);

        // Verify None values use base config defaults
        assert_eq!(
            restored_config.agent_config.runtime_config.max_tool_loops,
            base_config.agent_config.runtime_config.max_tool_loops
        );
        assert_eq!(
            restored_config
                .agent_config
                .runtime_config
                .tool_repeat_limit,
            base_config.agent_config.runtime_config.tool_repeat_limit
        );
        assert_eq!(
            restored_config
                .agent_config
                .runtime_config
                .llm_request_timeout_secs,
            base_config
                .agent_config
                .runtime_config
                .llm_request_timeout_secs
        );
        assert_eq!(
            restored_config.agent_config.core_config.tool_timeout_secs,
            base_config.agent_config.core_config.tool_timeout_secs
        );
    }

    #[test]
    fn test_apply_persisted_state_non_zero_values() {
        // Test that non-zero values are correctly restored
        use crate::manager::WorkspaceConfigState;

        let base_config = WorkspaceRuntimeConfig::default();
        let mut restored_config = base_config.clone();

        // Create persisted state with specific non-zero values
        let persisted = WorkspaceConfigState {
            max_tool_loops: Some(10),
            tool_repeat_limit: Some(8),
            llm_timeout_secs: Some(300),
            tool_timeout_secs: Some(60),
            llm_provider: None,
            llm_model: None,
            temperature: None,
            max_tokens: None,
            approval_policy: None,
            sandbox_mode: None,
        };

        restored_config.apply_persisted_state(&persisted);

        // Verify values were restored
        assert_eq!(
            restored_config.agent_config.runtime_config.max_tool_loops,
            10
        );
        assert_eq!(
            restored_config
                .agent_config
                .runtime_config
                .tool_repeat_limit,
            8
        );
        assert_eq!(
            restored_config
                .agent_config
                .runtime_config
                .llm_request_timeout_secs,
            300
        );
        assert_eq!(
            restored_config.agent_config.core_config.tool_timeout_secs,
            60
        );
    }

    #[test]
    fn test_apply_persisted_state_temperature_and_max_tokens() {
        use crate::manager::WorkspaceConfigState;

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: None,
            llm_model: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            approval_policy: None,
            sandbox_mode: None,
        };

        config.apply_persisted_state(&persisted);

        assert_eq!(config.agent_config.runtime_config.temperature, 0.7);
        assert_eq!(config.agent_config.runtime_config.max_tokens, 4096);
    }

    #[test]
    fn test_apply_persisted_state_gemini_provider() {
        use crate::config::LlmProvider;
        use crate::manager::{PersistedLlmProvider, WorkspaceConfigState};

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: Some(PersistedLlmProvider::Gemini),
            llm_model: Some("gemini-2.0-pro".to_string()),
            temperature: None,
            max_tokens: None,
            approval_policy: None,
            sandbox_mode: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::Gemini
        ));
        assert_eq!(
            config.agent_config.core_config.gemini_model,
            "gemini-2.0-pro"
        );
    }

    #[test]
    fn test_apply_persisted_state_openai_provider() {
        use crate::config::LlmProvider;
        use crate::manager::{PersistedLlmProvider, WorkspaceConfigState};

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: Some(PersistedLlmProvider::OpenaiCompatible),
            llm_model: Some("gpt-4o".to_string()),
            temperature: None,
            max_tokens: None,
            approval_policy: None,
            sandbox_mode: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::OpenaiCompatible
        ));
        assert_eq!(
            config.agent_config.core_config.openai_compat_model,
            "gpt-4o"
        );
    }

    #[test]
    fn test_apply_persisted_state_anthropic_provider() {
        use crate::config::LlmProvider;
        use crate::manager::{PersistedLlmProvider, WorkspaceConfigState};

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: Some(PersistedLlmProvider::AnthropicCompatible),
            llm_model: Some("claude-3-5-sonnet".to_string()),
            temperature: None,
            max_tokens: None,
            approval_policy: None,
            sandbox_mode: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::AnthropicCompatible
        ));
        assert_eq!(
            config.agent_config.core_config.anthropic_compat_model,
            "claude-3-5-sonnet"
        );
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_agent_runtime_config_set_workspace_dir() {
        let temp = TempDir::new().unwrap();
        let config = WorkspaceRuntimeConfig {
            workspace_dir: Some(temp.path().to_path_buf()),
            ..Default::default()
        };

        assert_eq!(config.workspace_dir, Some(temp.path().to_path_buf()));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_workspace_runtime_config_set_workspace_id() {
        let mut config = WorkspaceRuntimeConfig::default();
        config.workspace_id = "custom-workspace-123".to_string();

        assert_eq!(config.workspace_id, "custom-workspace-123");
    }

    #[test]
    fn test_apply_persisted_state_tool_policy_settings() {
        use crate::manager::WorkspaceConfigState;

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: None,
            llm_model: None,
            temperature: None,
            max_tokens: None,
            approval_policy: Some(alan_protocol::ApprovalPolicy::Never),
            sandbox_mode: Some(alan_protocol::SandboxMode::DangerFullAccess),
        };

        config.apply_persisted_state(&persisted);

        assert_eq!(
            config.agent_config.runtime_config.approval_policy,
            alan_protocol::ApprovalPolicy::Never
        );
        assert_eq!(
            config.agent_config.runtime_config.sandbox_mode,
            alan_protocol::SandboxMode::DangerFullAccess
        );
    }

    #[tokio::test]
    async fn test_agent_runtime_handle_shutdown_with_channel() {
        let (sub_tx, _sub_rx) = mpsc::channel::<Submission>(10);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let handle = RuntimeHandle {
            submission_tx: sub_tx,
            event_sender: tokio::sync::broadcast::channel(10).0,
            shutdown_tx: Some(shutdown_tx),
        };

        // Shutdown should send signal
        let result = handle.shutdown().await;
        assert!(result.is_ok());

        // Verify shutdown signal was sent
        let signal = shutdown_rx.recv().await;
        assert!(signal.is_some());
    }

    #[test]
    fn test_runtime_event_envelope_creation() {
        let envelope = RuntimeEventEnvelope {
            submission_id: Some("sub-123".to_string()),
            event: Event::TurnStarted {},
        };

        assert_eq!(envelope.submission_id, Some("sub-123".to_string()));
        assert!(matches!(envelope.event, Event::TurnStarted {}));
    }

    #[test]
    fn test_agent_runtime_config_with_workspace_dir() {
        let temp = TempDir::new().unwrap();
        let config = WorkspaceRuntimeConfig {
            workspace_dir: Some(temp.path().to_path_buf()),
            ..Default::default()
        };

        assert_eq!(config.workspace_dir, Some(temp.path().to_path_buf()));
    }

    #[test]
    fn test_agent_runtime_config_resume_rollout_path() {
        let temp = TempDir::new().unwrap();
        let rollout_path = temp.path().join("rollout.jsonl");

        let config = WorkspaceRuntimeConfig {
            resume_rollout_path: Some(rollout_path.clone()),
            ..Default::default()
        };

        assert_eq!(config.resume_rollout_path, Some(rollout_path));
    }

    #[test]
    fn test_should_drive_turn_submission() {
        // Input should be driven as turn
        assert!(should_drive_turn_submission(&Op::Input {
            parts: vec![alan_protocol::ContentPart::text("test")],
        }));

        // Turn should be driven as turn
        assert!(should_drive_turn_submission(&Op::Turn {
            parts: vec![alan_protocol::ContentPart::text("test")],
            context: None,
        }));

        // Other ops should not be driven as turn
        assert!(!should_drive_turn_submission(&Op::Compact));
        assert!(!should_drive_turn_submission(&Op::Rollback {
            num_turns: 1
        }));
        assert!(!should_drive_turn_submission(&Op::Interrupt));
        assert!(!should_drive_turn_submission(&Op::RegisterDynamicTools {
            tools: vec![]
        }));
        assert!(!should_drive_turn_submission(&Op::Resume {
            request_id: "req-123".to_string(),
            result: serde_json::json!({}),
        }));
    }

    #[test]
    fn test_apply_persisted_state_approval_policy() {
        use crate::manager::WorkspaceConfigState;

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: None,
            llm_model: None,
            temperature: None,
            max_tokens: None,
            approval_policy: Some(alan_protocol::ApprovalPolicy::Never),
            sandbox_mode: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.runtime_config.approval_policy,
            alan_protocol::ApprovalPolicy::Never
        ));
    }

    #[test]
    fn test_apply_persisted_state_sandbox_mode() {
        use crate::manager::WorkspaceConfigState;

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: None,
            llm_model: None,
            temperature: None,
            max_tokens: None,
            approval_policy: None,
            sandbox_mode: Some(alan_protocol::SandboxMode::DangerFullAccess),
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.runtime_config.sandbox_mode,
            alan_protocol::SandboxMode::DangerFullAccess
        ));
    }
}
