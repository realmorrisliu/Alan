use alan_runtime::{
    AlanHomePaths, TerminologyFileKind, migrate_config_toml, migrate_model_overlay_toml,
    migrate_workspace_state_json,
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct MigrationTarget {
    kind: TerminologyFileKind,
    path: PathBuf,
    label: &'static str,
}

pub fn run_migrate_terminology(
    workspace: Option<PathBuf>,
    config_path: Option<PathBuf>,
    write: bool,
) -> Result<()> {
    let write_command = format_write_command(workspace.as_ref(), config_path.as_ref());
    let targets = collect_migration_targets(workspace, config_path)?;
    if targets.is_empty() {
        println!("No config, model overlay, or workspace state files found to inspect.");
        return Ok(());
    }

    let mut pending = Vec::new();
    for target in targets {
        let raw = std::fs::read_to_string(&target.path)
            .with_context(|| format!("failed to read {}", target.path.display()))?;
        let migration = match target.kind {
            TerminologyFileKind::ConfigToml => migrate_config_toml(&raw)?,
            TerminologyFileKind::ModelCatalogOverlayToml => migrate_model_overlay_toml(&raw)?,
            TerminologyFileKind::WorkspaceStateJson => migrate_workspace_state_json(&raw)?,
        };

        if migration.changed() {
            pending.push((target, migration));
        } else {
            println!("Unchanged: {}", target.path.display());
        }
    }

    if pending.is_empty() {
        println!("No terminology migration needed.");
        return Ok(());
    }

    if !write {
        println!("Migration preview:");
        for (target, migration) in &pending {
            println!("Would migrate {}: {}", target.label, target.path.display());
            for change in migration.changes() {
                println!("  - {}", change);
            }
        }
        println!();
        println!("Run `{write_command}` to apply these changes.");
        return Ok(());
    }

    for (target, migration) in pending {
        write_migrated_file(&target.path, migration.rewritten())?;
        println!("Migrated {}: {}", target.label, target.path.display());
    }

    Ok(())
}

pub fn run_migrate_agent_home(legacy_config_path: Option<PathBuf>, write: bool) -> Result<()> {
    let paths = AlanHomePaths::detect().context("cannot resolve home directory")?;
    run_migrate_agent_home_with_paths(paths, legacy_config_path, write)
}

fn run_migrate_agent_home_with_paths(
    paths: AlanHomePaths,
    legacy_config_path: Option<PathBuf>,
    write: bool,
) -> Result<()> {
    let target_path = paths.global_agent_config_path.clone();
    let legacy_source = resolve_agent_home_legacy_source(legacy_config_path, &paths)?;

    let Some(source_path) = legacy_source else {
        if target_path.exists() {
            println!(
                "Canonical global agent config already present at {}",
                target_path.display()
            );
        } else {
            println!(
                "No legacy global agent config found at {}",
                paths.legacy_global_config_path.display()
            );
        }
        return Ok(());
    };

    let raw = std::fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let migration = migrate_config_toml(&raw)?;
    let rewritten = migration.rewritten().to_string();

    if target_path.exists() {
        let existing = std::fs::read_to_string(&target_path)
            .with_context(|| format!("failed to read {}", target_path.display()))?;
        if existing == rewritten {
            println!(
                "Canonical global agent config already up to date at {}",
                target_path.display()
            );
            return Ok(());
        }

        anyhow::bail!(
            "refusing to overwrite existing canonical agent config {}. Merge {} manually or remove the target first.",
            target_path.display(),
            source_path.display()
        );
    }

    if !write {
        let write_command = format_agent_home_write_command(Some(&source_path));
        println!("Migration preview:");
        println!("Would copy legacy global config:");
        println!("  {} -> {}", source_path.display(), target_path.display());
        if migration.changed() {
            for change in migration.changes() {
                println!("  - {}", change);
            }
        } else {
            println!("  - no terminology rewrite needed");
        }
        println!();
        println!("Run `{write_command}` to apply this migration.");
        return Ok(());
    }

    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(&target_path, rewritten)
        .with_context(|| format!("failed to write {}", target_path.display()))?;

    println!(
        "Migrated global agent config: {} -> {}",
        source_path.display(),
        target_path.display()
    );
    println!(
        "Legacy config was left in place at {} as a fallback. Remove it after verifying the new path.",
        source_path.display()
    );

    Ok(())
}

fn format_write_command(workspace: Option<&PathBuf>, config_path: Option<&PathBuf>) -> String {
    let mut command = String::from("alan migrate terminology --write");
    if let Some(workspace) = workspace {
        command.push_str(&format!(" --workspace \"{}\"", workspace.display()));
    }
    if let Some(config_path) = config_path {
        command.push_str(&format!(" --config-path \"{}\"", config_path.display()));
    }
    command
}

fn format_agent_home_write_command(legacy_config_path: Option<&PathBuf>) -> String {
    let mut command = String::from("alan migrate agent-home --write");
    if let Some(path) = legacy_config_path {
        command.push_str(&format!(" --legacy-config-path \"{}\"", path.display()));
    }
    command
}

fn collect_migration_targets(
    workspace: Option<PathBuf>,
    config_path: Option<PathBuf>,
) -> Result<Vec<MigrationTarget>> {
    let mut targets = Vec::new();
    let mut seen = HashSet::new();

    if let Some(path) = resolve_config_target(config_path)? {
        push_target(
            &mut targets,
            &mut seen,
            MigrationTarget {
                kind: TerminologyFileKind::ConfigToml,
                path,
                label: "config",
            },
        );
    }

    if let Some(path) = global_models_path()? {
        push_target(
            &mut targets,
            &mut seen,
            MigrationTarget {
                kind: TerminologyFileKind::ModelCatalogOverlayToml,
                path,
                label: "global model overlay",
            },
        );
    }

    if let Some(workspace_root) = resolve_workspace_root(workspace)? {
        let alan_dir = workspace_root.join(".alan");
        let workspace_models = alan_dir.join("models.toml");
        if workspace_models.exists() {
            push_target(
                &mut targets,
                &mut seen,
                MigrationTarget {
                    kind: TerminologyFileKind::ModelCatalogOverlayToml,
                    path: workspace_models,
                    label: "workspace model overlay",
                },
            );
        }

        let workspace_state = alan_dir.join("state.json");
        if workspace_state.exists() {
            push_target(
                &mut targets,
                &mut seen,
                MigrationTarget {
                    kind: TerminologyFileKind::WorkspaceStateJson,
                    path: workspace_state,
                    label: "workspace state",
                },
            );
        }
    }

    Ok(targets)
}

fn push_target(
    targets: &mut Vec<MigrationTarget>,
    seen: &mut HashSet<PathBuf>,
    target: MigrationTarget,
) {
    let canonical = target.path.clone();
    if seen.insert(canonical) {
        targets.push(target);
    }
}

fn resolve_config_target(explicit: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        if !path.exists() {
            anyhow::bail!("configuration file does not exist: {}", path.display());
        }
        return Ok(Some(path));
    }

    if let Ok(override_path) = std::env::var("ALAN_CONFIG_PATH") {
        let override_path = expand_tilde(Path::new(&override_path))?;
        if override_path.exists() {
            return Ok(Some(override_path));
        }
    }

    let Some(home) = dirs::home_dir() else {
        return Ok(None);
    };
    let path = home.join(".config").join("alan").join("config.toml");
    if path.exists() {
        return Ok(Some(path));
    }

    Ok(None)
}

