//! `alan daemon stop|status` + daemon lifecycle utilities.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// PID file location: `~/.alan/daemon.pid`
fn pid_file_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".alan").join("daemon.pid"))
}

/// Daemon URL (from env or default).
pub fn daemon_url() -> String {
    std::env::var("ALAN_AGENTD_URL").unwrap_or_else(|_| "http://127.0.0.1:8090".to_string())
}

/// Write the daemon PID to the PID file.
fn write_pid(pid: u32) -> Result<()> {
    let path = pid_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, pid.to_string())?;
    Ok(())
}

/// Read the daemon PID from the PID file.
fn read_pid() -> Result<Option<u32>> {
    let path = pid_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let pid = content.trim().parse::<u32>().ok();
    Ok(pid)
}

/// Remove the PID file.
fn remove_pid_file() -> Result<()> {
    let path = pid_file_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Check if a process with the given PID is alive using `kill -0`.
fn is_process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Send a signal to a process.
fn send_signal(pid: u32, signal: &str) -> bool {
    std::process::Command::new("kill")
        .args([signal, &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Start the daemon as a detached background process.
pub async fn start_daemon_background() -> Result<()> {
    // Check if already running
    if check_daemon_health().await {
        println!("✅ Daemon is already running at {}", daemon_url());
        return Ok(());
    }

    let alan_bin = std::env::current_exe().context("Cannot determine own executable path")?;

    eprintln!("Starting Alan daemon...");

    let child = std::process::Command::new(&alan_bin)
        .args(["daemon", "start", "--foreground"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .context("Failed to start daemon process")?;

    let pid = child.id();
    write_pid(pid)?;

    // Wait for daemon to become healthy
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);
    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Daemon failed to start within {:?}", timeout);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if check_daemon_health().await {
            println!("✅ Daemon started (pid: {})", pid);
            return Ok(());
        }
    }
}

/// Ensure the daemon is running, starting it if necessary.
pub async fn ensure_daemon_running() -> Result<()> {
    if check_daemon_health().await {
        return Ok(());
    }
    start_daemon_background().await
}

/// Stop the daemon.
pub async fn stop_daemon() -> Result<()> {
    // Try PID file first
    if let Ok(Some(pid)) = read_pid() {
        if is_process_alive(pid) {
            eprintln!("Stopping daemon (pid: {})...", pid);
            send_signal(pid, "-TERM");

            // Wait for process to exit
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(5);
            loop {
                if !is_process_alive(pid) {
                    break;
                }
                if start.elapsed() > timeout {
                    eprintln!("Daemon did not stop gracefully, sending SIGKILL...");
                    send_signal(pid, "-KILL");
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            remove_pid_file()?;
            println!("✅ Daemon stopped");
            return Ok(());
        } else {
            // PID file exists but process is dead — clean up stale file
            remove_pid_file()?;
        }
    }

    // Fallback: check if daemon is running but we don't have a PID
    if check_daemon_health().await {
        println!("⚠️  Daemon is running but no PID file found.");
        println!("   Cannot stop automatically. Find the process manually:");
        println!("   lsof -i :8090");
    } else {
        println!("Daemon is not running.");
    }

    Ok(())
}

/// Show daemon status.
pub async fn daemon_status() -> Result<()> {
    let url = daemon_url();

    if check_daemon_health().await {
        let pid_str = match read_pid() {
            Ok(Some(pid)) if is_process_alive(pid) => format!(" (pid: {})", pid),
            _ => String::new(),
        };
        println!("✅ Daemon is running at {}{}", url, pid_str);
    } else {
        // Clean up stale PID file if daemon isn't actually running
        if let Ok(Some(_)) = read_pid() {
            let _ = remove_pid_file();
        }
        println!("❌ Daemon is not running");
    }

    Ok(())
}

/// Check if the daemon is healthy via HTTP.
async fn check_daemon_health() -> bool {
    let url = format!("{}/health", daemon_url());
    let client = reqwest::Client::new();
    matches!(
        client
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await,
        Ok(resp) if resp.status().is_success()
    )
}
