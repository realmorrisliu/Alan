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
use sha2::{Digest, Sha256};
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

fn session_log_fingerprint(session_id: &str) -> String {
    let digest = Sha256::digest(session_id.as_bytes());
    let mut out = String::with_capacity(12);
    for byte in digest.iter().take(6) {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Effective durability state for a runtime session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionDurabilityState {
    /// Whether the active session has a persistent recorder attached.
    pub durable: bool,
    /// Whether startup required durability instead of allowing in-memory fallback.
    pub required: bool,
}

/// Metadata produced once runtime startup completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStartupMetadata {
    pub durability: SessionDurabilityState,
    pub warnings: Vec<String>,
}

struct SessionStartupOutcome {
    session: Session,
    metadata: RuntimeStartupMetadata,
}

fn best_effort_durability_warning(err: &anyhow::Error) -> String {
    format!("Session is running without persistent recorder; using in-memory mode: {err}")
}

async fn create_persistent_session(
    session_id: Option<&str>,
    model: &str,
    session_dir: Option<&std::path::PathBuf>,
) -> anyhow::Result<Session> {
    match (session_id, session_dir) {
        (Some(session_id), Some(dir)) => {
            Session::new_with_id_and_recorder_in_dir(session_id, model, dir).await
        }
        (None, Some(dir)) => Session::new_with_recorder_in_dir(model, dir).await,
        (Some(session_id), None) => Session::new_with_id_and_recorder(session_id, model).await,
        (None, None) => Session::new_with_recorder(model).await,
    }
}

async fn initialize_session(
    model: &str,
    resume_rollout_path: Option<&std::path::PathBuf>,
    session_dir: Option<&std::path::PathBuf>,
    desired_session_id: Option<&str>,
    durability_required: bool,
) -> anyhow::Result<SessionStartupOutcome> {
    let mut warnings = Vec::new();

    let session = if let Some(path) = resume_rollout_path {
        let load_result = if let Some(dir) = session_dir {
            if let Some(session_id) = desired_session_id {
                Session::load_from_rollout_in_dir_with_id(path, session_id, model, dir).await
            } else {
                Session::load_from_rollout_in_dir(path, model, dir).await
            }
        } else if let Some(session_id) = desired_session_id {
            Session::load_from_rollout_with_id(path, session_id, model).await
        } else {
            Session::load_from_rollout(path, model).await
        };

        match load_result {
            Ok(session) => session,
            Err(err) => {
                if durability_required {
                    return Err(anyhow::anyhow!(
                        "Strict durability required: failed to load persisted session from {}: {}",
                        path.display(),
                        err
                    ));
                }

                warn!(
                    error = %err,
                    path = %path.display(),
                    "Failed to load session from rollout; creating fresh persistent session"
                );
                match create_persistent_session(desired_session_id, model, session_dir).await {
                    Ok(session) => session,
                    Err(create_err) => {
                        warn!(
                            error = %create_err,
                            "Failed to create persistent session after resume fallback; using in-memory session"
                        );
                        warnings.push(best_effort_durability_warning(&create_err));
                        Session::new()
                    }
                }
            }
        }
    } else {
        match create_persistent_session(desired_session_id, model, session_dir).await {
            Ok(session) => session,
            Err(err) => {
                if durability_required {
                    return Err(anyhow::anyhow!(
                        "Strict durability required: failed to create persistent session: {}",
                        err
                    ));
                }

                warn!(error = %err, "Failed to create persistent session; using in-memory session");
                warnings.push(best_effort_durability_warning(&err));
                Session::new()
            }
        }
    };

    Ok(SessionStartupOutcome {
        metadata: RuntimeStartupMetadata {
            durability: SessionDurabilityState {
                durable: session.recorder.is_some(),
                required: durability_required,
            },
            warnings,
        },
        session,
    })
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
        let runtime_config = RuntimeConfig::default();
        Self {
            core_config: crate::config::Config::default(),
            runtime_config,
        }
    }
}

impl From<crate::config::Config> for AgentConfig {
    fn from(config: crate::config::Config) -> Self {
        let runtime_config = RuntimeConfig::from(&config);
        Self {
            core_config: config,
            runtime_config,
        }
    }
}

