use alan_runtime::{
    AlanHomePaths, TerminologyFileKind, migrate_config_toml, migrate_model_overlay_toml,
    migrate_workspace_state_json,
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::host_config::HostConfig;

#[derive(Debug, Clone)]
struct MigrationTarget {
    kind: TerminologyFileKind,
    path: PathBuf,
    label: &'static str,
}

#[derive(Debug, Clone)]
struct PendingWrite {
    path: PathBuf,
    content: String,
    label: &'static str,
    replace_existing_if_matches: Option<String>,
}

#[derive(Debug, Clone)]
struct SplitLegacyAgentHomeConfig {
    agent_content: Option<String>,
    host_content: Option<String>,
    migrated_legacy_content: String,
    moved_host_keys: Vec<&'static str>,
    terminology_changes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentHomeMigrationSourceKind {
    ExplicitLegacyPath,
    LegacyFallback,
    CanonicalRepair,
}

#[derive(Debug, Clone)]
struct AgentHomeMigrationSource {
    path: PathBuf,
    kind: AgentHomeMigrationSourceKind,
}

impl AgentHomeMigrationSource {
    fn write_command_path(&self) -> Option<&PathBuf> {
        match self.kind {
            AgentHomeMigrationSourceKind::ExplicitLegacyPath => Some(&self.path),
            AgentHomeMigrationSourceKind::LegacyFallback
            | AgentHomeMigrationSourceKind::CanonicalRepair => None,
        }
    }
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
    let source = resolve_agent_home_source(legacy_config_path, &paths)?;

    let Some(source) = source else {
        let agent_exists = paths.global_agent_config_path.exists();
        let host_exists = paths.global_host_config_path.exists();
        if agent_exists || host_exists {
            println!("Canonical global config already present:");
            if agent_exists {
                println!("  - agent: {}", paths.global_agent_config_path.display());
            }
            if host_exists {
                println!("  - host: {}", paths.global_host_config_path.display());
            }
        } else {
            println!(
                "No legacy global agent config found at {}",
                paths.legacy_global_config_path.display()
            );
        }
        return Ok(());
    };

    let raw = std::fs::read_to_string(&source.path)
        .with_context(|| format!("failed to read {}", source.path.display()))?;
    let split = split_legacy_agent_home_config(&raw)?;
    let mut pending_writes = Vec::new();
    if let Some(agent_content) = split.agent_content.clone() {
        pending_writes.push(PendingWrite {
            path: paths.global_agent_config_path.clone(),
            content: agent_content,
            label: "canonical agent config",
            replace_existing_if_matches: (!split.moved_host_keys.is_empty())
                .then(|| split.migrated_legacy_content.clone()),
        });
    }
    if let Some(host_content) = split.host_content.clone() {
        pending_writes.push(PendingWrite {
            path: paths.global_host_config_path.clone(),
            content: host_content,
            label: "canonical host config",
            replace_existing_if_matches: None,
        });
    }
    if pending_writes.is_empty() {
        println!(
            "No agent-facing or host-facing settings found in {}",
            source.path.display()
        );
        return Ok(());
    }

    let pending_writes = ensure_targets_do_not_conflict(pending_writes, &source.path)?;
    if pending_writes.is_empty() {
        println!(
            "Canonical global agent/host config already up to date for {}",
            source.path.display()
        );
        return Ok(());
    }

    if !write {
        let write_command = format_agent_home_write_command(source.write_command_path());
        println!("Migration preview:");
        for pending in &pending_writes {
            println!("Would write {}:", pending.label);
            println!("  {} -> {}", source.path.display(), pending.path.display());
        }
        if split.moved_host_keys.is_empty() {
            println!("  - no host-only keys moved");
        } else {
            println!(
                "  - move host-only keys: {}",
                split.moved_host_keys.join(", ")
            );
        }
        if split.terminology_changes.is_empty() {
            println!("  - no terminology rewrite needed");
        } else {
            for change in &split.terminology_changes {
                println!("  - {}", change);
            }
        }
        println!();
        println!("Run `{write_command}` to apply this migration.");
        return Ok(());
    }

    for pending in &pending_writes {
        if let Some(parent) = pending.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(&pending.path, &pending.content)
            .with_context(|| format!("failed to write {}", pending.path.display()))?;
        println!(
            "Migrated {}: {} -> {}",
            pending.label,
            source.path.display(),
            pending.path.display()
        );
    }
    if source.kind != AgentHomeMigrationSourceKind::CanonicalRepair {
        println!(
            "Legacy config was left in place at {} as a fallback. Remove it after verifying the new path.",
            source.path.display()
        );
    }

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

fn split_legacy_agent_home_config(raw: &str) -> Result<SplitLegacyAgentHomeConfig> {
    let migration = migrate_config_toml(raw)?;
    let migrated_legacy_content = migration.rewritten().to_string();
    let mut document: toml::Value = toml::from_str(&migrated_legacy_content)
        .context("failed to parse migrated legacy global config")?;
    let table = document
        .as_table_mut()
        .context("legacy global config must be a top-level TOML table")?;

    let bind_address = take_string_key(table, "bind_address")?;
    let daemon_url = take_string_key(table, "daemon_url")?;

    let mut moved_host_keys = Vec::new();
    if bind_address.is_some() {
        moved_host_keys.push("bind_address");
    }
    if daemon_url.is_some() {
        moved_host_keys.push("daemon_url");
    }

    let agent_content = if table.is_empty() {
        None
    } else {
        Some(
            toml::to_string_pretty(&document)
                .context("failed to render canonical agent config TOML")?,
        )
    };

    let host_content = if bind_address.is_some() || daemon_url.is_some() {
        let bind_address = bind_address.unwrap_or_else(|| HostConfig::default().bind_address);
        let daemon_url = daemon_url
            .unwrap_or_else(|| HostConfig::local_daemon_url_for_bind_address(&bind_address));
        Some(
            toml::to_string_pretty(&HostConfig {
                bind_address,
                daemon_url,
            })
            .context("failed to render canonical host config TOML")?,
        )
    } else {
        None
    };

    Ok(SplitLegacyAgentHomeConfig {
        agent_content,
        host_content,
        migrated_legacy_content,
        moved_host_keys,
        terminology_changes: migration
            .changes()
            .iter()
            .map(ToString::to_string)
            .collect(),
    })
}

fn take_string_key(table: &mut toml::value::Table, key: &'static str) -> Result<Option<String>> {
    let Some(value) = table.remove(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        anyhow::bail!("legacy config key `{key}` must be a string");
    };
    Ok(Some(value.to_string()))
}

fn ensure_targets_do_not_conflict(
    pending_writes: Vec<PendingWrite>,
    source_path: &Path,
) -> Result<Vec<PendingWrite>> {
    let mut filtered = Vec::new();
    for pending in pending_writes {
        if pending.path.exists() {
            let existing = std::fs::read_to_string(&pending.path)
                .with_context(|| format!("failed to read {}", pending.path.display()))?;
            if existing == pending.content || toml_semantically_equal(&existing, &pending.content) {
                continue;
            }
            if let Some(repairable_existing) = &pending.replace_existing_if_matches
                && (existing == *repairable_existing
                    || toml_semantically_equal(&existing, repairable_existing))
            {
                filtered.push(pending);
                continue;
            }
            anyhow::bail!(
                "refusing to overwrite existing {} {}. Merge {} manually or remove the target first.",
                pending.label,
                pending.path.display(),
                source_path.display()
            );
        }
        filtered.push(pending);
    }
    Ok(filtered)
}

fn toml_semantically_equal(left: &str, right: &str) -> bool {
    let Ok(left) = toml::from_str::<toml::Value>(left) else {
        return false;
    };
    let Ok(right) = toml::from_str::<toml::Value>(right) else {
        return false;
    };
    left == right
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
    resolve_config_target_with_paths(
        explicit,
        std::env::var("ALAN_CONFIG_PATH").ok().map(PathBuf::from),
        AlanHomePaths::detect(),
        dirs::home_dir(),
    )
}

fn resolve_config_target_with_paths(
    explicit: Option<PathBuf>,
    override_path: Option<PathBuf>,
    paths: Option<AlanHomePaths>,
    home_dir: Option<PathBuf>,
) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        let path = expand_tilde_with_home(&path, home_dir.as_deref())?;
        if !path.exists() {
            anyhow::bail!("configuration file does not exist: {}", path.display());
        }
        return Ok(Some(path));
    }

    if let Some(override_path) = override_path {
        let override_path = expand_tilde_with_home(Path::new(&override_path), home_dir.as_deref())?;
        if override_path.exists() {
            return Ok(Some(override_path));
        }
    }

    if let Some(paths) = paths {
        if paths.global_agent_config_path.exists() {
            return Ok(Some(paths.global_agent_config_path));
        }

        if paths.legacy_global_config_path.exists() {
            return Ok(Some(paths.legacy_global_config_path));
        }
    }

    Ok(None)
}

fn resolve_agent_home_source(
    explicit: Option<PathBuf>,
    paths: &AlanHomePaths,
) -> Result<Option<AgentHomeMigrationSource>> {
    if let Some(path) = explicit {
        let path = expand_tilde(&path)?;
        if !path.exists() {
            anyhow::bail!(
                "legacy configuration file does not exist: {}",
                path.display()
            );
        }
        return Ok(Some(AgentHomeMigrationSource {
            path,
            kind: AgentHomeMigrationSourceKind::ExplicitLegacyPath,
        }));
    }

    if paths.legacy_global_config_path.exists() {
        return Ok(Some(AgentHomeMigrationSource {
            path: paths.legacy_global_config_path.clone(),
            kind: AgentHomeMigrationSourceKind::LegacyFallback,
        }));
    }

    if paths.global_agent_config_path.exists() {
        return Ok(Some(AgentHomeMigrationSource {
            path: paths.global_agent_config_path.clone(),
            kind: AgentHomeMigrationSourceKind::CanonicalRepair,
        }));
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
    expand_tilde_with_home(path, dirs::home_dir().as_deref())
}

fn expand_tilde_with_home(path: &Path, home_dir: Option<&Path>) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    if path_str == "~" {
        return home_dir
            .map(Path::to_path_buf)
            .context("cannot resolve home directory");
    }
    if let Some(rest) = path_str.strip_prefix("~/") {
        let home = home_dir.context("cannot resolve home directory")?;
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
            "llm_provider = \"openai_responses\"\nopenai_responses_model = \"gpt-5.4\"\nbind_address = \"127.0.0.1:9123\"\n",
        )
        .unwrap();

        run_migrate_agent_home_with_paths(paths.clone(), None, true).unwrap();

        let migrated = std::fs::read_to_string(paths.global_agent_config_path).unwrap();
        assert!(migrated.contains("llm_provider = \"openai_responses\""));
        assert!(!migrated.contains("bind_address"));

        let host = std::fs::read_to_string(paths.global_host_config_path).unwrap();
        assert!(host.contains("bind_address = \"127.0.0.1:9123\""));
        assert!(host.contains("daemon_url = \"http://127.0.0.1:9123\""));
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
            "llm_provider = \"openai_responses\"\nbind_address = \"127.0.0.1:9123\"\n",
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

    #[test]
    fn run_migrate_agent_home_repairs_old_canonical_copy_with_host_keys() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        std::fs::create_dir_all(paths.legacy_global_config_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(paths.global_agent_config_path.parent().unwrap()).unwrap();
        let legacy = "llm_provider = \"openai_responses\"\nopenai_responses_model = \"gpt-5.4\"\nbind_address = \"127.0.0.1:9123\"\n";
        std::fs::write(&paths.legacy_global_config_path, legacy).unwrap();
        std::fs::write(&paths.global_agent_config_path, legacy).unwrap();

        run_migrate_agent_home_with_paths(paths.clone(), None, true).unwrap();

        let migrated = std::fs::read_to_string(paths.global_agent_config_path).unwrap();
        assert!(migrated.contains("llm_provider = \"openai_responses\""));
        assert!(!migrated.contains("bind_address"));

        let host = std::fs::read_to_string(paths.global_host_config_path).unwrap();
        assert!(host.contains("bind_address = \"127.0.0.1:9123\""));
        assert!(host.contains("daemon_url = \"http://127.0.0.1:9123\""));
    }

    #[test]
    fn run_migrate_agent_home_repairs_canonical_copy_without_legacy_fallback() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        std::fs::create_dir_all(paths.global_agent_config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &paths.global_agent_config_path,
            "llm_provider = \"openai_responses\"\nopenai_responses_model = \"gpt-5.4\"\nbind_address = \"127.0.0.1:9123\"\n",
        )
        .unwrap();

        run_migrate_agent_home_with_paths(paths.clone(), None, true).unwrap();

        let migrated = std::fs::read_to_string(paths.global_agent_config_path).unwrap();
        assert!(migrated.contains("llm_provider = \"openai_responses\""));
        assert!(!migrated.contains("bind_address"));

        let host = std::fs::read_to_string(paths.global_host_config_path).unwrap();
        assert!(host.contains("bind_address = \"127.0.0.1:9123\""));
        assert!(host.contains("daemon_url = \"http://127.0.0.1:9123\""));
    }

    #[test]
    fn resolve_config_target_prefers_canonical_agent_config_over_legacy_fallback() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        std::fs::create_dir_all(paths.global_agent_config_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(paths.legacy_global_config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &paths.global_agent_config_path,
            "llm_provider = \"openai_responses\"\n",
        )
        .unwrap();
        std::fs::write(
            &paths.legacy_global_config_path,
            "llm_provider = \"anthropic_messages\"\n",
        )
        .unwrap();

        let resolved =
            resolve_config_target_with_paths(None, None, Some(paths.clone()), Some(home.clone()))
                .unwrap();

        assert_eq!(resolved, Some(paths.global_agent_config_path));
    }

    #[test]
    fn resolve_config_target_prefers_env_override_over_canonical_agent_config() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        let override_path = temp.path().join("override.toml");
        std::fs::create_dir_all(paths.global_agent_config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &paths.global_agent_config_path,
            "llm_provider = \"openai_responses\"\n",
        )
        .unwrap();
        std::fs::write(&override_path, "llm_provider = \"anthropic_messages\"\n").unwrap();

        let resolved = resolve_config_target_with_paths(
            None,
            Some(override_path.clone()),
            Some(paths),
            Some(home),
        )
        .unwrap();

        assert_eq!(resolved, Some(override_path));
    }

    #[test]
    fn resolve_config_target_falls_back_to_legacy_config_when_canonical_missing() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        std::fs::create_dir_all(paths.legacy_global_config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &paths.legacy_global_config_path,
            "llm_provider = \"openai_responses\"\n",
        )
        .unwrap();

        let resolved =
            resolve_config_target_with_paths(None, None, Some(paths.clone()), Some(home)).unwrap();

        assert_eq!(resolved, Some(paths.legacy_global_config_path));
    }

    #[test]
    fn resolve_config_target_expands_tilde_for_explicit_path() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let paths = AlanHomePaths::from_home_dir(&home);
        std::fs::create_dir_all(paths.global_agent_config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &paths.global_agent_config_path,
            "llm_provider = \"openai_responses\"\n",
        )
        .unwrap();

        let resolved = resolve_config_target_with_paths(
            Some(PathBuf::from("~/.alan/agent/agent.toml")),
            None,
            None,
            Some(home),
        );

        assert_eq!(resolved.unwrap(), Some(paths.global_agent_config_path));
    }

    #[test]
    fn split_legacy_agent_home_config_skips_host_file_when_no_host_keys_exist() {
        let split =
            split_legacy_agent_home_config("llm_provider = \"openai_responses\"\n").unwrap();
        assert!(
            split
                .agent_content
                .unwrap()
                .contains("llm_provider = \"openai_responses\"")
        );
        assert!(split.host_content.is_none());
        assert!(split.moved_host_keys.is_empty());
    }
}
