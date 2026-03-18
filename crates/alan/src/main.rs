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

use alan::OutputMode;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "alan", about = "Alan — AI Turing Machine", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Migrate on-disk files across breaking terminology changes
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
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
enum MigrateAction {
    /// Rewrite config, model overlay, and workspace state files to the current terminology
    Terminology {
        /// Workspace root or `.alan` directory whose local files should be migrated
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Explicit config file path to migrate instead of the default resolution
        #[arg(long)]
        config_path: Option<PathBuf>,
        /// Apply changes in place. Without this flag, Alan prints a dry-run preview.
        #[arg(long)]
        write: bool,
    },
    /// Split the legacy global config into canonical ~/.alan/agent/agent.toml and ~/.alan/host.toml
    AgentHome {
        /// Explicit legacy config path to migrate instead of ~/.config/alan/config.toml
        #[arg(long)]
        legacy_config_path: Option<PathBuf>,
        /// Apply changes in place. Without this flag, Alan prints a dry-run preview.
        #[arg(long)]
        write: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Daemon { action }) => match action {
            DaemonAction::Start { foreground } => {
                if foreground {
                    // Run in foreground (blocking)
                    init_tracing();
                    let config = cli::load_agent_config_with_notice()?;
                    daemon::server::run_server(config).await?;
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
        Some(Commands::Migrate { action }) => match action {
            MigrateAction::Terminology {
                workspace,
                config_path,
                write,
            } => {
                cli::migrate::run_migrate_terminology(workspace, config_path, write)?;
            }
            MigrateAction::AgentHome {
                legacy_config_path,
                write,
            } => {
                cli::migrate::run_migrate_agent_home(legacy_config_path, write)?;
            }
        },
        Some(Commands::Chat) => {
            cli::load_agent_config_with_notice()?;
            cli::chat::run_chat().await?;
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
                workspace,
                output,
                thinking,
                timeout,
                streaming_mode,
                partial_stream_recovery_mode,
            )
            .await;
            std::process::exit(code);
        }
        None => {
            // Default: launch chat (TUI)
            cli::load_agent_config_with_notice()?;
            cli::chat::run_chat().await?;
        }
    }

    Ok(())
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