fn resolve_agent_home_legacy_source(
    explicit: Option<PathBuf>,
    paths: &AlanHomePaths,
) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        let path = expand_tilde(&path)?;
        if !path.exists() {
            anyhow::bail!(
                "legacy configuration file does not exist: {}",
                path.display()
            );
        }
        return Ok(Some(path));
    }

    if paths.legacy_global_config_path.exists() {
        return Ok(Some(paths.legacy_global_config_path.clone()));
    }

    Ok(None)
}

fn global_models_path() -> Result<Option<PathBuf>> {
    let Some(home) = dirs::home_dir() else {
        return Ok(None);
    };
    let path = home.join(".alan").join("models.toml");
    if path.exists() {
        return Ok(Some(path));
    }
    Ok(None)
}

fn resolve_workspace_root(explicit: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        let normalized = normalize_workspace_root(path)?;
        return Ok(Some(normalized));
    }

    let cwd = std::env::current_dir().context("failed to determine current directory")?;
    if cwd.file_name().and_then(|name| name.to_str()) == Some(".alan") {
        return cwd.parent().map(Path::to_path_buf).map(Ok).transpose();
    }
    if cwd.join(".alan").exists() {
        return Ok(Some(cwd));
    }

    Ok(None)
}

fn normalize_workspace_root(path: PathBuf) -> Result<PathBuf> {
    let path = expand_tilde(&path)?;
    let canonical = std::fs::canonicalize(&path)
        .with_context(|| format!("invalid workspace path {}", path.display()))?;
    if canonical.file_name().and_then(|name| name.to_str()) == Some(".alan") {
        return canonical
            .parent()
            .map(Path::to_path_buf)
            .context("workspace .alan directory has no parent");
    }
    Ok(canonical)
}

