//! `alan chat` — launch interactive TUI.

use anyhow::{Context, Result};
use std::process::Command;

/// Launch the interactive TUI chat.
///
/// Ensures the daemon is running, then spawns the Bun TUI process.
pub async fn run_chat() -> Result<()> {
    // Ensure daemon is running
    super::daemon::ensure_daemon_running().await?;

    // Find TUI bundle
    let tui_path = find_tui_bundle()?;

    // Build command
    let mut cmd = Command::new("bun");
    cmd.arg("run").arg(&tui_path);

    // Set agentd URL
    let daemon_url = super::daemon::daemon_url();
    cmd.env("ALAN_AGENTD_URL", &daemon_url);

    // Spawn TUI as the main process
    let status = cmd
        .status()
        .with_context(|| format!("Failed to launch TUI from {}", tui_path.display()))?;

    if !status.success() {
        anyhow::bail!("TUI exited with status: {}", status);
    }

    Ok(())
}

/// Find the TUI JS bundle.
fn find_tui_bundle() -> Result<std::path::PathBuf> {
    // 1. ALAN_TUI_PATH env
    if let Ok(path) = std::env::var("ALAN_TUI_PATH") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    // 2. Adjacent to binary (production install)
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let prod_path = dir.join("alan-tui.js");
        if prod_path.exists() {
            return Ok(prod_path);
        }
    }

    // 3. ~/.alan/bin/alan-tui.js
    if let Some(home) = dirs::home_dir() {
        let home_path = home.join(".alan/bin/alan-tui.js");
        if home_path.exists() {
            return Ok(home_path);
        }
    }

    // 4. Development mode: relative to project root
    let dev_paths = [
        "clients/tui/src/index.tsx",
        "../clients/tui/src/index.tsx",
        "../../clients/tui/src/index.tsx",
    ];
    for p in &dev_paths {
        let path = std::path::PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!("Cannot find TUI bundle. Set ALAN_TUI_PATH or run `just install`.")
}
