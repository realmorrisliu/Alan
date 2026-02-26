//! `alan chat` — launch interactive TUI.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Launch the interactive TUI chat.
///
/// Spawns the Bun TUI process, which manages the daemon lifecycle.
pub async fn run_chat() -> Result<()> {
    // Find TUI bundle
    let tui_path = find_tui_bundle()?;

    // Build command
    let mut cmd = Command::new("bun");
    cmd.arg("run").arg(&tui_path);

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
fn find_tui_bundle() -> Result<PathBuf> {
    find_tui_bundle_with_env(
        std::env::var("ALAN_TUI_PATH").ok().as_deref(),
        std::env::current_exe().ok().as_deref(),
        dirs::home_dir().as_deref(),
    )
}

/// Find the TUI JS bundle with injectable dependencies (for testing).
fn find_tui_bundle_with_env(
    env_path: Option<&str>,
    current_exe: Option<&Path>,
    home_dir: Option<&Path>,
) -> Result<PathBuf> {
    // 1. ALAN_TUI_PATH env
    if let Some(path) = env_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    // 2. Adjacent to binary (production install)
    if let Some(exe) = current_exe
        && let Some(dir) = exe.parent()
    {
        let prod_path = dir.join("alan-tui.js");
        if prod_path.exists() {
            return Ok(prod_path);
        }
    }

    // 3. ~/.alan/bin/alan-tui.js
    if let Some(home) = home_dir {
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
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!("Cannot find TUI bundle. Set ALAN_TUI_PATH or run `just install`.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_tui_bundle_from_env() {
        let tmp = TempDir::new().unwrap();
        let tui_file = tmp.path().join("alan-tui.js");
        std::fs::write(&tui_file, "// test").unwrap();

        let result = find_tui_bundle_with_env(Some(tui_file.to_str().unwrap()), None, None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tui_file);
    }

    #[test]
    fn test_find_tui_bundle_from_exe_parent() {
        let tmp = TempDir::new().unwrap();
        let exe_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let tui_file = exe_dir.join("alan-tui.js");
        std::fs::write(&tui_file, "// test").unwrap();

        let exe_path = exe_dir.join("alan");

        let result = find_tui_bundle_with_env(None, Some(&exe_path), None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tui_file);
    }

    #[test]
    fn test_find_tui_bundle_from_home() {
        let tmp = TempDir::new().unwrap();
        let home_dir = tmp.path();
        let bin_dir = home_dir.join(".alan/bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let tui_file = bin_dir.join("alan-tui.js");
        std::fs::write(&tui_file, "// test").unwrap();

        let result = find_tui_bundle_with_env(None, None, Some(home_dir));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tui_file);
    }

    #[test]
    fn test_find_tui_bundle_env_takes_precedence() {
        let tmp = TempDir::new().unwrap();

        // Create env path file
        let env_file = tmp.path().join("env-tui.js");
        std::fs::write(&env_file, "// env").unwrap();

        // Create exe parent file
        let exe_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let exe_file = exe_dir.join("alan-tui.js");
        std::fs::write(&exe_file, "// exe").unwrap();

        let exe_path = exe_dir.join("alan");

        // Env should take precedence
        let result =
            find_tui_bundle_with_env(Some(env_file.to_str().unwrap()), Some(&exe_path), None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), env_file);
    }

    #[test]
    fn test_find_tui_bundle_env_not_existing_falls_through() {
        let tmp = TempDir::new().unwrap();

        // Create env path that doesn't exist
        let env_file = tmp.path().join("nonexistent.js");

        // Create exe parent file
        let exe_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let exe_file = exe_dir.join("alan-tui.js");
        std::fs::write(&exe_file, "// exe").unwrap();

        let exe_path = exe_dir.join("alan");

        // Should fall through to exe parent
        let result =
            find_tui_bundle_with_env(Some(env_file.to_str().unwrap()), Some(&exe_path), None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exe_file);
    }

    // Note: Testing "not found" case is unreliable in dev environment
    // because dev_paths (clients/tui/src/index.tsx) may exist.
}
