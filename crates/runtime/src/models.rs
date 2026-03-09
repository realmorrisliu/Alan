use anyhow::Context;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCatalogProvider {
    Openai,
    OpenaiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    pub slug: String,
    pub aliases: Vec<String>,
    pub provider: ModelCatalogProvider,
    pub family: String,
    pub context_window_tokens: u32,
    pub supports_reasoning: bool,
}

#[derive(Debug, Clone)]
pub struct ModelCatalog {
    openai: ProviderCatalog,
    openai_compatible: ProviderCatalog,
}

#[derive(Debug, Clone)]
struct ProviderCatalog {
    default_model: String,
    entries: Vec<CatalogEntry>,
}

#[derive(Debug, Clone)]
struct CatalogEntry {
    info: ModelInfo,
    accepts_date_suffixes: bool,
}

#[derive(Debug, Deserialize)]
struct ModelCatalogToml {
    openai: ProviderCatalogToml,
    openai_compatible: ProviderCatalogToml,
}

#[derive(Debug, Deserialize)]
struct ProviderCatalogToml {
    defaults: ProviderDefaultsToml,
    models: Vec<ModelInfoToml>,
}

#[derive(Debug, Deserialize)]
struct ProviderDefaultsToml {
    model: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelInfoToml {
    slug: String,
    #[serde(default)]
    aliases: Vec<String>,
    family: String,
    context_window_tokens: u32,
    #[serde(default)]
    supports_reasoning: bool,
    #[serde(default)]
    accepts_date_suffixes: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ModelCatalogOverlayToml {
    openai_compatible: Option<ProviderCatalogOverlayToml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ProviderCatalogOverlayToml {
    models: Vec<ModelInfoToml>,
}

static BASE_MODEL_CATALOG: LazyLock<ModelCatalog> = LazyLock::new(|| {
    let catalog: ModelCatalogToml = toml::from_str(include_str!("../models/catalog.toml"))
        .expect("runtime model catalog TOML should parse");

    ModelCatalog {
        openai: ProviderCatalog::from_toml(ModelCatalogProvider::Openai, catalog.openai),
        openai_compatible: ProviderCatalog::from_toml(
            ModelCatalogProvider::OpenaiCompatible,
            catalog.openai_compatible,
        ),
    }
});

pub fn base_catalog() -> &'static ModelCatalog {
    &BASE_MODEL_CATALOG
}

pub fn default_model_slug(provider: ModelCatalogProvider) -> &'static str {
    base_catalog().default_model_slug(provider)
}

impl ModelCatalog {
    pub fn load_with_overlays(workspace_root: Option<&Path>) -> anyhow::Result<Self> {
        let global_overlay = dirs::home_dir().map(|home| home.join(".alan").join("models.toml"));
        let workspace_overlay = workspace_root.map(|root| root.join(".alan").join("models.toml"));
        Self::load_with_overlay_paths(global_overlay.as_deref(), workspace_overlay.as_deref())
    }

    pub fn default_model_slug(&self, provider: ModelCatalogProvider) -> &str {
        self.provider_catalog(provider).default_model.as_str()
    }

    pub fn find_model_info(
        &self,
        provider: ModelCatalogProvider,
        model: &str,
    ) -> Option<&ModelInfo> {
        let normalized = normalize_model_id(model);
        self.provider_catalog(provider)
            .entries
            .iter()
            .find(|entry| entry.matches(&normalized))
            .map(|entry| &entry.info)
    }

    pub fn supported_model_slugs(&self, provider: ModelCatalogProvider) -> Vec<&str> {
        self.provider_catalog(provider)
            .entries
            .iter()
            .map(|entry| entry.info.slug.as_str())
            .collect()
    }

    fn load_with_overlay_paths(
        global_overlay: Option<&Path>,
        workspace_overlay: Option<&Path>,
    ) -> anyhow::Result<Self> {
        let mut catalog = base_catalog().clone();
        if let Some(path) = global_overlay {
            catalog.apply_overlay_path(path)?;
        }
        if let Some(path) = workspace_overlay {
            catalog.apply_overlay_path(path)?;
        }
        Ok(catalog)
    }

    fn apply_overlay_path(&mut self, path: &Path) -> anyhow::Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read model catalog overlay {}", path.display()))?;
        let overlay: ModelCatalogOverlayToml = toml::from_str(&raw)
            .with_context(|| format!("failed to parse model catalog overlay {}", path.display()))?;

