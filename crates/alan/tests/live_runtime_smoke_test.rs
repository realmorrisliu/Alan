use alan_protocol::{ContentPart, Event, Op, Submission};
use alan_runtime::runtime::spawn_with_tool_registry;
use alan_runtime::{
    AlanHomePaths, Config, RuntimeEventEnvelope, ToolRegistry, WorkspaceRuntimeConfig,
};
use anyhow::{Context, Result, ensure};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

const LIVE_ENABLE_ENV: &str = "ALAN_LIVE_PROVIDER_TESTS";
const CHATGPT_AUTH_STORAGE_PATH_ENV: &str = "ALAN_LIVE_CHATGPT_AUTH_STORAGE_PATH";
const CHATGPT_BASE_URL_ENV: &str = "ALAN_LIVE_CHATGPT_BASE_URL";
const CHATGPT_MODEL_ENV: &str = "ALAN_LIVE_CHATGPT_MODEL";
const CHATGPT_ACCOUNT_ID_ENV: &str = "ALAN_LIVE_CHATGPT_ACCOUNT_ID";
const TURN_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
struct CollectedTurn {
    events: Vec<Event>,
    text: String,
    warnings: Vec<String>,
    errors: Vec<String>,
    saw_turn_completed: bool,
}

fn live_enabled() -> bool {
    env::var(LIVE_ENABLE_ENV)
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn collect_events_until_terminal(
    mut rx: tokio::sync::broadcast::Receiver<RuntimeEventEnvelope>,
    timeout: Duration,
) -> CollectedTurn {
    let mut events = Vec::new();
    let mut text = String::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut saw_turn_completed = false;
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(envelope) => {
                        let event = envelope.event.clone();
                        match &event {
                            Event::TextDelta { chunk, .. } => text.push_str(chunk),
                            Event::Warning { message } => warnings.push(message.clone()),
                            Event::Error { message, recoverable } => {
                                errors.push(format!("{message} (recoverable={recoverable})"));
                            }
                            Event::TurnCompleted { .. } => {
                                saw_turn_completed = true;
                            }
                            _ => {}
                        }
                        events.push(event);

                        if saw_turn_completed || !errors.is_empty() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("[live-runtime-smoke] WARNING: lagged {n} events");
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                eprintln!(
                    "[live-runtime-smoke] TIMEOUT waiting for terminal event after {:?}",
                    timeout
                );
                break;
            }
        }
    }

    CollectedTurn {
        events,
        text,
        warnings,
        errors,
        saw_turn_completed,
    }
}

fn print_event_summary(test_name: &str, turn: &CollectedTurn) {
    eprintln!(
        "\n=== {test_name}: collected {} events, warnings={}, errors={} ===",
        turn.events.len(),
        turn.warnings.len(),
        turn.errors.len()
    );
    for (idx, event) in turn.events.iter().enumerate() {
        match event {
            Event::TurnStarted { .. } => eprintln!("  [{idx}] TurnStarted"),
            Event::TurnCompleted { .. } => eprintln!("  [{idx}] TurnCompleted"),
            Event::TextDelta { chunk, is_final } => {
                eprintln!("  [{idx}] TextDelta(is_final={is_final}): {chunk:?}");
            }
            Event::ThinkingDelta { chunk, is_final } => {
                eprintln!(
                    "  [{idx}] ThinkingDelta(is_final={is_final}): {:?}",
                    &chunk[..chunk.len().min(80)]
                );
            }
            Event::Warning { message } => eprintln!("  [{idx}] Warning: {message}"),
            Event::Error {
                message,
                recoverable,
            } => {
                eprintln!("  [{idx}] Error(recoverable={recoverable}): {message}");
            }
            other => eprintln!("  [{idx}] {:?}", std::mem::discriminant(other)),
        }
    }
    eprintln!("=== end {test_name} ===\n");
}

#[tokio::test]
#[ignore = "live runtime smoke; requires ALAN_LIVE_PROVIDER_TESTS=1 and managed ChatGPT auth"]
async fn live_chatgpt_runtime_smoke() -> Result<()> {
    if !live_enabled() {
        eprintln!("[live-runtime-smoke] skipping chatgpt: set {LIVE_ENABLE_ENV}=1 to enable");
        return Ok(());
    }

    let Some(auth_storage_path) = non_empty_env(CHATGPT_AUTH_STORAGE_PATH_ENV) else {
        eprintln!(
            "[live-runtime-smoke] skipping chatgpt: {CHATGPT_AUTH_STORAGE_PATH_ENV} is unset"
        );
        return Ok(());
    };

    let temp_home = TempDir::new().context("create temp home")?;
    let temp_workspace = TempDir::new().context("create temp workspace root")?;
    let workspace_root = temp_workspace.path().join("workspace");
    let workspace_alan_dir = workspace_root.join(".alan");
    std::fs::create_dir_all(&workspace_alan_dir).context("create workspace .alan dir")?;

    let base_url = non_empty_env(CHATGPT_BASE_URL_ENV);
    let model = non_empty_env(CHATGPT_MODEL_ENV).unwrap_or_else(|| "gpt-5.3-codex".to_string());

    let mut core_config = Config::for_chatgpt(base_url.as_deref(), Some(&model));
    if let Some(account_id) = non_empty_env(CHATGPT_ACCOUNT_ID_ENV) {
        core_config.chatgpt_account_id = Some(account_id);
    }

    let mut runtime_config = WorkspaceRuntimeConfig::from(core_config);
    runtime_config.workspace_root_dir = Some(workspace_root.clone());
    runtime_config.workspace_alan_dir = Some(workspace_alan_dir);
    runtime_config.default_cwd_override = Some(workspace_root);
    runtime_config.agent_home_paths = Some(AlanHomePaths::from_home_dir(temp_home.path()));
    runtime_config.chatgpt_auth_storage_path = Some(PathBuf::from(auth_storage_path));

    let mut tools = ToolRegistry::new();
    if let Some(cwd) = runtime_config.default_cwd_override.clone() {
        tools.set_default_cwd(cwd);
    }

    let mut controller = spawn_with_tool_registry(runtime_config, tools)
        .context("spawn runtime with managed ChatGPT provider")?;
    controller
        .wait_until_ready()
        .await
        .context("runtime should become ready")?;

    let rx = controller.handle.event_sender.subscribe();
    let token = "ALAN_RUNTIME_LIVE_CHATGPT_OK";
    controller
        .handle
        .submission_tx
        .send(Submission::new(Op::Turn {
            parts: vec![ContentPart::text(format!(
                "Reply with exactly: {token}. Do not use tools, markdown, or punctuation."
            ))],
            context: None,
        }))
        .await
        .context("submit live runtime turn")?;

    let turn = collect_events_until_terminal(rx, TURN_TIMEOUT).await;
    print_event_summary("live_chatgpt_runtime_smoke", &turn);

    controller.shutdown().await.context("shutdown runtime")?;

    ensure!(
        turn.errors.is_empty(),
        "live runtime emitted unexpected errors: {:?}",
        turn.errors
    );
    ensure!(
        turn.saw_turn_completed,
        "live runtime did not reach TurnCompleted; warnings={:?}, text={:?}",
        turn.warnings,
        turn.text
    );
    ensure!(
        turn.text.contains(token),
        "live runtime text did not contain expected token `{token}`: {:?}",
        turn.text
    );

    Ok(())
}
