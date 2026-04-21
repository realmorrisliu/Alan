use super::agent_loop::RuntimeLoopState;
use super::engine::{
    AgentConfig, RuntimeController, RuntimeEventEnvelope, RuntimeStartupMetadata,
    WorkspaceRuntimeConfig, spawn_with_llm_client_and_tools,
};
use crate::llm::LlmClient;
use crate::tape::{ContentPart, Message};
use crate::tools::ToolRegistry;
use alan_protocol::{
    GovernanceConfig, Op, SpawnHandle, SpawnSpec, SpawnTarget, Submission, YieldKind,
};
use anyhow::{Context, Result, bail};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;

const MAX_CHILD_CONVERSATION_MESSAGES: usize = 8;
const MAX_CHILD_CONVERSATION_CHARS: usize = 4_000;
const MAX_CHILD_PLAN_ITEMS: usize = 16;
const MAX_CHILD_PLAN_ITEM_CHARS: usize = 240;
const MAX_CHILD_TOOL_RESULTS: usize = 6;
const MAX_CHILD_TOOL_RESULT_CHARS: usize = 1_200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChildRuntimeStatus {
    Completed,
    Paused,
    Cancelled,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChildRuntimePause {
    pub request_id: String,
    pub kind: YieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChildRuntimeResult {
    pub status: ChildRuntimeStatus,
    pub session_id: String,
    pub rollout_path: Option<PathBuf>,
    pub output_text: String,
    pub turn_summary: Option<String>,
    pub warnings: Vec<String>,
    pub error_message: Option<String>,
    pub pause: Option<ChildRuntimePause>,
}

#[derive(Debug)]
struct ObservedChildTerminalEvent {
    output_text: String,
    turn_summary: Option<String>,
    warnings: Vec<String>,
    error_message: Option<String>,
    pause: Option<ChildRuntimePause>,
    status: ChildRuntimeStatus,
}

enum ChildRuntimeWaitOutcome {
    Observed(ObservedChildTerminalEvent),
    Cancelled,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct ChildRuntimeController {
    runtime: Option<RuntimeController>,
    startup_metadata: RuntimeStartupMetadata,
    event_rx: tokio::sync::broadcast::Receiver<RuntimeEventEnvelope>,
    submission_id: String,
    timeout: Option<Duration>,
}

#[allow(dead_code)]
pub(crate) async fn spawn_child_runtime(
    parent: &RuntimeLoopState,
    spec: SpawnSpec,
) -> Result<ChildRuntimeController> {
    spawn_child_runtime_with_optional_cancel(parent, spec, None).await
}

#[allow(dead_code)]
pub(crate) async fn spawn_child_runtime_cancellable(
    parent: &RuntimeLoopState,
    spec: SpawnSpec,
    cancel: &CancellationToken,
) -> Result<ChildRuntimeController> {
    spawn_child_runtime_with_optional_cancel(parent, spec, Some(cancel)).await
}

async fn spawn_child_runtime_with_optional_cancel(
    parent: &RuntimeLoopState,
    spec: SpawnSpec,
    cancel: Option<&CancellationToken>,
) -> Result<ChildRuntimeController> {
    let chatgpt_auth_storage_path = parent.runtime_config.chatgpt_auth_storage_path.clone();
    spawn_child_runtime_with_client_factory_and_cancel(
        parent,
        spec,
        move |core_config| {
            LlmClient::from_core_config_with_chatgpt_auth_storage_path(
                core_config,
                chatgpt_auth_storage_path.clone(),
            )
        },
        cancel,
    )
    .await
}

#[cfg(test)]
async fn spawn_child_runtime_with_client_factory<F>(
    parent: &RuntimeLoopState,
    spec: SpawnSpec,
    llm_client_factory: F,
) -> Result<ChildRuntimeController>
where
    F: FnOnce(&crate::Config) -> Result<LlmClient>,
{
    spawn_child_runtime_with_client_factory_and_cancel(
        parent,
        spec,
        |core_config| llm_client_factory(core_config),
        None,
    )
    .await
}

async fn spawn_child_runtime_with_client_factory_and_cancel<F>(
    parent: &RuntimeLoopState,
    spec: SpawnSpec,
    llm_client_factory: F,
    cancel: Option<&CancellationToken>,
) -> Result<ChildRuntimeController>
where
    F: FnOnce(&crate::Config) -> Result<LlmClient>,
{
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        bail!("Child-agent launch cancelled");
    }

    validate_child_launch_contract(&spec)?;
    let launch_root_dir = resolve_launch_root_dir(parent, &spec.target)?;
    let child_agent_config = build_child_agent_config(parent, &spec);
    let workspace_root_dir = resolve_child_workspace_root(parent, &spec);
    let workspace_alan_dir = resolve_child_workspace_alan_dir(
        &spec,
        workspace_root_dir.as_deref(),
        parent.core_config.memory.workspace_dir.as_deref(),
    );
    let child_workspace_id = format!("{}:child:{}", parent.workspace_id, uuid::Uuid::new_v4());
    let default_cwd_override = spec
        .launch
        .cwd
        .clone()
        .or_else(|| workspace_root_dir.clone());

    let mut child_config = WorkspaceRuntimeConfig {
        agent_config: child_agent_config.clone(),
        // Child launches should still resolve their target/root overlays. Using the
        // default source keeps launch-root agent.toml in play instead of treating the
        // parent's effective config as a terminal env override.
        core_config_source: crate::ConfigSourceKind::Default,
        agent_name: None,
        session_id: None,
        workspace_id: child_workspace_id,
        workspace_root_dir,
        workspace_alan_dir,
        resume_rollout_path: None,
        launch_root_dir,
        default_cwd_override,
        agent_home_paths: None,
        chatgpt_auth_storage_path: parent.runtime_config.chatgpt_auth_storage_path.clone(),
    };
    let resolved_child_definition =
        crate::ResolvedAgentDefinition::from_runtime_config(&child_config)
            .context("Failed to resolve child-agent definition")?;
    let mut resolved_child_agent_config = child_agent_config.clone();
    if !resolved_child_definition.config_overlay_paths.is_empty() {
        resolved_child_agent_config = resolved_child_agent_config
            .with_agent_root_overlays(&resolved_child_definition.config_overlay_paths)
            .context("Failed to resolve effective child-agent config")?;
    }
    if spec.has_handle(SpawnHandle::Memory) {
        if let Some(alan_dir) = resolved_child_definition.workspace_alan_dir.as_ref() {
            resolved_child_agent_config.core_config.memory.workspace_dir =
                Some(crate::workspace_memory_dir_from_alan_dir(alan_dir));
        }
    } else {
        resolved_child_agent_config.core_config.memory.workspace_dir = None;
    }
    let effective_child_core_config = resolved_child_agent_config.core_config.clone();
    child_config.agent_config = resolved_child_agent_config;
    child_config.core_config_source = crate::ConfigSourceKind::EnvOverride;
    let child_tools = build_child_tool_registry(parent, &spec, &effective_child_core_config);

    let llm_client = llm_client_factory(&effective_child_core_config)
        .context("Failed to create child-agent LLM client")?;
    let runtime = spawn_with_llm_client_and_tools(child_config, llm_client, child_tools)
        .context("Failed to spawn child-agent runtime")?;
    let (runtime, startup_metadata) = wait_for_child_runtime_startup(runtime, cancel).await?;
    let event_rx = runtime.handle.event_sender.subscribe();
    let submission = Submission::new(Op::Turn {
        parts: vec![ContentPart::text(build_child_task_text(parent, &spec))],
        context: None,
    });
    let runtime = send_initial_child_submission(runtime, submission.clone(), cancel).await?;

    Ok(ChildRuntimeController {
        runtime: Some(runtime),
        startup_metadata,
        event_rx,
        submission_id: submission.id,
        timeout: spec.launch.timeout_secs.map(Duration::from_secs),
    })
}

async fn wait_for_child_runtime_startup(
    mut runtime: RuntimeController,
    cancel: Option<&CancellationToken>,
) -> Result<(RuntimeController, RuntimeStartupMetadata)> {
    let startup_metadata = if let Some(cancel) = cancel {
        if cancel.is_cancelled() {
            runtime.abort().await;
            bail!("Child-agent launch cancelled");
        }
        tokio::select! {
            _ = cancel.cancelled() => {
                runtime.abort().await;
                bail!("Child-agent launch cancelled");
            }
            ready = runtime.wait_until_ready() => {
                ready.context("Child-agent runtime failed to start")?
            }
        }
    } else {
        runtime
            .wait_until_ready()
            .await
            .context("Child-agent runtime failed to start")?
    };

    Ok((runtime, startup_metadata))
}

async fn send_initial_child_submission(
    runtime: RuntimeController,
    submission: Submission,
    cancel: Option<&CancellationToken>,
) -> Result<RuntimeController> {
    if let Some(cancel) = cancel {
        if cancel.is_cancelled() {
            runtime.abort().await;
            bail!("Child-agent launch cancelled");
        }
        tokio::select! {
            _ = cancel.cancelled() => {
                runtime.abort().await;
                bail!("Child-agent launch cancelled");
            }
            result = runtime.handle.submission_tx.send(submission) => {
                result.context("Failed to submit initial child-agent turn")?
            }
        }
    } else {
        runtime
            .handle
            .submission_tx
            .send(submission)
            .await
            .context("Failed to submit initial child-agent turn")?;
    }

    Ok(runtime)
}

fn validate_child_launch_contract(spec: &SpawnSpec) -> Result<()> {
    if spec.has_handle(SpawnHandle::Artifacts) || spec.launch.output_dir.is_some() {
        bail!(
            "Child-agent launches do not support artifact routing yet; omit SpawnHandle::Artifacts and launch.output_dir."
        );
    }

    if let Some(workspace_root) = spec.launch.workspace_root.as_deref()
        && !workspace_root.is_absolute()
    {
        bail!(
            "Child-agent launch workspace_root '{}' must be absolute.",
            workspace_root.display()
        );
    }

    if let Some(cwd) = spec.launch.cwd.as_deref()
        && !cwd.is_absolute()
    {
        bail!(
            "Child-agent launch cwd '{}' must be absolute.",
            cwd.display()
        );
    }

    if let (Some(workspace_root), Some(cwd)) = (
        spec.launch.workspace_root.as_deref(),
        spec.launch.cwd.as_deref(),
    ) {
        let normalized_workspace_root = lexically_normalize_path(workspace_root);
        let normalized_cwd = lexically_normalize_path(cwd);
        if !normalized_cwd.starts_with(&normalized_workspace_root) {
            bail!(
                "Child-agent launch cwd '{}' must stay within workspace_root '{}'.",
                normalized_cwd.display(),
                normalized_workspace_root.display()
            );
        }
    }

    Ok(())
}

fn lexically_normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn resolve_launch_root_dir(
    parent: &RuntimeLoopState,
    target: &SpawnTarget,
) -> Result<Option<PathBuf>> {
    match target {
        SpawnTarget::ResolvedAgentRoot { root_dir } => Ok(Some(root_dir.clone())),
        SpawnTarget::PackageChildAgent { .. } => parent
            .prompt_cache
            .capability_view()
            .map(crate::skills::ResolvedCapabilityView::refresh)
            .and_then(|view| view.resolve_child_agent_target(target))
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("Unknown package child-agent target: {target:?}")),
    }
}

#[allow(dead_code)]
impl ChildRuntimeController {
    pub(crate) fn startup_metadata(&self) -> &RuntimeStartupMetadata {
        &self.startup_metadata
    }

    pub(crate) async fn join(mut self) -> Result<ChildRuntimeResult> {
        let observed = match self
            .wait_for_terminal_event_with_optional_cancel(None)
            .await?
        {
            ChildRuntimeWaitOutcome::Observed(observed) => observed,
            ChildRuntimeWaitOutcome::Cancelled => {
                return Ok(self.cancelled_result());
            }
        };

        self.finish_after_observed_terminal_event(observed).await
    }

    pub(crate) async fn join_until_cancelled(
        mut self,
        cancel: &CancellationToken,
    ) -> Result<ChildRuntimeResult> {
        match self
            .wait_for_terminal_event_with_optional_cancel(Some(cancel))
            .await?
        {
            ChildRuntimeWaitOutcome::Observed(observed) => {
                self.finish_after_observed_terminal_event(observed).await
            }
            ChildRuntimeWaitOutcome::Cancelled => Ok(self.cancelled_result()),
        }
    }

    async fn finish_after_observed_terminal_event(
        &mut self,
        observed: ObservedChildTerminalEvent,
    ) -> Result<ChildRuntimeResult> {
        let mut warnings = self.startup_metadata.warnings.clone();
        warnings.extend(observed.warnings);
        self.terminate_runtime().await;

        Ok(ChildRuntimeResult {
            status: observed.status,
            session_id: self.startup_metadata.session_id.clone(),
            rollout_path: self.startup_metadata.rollout_path.clone(),
            output_text: observed.output_text,
            turn_summary: observed.turn_summary,
            warnings,
            error_message: observed.error_message,
            pause: observed.pause,
        })
    }

    pub(crate) async fn cancel(mut self) -> Result<ChildRuntimeResult> {
        let result = self.cancelled_result();
        self.terminate_runtime().await;
        Ok(result)
    }

    fn cancelled_result(&self) -> ChildRuntimeResult {
        ChildRuntimeResult {
            status: ChildRuntimeStatus::Cancelled,
            session_id: self.startup_metadata.session_id.clone(),
            rollout_path: self.startup_metadata.rollout_path.clone(),
            output_text: String::new(),
            turn_summary: None,
            warnings: self.startup_metadata.warnings.clone(),
            error_message: None,
            pause: None,
        }
    }

    async fn wait_for_terminal_event_with_optional_cancel(
        &mut self,
        cancel: Option<&CancellationToken>,
    ) -> Result<ChildRuntimeWaitOutcome> {
        if cancel.is_some_and(CancellationToken::is_cancelled) {
            self.terminate_runtime().await;
            return Ok(ChildRuntimeWaitOutcome::Cancelled);
        }

        let timeout_sleep = self.timeout.map(tokio::time::sleep);
        tokio::pin!(timeout_sleep);

        let mut output_text = String::new();
        let mut warnings = Vec::new();

        loop {
            let recv = if let Some(timeout_sleep) = timeout_sleep.as_mut().as_pin_mut() {
                if let Some(cancel) = cancel {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            self.terminate_runtime().await;
                            return Ok(ChildRuntimeWaitOutcome::Cancelled);
                        }
                        _ = timeout_sleep => {
                            self.abort_runtime().await;
                            return Ok(ChildRuntimeWaitOutcome::Observed(
                                self.timed_out_observed_event(),
                            ));
                        }
                        recv = self.event_rx.recv() => recv,
                    }
                } else {
                    tokio::select! {
                        _ = timeout_sleep => {
                            self.abort_runtime().await;
                            return Ok(ChildRuntimeWaitOutcome::Observed(
                                self.timed_out_observed_event(),
                            ));
                        }
                        recv = self.event_rx.recv() => recv,
                    }
                }
            } else if let Some(cancel) = cancel {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        self.terminate_runtime().await;
                        return Ok(ChildRuntimeWaitOutcome::Cancelled);
                    }
                    recv = self.event_rx.recv() => recv,
                }
            } else {
                self.event_rx.recv().await
            };

            match self.observe_terminal_event(recv, &mut output_text, &mut warnings) {
                Some(observed) => return Ok(ChildRuntimeWaitOutcome::Observed(observed)),
                None => continue,
            }
        }
    }

    fn observe_terminal_event(
        &self,
        recv: std::result::Result<RuntimeEventEnvelope, RecvError>,
        output_text: &mut String,
        warnings: &mut Vec<String>,
    ) -> Option<ObservedChildTerminalEvent> {
        match recv {
            Ok(envelope) => {
                if envelope.submission_id.as_deref() != Some(self.submission_id.as_str()) {
                    return None;
                }

                match envelope.event {
                    alan_protocol::Event::TextDelta { chunk, .. } => {
                        if !chunk.is_empty() {
                            output_text.push_str(&chunk);
                        }
                        None
                    }
                    alan_protocol::Event::Warning { message } => {
                        warnings.push(message);
                        None
                    }
                    alan_protocol::Event::TurnCompleted { summary } => {
                        Some(ObservedChildTerminalEvent {
                            output_text: output_text.clone(),
                            turn_summary: summary,
                            warnings: warnings.clone(),
                            error_message: None,
                            pause: None,
                            status: ChildRuntimeStatus::Completed,
                        })
                    }
                    alan_protocol::Event::Yield {
                        request_id, kind, ..
                    } => Some(ObservedChildTerminalEvent {
                        output_text: output_text.clone(),
                        turn_summary: None,
                        warnings: warnings.clone(),
                        error_message: None,
                        pause: Some(ChildRuntimePause { request_id, kind }),
                        status: ChildRuntimeStatus::Paused,
                    }),
                    alan_protocol::Event::Error {
                        message,
                        recoverable,
                    } if !recoverable => Some(ObservedChildTerminalEvent {
                        output_text: output_text.clone(),
                        turn_summary: None,
                        warnings: warnings.clone(),
                        error_message: Some(message),
                        pause: None,
                        status: ChildRuntimeStatus::Failed,
                    }),
                    alan_protocol::Event::Error { message, .. } => {
                        warnings.push(message);
                        None
                    }
                    _ => None,
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                let message = format!(
                    "Child-agent runtime event stream lagged by {skipped} event(s) before a terminal event could be observed"
                );
                warnings.push(message.clone());
                Some(ObservedChildTerminalEvent {
                    output_text: output_text.clone(),
                    turn_summary: None,
                    warnings: warnings.clone(),
                    error_message: Some(message),
                    pause: None,
                    status: ChildRuntimeStatus::Failed,
                })
            }
            Err(RecvError::Closed) => Some(ObservedChildTerminalEvent {
                output_text: output_text.clone(),
                turn_summary: None,
                warnings: warnings.clone(),
                error_message: Some(
                    "Child-agent runtime stopped before producing a terminal event".to_string(),
                ),
                pause: None,
                status: ChildRuntimeStatus::Failed,
            }),
        }
    }

    async fn terminate_runtime(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            let _ = runtime.shutdown().await;
        }
    }

    async fn abort_runtime(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.abort().await;
        }
    }

    fn timed_out_observed_event(&self) -> ObservedChildTerminalEvent {
        ObservedChildTerminalEvent {
            output_text: String::new(),
            turn_summary: None,
            warnings: Vec::new(),
            error_message: Some("Child-agent turn timed out".to_string()),
            pause: None,
            status: ChildRuntimeStatus::TimedOut,
        }
    }
}