impl AgentConfig {
    pub fn refresh_runtime_derived_fields(&mut self) {
        if self.core_config.context_window_tokens.is_none() {
            self.runtime_config.context_window_tokens =
                self.core_config.effective_context_window_tokens();
        }
    }

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
        if let Some(compaction_trigger_ratio) = persisted.compaction_trigger_ratio {
            self.runtime_config.compaction_trigger_ratio = compaction_trigger_ratio;
        }
        if let Some(streaming_mode) = persisted.streaming_mode {
            self.runtime_config.streaming_mode = streaming_mode;
        }
        if let Some(partial_stream_recovery_mode) = persisted.partial_stream_recovery_mode {
            self.runtime_config.partial_stream_recovery_mode = partial_stream_recovery_mode;
        }
        if let Some(governance) = persisted.governance.clone() {
            self.runtime_config.governance = governance;
        }

        // Restore LLM provider and model
        if let Some(provider) = persisted.llm_provider {
            self.core_config.llm_provider = match provider {
                PersistedLlmProvider::GoogleGeminiGenerateContent => {
                    LlmProvider::GoogleGeminiGenerateContent
                }
                PersistedLlmProvider::OpenAiResponses => LlmProvider::OpenAiResponses,
                PersistedLlmProvider::OpenAiChatCompletions => LlmProvider::OpenAiChatCompletions,
                PersistedLlmProvider::OpenAiChatCompletionsCompatible => {
                    LlmProvider::OpenAiChatCompletionsCompatible
                }
                PersistedLlmProvider::AnthropicMessages => LlmProvider::AnthropicMessages,
            };
        }

        // Restore model based on provider
        if let Some(ref model) = persisted.llm_model {
            match self.core_config.llm_provider {
                LlmProvider::GoogleGeminiGenerateContent => {
                    self.core_config.google_gemini_generate_content_model = model.clone()
                }
                LlmProvider::OpenAiResponses => {
                    self.core_config.openai_responses_model = model.clone()
                }
                LlmProvider::OpenAiChatCompletions => {
                    self.core_config.openai_chat_completions_model = model.clone()
                }
                LlmProvider::OpenAiChatCompletionsCompatible => {
                    self.core_config.openai_chat_completions_compatible_model = model.clone()
                }
                LlmProvider::AnthropicMessages => {
                    self.core_config.anthropic_messages_model = model.clone()
                }
            }
        }

        if let Some(context_window_tokens) = persisted.context_window_tokens {
            self.runtime_config.context_window_tokens = context_window_tokens;
        } else {
            self.refresh_runtime_derived_fields();
        }
    }
}

/// Combined config for spawning a runtime within a workspace
#[derive(Debug, Clone)]
pub struct WorkspaceRuntimeConfig {
    /// Agent capabilities (reusable across workspaces)
    pub agent_config: AgentConfig,
    /// Session identifier to use when creating a fresh persistent runtime session.
    pub session_id: Option<String>,
    /// Workspace identifier
    pub workspace_id: String,
    /// Workspace root directory for tool cwd/sandbox context
    pub workspace_root_dir: Option<std::path::PathBuf>,
    /// Workspace `.alan` state directory for persona, memory, and sessions
    pub workspace_alan_dir: Option<std::path::PathBuf>,
    /// Optional rollout path to resume/fork from when starting this runtime
    pub resume_rollout_path: Option<std::path::PathBuf>,
}