fn expand_tilde(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    if path_str == "~" {
        return dirs::home_dir().context("cannot resolve home directory");
    }
    if let Some(rest) = path_str.strip_prefix("~/") {
        let home = dirs::home_dir().context("cannot resolve home directory")?;
        return Ok(home.join(rest));
    }
    Ok(path.to_path_buf())
}

fn write_migrated_file(path: &Path, content: &str) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("migration target has no filename")?;
    let backup_path = path.with_file_name(format!("{file_name}.bak"));
    let temp_path = path.with_file_name(format!("{file_name}.tmp"));

    std::fs::write(&backup_path, std::fs::read(path)?)
        .with_context(|| format!("failed to write backup {}", backup_path.display()))?;
    std::fs::write(&temp_path, content)
        .with_context(|| format!("failed to write temp file {}", temp_path.display()))?;
    std::fs::rename(&temp_path, path)
        .with_context(|| format!("failed to replace {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn normalize_workspace_root_accepts_dot_alan_path() {
        let temp = TempDir::new().unwrap();
        let alan_dir = temp.path().join(".alan");
        std::fs::create_dir_all(&alan_dir).unwrap();

        let normalized = normalize_workspace_root(alan_dir).unwrap();
        assert_eq!(normalized, std::fs::canonicalize(temp.path()).unwrap());
    }

    #[test]
    fn run_migrate_terminology_writes_backup_and_updates_workspace_files() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        let alan_dir = workspace.join(".alan");
        std::fs::create_dir_all(&alan_dir).unwrap();
        let state_path = alan_dir.join("state.json");
        std::fs::write(
            &state_path,
            r#"{"config":{"llm_provider":"openai_compatible"}}"#,
        )
        .unwrap();

        run_migrate_terminology(Some(workspace.clone()), None, true).unwrap();

        let updated = std::fs::read_to_string(&state_path).unwrap();
        assert!(updated.contains("\"openai_chat_completions_compatible\""));
        assert!(state_path.with_file_name("state.json.bak").exists());
    }

    #[test]
    fn run_migrate_agent_home_copies_legacy_config_to_canonical_path() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        std::fs::create_dir_all(paths.legacy_global_config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &paths.legacy_global_config_path,
            "llm_provider = \"openai_responses\"\nopenai_responses_model = \"gpt-5.4\"\n",
        )
        .unwrap();

        run_migrate_agent_home_with_paths(paths.clone(), None, true).unwrap();

        let migrated = std::fs::read_to_string(paths.global_agent_config_path).unwrap();
        assert!(migrated.contains("llm_provider = \"openai_responses\""));
    }

    #[test]
    fn run_migrate_agent_home_refuses_to_overwrite_existing_target() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        std::fs::create_dir_all(paths.legacy_global_config_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(paths.global_agent_config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &paths.legacy_global_config_path,
            "llm_provider = \"openai_responses\"\n",
        )
        .unwrap();
        std::fs::write(
            &paths.global_agent_config_path,
            "llm_provider = \"anthropic_messages\"\n",
        )
        .unwrap();

        let err = run_migrate_agent_home_with_paths(paths, None, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("refusing to overwrite existing canonical agent config"));
    }
}