fn build_child_agent_config(parent: &RuntimeLoopState, spec: &SpawnSpec) -> AgentConfig {
    let mut child_agent_config = AgentConfig::from(parent.core_config.clone());
    child_agent_config.runtime_config = parent.runtime_config.clone();

    if !spec.has_handle(SpawnHandle::Memory) {
        child_agent_config.core_config.memory.workspace_dir = None;
    }

    if spec.has_handle(SpawnHandle::ApprovalScope) {
        child_agent_config.runtime_config.governance = parent.runtime_config.governance.clone();
    } else {
        child_agent_config.runtime_config.governance = GovernanceConfig::default();
    }

    if let Some(model) = spec.runtime_overrides.model.as_deref() {
        child_agent_config.set_model_override(model);
    }
    if let Some(budget_tokens) = spec.launch.budget_tokens {
        child_agent_config.set_thinking_budget_override(Some(budget_tokens));
    }
    if let Some(policy_path) = spec.runtime_overrides.policy_path.clone() {
        child_agent_config.runtime_config.governance.policy_path = Some(policy_path);
    }

    child_agent_config
}

fn build_child_tool_registry(
    parent: &RuntimeLoopState,
    spec: &SpawnSpec,
    child_core_config: &crate::Config,
) -> ToolRegistry {
    let child_config = Arc::new(child_core_config.clone());
    if !spec.has_handle(SpawnHandle::Workspace) {
        return ToolRegistry::with_config(child_config);
    }

    let mut tools = if let Some(workspace_root) = spec.launch.workspace_root.as_deref() {
        let mut rebound = ToolRegistry::with_config(Arc::clone(&child_config));
        let selected_tool_names = spec
            .runtime_overrides
            .tool_profile
            .as_ref()
            .map(|tool_profile| tool_profile.allowed_tools.clone())
            .unwrap_or_else(|| {
                parent
                    .tools
                    .list_tools()
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            });

        for tool_name in selected_tool_names {
            if let Some(tool) = parent.tools.get(&tool_name) {
                if let Some(rebound_tool) = tool.rebind_workspace(workspace_root) {
                    rebound.register_boxed(rebound_tool);
                } else {
                    rebound.register_shared(tool);
                }
                continue;
            }

            if let Some(materialized_tool) = parent
                .tools
                .materialize_for_workspace(&tool_name, workspace_root)
            {
                rebound.register_boxed(materialized_tool);
            }
        }
        rebound
    } else if let Some(tool_profile) = spec.runtime_overrides.tool_profile.as_ref() {
        parent
            .tools
            .filtered_clone_with_config(&tool_profile.allowed_tools, child_config)
    } else {
        parent.tools.clone_with_config(child_config)
    };

    if let Some(cwd) = spec
        .launch
        .cwd
        .clone()
        .or_else(|| spec.launch.workspace_root.clone())
        .or_else(|| parent.tools.default_cwd())
    {
        tools.set_default_cwd(cwd);
    }
    tools
}

