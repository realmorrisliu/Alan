use anyhow::Context;
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminologyMigration {
    rewritten: String,
    changes: Vec<String>,
}

impl TerminologyMigration {
    pub fn new(rewritten: String, changes: Vec<String>) -> Self {
        Self { rewritten, changes }
    }

    pub fn rewritten(&self) -> &str {
        &self.rewritten
    }

    pub fn changes(&self) -> &[String] {
        &self.changes
    }

    pub fn changed(&self) -> bool {
        !self.changes.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminologyFileKind {
    ConfigToml,
    ModelCatalogOverlayToml,
    WorkspaceStateJson,
}

pub fn migrate_config_toml(raw: &str) -> anyhow::Result<TerminologyMigration> {
    let mut value: toml::Value =
        toml::from_str(raw).context("failed to parse configuration TOML")?;
    let table = value
        .as_table_mut()
        .context("configuration TOML root must be a table")?;
    let mut changes = BTreeSet::new();

    migrate_provider_value(table, "llm_provider", &mut changes);

    for (old, new) in [
        (
            "gemini_project_id",
            "google_gemini_generate_content_project_id",
        ),
        ("gemini_location", "google_gemini_generate_content_location"),
        ("gemini_model", "google_gemini_generate_content_model"),
        ("openai_api_key", "openai_responses_api_key"),
        ("openai_base_url", "openai_responses_base_url"),
        ("openai_model", "openai_responses_model"),
        (
            "openai_compat_api_key",
            "openai_chat_completions_compatible_api_key",
        ),
        (
            "openai_compat_base_url",
            "openai_chat_completions_compatible_base_url",
        ),
        (
            "openai_compat_model",
            "openai_chat_completions_compatible_model",
        ),
        ("anthropic_compat_api_key", "anthropic_messages_api_key"),
        ("anthropic_compat_base_url", "anthropic_messages_base_url"),
        ("anthropic_compat_model", "anthropic_messages_model"),
        (
            "anthropic_compat_client_name",
            "anthropic_messages_client_name",
        ),
        (
            "anthropic_compat_user_agent",
            "anthropic_messages_user_agent",
        ),
    ] {
        rename_table_key(table, old, new, &mut changes)?;
    }

    let rewritten = toml::to_string_pretty(&value)?;
    Ok(TerminologyMigration::new(
        rewritten,
        changes.into_iter().collect(),
    ))
}

pub fn migrate_model_overlay_toml(raw: &str) -> anyhow::Result<TerminologyMigration> {
    let mut value: toml::Value =
        toml::from_str(raw).context("failed to parse model overlay TOML")?;
    let table = value
        .as_table_mut()
        .context("model overlay TOML root must be a table")?;
    let mut changes = BTreeSet::new();

    rename_table_key(
        table,
        "openai_compatible",
        "openai_chat_completions_compatible",
        &mut changes,
    )?;

    let rewritten = toml::to_string_pretty(&value)?;
    Ok(TerminologyMigration::new(
        rewritten,
        changes.into_iter().collect(),
    ))
}

pub fn migrate_workspace_state_json(raw: &str) -> anyhow::Result<TerminologyMigration> {
    let mut value: JsonValue =
        serde_json::from_str(raw).context("failed to parse workspace state JSON")?;
    let mut changes = BTreeSet::new();
    migrate_workspace_state_value(&mut value, &mut changes);

    let rewritten = serde_json::to_string_pretty(&value)?;
    Ok(TerminologyMigration::new(
        rewritten,
        changes.into_iter().collect(),
    ))
}

pub fn migration_command_hint(path: &Path, kind: TerminologyFileKind) -> String {
    match kind {
        TerminologyFileKind::ConfigToml => format!(
            "alan migrate terminology --write --config-path \"{}\"",
            path.display()
        ),
        TerminologyFileKind::ModelCatalogOverlayToml | TerminologyFileKind::WorkspaceStateJson => {
            if let Some(workspace_root) = workspace_root_from_artifact_path(path) {
                format!(
                    "alan migrate terminology --write --workspace \"{}\"",
                    workspace_root.display()
                )
            } else {
                "alan migrate terminology --write".to_string()
            }
        }
    }
}

fn migrate_provider_value(
    table: &mut toml::map::Map<String, toml::Value>,
    key: &str,
    changes: &mut BTreeSet<String>,
) {
    let Some(legacy_value) = table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    let Some(new_value) = map_legacy_provider_value(&legacy_value) else {
        return;
    };

    table.insert(key.to_string(), toml::Value::String(new_value.to_string()));
    changes.insert(format!(
        "value `{legacy_value}` -> `{new_value}` for `{key}`"
    ));
}

fn rename_table_key(
    table: &mut toml::map::Map<String, toml::Value>,
    old: &str,
    new: &str,
    changes: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    let Some(old_value) = table.remove(old) else {
        return Ok(());
    };

    match table.get(new) {
        None => {
            table.insert(new.to_string(), old_value);
            changes.insert(format!("key `{old}` -> `{new}`"));
            Ok(())
        }
        Some(existing) if *existing == old_value => {
            changes.insert(format!("remove duplicate legacy key `{old}`"));
            Ok(())
        }
        Some(_) => anyhow::bail!(
            "cannot migrate automatically because `{old}` and `{new}` are both present with different values"
        ),
    }
}

fn migrate_workspace_state_value(value: &mut JsonValue, changes: &mut BTreeSet<String>) {
    match value {
        JsonValue::Object(map) => {
            if let Some(provider) = map.get_mut("llm_provider")
                && let Some(legacy) = provider.as_str().map(str::to_string)
                && let Some(new) = map_legacy_provider_value(&legacy)
            {
                *provider = JsonValue::String(new.to_string());
                changes.insert(format!("value `{legacy}` -> `{new}` for `llm_provider`"));
            }

            for child in map.values_mut() {
                migrate_workspace_state_value(child, changes);
            }
        }
        JsonValue::Array(items) => {
            for child in items {
                migrate_workspace_state_value(child, changes);
            }
        }
        _ => {}
    }
}

fn workspace_root_from_artifact_path(path: &Path) -> Option<&Path> {
    let parent = path.parent()?;
    if parent.file_name().and_then(|name| name.to_str()) == Some(".alan") {
        return parent.parent();
    }
    None
}

fn map_legacy_provider_value(value: &str) -> Option<&'static str> {
    match value {
        "gemini" => Some("google_gemini_generate_content"),
        "openai" => Some("openai_responses"),
        "openai_compatible" => Some("openai_chat_completions_compatible"),
        "anthropic_compatible" => Some("anthropic_messages"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_config_toml_renames_legacy_keys_and_values() {
        let migration = migrate_config_toml(
            r#"
llm_provider = "openai_compatible"
openai_compat_api_key = "sk-test"
openai_compat_base_url = "https://api.example.com/v1"
openai_compat_model = "qwen"
"#,
        )
        .unwrap();

        assert!(migration.changed());
        assert!(
            migration
                .rewritten()
                .contains("llm_provider = \"openai_chat_completions_compatible\"")
        );
        assert!(
            migration
                .rewritten()
                .contains("openai_chat_completions_compatible_api_key = \"sk-test\"")
        );
        assert!(!migration.rewritten().contains("openai_compat_api_key"));
    }

    #[test]
    fn migrate_config_toml_reports_conflicting_mixed_keys() {
        let err = migrate_config_toml(
            r#"
openai_api_key = "old"
openai_responses_api_key = "new"
"#,
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("cannot migrate automatically because `openai_api_key` and `openai_responses_api_key` are both present"));
    }

    #[test]
    fn migrate_model_overlay_toml_renames_legacy_section() {
        let migration = migrate_model_overlay_toml(
            r#"
[openai_compatible]
[[openai_compatible.models]]
slug = "custom"
family = "custom"
context_window_tokens = 1
"#,
        )
        .unwrap();

        assert!(migration.changed());
        assert!(
            migration.changes().iter().any(|change| change
                == "key `openai_compatible` -> `openai_chat_completions_compatible`")
        );
        assert!(
            migration
                .rewritten()
                .contains("[[openai_chat_completions_compatible.models]]")
        );
    }

    #[test]
    fn migrate_workspace_state_json_updates_legacy_provider_values() {
        let migration = migrate_workspace_state_json(
            r#"{
  "config": {
    "llm_provider": "anthropic_compatible"
  }
}"#,
        )
        .unwrap();

        assert!(migration.changed());
        assert!(
            migration
                .rewritten()
                .contains("\"llm_provider\": \"anthropic_messages\"")
        );
    }

    #[test]
    fn migration_command_hint_uses_workspace_when_path_is_under_dot_alan() {
        let hint = migration_command_hint(
            Path::new("/tmp/demo/.alan/models.toml"),
            TerminologyFileKind::ModelCatalogOverlayToml,
        );

        assert_eq!(
            hint,
            "alan migrate terminology --write --workspace \"/tmp/demo\""
        );
    }
}