        if let Some(openai_compatible) = overlay.openai_compatible {
            self.openai_compatible
                .apply_overlay(ModelCatalogProvider::OpenaiCompatible, openai_compatible)?;
        }

        Ok(())
    }

    fn provider_catalog(&self, provider: ModelCatalogProvider) -> &ProviderCatalog {
        match provider {
            ModelCatalogProvider::Openai => &self.openai,
            ModelCatalogProvider::OpenaiCompatible => &self.openai_compatible,
        }
    }
}

impl ProviderCatalog {
    fn from_toml(provider: ModelCatalogProvider, raw: ProviderCatalogToml) -> Self {
        let entries = raw
            .models
            .into_iter()
            .map(|model| CatalogEntry::from_toml(provider, model))
            .collect::<Vec<_>>();

        let catalog = Self {
            default_model: raw.defaults.model,
            entries,
        };
        catalog
            .validate()
            .expect("bundled runtime model catalog is valid");
        catalog
    }

    fn apply_overlay(
        &mut self,
        provider: ModelCatalogProvider,
        overlay: ProviderCatalogOverlayToml,
    ) -> anyhow::Result<()> {
        for model in overlay.models {
            let entry = CatalogEntry::from_toml(provider, model);
            if let Some(existing) = self
                .entries
                .iter_mut()
                .find(|existing| existing.info.slug == entry.info.slug)
            {
                *existing = entry;
            } else {
                self.entries.push(entry);
            }
        }

        self.validate()
    }

    fn validate(&self) -> anyhow::Result<()> {
        if !self
            .entries
            .iter()
            .any(|entry| entry.info.slug == self.default_model)
        {
            anyhow::bail!(
                "default model `{}` must exist in resolved model catalog",
                self.default_model
            );
        }

        let mut seen = HashSet::new();
        for entry in &self.entries {
            for name in std::iter::once(entry.info.slug.as_str())
                .chain(entry.info.aliases.iter().map(String::as_str))
            {
                let normalized = normalize_model_id(name);
                if !seen.insert(normalized.clone()) {
                    anyhow::bail!(
                        "duplicate model slug or alias `{}` in resolved model catalog",
                        normalized
                    );
                }
            }
        }

        Ok(())
    }
}

impl CatalogEntry {
    fn from_toml(provider: ModelCatalogProvider, raw: ModelInfoToml) -> Self {
        Self {
            accepts_date_suffixes: raw.accepts_date_suffixes,
            info: ModelInfo {
                slug: raw.slug,
                aliases: raw.aliases,
                provider,
                family: raw.family,
                context_window_tokens: raw.context_window_tokens,
                supports_reasoning: raw.supports_reasoning,
            },
        }
    }

    fn matches(&self, candidate: &str) -> bool {
        self.matches_alias(candidate, &self.info.slug)
            || self
                .info
                .aliases
                .iter()
                .any(|alias| self.matches_alias(candidate, alias))
    }

    fn matches_alias(&self, candidate: &str, alias: &str) -> bool {
        let normalized_alias = normalize_model_id(alias);
        candidate == normalized_alias
            || (self.accepts_date_suffixes
                && candidate
                    .strip_prefix(normalized_alias.as_str())
                    .is_some_and(is_supported_snapshot_suffix))
    }
}

fn normalize_model_id(model: &str) -> String {
    model.trim().to_ascii_lowercase()
}

fn is_supported_snapshot_suffix(suffix: &str) -> bool {
    let Some(snapshot) = suffix.strip_prefix('-') else {
        return false;
    };

    is_compact_date_snapshot(snapshot) || is_iso_date_snapshot(snapshot)
}