fn resolve_child_workspace_root(parent: &RuntimeLoopState, spec: &SpawnSpec) -> Option<PathBuf> {
    spec.launch.workspace_root.clone().or_else(|| {
        if spec.has_handle(SpawnHandle::Workspace) {
            infer_workspace_root_from_memory_dir(parent.core_config.memory.workspace_dir.as_deref())
        } else {
            None
        }
    })
}

fn resolve_child_workspace_alan_dir(
    spec: &SpawnSpec,
    workspace_root_dir: Option<&Path>,
    memory_dir: Option<&Path>,
) -> Option<PathBuf> {
    if !spec.has_handle(SpawnHandle::Memory) && !preserves_workspace_policy_context(spec) {
        return None;
    }

    workspace_root_dir
        .map(|root| root.join(".alan"))
        .or_else(|| infer_workspace_alan_dir_from_memory_dir(memory_dir))
}

fn preserves_workspace_policy_context(spec: &SpawnSpec) -> bool {
    spec.has_handle(SpawnHandle::ApprovalScope) || spec.runtime_overrides.policy_path.is_some()
}

fn infer_workspace_alan_dir_from_memory_dir(memory_dir: Option<&Path>) -> Option<PathBuf> {
    let memory_dir = memory_dir?;
    if memory_dir.file_name()? != "memory" {
        return None;
    }
    let alan_dir = memory_dir.parent()?;
    (alan_dir.file_name()? == ".alan").then(|| alan_dir.to_path_buf())
}

