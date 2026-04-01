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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

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
    spawn_child_runtime_with_client_factory(parent, spec, |core_config| {
        LlmClient::from_core_config(core_config)
    })
    .await
}

async fn spawn_child_runtime_with_client_factory<F>(
    parent: &RuntimeLoopState,
    spec: SpawnSpec,
    llm_client_factory: F,
) -> Result<ChildRuntimeController>
where
    F: FnOnce(&crate::Config) -> Result<LlmClient>,
{
    validate_child_launch_contract(&spec)?;
    let child_agent_config = build_child_agent_config(parent, &spec);
    let workspace_root_dir = resolve_child_workspace_root(parent, &spec);
    let workspace_alan_dir = resolve_child_workspace_alan_dir(
        &spec,
        workspace_root_dir.as_deref(),
        parent.core_config.memory.workspace_dir.as_deref(),
    );
    let launch_root_dir = match &spec.target {
        SpawnTarget::ResolvedAgentRoot { root_dir } => Some(root_dir.clone()),
    };
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
    if let Some(alan_dir) = resolved_child_definition.workspace_alan_dir.as_ref() {
        resolved_child_agent_config.core_config.memory.workspace_dir =
            Some(crate::workspace_memory_dir_from_alan_dir(alan_dir));
    }
    let effective_child_core_config = resolved_child_agent_config.core_config.clone();
    child_config.agent_config = resolved_child_agent_config;
    child_config.core_config_source = crate::ConfigSourceKind::EnvOverride;
    let child_tools = build_child_tool_registry(parent, &spec, &effective_child_core_config);

    let llm_client = llm_client_factory(&effective_child_core_config)
        .context("Failed to create child-agent LLM client")?;
    let mut runtime = spawn_with_llm_client_and_tools(child_config, llm_client, child_tools)
        .context("Failed to spawn child-agent runtime")?;
    let startup_metadata = runtime
        .wait_until_ready()
        .await
        .context("Child-agent runtime failed to start")?;
    let event_rx = runtime.handle.event_sender.subscribe();
    let submission = Submission::new(Op::Turn {
        parts: vec![ContentPart::text(build_child_task_text(parent, &spec))],
        context: None,
    });
    runtime
        .handle
        .submission_tx
        .send(submission.clone())
        .await
        .context("Failed to submit initial child-agent turn")?;

    Ok(ChildRuntimeController {
        runtime: Some(runtime),
        startup_metadata,
        event_rx,
        submission_id: submission.id,
        timeout: spec.launch.timeout_secs.map(Duration::from_secs),
    })
}

fn validate_child_launch_contract(spec: &SpawnSpec) -> Result<()> {
    if spec.has_handle(SpawnHandle::Artifacts) || spec.launch.output_dir.is_some() {
        bail!(
            "Child-agent launches do not support artifact routing yet; omit SpawnHandle::Artifacts and launch.output_dir."
        );
    }

    Ok(())
}

#[allow(dead_code)]
impl ChildRuntimeController {
    pub(crate) fn startup_metadata(&self) -> &RuntimeStartupMetadata {
        &self.startup_metadata
    }

    pub(crate) async fn join(mut self) -> Result<ChildRuntimeResult> {
        let observed = if let Some(timeout) = self.timeout {
            match tokio::time::timeout(timeout, self.wait_for_terminal_event()).await {
                Ok(result) => result?,
                Err(_) => {
                    self.abort_runtime().await;
                    return Ok(ChildRuntimeResult {
                        status: ChildRuntimeStatus::TimedOut,
                        session_id: self.startup_metadata.session_id,
                        rollout_path: self.startup_metadata.rollout_path,
                        output_text: String::new(),
                        turn_summary: None,
                        warnings: self.startup_metadata.warnings,
                        error_message: Some("Child-agent turn timed out".to_string()),
                        pause: None,
                    });
                }
            }
        } else {
            self.wait_for_terminal_event().await?
        };

        let mut warnings = self.startup_metadata.warnings.clone();
        warnings.extend(observed.warnings);
        self.terminate_runtime().await;

        Ok(ChildRuntimeResult {
            status: observed.status,
            session_id: self.startup_metadata.session_id,
            rollout_path: self.startup_metadata.rollout_path,
            output_text: observed.output_text,
            turn_summary: observed.turn_summary,
            warnings,
            error_message: observed.error_message,
            pause: observed.pause,
        })
    }

    pub(crate) async fn cancel(mut self) -> Result<ChildRuntimeResult> {
        self.terminate_runtime().await;
        Ok(ChildRuntimeResult {
            status: ChildRuntimeStatus::Cancelled,
            session_id: self.startup_metadata.session_id,
            rollout_path: self.startup_metadata.rollout_path,
            output_text: String::new(),
            turn_summary: None,
            warnings: self.startup_metadata.warnings,
            error_message: None,
            pause: None,
        })
    }