impl Default for WorkspaceRuntimeConfig {
    fn default() -> Self {
        Self {
            agent_config: AgentConfig::default(),
            session_id: None,
            workspace_id: format!(
                "workspace-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            ),
            workspace_root_dir: None,
            workspace_alan_dir: None,
            resume_rollout_path: None,
        }
    }
}

impl From<crate::config::Config> for WorkspaceRuntimeConfig {
    fn from(config: crate::config::Config) -> Self {
        Self {
            agent_config: AgentConfig::from(config),
            session_id: None,
            workspace_id: format!(
                "workspace-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            ),
            workspace_root_dir: None,
            workspace_alan_dir: None,
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
    ready_rx: Option<oneshot::Receiver<std::result::Result<RuntimeStartupMetadata, String>>>,
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
    pub async fn wait_until_ready(&mut self) -> Result<RuntimeStartupMetadata> {
        let Some(ready_rx) = self.ready_rx.take() else {
            return Ok(RuntimeStartupMetadata {
                durability: SessionDurabilityState {
                    durable: true,
                    required: false,
                },
                warnings: Vec::new(),
            });
        };

        match ready_rx.await {
            Ok(Ok(metadata)) => Ok(metadata),
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
    if let Some(alan_dir) = config.workspace_alan_dir.as_ref() {
        core_config.memory.workspace_dir = Some(alan_dir.join("memory"));
    }

    let llm_client = LlmClient::from_core_config(&core_config)
        .context("Failed to create LLM client for runtime")?;
    let tools = crate::tools::ToolRegistry::with_config(Arc::new(core_config.clone()));

    spawn_with_llm_client_and_tools(config, llm_client, tools)
}

/// Spawn a new agent runtime with an externally-provided LLM client.
///
/// This is useful for testing with a mock LLM provider.
pub fn spawn_with_tool_registry(
    config: WorkspaceRuntimeConfig,
    tools: crate::tools::ToolRegistry,
) -> Result<RuntimeController> {
    let mut core_config = config.agent_config.core_config.clone();
    if let Some(alan_dir) = config.workspace_alan_dir.as_ref() {
        core_config.memory.workspace_dir = Some(alan_dir.join("memory"));
    }

    let llm_client = LlmClient::from_core_config(&core_config)
        .context("Failed to create LLM client for runtime")?;

    spawn_with_llm_client_and_tools(config, llm_client, tools)
}

/// Spawn a new agent runtime with an externally-provided LLM client.
///
/// This is useful for testing with a mock LLM provider.
pub fn spawn_with_llm_client(
    config: WorkspaceRuntimeConfig,
    llm_client: LlmClient,
) -> Result<RuntimeController> {
    let mut core_config = config.agent_config.core_config.clone();
    if let Some(alan_dir) = config.workspace_alan_dir.as_ref() {
        core_config.memory.workspace_dir = Some(alan_dir.join("memory"));
    }
    let tools = crate::tools::ToolRegistry::with_config(Arc::new(core_config));

    spawn_with_llm_client_and_tools(config, llm_client, tools)
}

/// Spawn a new agent runtime with an externally-provided LLM client and tools.
///
/// Hosts should use this when they need to inject concrete tool implementations
/// while keeping the runtime crate generic.
pub fn spawn_with_llm_client_and_tools(
    config: WorkspaceRuntimeConfig,
    llm_client: LlmClient,
    mut tools: crate::tools::ToolRegistry,
) -> Result<RuntimeController> {
    let (sub_tx, mut sub_rx) = mpsc::channel::<Submission>(32);
    let (evt_tx, mut evt_rx) = mpsc::channel::<RuntimeEventEnvelope>(256);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let (ready_tx, ready_rx) =
        oneshot::channel::<std::result::Result<RuntimeStartupMetadata, String>>();

    let workspace_root_dir = config.workspace_root_dir.clone();
    let workspace_alan_dir = config.workspace_alan_dir.clone();
    if let Some(ws_root) = workspace_root_dir.as_ref() {
        tools.set_default_cwd(ws_root.clone());
    }

    let mut core_config = config.agent_config.core_config.clone();
    if let Some(alan_dir) = workspace_alan_dir.as_ref() {
        core_config.memory.workspace_dir = Some(alan_dir.join("memory"));
    }

    let mut runtime_config = config.agent_config.runtime_config.clone();
    runtime_config.policy_engine = crate::policy::PolicyEngine::load_for_governance(
        workspace_alan_dir.as_deref(),
        &runtime_config.governance,
    );
    let workspace_persona_override = workspace_alan_dir.as_ref().map(|dir| dir.join("persona"));
    let prompt_cache_persona_dir = crate::prompts::resolve_workspace_persona_dir_for_workspace(
        &core_config,
        workspace_persona_override.as_deref(),
    );
    if let Some(persona_dir) = prompt_cache_persona_dir.as_deref()
        && let Err(err) = crate::prompts::ensure_workspace_bootstrap_files_at(persona_dir)
    {
        warn!(
            path = %persona_dir.display(),
            error = %err,
            "Failed to initialize workspace persona files; continuing without bootstrap writes"
        );
    }
    let session_dir = workspace_alan_dir.as_ref().map(|dir| dir.join("sessions"));
    let resume_rollout_path = config.resume_rollout_path.clone();
    let desired_session_id = config.session_id.clone();
    let skills_cwd = super::prompt_cache::resolve_skills_registry_cwd(
        tools.default_cwd().as_deref(),
        core_config.memory.workspace_dir.as_deref(),
    );

    // Spawn the main runtime task
    let task_handle = tokio::spawn(async move {
        let model = config
            .agent_config
            .core_config
            .effective_model()
            .to_string();
        let startup = match initialize_session(
            &model,
            resume_rollout_path.as_ref(),
            session_dir.as_ref(),
            desired_session_id.as_deref(),
            runtime_config.durability_required,
        )
        .await
        {
            Ok(startup) => startup,
            Err(err) => {
                let _ = ready_tx.send(Err(format!("{:#}", err)));
                return;
            }
        };
        let session = startup.session;

        // Build agent loop state
        let mut state = RuntimeLoopState {
            workspace_id: config.workspace_id.clone(),
            session,
            llm_client,
            tools,
            core_config,
            runtime_config,
            workspace_persona_dir: prompt_cache_persona_dir.clone(),
            prompt_cache: super::prompt_cache::PromptAssemblyCache::new(
                skills_cwd,
                prompt_cache_persona_dir,
            ),
            turn_state: super::TurnState::default(),
        };

        info!(
            session_fingerprint = %session_log_fingerprint(&state.session.id),
            "Agent runtime started"
        );
        let _ = ready_tx.send(Ok(startup.metadata));

        // Main event loop with graceful shutdown support and interruptible submissions.
        let mut submissions_closed = false;
        let mut shutdown_requested = false;

        let mut queues = RuntimeSubmissionQueues::default();

        'runtime: loop {
            if shutdown_requested {
                info!(
                    session_fingerprint = %session_log_fingerprint(&state.session.id),
                    "Shutdown signal received, stopping runtime"
                );
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

        info!(
            session_fingerprint = %session_log_fingerprint(&state.session.id),
            "Agent runtime stopped"
        );
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
    use alan_llm::MockLlmProvider;
    use alan_protocol::Op;
    use tempfile::TempDir;

    #[test]
    fn test_agent_runtime_config_default() {
        let config = WorkspaceRuntimeConfig::default();
        assert!(config.workspace_id.starts_with("workspace-"));
        assert!(config.workspace_root_dir.is_none());
        assert!(config.workspace_alan_dir.is_none());
    }

    #[test]
    fn test_agent_runtime_config_from_core_config() {
        let core_config = crate::config::Config::default();
        let runtime_config = WorkspaceRuntimeConfig::from(core_config.clone());

        assert!(runtime_config.workspace_id.starts_with("workspace-"));
        assert_eq!(runtime_config.workspace_root_dir, None);
        assert_eq!(runtime_config.workspace_alan_dir, None);
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
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
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
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
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
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
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
            context_window_tokens: Some(32_768),
            compaction_trigger_ratio: Some(0.7),
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert_eq!(config.agent_config.runtime_config.temperature, 0.7);
        assert_eq!(config.agent_config.runtime_config.max_tokens, 4096);
        assert_eq!(
            config.agent_config.runtime_config.context_window_tokens,
            32_768
        );
        assert_eq!(
            config.agent_config.runtime_config.compaction_trigger_ratio,
            0.7
        );
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
            llm_provider: Some(PersistedLlmProvider::GoogleGeminiGenerateContent),
            llm_model: Some("gemini-2.0-pro".to_string()),
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::GoogleGeminiGenerateContent
        ));
        assert_eq!(
            config
                .agent_config
                .core_config
                .google_gemini_generate_content_model,
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
            llm_provider: Some(PersistedLlmProvider::OpenAiResponses),
            llm_model: Some("gpt-5.4".to_string()),
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::OpenAiResponses
        ));
        assert_eq!(
            config.agent_config.core_config.openai_responses_model,
            "gpt-5.4"
        );
    }

    #[test]
    fn test_apply_persisted_state_openai_chat_completions_compatible_provider() {
        use crate::config::LlmProvider;
        use crate::manager::{PersistedLlmProvider, WorkspaceConfigState};

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: Some(PersistedLlmProvider::OpenAiChatCompletionsCompatible),
            llm_model: Some("qwen3.5-plus-2026-02-15".to_string()),
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::OpenAiChatCompletionsCompatible
        ));
        assert_eq!(
            config
                .agent_config
                .core_config
                .openai_chat_completions_compatible_model,
            "qwen3.5-plus-2026-02-15"
        );
    }

    #[test]
    fn test_apply_persisted_state_openai_chat_completions_provider() {
        use crate::config::LlmProvider;
        use crate::manager::{PersistedLlmProvider, WorkspaceConfigState};

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: Some(PersistedLlmProvider::OpenAiChatCompletions),
            llm_model: Some("gpt-5.4".to_string()),
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::OpenAiChatCompletions
        ));
        assert_eq!(
            config
                .agent_config
                .core_config
                .openai_chat_completions_model,
            "gpt-5.4"
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
            llm_provider: Some(PersistedLlmProvider::AnthropicMessages),
            llm_model: Some("claude-3-5-sonnet".to_string()),
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert!(matches!(
            config.agent_config.core_config.llm_provider,
            LlmProvider::AnthropicMessages
        ));
        assert_eq!(
            config.agent_config.core_config.anthropic_messages_model,
            "claude-3-5-sonnet"
        );
        assert_eq!(
            config.agent_config.runtime_config.context_window_tokens,
            200_000
        );
    }

    #[test]
    fn test_apply_persisted_state_refreshes_legacy_context_window_fallback() {
        use crate::manager::{PersistedLlmProvider, WorkspaceConfigState};

        let mut config = WorkspaceRuntimeConfig::default();
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: Some(PersistedLlmProvider::GoogleGeminiGenerateContent),
            llm_model: Some("gemini-2.5-pro".to_string()),
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert_eq!(
            config.agent_config.runtime_config.context_window_tokens,
            1_048_576
        );
    }

    #[test]
    fn test_apply_persisted_state_keeps_explicit_context_window_override() {
        use crate::config::Config;
        use crate::manager::{PersistedLlmProvider, WorkspaceConfigState};

        let mut config = WorkspaceRuntimeConfig::from(Config {
            llm_provider: crate::config::LlmProvider::OpenAiResponses,
            openai_responses_model: "gpt-5.4".to_string(),
            context_window_tokens: Some(42_000),
            ..Config::default()
        });
        let persisted = WorkspaceConfigState {
            max_tool_loops: None,
            tool_repeat_limit: None,
            llm_timeout_secs: None,
            tool_timeout_secs: None,
            llm_provider: Some(PersistedLlmProvider::GoogleGeminiGenerateContent),
            llm_model: Some("gemini-2.5-pro".to_string()),
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: None,
        };

        config.apply_persisted_state(&persisted);

        assert_eq!(
            config.agent_config.runtime_config.context_window_tokens,
            42_000
        );
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_agent_runtime_config_set_workspace_paths() {
        let temp = TempDir::new().unwrap();
        let config = WorkspaceRuntimeConfig {
            workspace_root_dir: Some(temp.path().to_path_buf()),
            workspace_alan_dir: Some(temp.path().join(".alan")),
            ..Default::default()
        };

        assert_eq!(config.workspace_root_dir, Some(temp.path().to_path_buf()));
        assert_eq!(config.workspace_alan_dir, Some(temp.path().join(".alan")));
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
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: Some(alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: Some(".alan/policy.yaml".to_string()),
            }),
        };

        config.apply_persisted_state(&persisted);

        assert_eq!(
            config.agent_config.runtime_config.governance,
            alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: Some(".alan/policy.yaml".to_string()),
            }
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
    fn test_session_log_fingerprint_is_stable_and_redacted() {
        let fingerprint = session_log_fingerprint("session-secret-123");
        assert_eq!(fingerprint.len(), 12);
        assert_eq!(fingerprint, session_log_fingerprint("session-secret-123"));
        assert_ne!(fingerprint, "session-secret-123");
    }

    #[test]
    fn test_agent_runtime_config_with_workspace_paths() {
        let temp = TempDir::new().unwrap();
        let config = WorkspaceRuntimeConfig {
            workspace_root_dir: Some(temp.path().to_path_buf()),
            workspace_alan_dir: Some(temp.path().join(".alan")),
            ..Default::default()
        };

        assert_eq!(config.workspace_root_dir, Some(temp.path().to_path_buf()));
        assert_eq!(config.workspace_alan_dir, Some(temp.path().join(".alan")));
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
    fn test_agent_runtime_config_session_id() {
        let config = WorkspaceRuntimeConfig {
            session_id: Some("sess-123".to_string()),
            ..Default::default()
        };

        assert_eq!(config.session_id.as_deref(), Some("sess-123"));
    }

    #[test]
    fn test_should_drive_turn_submission() {
        // steer/follow_up should be driven as turn
        assert!(should_drive_turn_submission(&Op::Input {
            parts: vec![alan_protocol::ContentPart::text("test")],
            mode: alan_protocol::InputMode::Steer,
        }));
        assert!(should_drive_turn_submission(&Op::Input {
            parts: vec![alan_protocol::ContentPart::text("test")],
            mode: alan_protocol::InputMode::FollowUp,
        }));
        // next_turn should be queue-only, not immediate execution.
        assert!(!should_drive_turn_submission(&Op::Input {
            parts: vec![alan_protocol::ContentPart::text("test")],
            mode: alan_protocol::InputMode::NextTurn,
        }));

        // Turn should be driven as turn
        assert!(should_drive_turn_submission(&Op::Turn {
            parts: vec![alan_protocol::ContentPart::text("test")],
            context: None,
        }));

        // Other ops should not be driven as turn
        assert!(!should_drive_turn_submission(&Op::CompactWithOptions {
            focus: None,
        }));
        assert!(!should_drive_turn_submission(&Op::CompactWithOptions {
            focus: Some("preserve todos".to_string()),
        }));
        assert!(!should_drive_turn_submission(&Op::Rollback { turns: 1 }));
        assert!(!should_drive_turn_submission(&Op::Interrupt));
        assert!(!should_drive_turn_submission(&Op::RegisterDynamicTools {
            tools: vec![]
        }));
        assert!(!should_drive_turn_submission(&Op::SetClientCapabilities {
            capabilities: alan_protocol::ClientCapabilities::default(),
        }));
        assert!(!should_drive_turn_submission(&Op::Resume {
            request_id: "req-123".to_string(),
            content: vec![alan_protocol::ContentPart::structured(
                serde_json::json!({})
            )],
        }));
    }

    #[test]
    fn test_apply_persisted_state_governance_profile() {
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
            context_window_tokens: None,
            compaction_trigger_ratio: None,
            streaming_mode: None,
            partial_stream_recovery_mode: None,
            governance: Some(alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Conservative,
                policy_path: None,
            }),
        };

        config.apply_persisted_state(&persisted);

        assert_eq!(
            config.agent_config.runtime_config.governance.profile,
            alan_protocol::GovernanceProfile::Conservative
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_spawn_continues_when_workspace_persona_bootstrap_is_unwritable() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let alan_dir = workspace_root.join(".alan");
        let persona_dir = alan_dir.join("persona");

        std::fs::create_dir_all(&persona_dir).unwrap();
        std::fs::write(persona_dir.join("SOUL.md"), "existing persona").unwrap();

        let mut permissions = std::fs::metadata(&persona_dir).unwrap().permissions();
        permissions.set_mode(0o555);
        std::fs::set_permissions(&persona_dir, permissions).unwrap();

        let config = WorkspaceRuntimeConfig {
            workspace_root_dir: Some(workspace_root),
            workspace_alan_dir: Some(alan_dir),
            ..WorkspaceRuntimeConfig::default()
        };

        let llm_client = LlmClient::new(MockLlmProvider::new());
        let mut controller = spawn_with_llm_client(config, llm_client).unwrap();
        let ready = controller.wait_until_ready().await;

        let mut cleanup_permissions = std::fs::metadata(&persona_dir).unwrap().permissions();
        cleanup_permissions.set_mode(0o755);
        std::fs::set_permissions(&persona_dir, cleanup_permissions).unwrap();

        assert!(ready.is_ok());
        controller.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_spawn_initializes_bootstrap_for_memory_dir_persona_fallback() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let memory_dir = workspace_root.join(".alan/memory");
        let persona_dir = workspace_root.join(".alan/persona");
        std::fs::create_dir_all(&memory_dir).unwrap();

        let config = WorkspaceRuntimeConfig {
            agent_config: crate::AgentConfig {
                core_config: crate::Config {
                    memory: crate::config::MemoryConfig {
                        workspace_dir: Some(memory_dir),
                        strict_workspace: false,
                        ..crate::config::MemoryConfig::default()
                    },
                    ..crate::Config::default()
                },
                ..crate::AgentConfig::default()
            },
            ..WorkspaceRuntimeConfig::default()
        };

        let llm_client = LlmClient::new(MockLlmProvider::new());
        let mut controller = spawn_with_llm_client(config, llm_client).unwrap();
        let ready = controller.wait_until_ready().await;

        assert!(ready.is_ok());
        assert!(persona_dir.join("SOUL.md").exists());
        controller.shutdown().await.unwrap();
    }
}