pub(super) fn infer_workspace_root_from_memory_dir(memory_dir: Option<&Path>) -> Option<PathBuf> {
    let alan_dir = infer_workspace_alan_dir_from_memory_dir(memory_dir);
    let alan_dir = alan_dir.as_deref()?;
    (alan_dir.file_name()? == ".alan").then(|| alan_dir.parent().map(Path::to_path_buf))?
}

fn build_child_task_text(parent: &RuntimeLoopState, spec: &SpawnSpec) -> String {
    let mut sections = vec![spec.launch.task.trim().to_string()];

    if let Some(metadata) = render_launch_metadata(spec) {
        sections.push(metadata);
    }
    if spec.has_handle(SpawnHandle::ConversationSnapshot)
        && let Some(snapshot) = render_conversation_snapshot(parent)
    {
        sections.push(snapshot);
    }
    if spec.has_handle(SpawnHandle::Plan)
        && let Some(snapshot) = render_plan_snapshot(parent)
    {
        sections.push(snapshot);
    }
    if spec.has_handle(SpawnHandle::ToolResults)
        && let Some(snapshot) = render_tool_results_snapshot(parent)
    {
        sections.push(snapshot);
    }

    sections
        .into_iter()
        .filter(|section| !section.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_launch_metadata(spec: &SpawnSpec) -> Option<String> {
    let mut lines = Vec::new();
    if let Some(cwd) = spec.launch.cwd.as_ref() {
        lines.push(format!("cwd: {}", cwd.display()));
    }
    if let Some(workspace_root) = spec.launch.workspace_root.as_ref() {
        lines.push(format!("workspace_root: {}", workspace_root.display()));
    }
    if let Some(output_dir) = spec.launch.output_dir.as_ref() {
        lines.push(format!("output_dir: {}", output_dir.display()));
    }
    if let Some(budget_tokens) = spec.launch.budget_tokens {
        lines.push(format!("budget_tokens: {budget_tokens}"));
    }

    (!lines.is_empty()).then(|| format!("Execution Context\n{}", lines.join("\n")))
}

fn render_conversation_snapshot(parent: &RuntimeLoopState) -> Option<String> {
    let mut lines = Vec::new();
    if let Some(summary) = parent.session.tape.summary() {
        lines.push("Summary:".to_string());
        lines.push(truncate_chars(summary.trim(), MAX_CHILD_CONVERSATION_CHARS));
    }

    let recent_messages = parent
        .session
        .tape
        .messages()
        .iter()
        .rev()
        .filter(|message| matches!(message, Message::User { .. } | Message::Assistant { .. }))
        .take(MAX_CHILD_CONVERSATION_MESSAGES)
        .cloned()
        .collect::<Vec<_>>();

    if !recent_messages.is_empty() {
        lines.push("Recent Messages:".to_string());
        for message in recent_messages.into_iter().rev() {
            let role = match &message {
                Message::User { .. } => "user",
                Message::Assistant { .. } => "assistant",
                Message::Tool { .. } => unreachable!("tool messages are filtered out above"),
                Message::System { .. } => "system",
                Message::Context { .. } => "context",
            };
            let text = match &message {
                Message::Assistant { .. } => message.non_thinking_text_content(),
                _ => message.text_content(),
            };
            if !text.trim().is_empty() {
                lines.push(format!(
                    "- {role}: {}",
                    truncate_chars(text.trim(), MAX_CHILD_CONVERSATION_CHARS / 2)
                ));
            }
        }
    }

    (!lines.is_empty()).then(|| format!("Parent Conversation Snapshot\n{}", lines.join("\n")))
}

fn render_plan_snapshot(parent: &RuntimeLoopState) -> Option<String> {
    let plan_snapshot = parent.turn_state.plan_snapshot()?;
    let mut lines = Vec::new();
    if let Some(explanation) = plan_snapshot.explanation.as_deref()
        && !explanation.trim().is_empty()
    {
        lines.push(format!(
            "Explanation: {}",
            truncate_chars(explanation.trim(), MAX_CHILD_PLAN_ITEM_CHARS)
        ));
    }
    for item in plan_snapshot.items.iter().take(MAX_CHILD_PLAN_ITEMS) {
        lines.push(format!(
            "- [{}] {}",
            match item.status {
                alan_protocol::PlanItemStatus::Pending => "pending",
                alan_protocol::PlanItemStatus::InProgress => "in_progress",
                alan_protocol::PlanItemStatus::Completed => "completed",
            },
            truncate_chars(item.content.trim(), MAX_CHILD_PLAN_ITEM_CHARS)
        ));
    }

    (!lines.is_empty()).then(|| format!("Parent Plan Snapshot\n{}", lines.join("\n")))
}

fn render_tool_results_snapshot(parent: &RuntimeLoopState) -> Option<String> {
    let mut lines = Vec::new();
    for message in parent
        .session
        .tape
        .messages()
        .iter()
        .rev()
        .filter(|message| matches!(message, Message::Tool { .. }))
        .take(MAX_CHILD_TOOL_RESULTS)
    {
        for response in message.tool_responses() {
            let content =
                truncate_chars(response.text_content().trim(), MAX_CHILD_TOOL_RESULT_CHARS);
            if !content.is_empty() {
                lines.push(format!("- {}: {}", response.id, content));
            }
        }
    }
    lines.reverse();
    (!lines.is_empty()).then(|| format!("Parent Tool Results\n{}", lines.join("\n")))
}

fn truncate_chars(text: &str, limit: usize) -> String {
    let truncated: String = text.chars().take(limit).collect();
    if truncated.chars().count() == text.chars().count() {
        truncated
    } else {
        format!("{truncated}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{GenerationRequest, GenerationResponse, StreamChunk, TokenUsage};
    use crate::runtime::RuntimeConfig;
    use crate::skills::SkillHostCapabilities;
    use crate::tools::Tool;
    use alan_llm::LlmProvider;
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[derive(Clone, Default)]
    struct RecordedRequests(Arc<Mutex<Vec<GenerationRequest>>>);

    #[derive(Clone)]
    struct RecordingProvider {
        requests: RecordedRequests,
        response: GenerationResponse,
        delay: Option<Duration>,
    }

    impl RecordingProvider {
        fn new(requests: RecordedRequests, response: GenerationResponse) -> Self {
            Self {
                requests,
                response,
                delay: None,
            }
        }

        fn with_delay(mut self, delay: Duration) -> Self {
            self.delay = Some(delay);
            self
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for RecordingProvider {
        async fn generate(
            &mut self,
            request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            self.requests.0.lock().unwrap().push(request);
            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }
            Ok(self.response.clone())
        }

        async fn chat(&mut self, _system: Option<&str>, user: &str) -> anyhow::Result<String> {
            Ok(format!("chat: {user}"))
        }

        async fn generate_stream(
            &mut self,
            request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            self.requests.0.lock().unwrap().push(request);
            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(StreamChunk {
                    text: Some(self.response.content.clone()),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: Some(TokenUsage {
                        prompt_tokens: 1,
                        cached_prompt_tokens: None,
                        completion_tokens: 1,
                        total_tokens: 2,
                        reasoning_tokens: None,
                    }),
                    provider_response_id: None,
                    provider_response_status: None,
                    sequence_number: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stop".to_string()),
                })
                .await;
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "openai_responses"
        }
    }

    struct NamedTestTool {
        name: String,
    }

    impl NamedTestTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    impl Tool for NamedTestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "test tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {}
            })
        }

        fn execute(
            &self,
            _arguments: serde_json::Value,
            _ctx: &crate::tools::ToolContext,
        ) -> crate::tools::ToolResult {
            Box::pin(async { Ok(json!({"ok": true})) })
        }
    }

    struct WorkspaceBoundTestTool {
        name: String,
        workspace_root: PathBuf,
    }

    impl WorkspaceBoundTestTool {
        fn new(name: &str, workspace_root: PathBuf) -> Self {
            Self {
                name: name.to_string(),
                workspace_root,
            }
        }
    }

    impl Tool for WorkspaceBoundTestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "workspace-bound test tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {
                        "type": "string"
                    }
                }
            })
        }

        fn execute(
            &self,
            arguments: serde_json::Value,
            ctx: &crate::tools::ToolContext,
        ) -> crate::tools::ToolResult {
            let workspace_root = self.workspace_root.clone();
            let path = ctx.resolve_path(arguments["path"].as_str().unwrap_or(""));
            Box::pin(async move {
                if !path.starts_with(&workspace_root) {
                    anyhow::bail!(
                        "outside workspace: '{}' not within '{}'",
                        path.display(),
                        workspace_root.display()
                    );
                }

                let content = tokio::fs::read_to_string(&path).await?;
                Ok(json!({
                    "path": path.to_string_lossy(),
                    "content": content
                }))
            })
        }

        fn rebind_workspace(&self, workspace_root: &Path) -> Option<Box<dyn Tool>> {
            Some(Box::new(Self::new(
                &self.name,
                workspace_root.to_path_buf(),
            )))
        }
    }

    fn make_parent_state(
        temp: &TempDir,
        requests: RecordedRequests,
        response: GenerationResponse,
    ) -> RuntimeLoopState {
        make_parent_state_with_capability_view(
            temp,
            requests,
            response,
            crate::skills::ResolvedCapabilityView::default(),
        )
    }

    fn make_parent_state_with_capability_view(
        temp: &TempDir,
        requests: RecordedRequests,
        response: GenerationResponse,
        capability_view: crate::skills::ResolvedCapabilityView,
    ) -> RuntimeLoopState {
        let workspace_root = temp.path().join("repo");
        let workspace_alan_dir = workspace_root.join(".alan");
        let launch_root = workspace_root.join(".alan/agents/grader");
        std::fs::create_dir_all(launch_root.join("persona")).unwrap();
        std::fs::create_dir_all(workspace_alan_dir.join("sessions")).unwrap();
        std::fs::create_dir_all(launch_root.join("skills")).unwrap();
        std::fs::write(launch_root.join("agent.toml"), "tool_repeat_limit = 4\n").unwrap();

        let mut core_config = crate::Config::default();
        core_config.memory.workspace_dir = Some(workspace_alan_dir.join("memory"));
        core_config.openai_responses_model = "gpt-5.4".to_string();
        let mut tools = ToolRegistry::with_config(Arc::new(core_config.clone()));
        tools.set_default_cwd(workspace_root.clone());
        tools.register(NamedTestTool::new("alpha"));
        tools.register(NamedTestTool::new("beta"));

        let mut session = crate::Session::new();
        session.add_user_message("Parent user asks for review");
        session.add_assistant_message("Parent assistant explains the approach", None);
        session.add_tool_message("tool_call_1", "alpha", json!({"summary": "tool output"}));

        let mut turn_state = super::super::TurnState::default();
        turn_state.set_plan_snapshot(
            Some("Finish the delegated check".to_string()),
            vec![alan_protocol::PlanItem {
                id: "plan-1".to_string(),
                content: "Inspect the changed files".to_string(),
                status: alan_protocol::PlanItemStatus::InProgress,
            }],
        );

        RuntimeLoopState {
            workspace_id: "parent-workspace".to_string(),
            session,
            current_submission_id: None,
            llm_client: LlmClient::new(RecordingProvider::new(requests, response)),
            core_config,
            runtime_config: RuntimeConfig::default(),
            workspace_persona_dirs: Vec::new(),
            tools,
            prompt_cache:
                super::super::prompt_cache::PromptAssemblyCache::with_fixed_capability_view(
                    capability_view,
                    Vec::new(),
                    SkillHostCapabilities::with_tools(["alpha", "beta"]),
                ),
            turn_state,
        }
    }

    fn launch_spec(root_dir: PathBuf) -> SpawnSpec {
        SpawnSpec {
            target: SpawnTarget::ResolvedAgentRoot { root_dir },
            launch: alan_protocol::SpawnLaunchInputs {
                task: "Review the repository changes".to_string(),
                cwd: None,
                workspace_root: None,
                timeout_secs: Some(30),
                budget_tokens: None,
                output_dir: None,
            },
            handles: Vec::new(),
            runtime_overrides: alan_protocol::SpawnRuntimeOverrides::default(),
        }
    }

    fn completed_response(text: &str) -> GenerationResponse {
        GenerationResponse {
            content: text.to_string(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: Vec::new(),
            tool_calls: Vec::new(),
            usage: Some(TokenUsage {
                prompt_tokens: 8,
                cached_prompt_tokens: None,
                completion_tokens: 4,
                total_tokens: 12,
                reasoning_tokens: None,
            }),
            finish_reason: None,
            warnings: Vec::new(),
            provider_response_id: None,
            provider_response_status: None,
        }
    }

    fn capability_view_with_package_child_agent(
        temp: &TempDir,
    ) -> crate::skills::ResolvedCapabilityView {
        let workspace_root = temp.path().join("repo");
        let package_root = workspace_root.join(".alan/agent/skills/repo-review");
        std::fs::create_dir_all(package_root.join("agents/reviewer")).unwrap();
        std::fs::write(
            package_root.join("SKILL.md"),
            r#"---
name: Repo Review
description: Review repository changes
---

Body
"#,
        )
        .unwrap();
        std::fs::write(
            package_root.join("agents/reviewer/agent.toml"),
            "tool_repeat_limit = 4\n",
        )
        .unwrap();
        crate::skills::ResolvedCapabilityView::from_package_dirs(vec![
            crate::skills::ScopedPackageDir {
                path: workspace_root.join(".alan/agent/skills"),
                scope: crate::skills::SkillScope::Repo,
            },
        ])
    }

    #[tokio::test]
    async fn spawn_child_runtime_defaults_to_exec_like_non_inheritance() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let parent = make_parent_state_with_capability_view(
            &temp,
            requests.clone(),
            response.clone(),
            crate::skills::ResolvedCapabilityView::default(),
        );
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let spec = launch_spec(root_dir);

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        assert_eq!(result.output_text, "Child finished cleanly.");
        let recorded = requests.0.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert!(request.tools.iter().all(|tool| tool.name != "alpha"));
        assert!(request.tools.iter().all(|tool| tool.name != "beta"));
        let user_text = request
            .messages
            .iter()
            .map(|message| message.content.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(user_text.contains("Review the repository changes"));
        assert!(!user_text.contains("Parent Conversation Snapshot"));
        assert!(!user_text.contains("Parent Plan Snapshot"));
        assert!(!user_text.contains("Parent Tool Results"));
    }

    #[tokio::test]
    async fn spawn_child_runtime_binds_requested_parent_handles() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Bound handles processed.");
        let parent = make_parent_state_with_capability_view(
            &temp,
            requests.clone(),
            response.clone(),
            crate::skills::ResolvedCapabilityView::default(),
        );
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let mut spec = launch_spec(root_dir);
        spec.handles = vec![
            SpawnHandle::ConversationSnapshot,
            SpawnHandle::Plan,
            SpawnHandle::ToolResults,
        ];

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        let recorded = requests.0.lock().unwrap();
        let user_text = recorded
            .iter()
            .flat_map(|request| {
                request
                    .messages
                    .iter()
                    .map(|message| message.content.clone())
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(user_text.contains("Parent Conversation Snapshot"));
        assert!(user_text.contains("Parent Plan Snapshot"));
        assert!(user_text.contains("Parent Tool Results"));
        assert!(user_text.contains("Inspect the changed files"));
        assert!(user_text.contains("tool output"));
    }

    #[tokio::test]
    async fn spawn_child_runtime_rejects_artifact_handle_without_runtime_binding() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Artifacts are not supported.");
        let parent = make_parent_state(&temp, requests, response);
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let mut spec = launch_spec(root_dir);
        spec.handles = vec![SpawnHandle::Artifacts];

        let err = match spawn_child_runtime_with_client_factory(&parent, spec, |_| unreachable!())
            .await
        {
            Ok(_) => panic!("artifact handle should be rejected until artifact routing exists"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("Child-agent launches do not support artifact routing yet")
        );
    }

    #[tokio::test]
    async fn spawn_child_runtime_rejects_output_dir_without_runtime_binding() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Artifacts are not supported.");
        let parent = make_parent_state(&temp, requests, response);
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let mut spec = launch_spec(root_dir);
        spec.launch.output_dir = Some(temp.path().join("repo/out"));

        let err = match spawn_child_runtime_with_client_factory(&parent, spec, |_| unreachable!())
            .await
        {
            Ok(_) => panic!("output_dir should be rejected until artifact routing exists"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("Child-agent launches do not support artifact routing yet")
        );
    }

    #[tokio::test]
    async fn spawn_child_runtime_filters_workspace_tools_with_override() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Only one tool should be visible.");
        let parent = make_parent_state_with_capability_view(
            &temp,
            requests.clone(),
            response.clone(),
            crate::skills::ResolvedCapabilityView::default(),
        );
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let mut spec = launch_spec(root_dir);
        spec.handles = vec![SpawnHandle::Workspace];
        spec.runtime_overrides.tool_profile = Some(alan_protocol::SpawnToolProfileOverride {
            allowed_tools: vec!["alpha".to_string()],
        });

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        let recorded = requests.0.lock().unwrap();
        let tool_names = recorded[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();
        assert!(tool_names.contains(&"alpha"));
        assert!(!tool_names.contains(&"beta"));
    }

    #[tokio::test]
    async fn spawn_child_runtime_respects_empty_workspace_tool_override() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("No tools should be visible.");
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let mut spec = launch_spec(root_dir);
        spec.handles = vec![SpawnHandle::Workspace];
        spec.runtime_overrides.tool_profile = Some(alan_protocol::SpawnToolProfileOverride {
            allowed_tools: Vec::new(),
        });

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        let recorded = requests.0.lock().unwrap();
        let tool_names = recorded[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();
        assert!(!tool_names.contains(&"alpha"));
        assert!(!tool_names.contains(&"beta"));
    }

    #[tokio::test]
    async fn build_child_tool_registry_rebinds_workspace_sensitive_tools_for_requested_workspace() {
        let temp = TempDir::new().unwrap();
        let parent_root = temp.path().join("repo");
        let child_root = temp.path().join("other-repo");
        std::fs::create_dir_all(&child_root).unwrap();
        std::fs::write(child_root.join("target.txt"), "child workspace contents\n").unwrap();

        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let mut parent = make_parent_state(&temp, requests, response);
        let mut parent_tools = ToolRegistry::new();
        parent_tools.set_default_cwd(parent_root.clone());
        parent_tools.register(WorkspaceBoundTestTool::new("workspace_read", parent_root));
        parent.tools = parent_tools;

        let mut spec = launch_spec(temp.path().join("repo/.alan/agents/grader"));
        spec.handles = vec![SpawnHandle::Workspace];
        spec.launch.workspace_root = Some(child_root.clone());
        spec.launch.cwd = Some(child_root.clone());

        let child_tools = build_child_tool_registry(&parent, &spec, &parent.core_config);
        let result = child_tools
            .execute("workspace_read", json!({ "path": "target.txt" }))
            .await
            .unwrap();

        assert_eq!(result["content"], json!("child workspace contents\n"));
        assert_eq!(
            result["path"],
            json!(child_root.join("target.txt").to_string_lossy().to_string())
        );
    }

    #[tokio::test]
    async fn build_child_tool_registry_materializes_workspace_tools_from_parent_factories() {
        let temp = TempDir::new().unwrap();
        let parent_root = temp.path().join("repo");
        let child_root = temp.path().join("other-repo");
        std::fs::create_dir_all(&child_root).unwrap();
        std::fs::write(child_root.join("target.txt"), "child workspace contents\n").unwrap();

        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let mut parent = make_parent_state(&temp, requests, response);
        let mut parent_tools = ToolRegistry::new();
        parent_tools.set_default_cwd(parent_root);
        parent_tools.register_workspace_factory("workspace_read", |workspace_root| {
            Box::new(WorkspaceBoundTestTool::new(
                "workspace_read",
                workspace_root.to_path_buf(),
            ))
        });
        parent.tools = parent_tools;

        let mut spec = launch_spec(temp.path().join("repo/.alan/agents/grader"));
        spec.handles = vec![SpawnHandle::Workspace];
        spec.launch.workspace_root = Some(child_root.clone());
        spec.launch.cwd = Some(child_root.clone());
        spec.runtime_overrides.tool_profile = Some(alan_protocol::SpawnToolProfileOverride {
            allowed_tools: vec!["workspace_read".to_string()],
        });

        let child_tools = build_child_tool_registry(&parent, &spec, &parent.core_config);
        let result = child_tools
            .execute("workspace_read", json!({ "path": "target.txt" }))
            .await
            .unwrap();

        assert_eq!(result["content"], json!("child workspace contents\n"));
        assert_eq!(
            result["path"],
            json!(child_root.join("target.txt").to_string_lossy().to_string())
        );
    }

    #[tokio::test]
    async fn spawn_child_runtime_conversation_snapshot_excludes_tool_outputs_without_handle() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Snapshot captured.");
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let mut spec = launch_spec(root_dir);
        spec.handles = vec![SpawnHandle::ConversationSnapshot];

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        let recorded = requests.0.lock().unwrap();
        let user_text = recorded
            .iter()
            .flat_map(|request| {
                request
                    .messages
                    .iter()
                    .map(|message| message.content.clone())
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(user_text.contains("Parent Conversation Snapshot"));
        assert!(!user_text.contains("tool output"));
    }

    #[tokio::test]
    async fn spawn_child_runtime_uses_effective_launch_root_config_for_llm_setup() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        std::fs::write(
            root_dir.join("agent.toml"),
            r#"
tool_repeat_limit = 9
"#,
        )
        .unwrap();
        let seen_config = Arc::new(Mutex::new(None::<crate::Config>));
        let seen_config_for_factory = seen_config.clone();

        let child =
            spawn_child_runtime_with_client_factory(&parent, launch_spec(root_dir), |config| {
                *seen_config_for_factory.lock().unwrap() = Some(config.clone());
                Ok(LlmClient::new(RecordingProvider::new(
                    requests.clone(),
                    response.clone(),
                )))
            })
            .await
            .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        let seen_config = seen_config.lock().unwrap().clone().unwrap();
        assert_eq!(seen_config.effective_model(), "gpt-5.4");
        assert_eq!(seen_config.tool_repeat_limit, 9);
    }

    #[tokio::test]
    async fn spawn_child_runtime_reapplies_model_and_budget_overrides_after_overlay() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        std::fs::write(
            root_dir.join("agent.toml"),
            r#"
thinking_budget_tokens = 1024
"#,
        )
        .unwrap();
        let seen_config = Arc::new(Mutex::new(None::<crate::Config>));
        let seen_config_for_factory = seen_config.clone();
        let mut spec = launch_spec(root_dir);
        spec.runtime_overrides.model = Some("gpt-5-mini".to_string());
        spec.launch.budget_tokens = Some(512);

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |config| {
            *seen_config_for_factory.lock().unwrap() = Some(config.clone());
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        let seen_config = seen_config.lock().unwrap().clone().unwrap();
        assert_eq!(seen_config.effective_model(), "gpt-5-mini");
        assert_eq!(seen_config.thinking_budget_tokens, Some(512));

        let recorded = requests.0.lock().unwrap();
        assert_eq!(recorded[0].thinking_budget_tokens, Some(512));
    }

    #[test]
    fn child_workspace_alan_dir_requires_memory_or_policy_context() {
        let workspace_root = PathBuf::from("/tmp/repo");
        let memory_dir = PathBuf::from("/tmp/repo/.alan/memory");
        let mut spec = launch_spec(workspace_root.join(".alan/agents/grader"));

        assert_eq!(
            resolve_child_workspace_alan_dir(
                &spec,
                Some(workspace_root.as_path()),
                Some(memory_dir.as_path()),
            ),
            None
        );

        spec.handles.push(SpawnHandle::ApprovalScope);
        assert_eq!(
            resolve_child_workspace_alan_dir(
                &spec,
                Some(workspace_root.as_path()),
                Some(memory_dir.as_path()),
            ),
            Some(workspace_root.join(".alan"))
        );

        spec.handles.clear();
        spec.runtime_overrides.policy_path = Some(".alan/agent/policy.yaml".to_string());
        assert_eq!(
            resolve_child_workspace_alan_dir(
                &spec,
                Some(workspace_root.as_path()),
                Some(memory_dir.as_path()),
            ),
            Some(workspace_root.join(".alan"))
        );
    }

    #[test]
    fn child_agent_config_requires_memory_handle_for_memory_dir() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let parent = make_parent_state(&temp, requests, response);
        let root_dir = temp.path().join("repo/.alan/agents/grader");

        let mut approval_spec = launch_spec(root_dir.clone());
        approval_spec.handles = vec![SpawnHandle::ApprovalScope];
        let approval_config = build_child_agent_config(&parent, &approval_spec);
        assert_eq!(approval_config.core_config.memory.workspace_dir, None);

        let mut override_spec = launch_spec(root_dir);
        override_spec.runtime_overrides.policy_path = Some(".alan/agent/policy.yaml".to_string());
        let override_config = build_child_agent_config(&parent, &override_spec);
        assert_eq!(override_config.core_config.memory.workspace_dir, None);
    }

    #[tokio::test]
    async fn spawn_child_runtime_does_not_bind_memory_dir_for_policy_context_only_launches() {
        let temp = TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(workspace_root.join(".alan/agent")).unwrap();
        std::fs::write(
            workspace_root.join(".alan/agent/policy.yaml"),
            "version: 1\nrules: []\n",
        )
        .unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let mut parent = make_parent_state(&temp, requests.clone(), response.clone());
        parent.runtime_config.governance.policy_path = Some(".alan/agent/policy.yaml".to_string());
        let root_dir = workspace_root.join(".alan/agents/grader");
        std::fs::write(
            root_dir.join("agent.toml"),
            format!(
                "[memory]\nworkspace_dir = \"{}\"\n",
                workspace_root.join(".alan/overlay-memory").display()
            ),
        )
        .unwrap();
        let seen_configs = Arc::new(Mutex::new(Vec::<crate::Config>::new()));
        let seen_configs_for_factory = seen_configs.clone();

        let mut approval_spec = launch_spec(root_dir.clone());
        approval_spec.handles = vec![SpawnHandle::ApprovalScope];
        let child = spawn_child_runtime_with_client_factory(&parent, approval_spec, |config| {
            seen_configs_for_factory
                .lock()
                .unwrap()
                .push(config.clone());
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();
        assert_eq!(result.status, ChildRuntimeStatus::Completed);

        let mut override_spec = launch_spec(root_dir);
        override_spec.runtime_overrides.policy_path = Some(".alan/agent/policy.yaml".to_string());
        let child = spawn_child_runtime_with_client_factory(&parent, override_spec, |config| {
            seen_configs_for_factory
                .lock()
                .unwrap()
                .push(config.clone());
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();
        assert_eq!(result.status, ChildRuntimeStatus::Completed);

        let seen_configs = seen_configs.lock().unwrap();
        assert_eq!(seen_configs.len(), 2);
        assert_eq!(seen_configs[0].memory.workspace_dir, None);
        assert_eq!(seen_configs[1].memory.workspace_dir, None);
    }

    #[test]
    fn child_workspace_root_uses_parent_workspace_instead_of_nested_tool_cwd() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let mut parent = make_parent_state(&temp, requests, response);
        let workspace_root = temp.path().join("repo");
        let nested_cwd = workspace_root.join("nested/src");
        std::fs::create_dir_all(&nested_cwd).unwrap();
        parent.tools.set_default_cwd(nested_cwd);

        let mut spec = launch_spec(workspace_root.join(".alan/agents/grader"));
        spec.handles = vec![SpawnHandle::Workspace];

        assert_eq!(
            resolve_child_workspace_root(&parent, &spec),
            Some(workspace_root)
        );
    }

    #[test]
    fn child_launch_contract_rejects_cwd_outside_workspace_root() {
        let workspace_root = PathBuf::from("/tmp/repo");
        let mut spec = launch_spec(workspace_root.join(".alan/agents/grader"));
        spec.launch.workspace_root = Some(workspace_root);
        spec.launch.cwd = Some(PathBuf::from("/tmp/other-workspace/docs"));

        let err = validate_child_launch_contract(&spec).unwrap_err();
        assert!(
            err.to_string().contains("cwd"),
            "expected cwd validation error, got {err:#}"
        );
    }

    #[test]
    fn child_launch_contract_rejects_relative_launch_paths() {
        let mut spec = launch_spec(PathBuf::from("/tmp/repo/.alan/agents/grader"));
        spec.launch.workspace_root = Some(PathBuf::from("repo"));

        let err = validate_child_launch_contract(&spec).unwrap_err();
        assert!(
            err.to_string().contains("absolute"),
            "expected absolute-path validation error, got {err:#}"
        );

        spec.launch.workspace_root = Some(PathBuf::from("/tmp/repo"));
        spec.launch.cwd = Some(PathBuf::from("docs"));

        let err = validate_child_launch_contract(&spec).unwrap_err();
        assert!(
            err.to_string().contains("absolute"),
            "expected absolute-path validation error, got {err:#}"
        );
    }

    #[tokio::test]
    async fn child_runtime_join_captures_non_empty_final_text_delta() {
        let (tx, rx) = tokio::sync::broadcast::channel(8);
        let submission_id = "sub-123".to_string();
        let _ = tx.send(RuntimeEventEnvelope {
            submission_id: Some(submission_id.clone()),
            event: alan_protocol::Event::TextDelta {
                chunk: "final child output".to_string(),
                is_final: true,
            },
        });
        let _ = tx.send(RuntimeEventEnvelope {
            submission_id: Some(submission_id.clone()),
            event: alan_protocol::Event::TurnCompleted { summary: None },
        });

        let controller = ChildRuntimeController {
            runtime: None,
            startup_metadata: RuntimeStartupMetadata {
                session_id: "child-session".to_string(),
                rollout_path: None,
                durability: super::super::engine::SessionDurabilityState {
                    durable: false,
                    required: false,
                },
                execution_backend: crate::tools::Sandbox::backend_name_static().to_string(),
                warnings: Vec::new(),
            },
            event_rx: rx,
            submission_id,
            timeout: None,
        };

        let result = controller.join().await.unwrap();
        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        assert_eq!(result.output_text, "final child output");
    }

    #[tokio::test]
    async fn child_runtime_join_fails_when_event_stream_lags() {
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        let submission_id = "sub-456".to_string();
        let _ = tx.send(RuntimeEventEnvelope {
            submission_id: Some(submission_id.clone()),
            event: alan_protocol::Event::TextDelta {
                chunk: "partial child output".to_string(),
                is_final: false,
            },
        });
        let _ = tx.send(RuntimeEventEnvelope {
            submission_id: Some(submission_id.clone()),
            event: alan_protocol::Event::TurnCompleted {
                summary: Some("done".to_string()),
            },
        });

        let controller = ChildRuntimeController {
            runtime: None,
            startup_metadata: RuntimeStartupMetadata {
                session_id: "child-session".to_string(),
                rollout_path: None,
                durability: super::super::engine::SessionDurabilityState {
                    durable: false,
                    required: false,
                },
                execution_backend: crate::tools::Sandbox::backend_name_static().to_string(),
                warnings: Vec::new(),
            },
            event_rx: rx,
            submission_id,
            timeout: None,
        };

        let result = controller.join().await.unwrap();
        assert_eq!(result.status, ChildRuntimeStatus::Failed);
        assert_eq!(
            result.error_message.as_deref(),
            Some(
                "Child-agent runtime event stream lagged by 1 event(s) before a terminal event could be observed"
            )
        );
        assert_eq!(
            result.warnings,
            vec![
                "Child-agent runtime event stream lagged by 1 event(s) before a terminal event could be observed"
                    .to_string()
            ]
        );
    }

    #[tokio::test]
    async fn child_runtime_join_until_cancelled_handles_none_timeout_without_panicking() {
        let (tx, rx) = tokio::sync::broadcast::channel(8);
        let submission_id = "sub-789".to_string();
        let submission_id_for_task = submission_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let _ = tx.send(RuntimeEventEnvelope {
                submission_id: Some(submission_id_for_task),
                event: alan_protocol::Event::TurnCompleted {
                    summary: Some("done".to_string()),
                },
            });
        });

        let controller = ChildRuntimeController {
            runtime: None,
            startup_metadata: RuntimeStartupMetadata {
                session_id: "child-session".to_string(),
                rollout_path: None,
                durability: super::super::engine::SessionDurabilityState {
                    durable: false,
                    required: false,
                },
                execution_backend: crate::tools::Sandbox::backend_name_static().to_string(),
                warnings: Vec::new(),
            },
            event_rx: rx,
            submission_id,
            timeout: None,
        };
        let cancel = CancellationToken::new();

        let result = controller.join_until_cancelled(&cancel).await.unwrap();
        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        assert_eq!(result.turn_summary.as_deref(), Some("done"));
    }

    #[tokio::test]
    async fn cancel_child_runtime_returns_cancelled_status() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("This should not finish before cancellation.");
        let parent = make_parent_state_with_capability_view(
            &temp,
            requests.clone(),
            response.clone(),
            crate::skills::ResolvedCapabilityView::default(),
        );
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let spec = launch_spec(root_dir);

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(
                RecordingProvider::new(requests.clone(), response.clone())
                    .with_delay(Duration::from_secs(5)),
            ))
        })
        .await
        .unwrap();
        let result = child.cancel().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Cancelled);
    }

    #[tokio::test]
    async fn child_runtime_join_until_cancelled_returns_cancelled_status() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("This should not finish before cancellation.");
        let parent = make_parent_state_with_capability_view(
            &temp,
            requests.clone(),
            response.clone(),
            crate::skills::ResolvedCapabilityView::default(),
        );
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let spec = launch_spec(root_dir);

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(
                RecordingProvider::new(requests.clone(), response.clone())
                    .with_delay(Duration::from_secs(5)),
            ))
        })
        .await
        .unwrap();

        let cancel = CancellationToken::new();
        let cancel_for_task = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancel_for_task.cancel();
        });

        let result = child.join_until_cancelled(&cancel).await.unwrap();
        assert_eq!(result.status, ChildRuntimeStatus::Cancelled);
    }

    #[tokio::test]
    async fn spawn_child_runtime_cancellable_aborts_pre_cancelled_launch() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("This should never run.");
        let parent = make_parent_state(&temp, requests, response);
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let spec = launch_spec(root_dir);
        let cancel = CancellationToken::new();
        cancel.cancel();

        let err = match spawn_child_runtime_cancellable(&parent, spec, &cancel).await {
            Ok(_) => {
                panic!("pre-cancelled launch should abort before returning a child controller")
            }
            Err(err) => err,
        };

        assert!(err.to_string().contains("Child-agent launch cancelled"));
    }

    #[tokio::test]
    async fn child_runtime_join_returns_promptly_after_timeout() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("This should not finish before timeout.");
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
        let root_dir = temp.path().join("repo/.alan/agents/grader");
        let mut spec = launch_spec(root_dir);
        spec.launch.timeout_secs = Some(1);

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(
                RecordingProvider::new(requests.clone(), response.clone())
                    .with_delay(Duration::from_secs(30)),
            ))
        })
        .await
        .unwrap();

        let started_at = std::time::Instant::now();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::TimedOut);
        assert!(
            started_at.elapsed() < Duration::from_secs(8),
            "timed-out child join should abort promptly instead of waiting for graceful shutdown"
        );
    }

    #[tokio::test]
    async fn spawn_child_runtime_resolves_package_child_agent_target() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Package child target resolved.");
        let capability_view = capability_view_with_package_child_agent(&temp);
        let parent = make_parent_state_with_capability_view(
            &temp,
            requests.clone(),
            response.clone(),
            capability_view,
        );
        let spec = SpawnSpec {
            target: SpawnTarget::PackageChildAgent {
                package_id: "skill:repo-review".to_string(),
                export_name: "reviewer".to_string(),
            },
            launch: alan_protocol::SpawnLaunchInputs {
                task: "Review the repository changes".to_string(),
                workspace_root: Some(temp.path().join("repo")),
                timeout_secs: Some(30),
                ..alan_protocol::SpawnLaunchInputs::default()
            },
            handles: vec![SpawnHandle::Workspace],
            runtime_overrides: alan_protocol::SpawnRuntimeOverrides::default(),
        };

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        assert_eq!(result.output_text, "Package child target resolved.");
    }

    #[tokio::test]
    async fn spawn_child_runtime_resolves_package_child_agent_target_from_refreshed_view() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Package child target resolved after refresh.");
        let workspace_root = temp.path().join("repo");
        let package_root = workspace_root.join(".alan/agent/skills/repo-review");
        std::fs::create_dir_all(&package_root).unwrap();
        std::fs::write(
            package_root.join("SKILL.md"),
            r#"---
name: Repo Review
description: Review repository changes
---

Body
"#,
        )
        .unwrap();

        let capability_view = crate::skills::ResolvedCapabilityView::from_package_dirs(vec![
            crate::skills::ScopedPackageDir {
                path: workspace_root.join(".alan/agent/skills"),
                scope: crate::skills::SkillScope::Repo,
            },
        ]);
        let parent = make_parent_state_with_capability_view(
            &temp,
            requests.clone(),
            response.clone(),
            capability_view,
        );

        std::fs::create_dir_all(package_root.join("agents/reviewer")).unwrap();
        std::fs::write(
            package_root.join("agents/reviewer/agent.toml"),
            "tool_repeat_limit = 4\n",
        )
        .unwrap();

        let spec = SpawnSpec {
            target: SpawnTarget::PackageChildAgent {
                package_id: "skill:repo-review".to_string(),
                export_name: "reviewer".to_string(),
            },
            launch: alan_protocol::SpawnLaunchInputs {
                task: "Review the repository changes".to_string(),
                workspace_root: Some(workspace_root),
                timeout_secs: Some(30),
                ..alan_protocol::SpawnLaunchInputs::default()
            },
            handles: vec![SpawnHandle::Workspace],
            runtime_overrides: alan_protocol::SpawnRuntimeOverrides::default(),
        };

        let child = spawn_child_runtime_with_client_factory(&parent, spec, |_| {
            Ok(LlmClient::new(RecordingProvider::new(
                requests.clone(),
                response.clone(),
            )))
        })
        .await
        .unwrap();
        let result = child.join().await.unwrap();

        assert_eq!(result.status, ChildRuntimeStatus::Completed);
        assert_eq!(
            result.output_text,
            "Package child target resolved after refresh."
        );
    }
}
