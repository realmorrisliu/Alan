//! Alan — AI Turing Machine CLI & daemon.
//!
//! This is the unified entry point for all Alan operations:
//! - `alan daemon start` — run the workspace daemon
//! - `alan init` — initialize a workspace
//! - `alan workspace` — manage workspaces
//! - `alan chat` — interactive TUI (spawns Bun TUI)
//! - `alan ask` — one-shot question

mod cli;
mod daemon;
mod host_config;
pub mod registry;
mod skill_catalog;

use alan::OutputMode;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::Path;
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "alan", about = "Alan — AI Turing Machine", version)]
struct Cli {
    /// Select a named agent root on top of the base workspace/global agent
    #[arg(long, global = true)]
    agent: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage local provider login state
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Start or manage the daemon server
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Initialize a directory as a workspace
    Init {
        /// Path to initialize (defaults to current directory)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Workspace alias (defaults to directory name)
        #[arg(long)]
        name: Option<String>,
        /// Suppress output (used by install script)
        #[arg(long, hide = true)]
        silent: bool,
    },
    /// Manage workspaces
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    /// Inspect resolved skills, packages, and exposure state
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Interactive chat (launches TUI)
    Chat,
    /// Ask a one-shot question
    Ask {
        /// The question to ask
        question: String,
        /// Workspace directory (defaults to current directory)
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Output mode: text (human), json (NDJSON for agents), quiet (script)
        #[arg(long, value_enum, default_value_t = OutputMode::Text)]
        output: OutputMode,
        /// Show thinking/reasoning in text mode
        #[arg(long)]
        thinking: bool,
        /// Timeout in seconds
        #[arg(long, default_value_t = 30)]
        timeout: u64,
        /// Force streaming generation path for this session
        #[arg(long, conflicts_with = "no_stream")]
        stream: bool,
        /// Force non-streaming generation path for this session
        #[arg(long = "no-stream", conflicts_with = "stream")]
        no_stream: bool,
        /// Partial stream recovery behavior for interrupted visible output
        #[arg(long = "partial-stream-recovery", value_parser = ["continue_once", "off"])]
        partial_stream_recovery: Option<String>,
    },
    /// Control a local Alan Shell host via IPC
    Shell {
        #[command(subcommand)]
        action: ShellAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon server (default: detach to background)
    Start {
        /// Run in foreground instead of detaching
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Log in to a managed provider account
    Login {
        #[command(subcommand)]
        provider: AuthLoginProvider,
    },
    /// Inspect local managed login state
    Status,
    /// Clear local managed login state
    Logout,
}

#[derive(Subcommand)]
enum AuthLoginProvider {
    /// Sign in with ChatGPT/Codex subscription auth
    Chatgpt {
        /// Use device-code flow instead of the local browser callback flow
        #[arg(long)]
        device_code: bool,
        /// Print the URL instead of opening a browser automatically
        #[arg(long = "no-browser")]
        no_browser: bool,
        /// Restrict login to a specific ChatGPT workspace/account id
        #[arg(long = "workspace")]
        workspace_id: Option<String>,
    },
}

#[derive(Subcommand)]
enum WorkspaceAction {
    /// List all registered workspaces
    List,
    /// Register an existing workspace directory
    Add {
        /// Path to the workspace directory (must contain .alan/)
        path: PathBuf,
        /// Workspace alias
        #[arg(long)]
        name: Option<String>,
    },
    /// Unregister a workspace (does not delete files)
    Remove {
        /// Workspace alias, short ID, or path
        workspace: String,
    },
    /// Show workspace details
    Info {
        /// Workspace alias, short ID, or path
        workspace: String,
    },
}

#[derive(Subcommand)]
enum SkillsAction {
    /// List exposed skills for the resolved workspace/agent
    List {
        /// Workspace directory to inspect (defaults to current directory)
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
    /// List resolved capability packages and their exported skills
    Packages {
        /// Workspace directory to inspect (defaults to current directory)
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
    /// Scaffold a new skill package from a first-party template
    Init {
        /// Directory to create for the new skill package
        path: PathBuf,
        /// Template shape to generate
        #[arg(long, value_enum, default_value_t = cli::skill_authoring::SkillTemplateKind::Inline)]
        template: cli::skill_authoring::SkillTemplateKind,
        /// Human-facing skill name written into SKILL.md
        #[arg(long)]
        name: Option<String>,
        /// Skill description written into SKILL.md
        #[arg(long)]
        description: Option<String>,
        /// Short UI-facing description
        #[arg(long = "short-description")]
        short_description: Option<String>,
        /// Overwrite an existing non-empty directory
        #[arg(long)]
        force: bool,
    },
    /// Validate a skill package against Alan's current package contract
    Validate {
        /// Skill package directory (defaults to current directory)
        path: Option<PathBuf>,
        /// Emit structured JSON instead of human-readable text
        #[arg(long)]
        json: bool,
        /// Treat warnings as failures
        #[arg(long)]
        strict: bool,
    },
    /// Run explicit package-local evaluation hooks for a skill package
    Eval {
        /// Skill package directory (defaults to current directory)
        path: Option<PathBuf>,
        /// Structured eval manifest path (defaults to evals/evals.json when present)
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Output directory for structured eval artifacts
        #[arg(long = "output-dir")]
        output_dir: Option<PathBuf>,
        /// Fail if the package does not define an eval hook
        #[arg(long)]
        require_hook: bool,
    },
}

#[derive(Args, Clone)]
struct ShellTargetArgs {
    /// Explicit Alan Shell socket path
    #[arg(long)]
    socket: Option<PathBuf>,
    /// Explicit Alan Shell control directory
    #[arg(long = "control-dir")]
    control_dir: Option<PathBuf>,
    /// Window id used to derive the local Alan Shell control directory
    #[arg(long)]
    window: Option<String>,
    /// Timeout for IPC requests in milliseconds
    #[arg(long, default_value_t = 3000)]
    timeout_ms: u64,
}

#[derive(Subcommand)]
enum ShellAction {
    /// Print the canonical shell state snapshot
    State {
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Operate on shell spaces
    Space {
        #[command(subcommand)]
        action: ShellSpaceAction,
    },
    /// Operate on shell surfaces
    Surface {
        #[command(subcommand)]
        action: ShellSurfaceAction,
    },
    /// Operate on shell panes
    Pane {
        #[command(subcommand)]
        action: ShellPaneAction,
    },
    /// Attention inbox and overrides
    Attention {
        #[command(subcommand)]
        action: ShellAttentionAction,
    },
    /// Rank candidate panes for shell routing
    Routing {
        #[command(subcommand)]
        action: ShellRoutingAction,
    },
    /// Read shell events or follow the event stream
    Events {
        /// Resume after this event id
        #[arg(long = "after-event-id")]
        after_event_id: Option<String>,
        /// Maximum number of events per read
        #[arg(long)]
        limit: Option<u64>,
        /// Keep polling and emit NDJSON as new events arrive
        #[arg(long)]
        follow: bool,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
}

#[derive(Subcommand)]
enum ShellSpaceAction {
    /// List spaces
    List {
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Create a new space
    Create {
        /// Optional title for the space
        #[arg(long)]
        title: Option<String>,
        /// Optional working directory for the initial pane
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Open a new Alan space directly
    OpenAlan {
        /// Optional title for the space
        #[arg(long)]
        title: Option<String>,
        /// Optional working directory for the initial pane
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
}

#[derive(Subcommand)]
enum ShellSurfaceAction {
    /// List surfaces
    List {
        /// Restrict to a specific space
        #[arg(long)]
        space: Option<String>,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Open a new surface
    Open {
        /// Target space id
        #[arg(long)]
        space: Option<String>,
        /// Optional surface title
        #[arg(long)]
        title: Option<String>,
        /// Optional working directory for the initial pane
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Close a surface
    Close {
        /// Surface id to close
        #[arg(long)]
        surface: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
}

#[derive(Subcommand)]
enum ShellPaneAction {
    /// List panes
    List {
        /// Restrict to a specific surface
        #[arg(long)]
        surface: Option<String>,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Print a single pane snapshot
    Snapshot {
        /// Pane id to inspect
        #[arg(long)]
        pane: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Split a pane
    Split {
        /// Pane id to split
        #[arg(long)]
        pane: String,
        /// Split direction
        #[arg(long, value_parser = ["horizontal", "vertical"])]
        direction: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Close a pane
    Close {
        /// Pane id to close
        #[arg(long)]
        pane: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Move a pane into its own surface
    Lift {
        /// Pane id to lift
        #[arg(long)]
        pane: String,
        /// Optional title for the new surface
        #[arg(long)]
        title: Option<String>,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Move a pane into another existing surface
    Move {
        /// Pane id to move
        #[arg(long)]
        pane: String,
        /// Target surface id
        #[arg(long)]
        surface: String,
        /// Split direction used when attaching onto the destination surface
        #[arg(long, default_value = "vertical", value_parser = ["horizontal", "vertical"])]
        direction: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Focus a pane
    Focus {
        /// Pane id to focus
        #[arg(long)]
        pane: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Send text to a pane
    SendText {
        /// Pane id to target
        #[arg(long)]
        pane: String,
        /// Text to send
        #[arg(long)]
        text: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
}

#[derive(Subcommand)]
enum ShellAttentionAction {
    /// List panes that currently require attention
    Inbox {
        #[command(flatten)]
        target: ShellTargetArgs,
    },
    /// Override a pane attention state
    Set {
        /// Pane id to target
        #[arg(long)]
        pane: String,
        /// Attention state
        #[arg(long, value_parser = ["idle", "active", "awaiting_user", "notable"])]
        state: String,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
}

#[derive(Subcommand)]
enum ShellRoutingAction {
    /// Rank candidate panes for intent routing
    Candidates {
        /// Optional preferred pane id
        #[arg(long)]
        pane: Option<String>,
        #[command(flatten)]
        target: ShellTargetArgs,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let agent_name = alan_runtime::normalize_agent_name(cli.agent.as_deref()).map(str::to_owned);

    match cli.command {
        Some(Commands::Auth { action }) => match action {
            AuthAction::Login { provider } => match provider {
                AuthLoginProvider::Chatgpt {
                    device_code,
                    no_browser,
                    workspace_id,
                } => {
                    cli::auth::run_auth_login_chatgpt(
                        device_code,
                        !no_browser,
                        workspace_id.as_deref(),
                    )
                    .await?;
                }
            },
            AuthAction::Status => {
                let found = cli::auth::run_auth_status().await?;
                if !found {
                    std::process::exit(1);
                }
            }
            AuthAction::Logout => {
                let _ = cli::auth::run_auth_logout().await?;
            }
        },
        Some(Commands::Daemon { action }) => match action {
            DaemonAction::Start { foreground } => {
                if foreground {
                    // Run in foreground (blocking)
                    init_tracing();
                    let loaded_config = cli::load_agent_config_metadata_with_notice()?;
                    daemon::server::run_server_with_loaded_config(loaded_config).await?;
                } else {
                    // Detach to background
                    cli::daemon::start_daemon_background().await?;
                }
            }
            DaemonAction::Stop => {
                cli::daemon::stop_daemon().await?;
            }
            DaemonAction::Status => {
                cli::daemon::daemon_status().await?;
            }
        },
        Some(Commands::Init { path, name, silent }) => {
            cli::init::run_init(path, name, silent)?;
        }
        Some(Commands::Workspace { action }) => match action {
            WorkspaceAction::List => {
                cli::workspace::list_workspaces()?;
            }
            WorkspaceAction::Add { path, name } => {
                cli::workspace::add_workspace(path, name)?;
            }
            WorkspaceAction::Remove { workspace } => {
                cli::workspace::remove_workspace(&workspace)?;
            }
            WorkspaceAction::Info { workspace } => {
                cli::workspace::workspace_info(&workspace)?;
            }
        },
        Some(Commands::Skills { action }) => match action {
            SkillsAction::List { workspace } => {
                cli::skills::run_list_skills(workspace, agent_name.as_deref())?;
            }
            SkillsAction::Packages { workspace } => {
                cli::skills::run_list_packages(workspace, agent_name.as_deref())?;
            }
            SkillsAction::Init {
                path,
                template,
                name,
                description,
                short_description,
                force,
            } => {
                cli::skills::run_init_skill_package(
                    path,
                    template,
                    name.as_deref(),
                    description.as_deref(),
                    short_description.as_deref(),
                    force,
                )?;
            }
            SkillsAction::Validate { path, json, strict } => {
                let passed = cli::skills::run_validate_skill_package(path, json, strict)?;
                if !passed {
                    std::process::exit(1);
                }
            }
            SkillsAction::Eval {
                path,
                manifest,
                output_dir,
                require_hook,
            } => {
                let passed =
                    cli::skills::run_eval_skill_package(path, manifest, output_dir, require_hook)?;
                if !passed {
                    std::process::exit(1);
                }
            }
        },
        Some(Commands::Chat) => {
            preflight_chat_agent_config()?;
            cli::chat::run_chat(agent_name.as_deref()).await?;
        }
        Some(Commands::Ask {
            question,
            workspace,
            output,
            thinking,
            timeout,
            stream,
            no_stream,
            partial_stream_recovery,
        }) => {
            let streaming_mode = if stream {
                Some(alan_runtime::StreamingMode::On)
            } else if no_stream {
                Some(alan_runtime::StreamingMode::Off)
            } else {
                None
            };
            let partial_stream_recovery_mode =
                partial_stream_recovery.as_deref().map(|mode| match mode {
                    "continue_once" => alan_runtime::PartialStreamRecoveryMode::ContinueOnce,
                    "off" => alan_runtime::PartialStreamRecoveryMode::Off,
                    _ => unreachable!("validated by clap value_parser"),
                });
            let code = cli::ask::run_ask(
                &question,
                cli::ask::AskOptions {
                    workspace,
                    mode: output,
                    show_thinking: thinking,
                    timeout_secs: timeout,
                    agent_name,
                    streaming_mode,
                    partial_stream_recovery_mode,
                },
            )
            .await;
            std::process::exit(code);
        }
        Some(Commands::Shell { action }) => match action {
            ShellAction::State { target } => {
                cli::shell::run_shell_state(shell_target_options(target))?;
            }
            ShellAction::Space { action } => match action {
                ShellSpaceAction::List { target } => {
                    cli::shell::run_shell_space_list(shell_target_options(target))?;
                }
                ShellSpaceAction::Create { title, cwd, target } => {
                    cli::shell::run_shell_space_create(
                        title.as_deref(),
                        cwd.as_ref().map(|path| path_to_string(path)).as_deref(),
                        shell_target_options(target),
                    )?;
                }
                ShellSpaceAction::OpenAlan { title, cwd, target } => {
                    cli::shell::run_shell_space_open_alan(
                        title.as_deref(),
                        cwd.as_ref().map(|path| path_to_string(path)).as_deref(),
                        shell_target_options(target),
                    )?;
                }
            },
            ShellAction::Surface { action } => match action {
                ShellSurfaceAction::List { space, target } => {
                    cli::shell::run_shell_surface_list(
                        space.as_deref(),
                        shell_target_options(target),
                    )?;
                }
                ShellSurfaceAction::Open {
                    space,
                    title,
                    cwd,
                    target,
                } => {
                    cli::shell::run_shell_surface_open(
                        space.as_deref(),
                        title.as_deref(),
                        cwd.as_ref().map(|path| path_to_string(path)).as_deref(),
                        shell_target_options(target),
                    )?;
                }
                ShellSurfaceAction::Close { surface, target } => {
                    cli::shell::run_shell_surface_close(&surface, shell_target_options(target))?;
                }
            },
            ShellAction::Pane { action } => match action {
                ShellPaneAction::List { surface, target } => {
                    cli::shell::run_shell_pane_list(
                        surface.as_deref(),
                        shell_target_options(target),
                    )?;
                }
                ShellPaneAction::Snapshot { pane, target } => {
                    cli::shell::run_shell_pane_snapshot(&pane, shell_target_options(target))?;
                }
                ShellPaneAction::Split {
                    pane,
                    direction,
                    target,
                } => {
                    cli::shell::run_shell_pane_split(
                        &pane,
                        &direction,
                        shell_target_options(target),
                    )?;
                }
                ShellPaneAction::Close { pane, target } => {
                    cli::shell::run_shell_pane_close(&pane, shell_target_options(target))?;
                }
                ShellPaneAction::Lift {
                    pane,
                    title,
                    target,
                } => {
                    cli::shell::run_shell_pane_lift(
                        &pane,
                        title.as_deref(),
                        shell_target_options(target),
                    )?;
                }
                ShellPaneAction::Move {
                    pane,
                    surface,
                    direction,
                    target,
                } => {
                    cli::shell::run_shell_pane_move(
                        &pane,
                        &surface,
                        &direction,
                        shell_target_options(target),
                    )?;
                }
                ShellPaneAction::Focus { pane, target } => {
                    cli::shell::run_shell_pane_focus(&pane, shell_target_options(target))?;
                }
                ShellPaneAction::SendText { pane, text, target } => {
                    cli::shell::run_shell_pane_send_text(
                        &pane,
                        &text,
                        shell_target_options(target),
                    )?;
                }
            },
            ShellAction::Attention { action } => match action {
                ShellAttentionAction::Inbox { target } => {
                    cli::shell::run_shell_attention_inbox(shell_target_options(target))?;
                }
                ShellAttentionAction::Set {
                    pane,
                    state,
                    target,
                } => {
                    cli::shell::run_shell_attention_set(
                        &pane,
                        &state,
                        shell_target_options(target),
                    )?;
                }
            },
            ShellAction::Routing { action } => match action {
                ShellRoutingAction::Candidates { pane, target } => {
                    cli::shell::run_shell_routing_candidates(
                        pane.as_deref(),
                        shell_target_options(target),
                    )?;
                }
            },
            ShellAction::Events {
                after_event_id,
                limit,
                follow,
                target,
            } => {
                cli::shell::run_shell_events(
                    after_event_id.as_deref(),
                    limit,
                    follow,
                    shell_target_options(target),
                )?;
            }
        },
        None => {
            // Default: launch chat (TUI)
            preflight_chat_agent_config()?;
            cli::chat::run_chat(agent_name.as_deref()).await?;
        }
    }

    Ok(())
}

fn preflight_chat_agent_config() -> Result<()> {
    let agentd_url_override = host_config::daemon_url_env_override();
    if should_preflight_chat_agent_config(agentd_url_override.as_deref()) {
        cli::load_agent_config_with_notice()?;
    }
    Ok(())
}

fn should_preflight_chat_agent_config(agentd_url_override: Option<&str>) -> bool {
    match agentd_url_override {
        Some(url) => url.trim().is_empty(),
        None => true,
    }
}

fn shell_target_options(args: ShellTargetArgs) -> cli::shell::ShellTargetOptions {
    cli::shell::ShellTargetOptions {
        socket: args.socket,
        control_dir: args.control_dir,
        window: args.window,
        timeout_ms: args.timeout_ms,
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                .add_directive("alan=debug".parse().unwrap()),
        )
        .init();
}

#[cfg(test)]
mod tests {
    use super::should_preflight_chat_agent_config;

    #[test]
    fn test_chat_preflight_runs_without_remote_daemon_override() {
        assert!(should_preflight_chat_agent_config(None));
    }

    #[test]
    fn test_chat_preflight_skips_with_remote_daemon_override() {
        assert!(!should_preflight_chat_agent_config(Some(
            "http://remote-agentd:8090"
        )));
    }

    #[test]
    fn test_chat_preflight_treats_blank_override_as_local_mode() {
        assert!(should_preflight_chat_agent_config(Some("   ")));
    }
}