    async fn wait_for_terminal_event(&mut self) -> Result<ObservedChildTerminalEvent> {
        let mut output_text = String::new();
        let mut warnings = Vec::new();

        loop {
            match self.event_rx.recv().await {
                Ok(envelope) => {
                    if envelope.submission_id.as_deref() != Some(self.submission_id.as_str()) {
                        continue;
                    }

                    match envelope.event {
                        alan_protocol::Event::TextDelta { chunk, .. } => {
                            if !chunk.is_empty() {
                                output_text.push_str(&chunk);
                            }
                        }
                        alan_protocol::Event::Warning { message } => warnings.push(message),
                        alan_protocol::Event::TurnCompleted { summary } => {
                            return Ok(ObservedChildTerminalEvent {
                                output_text,
                                turn_summary: summary,
                                warnings,
                                error_message: None,
                                pause: None,
                                status: ChildRuntimeStatus::Completed,
                            });
                        }
                        alan_protocol::Event::Yield {
                            request_id, kind, ..
                        } => {
                            return Ok(ObservedChildTerminalEvent {
                                output_text,
                                turn_summary: None,
                                warnings,
                                error_message: None,
                                pause: Some(ChildRuntimePause { request_id, kind }),
                                status: ChildRuntimeStatus::Paused,
                            });
                        }
                        alan_protocol::Event::Error {
                            message,
                            recoverable,
                        } if !recoverable => {
                            return Ok(ObservedChildTerminalEvent {
                                output_text,
                                turn_summary: None,
                                warnings,
                                error_message: Some(message),
                                pause: None,
                                status: ChildRuntimeStatus::Failed,
                            });
                        }
                        alan_protocol::Event::Error { message, .. } => warnings.push(message),
                        _ => {}
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    let message = format!(
                        "Child-agent runtime event stream lagged by {skipped} event(s) before a terminal event could be observed"
                    );
                    warnings.push(message.clone());
                    return Ok(ObservedChildTerminalEvent {
                        output_text,
                        turn_summary: None,
                        warnings,
                        error_message: Some(message),
                        pause: None,
                        status: ChildRuntimeStatus::Failed,
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return Ok(ObservedChildTerminalEvent {
                        output_text,
                        turn_summary: None,
                        warnings,
                        error_message: Some(
                            "Child-agent runtime stopped before producing a terminal event"
                                .to_string(),
                        ),
                        pause: None,
                        status: ChildRuntimeStatus::Failed,
                    });
                }
            }
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

    let mut tools = if let Some(tool_profile) = spec.runtime_overrides.tool_profile.as_ref() {
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
    if !spec.has_handle(SpawnHandle::Memory) {
        return None;
    }

    workspace_root_dir
        .map(|root| root.join(".alan"))
        .or_else(|| infer_workspace_alan_dir_from_memory_dir(memory_dir))
}

fn infer_workspace_alan_dir_from_memory_dir(memory_dir: Option<&Path>) -> Option<PathBuf> {
    let memory_dir = memory_dir?;
    if memory_dir.file_name()? != "memory" {
        return None;
    }
    let alan_dir = memory_dir.parent()?;
    (alan_dir.file_name()? == ".alan").then(|| alan_dir.to_path_buf())
}

fn infer_workspace_root_from_memory_dir(memory_dir: Option<&Path>) -> Option<PathBuf> {
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
                        completion_tokens: 1,
                        total_tokens: 2,
                        reasoning_tokens: None,
                    }),
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

    fn make_parent_state(
        temp: &TempDir,
        requests: RecordedRequests,
        response: GenerationResponse,
    ) -> RuntimeLoopState {
        let workspace_root = temp.path().join("repo");
        let workspace_alan_dir = workspace_root.join(".alan");
        let launch_root = workspace_root.join(".alan/agents/grader");
        std::fs::create_dir_all(launch_root.join("persona")).unwrap();
        std::fs::create_dir_all(workspace_alan_dir.join("sessions")).unwrap();
        std::fs::create_dir_all(launch_root.join("skills")).unwrap();
        std::fs::write(
            launch_root.join("agent.toml"),
            "openai_responses_model = \"gpt-5.4\"\n",
        )
        .unwrap();

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
                    crate::skills::ResolvedCapabilityView::default(),
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
                completion_tokens: 4,
                total_tokens: 12,
                reasoning_tokens: None,
            }),
            warnings: Vec::new(),
        }
    }

    #[tokio::test]
    async fn spawn_child_runtime_defaults_to_exec_like_non_inheritance() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("Child finished cleanly.");
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
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
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
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
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
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
openai_responses_model = "launch-root-model"
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
        assert_eq!(seen_config.openai_responses_model, "launch-root-model");
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
openai_responses_model = "launch-root-model"
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
    fn child_workspace_alan_dir_requires_memory_handle() {
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

        spec.handles.push(SpawnHandle::Memory);
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
    async fn cancel_child_runtime_returns_cancelled_status() {
        let temp = TempDir::new().unwrap();
        let requests = RecordedRequests::default();
        let response = completed_response("This should not finish before cancellation.");
        let parent = make_parent_state(&temp, requests.clone(), response.clone());
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
}