fn is_compact_date_snapshot(snapshot: &str) -> bool {
    snapshot.len() == 8 && snapshot.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_iso_date_snapshot(snapshot: &str) -> bool {
    let mut parts = snapshot.split('-');
    let Some(year) = parts.next() else {
        return false;
    };
    let Some(month) = parts.next() else {
        return false;
    };
    let Some(day) = parts.next() else {
        return false;
    };

    parts.next().is_none()
        && year.len() == 4
        && month.len() == 2
        && day.len() == 2
        && [year, month, day]
            .into_iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn catalog_exposes_default_models() {
        assert_eq!(default_model_slug(ModelCatalogProvider::Openai), "gpt-5.4");
        assert_eq!(
            default_model_slug(ModelCatalogProvider::OpenaiCompatible),
            "qwen3.5-plus"
        );
    }

    #[test]
    fn finds_exact_canonical_slug() {
        let kimi = base_catalog()
            .find_model_info(ModelCatalogProvider::OpenaiCompatible, "kimi-k2.5")
            .unwrap();
        assert_eq!(kimi.slug, "kimi-k2.5");
        assert_eq!(kimi.context_window_tokens, 250_000);
    }

    #[test]
    fn finds_date_snapshot_aliases() {
        let qwen = base_catalog()
            .find_model_info(
                ModelCatalogProvider::OpenaiCompatible,
                "bailian/qwen3.5-plus-2026-02-15",
            )
            .unwrap();
        assert_eq!(qwen.slug, "qwen3.5-plus");
        assert_eq!(qwen.family, "qwen3.5");
    }

    #[test]
    fn rejects_non_snapshot_suffix_variants() {
        assert!(
            base_catalog()
                .find_model_info(ModelCatalogProvider::OpenaiCompatible, "kimi-k2.5-thinking")
                .is_none()
        );
    }

    #[test]
    fn workspace_overlay_adds_custom_openai_compatible_model() {
        let temp = TempDir::new().unwrap();
        let alan_dir = temp.path().join(".alan");
        std::fs::create_dir_all(&alan_dir).unwrap();
        std::fs::write(
            alan_dir.join("models.toml"),
            r#"
[openai_compatible]
[[openai_compatible.models]]
slug = "custom-kimi"
family = "custom"
context_window_tokens = 654321
supports_reasoning = true
"#,
        )
        .unwrap();

        let catalog =
            ModelCatalog::load_with_overlay_paths(None, Some(&alan_dir.join("models.toml")))
                .unwrap();
        let custom = catalog
            .find_model_info(ModelCatalogProvider::OpenaiCompatible, "custom-kimi")
            .unwrap();
        assert_eq!(custom.context_window_tokens, 654_321);
    }

    #[test]
    fn workspace_overlay_replaces_existing_model_metadata() {
        let temp = TempDir::new().unwrap();
        let alan_dir = temp.path().join(".alan");
        std::fs::create_dir_all(&alan_dir).unwrap();
        std::fs::write(
            alan_dir.join("models.toml"),
            r#"
[openai_compatible]
[[openai_compatible.models]]
slug = "deepseek-chat"
family = "deepseek-custom"
context_window_tokens = 64000
supports_reasoning = true
"#,
        )
        .unwrap();

        let catalog =
            ModelCatalog::load_with_overlay_paths(None, Some(&alan_dir.join("models.toml")))
                .unwrap();
        let custom = catalog
            .find_model_info(ModelCatalogProvider::OpenaiCompatible, "deepseek-chat")
            .unwrap();
        assert_eq!(custom.family, "deepseek-custom");
        assert_eq!(custom.context_window_tokens, 64_000);
        assert!(custom.supports_reasoning);
    }

    #[test]
    fn workspace_overlay_wins_over_global_overlay() {
        let temp = TempDir::new().unwrap();
        let global_dir = temp.path().join("global");
        let workspace_dir = temp.path().join("workspace");
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::create_dir_all(workspace_dir.join(".alan")).unwrap();

        std::fs::write(
            global_dir.join("models.toml"),
            r#"
[openai_compatible]
[[openai_compatible.models]]
slug = "custom-kimi"
family = "global"
context_window_tokens = 111111
supports_reasoning = false
"#,
        )
        .unwrap();
        std::fs::write(
            workspace_dir.join(".alan").join("models.toml"),
            r#"
[openai_compatible]
[[openai_compatible.models]]
slug = "custom-kimi"
family = "workspace"
context_window_tokens = 222222
supports_reasoning = true
"#,
        )
        .unwrap();

        let catalog = ModelCatalog::load_with_overlay_paths(
            Some(&global_dir.join("models.toml")),
            Some(&workspace_dir.join(".alan").join("models.toml")),
        )
        .unwrap();
        let custom = catalog
            .find_model_info(ModelCatalogProvider::OpenaiCompatible, "custom-kimi")
            .unwrap();
        assert_eq!(custom.family, "workspace");
        assert_eq!(custom.context_window_tokens, 222_222);
        assert!(custom.supports_reasoning);
    }
}
