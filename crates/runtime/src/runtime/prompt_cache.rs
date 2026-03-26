use crate::prompts;
use crate::skills::{
    ActiveSkillEnvelope, PromptTrackedPath, PromptTrackedPathFingerprint, ResolvedCapabilityView,
    Skill, SkillActivationReason, SkillHostCapabilities, SkillMetadata, SkillsRegistry,
    extract_mentions, format_skill_availability_issues, name_to_id, render_active_skill_prompt,
    render_skill_not_found, render_skill_unavailable, render_skills_list,
    skill_availability_issues,
};
use crate::tape::{ContentPart, parts_to_text};
use regex::RegexBuilder;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::Metadata;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PromptAssemblyMetrics {
    pub builds: u64,
    pub hits: u64,
    pub misses: u64,
    pub skills_hits: u64,
    pub skills_misses: u64,
    pub persona_hits: u64,
    pub persona_misses: u64,
}

impl PromptAssemblyMetrics {
    fn record_build(&mut self, skills_hit: bool, persona_hit: bool) {
        self.builds += 1;
        if skills_hit && persona_hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        if skills_hit {
            self.skills_hits += 1;
        } else {
            self.skills_misses += 1;
        }
        if persona_hit {
            self.persona_hits += 1;
        } else {
            self.persona_misses += 1;
        }
    }

    fn hit_ratio(&self) -> f64 {
        if self.builds == 0 {
            0.0
        } else {
            self.hits as f64 / self.builds as f64
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PromptAssemblyResult {
    pub domain_prompt: String,
    pub system_prompt: String,
    pub active_skills: Vec<ActiveSkillEnvelope>,
    pub metrics: PromptAssemblyMetrics,
    pub elapsed_ms: u128,
    pub skills_cache_hit: bool,
    pub persona_cache_hit: bool,
}

#[derive(Debug, Clone)]
struct RenderedDomainPrompt {
    prompt: String,
    active_skills: Vec<ActiveSkillEnvelope>,
    cache_hit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathFingerprint {
    path: PathBuf,
    content_fingerprint_mode: ContentFingerprintMode,
    state: PathState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathState {
    Missing,
    File(MetadataFingerprint),
    Directory(MetadataFingerprint),
    Other(MetadataFingerprint),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetadataFingerprint {
    modified: Option<SystemTime>,
    len: u64,
    content_digest: Option<[u8; 32]>,
    platform: PlatformFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlatformFingerprint {
    #[cfg(unix)]
    device_id: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    change_secs: i64,
    #[cfg(unix)]
    change_nanos: i64,
    #[cfg(not(unix))]
    readonly: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentFingerprintMode {
    MetadataOnly,
    FullFile,
    PrefixBytes(u64),
}

impl PlatformFingerprint {
    fn capture(metadata: &Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            Self {
                device_id: metadata.dev(),
                inode: metadata.ino(),
                change_secs: metadata.ctime(),
                change_nanos: metadata.ctime_nsec(),
            }
        }

        #[cfg(not(unix))]
        {
            Self {
                readonly: metadata.permissions().readonly(),
            }
        }
    }
}

impl MetadataFingerprint {
    fn capture(path: &Path, metadata: &Metadata, mode: ContentFingerprintMode) -> Self {
        Self {
            modified: metadata.modified().ok(),
            len: metadata.len(),
            content_digest: match mode {
                ContentFingerprintMode::MetadataOnly => None,
                ContentFingerprintMode::FullFile => hash_file_contents(path, None),
                ContentFingerprintMode::PrefixBytes(max_bytes) => {
                    hash_file_contents(path, Some(max_bytes))
                }
            },
            platform: PlatformFingerprint::capture(metadata),
        }
    }
}

impl PathFingerprint {
    fn capture(path: impl Into<PathBuf>) -> Self {
        Self::capture_with_mode(path, ContentFingerprintMode::FullFile)
    }

    fn capture_prompt_path(tracked_path: PromptTrackedPath) -> Self {
        Self::capture_with_mode(tracked_path.path, tracked_path.fingerprint.into())
    }

    fn capture_with_mode(
        path: impl Into<PathBuf>,
        content_fingerprint_mode: ContentFingerprintMode,
    ) -> Self {
        let path = path.into();
        let state =
            match std::fs::metadata(&path) {
                Ok(metadata) if metadata.is_file() => PathState::File(
                    MetadataFingerprint::capture(&path, &metadata, content_fingerprint_mode),
                ),
                Ok(metadata) if metadata.is_dir() => {
                    PathState::Directory(MetadataFingerprint::capture(
                        &path,
                        &metadata,
                        ContentFingerprintMode::MetadataOnly,
                    ))
                }
                Ok(metadata) => PathState::Other(MetadataFingerprint::capture(
                    &path,
                    &metadata,
                    ContentFingerprintMode::MetadataOnly,
                )),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => PathState::Missing,
                Err(_) => PathState::Missing,
            };
        Self {
            path,
            content_fingerprint_mode,
            state,
        }
    }

    fn matches_current(&self) -> bool {
        Self::capture_with_mode(self.path.clone(), self.content_fingerprint_mode) == *self
    }
}

impl From<PromptTrackedPathFingerprint> for ContentFingerprintMode {
    fn from(value: PromptTrackedPathFingerprint) -> Self {
        match value {
            PromptTrackedPathFingerprint::PrefixBytes(max_bytes) => Self::PrefixBytes(max_bytes),
        }
    }
}

fn hash_file_contents(path: &Path, max_bytes: Option<u64>) -> Option<[u8; 32]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];
    let mut remaining = max_bytes;
    loop {
        let slice_len = remaining
            .map(|bytes| bytes.min(buffer.len() as u64) as usize)
            .unwrap_or(buffer.len());
        if slice_len == 0 {
            break;
        }
        let bytes_read = file.read(&mut buffer[..slice_len]).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        if let Some(bytes_left) = remaining.as_mut() {
            *bytes_left = bytes_left.saturating_sub(bytes_read as u64);
        }
    }
    Some(hasher.finalize().into())
}

#[derive(Debug, Clone)]
struct CachedWorkspacePersona {
    tracked_paths: Vec<PathFingerprint>,
    rendered_section: String,
}

impl CachedWorkspacePersona {
    fn load(workspace_persona_dirs: &[PathBuf]) -> Self {
        let tracked_paths =
            prompts::workspace_persona_tracked_paths_from_dirs(workspace_persona_dirs)
                .into_iter()
                .map(PathFingerprint::capture)
                .collect();
        let rendered_section =
            prompts::render_workspace_persona_context_from_dirs(workspace_persona_dirs);
        Self {
            tracked_paths,
            rendered_section,
        }
    }

    fn is_current(&self) -> bool {
        self.tracked_paths
            .iter()
            .all(PathFingerprint::matches_current)
    }
}

#[derive(Debug, Clone)]
struct CachedSkillRender {
    tracked_paths: Vec<PathFingerprint>,
    rendered: String,
}

impl CachedSkillRender {
    fn load(skill: &Skill, envelope: &ActiveSkillEnvelope) -> Self {
        let rendered = render_active_skill_prompt(skill, envelope);
        let tracked_paths = rendered
            .tracked_paths
            .into_iter()
            .map(PathFingerprint::capture_prompt_path)
            .collect();
        Self {
            tracked_paths,
            rendered: rendered.rendered,
        }
    }

    fn is_current(&self) -> bool {
        self.tracked_paths
            .iter()
            .all(PathFingerprint::matches_current)
    }
}

#[derive(Clone)]
struct CachedSkillsRegistry {
    registry: SkillsRegistry,
    tracked_paths: Vec<PathFingerprint>,
    listed_skills: Vec<SkillMetadata>,
    mentionable_skill_ids: BTreeSet<String>,
    explicit_skill_aliases: HashMap<String, String>,
    always_active_skill_ids: BTreeSet<String>,
    unavailable_skill_messages: HashMap<String, String>,
    skills_list: Option<String>,
    active_skill_cache: HashMap<String, CachedSkillRender>,
}

impl CachedSkillsRegistry {
    fn load_capability_view(
        capability_view: &ResolvedCapabilityView,
        host_capabilities: &SkillHostCapabilities,
    ) -> Result<Self, crate::skills::SkillsError> {
        let registry = SkillsRegistry::load_capability_view(capability_view)?;
        Self::from_registry(registry, host_capabilities)
    }

    fn from_registry(
        registry: SkillsRegistry,
        host_capabilities: &SkillHostCapabilities,
    ) -> Result<Self, crate::skills::SkillsError> {
        let mut listed_skills = Vec::new();
        let mut mentionable_skill_ids = BTreeSet::new();
        let mut explicit_skill_aliases = HashMap::new();
        let mut always_active_skill_ids = BTreeSet::new();
        let mut unavailable_skill_messages = HashMap::new();

        for skill in registry.list_sorted().into_iter().cloned() {
            let availability_issues = skill_availability_issues(&skill, host_capabilities);
            let explicit_aliases = skill
                .capabilities
                .as_ref()
                .map(|capabilities| capabilities.triggers.explicit.clone())
                .unwrap_or_default();
            if !availability_issues.is_empty() {
                let message = render_skill_unavailable(
                    &skill.id,
                    &format_skill_availability_issues(&availability_issues),
                );
                unavailable_skill_messages.insert(skill.id.clone(), message.clone());
                for alias in explicit_aliases {
                    let alias = name_to_id(&alias);
                    if alias.is_empty() {
                        continue;
                    }
                    unavailable_skill_messages
                        .entry(alias)
                        .or_insert_with(|| message.clone());
                }
                continue;
            }
            if skill.mount_mode.is_catalog_visible() {
                listed_skills.push(skill.clone());
            }
            if skill.mount_mode.allows_explicit_activation() {
                mentionable_skill_ids.insert(skill.id.clone());
                for alias in explicit_aliases {
                    let alias = name_to_id(&alias);
                    if alias.is_empty() {
                        continue;
                    }
                    explicit_skill_aliases
                        .entry(alias)
                        .or_insert_with(|| skill.id.clone());
                }
            }
            if skill.mount_mode.is_active_by_default() {
                always_active_skill_ids.insert(skill.id.clone());
            }
        }

        let skills_list = render_skills_list(&listed_skills);
        let tracked_paths = registry
            .tracked_paths()
            .iter()
            .cloned()
            .map(PathFingerprint::capture)
            .collect();
        Ok(Self {
            registry,
            tracked_paths,
            listed_skills,
            mentionable_skill_ids,
            explicit_skill_aliases,
            always_active_skill_ids,
            unavailable_skill_messages,
            skills_list,
            active_skill_cache: HashMap::new(),
        })
    }

    fn is_current(&self) -> bool {
        self.tracked_paths
            .iter()
            .all(PathFingerprint::matches_current)
    }

    fn resolve_explicit_mention(&self, mention: &str) -> Option<String> {
        if self.mentionable_skill_ids.contains(mention) {
            Some(mention.to_string())
        } else {
            self.explicit_skill_aliases.get(mention).cloned()
        }
    }

    fn render_domain_prompt(&mut self, user_input: Option<&[ContentPart]>) -> RenderedDomainPrompt {
        if !self.registry.errors().is_empty() {
            warn!(
                errors = self.registry.errors().len(),
                "Loaded skills with non-fatal parse/scan errors"
            );
        }

        let mut sections = Vec::new();
        let mut active_skills = Vec::new();
        if let Some(skills_list) = &self.skills_list {
            sections.push(skills_list.clone());
        }

        let mention_text = user_input.map(parts_to_text).unwrap_or_default();
        let mentioned_ids = extract_mentions(&mention_text);
        let mut active_skill_cache_hit = true;
        let mention_text_lower = mention_text.to_lowercase();

        let mut active_reasons = BTreeMap::new();
        for skill_id in &self.always_active_skill_ids {
            active_reasons.insert(skill_id.clone(), SkillActivationReason::AlwaysActiveMount);
        }
        for mention in &mentioned_ids {
            if let Some(skill_id) = self.resolve_explicit_mention(mention) {
                active_reasons.insert(
                    skill_id,
                    SkillActivationReason::ExplicitMention {
                        mention: mention.clone(),
                    },
                );
            }
        }
        for skill in &self.listed_skills {
            if active_reasons.contains_key(&skill.id) {
                continue;
            }
            if let Some(activation_reason) =
                match_declared_trigger(skill, &mention_text, &mention_text_lower)
            {
                active_reasons.insert(skill.id.clone(), activation_reason);
            }
        }

        let mut active_sections = Vec::new();
        for (skill_id, activation_reason) in active_reasons {
            let Some(metadata) = self.registry.get(&skill_id).cloned() else {
                continue;
            };
            let envelope = ActiveSkillEnvelope::available(metadata, activation_reason);
            match self.render_active_skill(&envelope) {
                Ok((rendered, cache_hit)) => {
                    active_skills.push(envelope);
                    active_sections.push(rendered);
                    active_skill_cache_hit &= cache_hit;
                }
                Err(err) => {
                    warn!(skill_id = %skill_id, error = %err, "Failed to load active skill");
                    active_skill_cache_hit = false;
                }
            }
        }

        if !active_sections.is_empty() {
            sections.push(
                "## Active Skill Instructions\nFollow these active skill instructions when relevant."
                    .to_string(),
            );
            sections.push(active_sections.join("\n\n"));
        }

        if !mentioned_ids.is_empty() {
            for mention in mentioned_ids {
                if self.resolve_explicit_mention(&mention).is_none() {
                    if let Some(message) = self.unavailable_skill_messages.get(&mention) {
                        sections.push(message.clone());
                    } else {
                        sections.push(render_skill_not_found(&mention, &self.listed_skills));
                    }
                }
            }
        }

        RenderedDomainPrompt {
            prompt: sections.join("\n\n"),
            active_skills,
            cache_hit: active_skill_cache_hit,
        }
    }

    fn render_active_skill(
        &mut self,
        envelope: &ActiveSkillEnvelope,
    ) -> Result<(String, bool), crate::skills::SkillsError> {
        let cache_key = envelope.cache_key();
        if let Some(cached) = self.active_skill_cache.get(&cache_key)
            && cached.is_current()
        {
            return Ok((cached.rendered.clone(), true));
        }

        let skill = self.registry.load_skill(&envelope.metadata.id)?;
        let cached = CachedSkillRender::load(&skill, envelope);
        let rendered = cached.rendered.clone();
        self.active_skill_cache.insert(cache_key, cached);
        Ok((rendered, false))
    }
}

fn match_declared_trigger(
    skill: &SkillMetadata,
    text: &str,
    text_lower: &str,
) -> Option<SkillActivationReason> {
    let triggers = &skill.capabilities.as_ref()?.triggers;
    if matches_trigger_keyword(text_lower, &triggers.negative_keywords).is_some() {
        return None;
    }

    if let Some(keyword) = matches_trigger_keyword(text_lower, &triggers.keywords) {
        return Some(SkillActivationReason::Keyword { keyword });
    }

    for pattern in &triggers.patterns {
        let regex = RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .ok()?;
        if regex.is_match(text) {
            return Some(SkillActivationReason::Pattern {
                pattern: pattern.clone(),
            });
        }
    }

    None
}

fn matches_trigger_keyword(text_lower: &str, keywords: &[String]) -> Option<String> {
    keywords
        .iter()
        .find(|keyword| text_lower.contains(&keyword.to_lowercase()))
        .cloned()
}

pub(crate) struct PromptAssemblyCache {
    fixed_capability_view: Option<ResolvedCapabilityView>,
    workspace_persona_dirs: Vec<PathBuf>,
    host_capabilities: SkillHostCapabilities,
    skills_snapshot: Option<CachedSkillsRegistry>,
    workspace_persona_snapshot: Option<CachedWorkspacePersona>,
    metrics: PromptAssemblyMetrics,
}

impl PromptAssemblyCache {
    #[cfg(test)]
    pub(crate) fn new(workspace_persona_dirs: Vec<PathBuf>) -> Self {
        Self {
            fixed_capability_view: None,
            workspace_persona_dirs,
            host_capabilities: SkillHostCapabilities::default(),
            skills_snapshot: None,
            workspace_persona_snapshot: None,
            metrics: PromptAssemblyMetrics::default(),
        }
    }

    pub(crate) fn with_fixed_capability_view(
        fixed_capability_view: ResolvedCapabilityView,
        workspace_persona_dirs: Vec<PathBuf>,
        host_capabilities: SkillHostCapabilities,
    ) -> Self {
        Self {
            fixed_capability_view: Some(fixed_capability_view),
            workspace_persona_dirs,
            host_capabilities,
            skills_snapshot: None,
            workspace_persona_snapshot: None,
            metrics: PromptAssemblyMetrics::default(),
        }
    }

    pub(crate) fn rebind_paths(&mut self, workspace_persona_dirs: Vec<PathBuf>) {
        if self.workspace_persona_dirs != workspace_persona_dirs {
            self.workspace_persona_dirs = workspace_persona_dirs;
            self.workspace_persona_snapshot = None;
        }
    }

    pub(crate) fn set_host_capabilities(&mut self, host_capabilities: SkillHostCapabilities) {
        if self.host_capabilities != host_capabilities {
            self.host_capabilities = host_capabilities;
            self.skills_snapshot = None;
        }
    }

    pub(crate) fn build(&mut self, user_input: Option<&[ContentPart]>) -> PromptAssemblyResult {
        let started_at = Instant::now();
        let (domain_prompt, active_skills, skills_cache_hit) =
            self.domain_prompt_with_cache(user_input);
        let (workspace_section, persona_cache_hit) = self.workspace_section_with_cache();
        let system_prompt = prompts::build_agent_system_prompt_with_workspace_context(
            &domain_prompt,
            workspace_section.as_deref(),
        );

        self.metrics
            .record_build(skills_cache_hit, persona_cache_hit);
        let elapsed_ms = started_at.elapsed().as_millis();
        debug!(
            elapsed_ms,
            skills_cache_hit,
            persona_cache_hit,
            builds = self.metrics.builds,
            hit_ratio = self.metrics.hit_ratio(),
            "Prompt assembly completed"
        );

        PromptAssemblyResult {
            domain_prompt,
            system_prompt,
            active_skills,
            metrics: self.metrics,
            elapsed_ms,
            skills_cache_hit,
            persona_cache_hit,
        }
    }

    fn domain_prompt_with_cache(
        &mut self,
        user_input: Option<&[ContentPart]>,
    ) -> (String, Vec<ActiveSkillEnvelope>, bool) {
        let Some(capability_view) = self.fixed_capability_view.as_ref() else {
            return (String::new(), Vec::new(), true);
        };

        let cache_hit = self
            .skills_snapshot
            .as_ref()
            .is_some_and(CachedSkillsRegistry::is_current);
        if !cache_hit {
            let load_result = CachedSkillsRegistry::load_capability_view(
                capability_view,
                &self.host_capabilities,
            );

            match load_result {
                Ok(snapshot) => {
                    self.skills_snapshot = Some(snapshot);
                }
                Err(err) => {
                    let path = capability_view
                        .package_dirs
                        .first()
                        .map(|dir| dir.path.display().to_string())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    warn!(path = %path, error = %err, "Failed to load skills registry; continuing without skill injection");
                    self.skills_snapshot = None;
                    return (String::new(), Vec::new(), false);
                }
            }
        }

        let rendered = self
            .skills_snapshot
            .as_mut()
            .map(|snapshot| snapshot.render_domain_prompt(user_input))
            .unwrap_or_else(|| RenderedDomainPrompt {
                prompt: String::new(),
                active_skills: Vec::new(),
                cache_hit: true,
            });
        (
            rendered.prompt,
            rendered.active_skills,
            cache_hit && rendered.cache_hit,
        )
    }

    fn workspace_section_with_cache(&mut self) -> (Option<String>, bool) {
        if self.workspace_persona_dirs.is_empty() {
            return (None, true);
        }
        if !self.workspace_persona_dirs.iter().any(|dir| dir.exists()) {
            self.workspace_persona_snapshot = None;
            return (None, true);
        }

        let cache_hit = self
            .workspace_persona_snapshot
            .as_ref()
            .is_some_and(CachedWorkspacePersona::is_current);
        if !cache_hit {
            self.workspace_persona_snapshot =
                Some(CachedWorkspacePersona::load(&self.workspace_persona_dirs));
        }

        let rendered = self
            .workspace_persona_snapshot
            .as_ref()
            .map(|snapshot| snapshot.rendered_section.clone())
            .filter(|section| !section.is_empty());
        (rendered, cache_hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::ensure_workspace_bootstrap_files_at;
    use crate::skills::{
        PackageMount, PackageMountMode, ScopedPackageDir, SkillHostCapabilities, SkillScope,
        default_builtin_package_mounts,
    };
    use sha2::{Digest, Sha256};

    fn create_repo_skill(
        workspace_root: &std::path::Path,
        dir_name: &str,
        skill_name: &str,
        description: &str,
        body: &str,
    ) {
        let skill_dir = workspace_root.join(".alan/agent/skills").join(dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!(
                r#"---
name: {skill_name}
description: {description}
---

{body}
"#
            ),
        )
        .unwrap();
    }

    fn create_repo_skill_with_frontmatter(
        workspace_root: &std::path::Path,
        dir_name: &str,
        frontmatter: &str,
        body: &str,
    ) {
        let skill_dir = workspace_root.join(".alan/agent/skills").join(dir_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\n{frontmatter}\n---\n\n{body}\n"),
        )
        .unwrap();
    }

    fn capability_view_for_workspace_root(
        workspace_root: &std::path::Path,
    ) -> ResolvedCapabilityView {
        let mut capability_view =
            ResolvedCapabilityView::from_package_dirs(vec![ScopedPackageDir {
                path: workspace_root.join(".alan/agent/skills"),
                scope: SkillScope::Repo,
            }])
            .with_default_mounts();
        capability_view.apply_mount_overrides(&default_builtin_package_mounts());
        capability_view
    }

    fn prompt_cache_for_workspace_root(
        workspace_root: &std::path::Path,
        workspace_persona_dirs: Vec<PathBuf>,
    ) -> PromptAssemblyCache {
        PromptAssemblyCache::with_fixed_capability_view(
            capability_view_for_workspace_root(workspace_root),
            workspace_persona_dirs,
            SkillHostCapabilities::default(),
        )
    }

    #[test]
    fn prompt_cache_hits_on_repeated_builds() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let persona_dir = workspace_root.join(".alan/agent/persona");
        std::fs::create_dir_all(&workspace_root).unwrap();
        ensure_workspace_bootstrap_files_at(&persona_dir).unwrap();
        create_repo_skill(
            &workspace_root,
            "my-skill",
            "My Skill",
            "Custom test skill",
            "# Instructions\nUse this skill when asked.",
        );

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, vec![persona_dir.clone()]);
        let user_input = vec![ContentPart::text("please use $my-skill for this task")];

        let first = cache.build(Some(&user_input));
        let second = cache.build(Some(&user_input));

        assert!(first.system_prompt.contains("Workspace Persona Context"));
        assert!(first.system_prompt.contains("## Skill: My Skill"));
        assert!(!first.skills_cache_hit);
        assert!(!first.persona_cache_hit);
        assert!(second.skills_cache_hit);
        assert!(second.persona_cache_hit);
        assert_eq!(second.metrics.builds, 2);
        assert_eq!(second.metrics.hits, 1);
    }

    #[test]
    fn hash_file_contents_matches_sha256_for_large_files() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("large.txt");
        let content = "0123456789abcdef".repeat(16 * 1024);
        std::fs::write(&path, &content).unwrap();

        let digest = hash_file_contents(&path, None).unwrap();
        let mut expected = Sha256::new();
        expected.update(content.as_bytes());

        assert_eq!(digest, <[u8; 32]>::from(expected.finalize()));
    }

    #[test]
    fn hash_file_contents_respects_prefix_limit() {
        let temp = tempfile::TempDir::new().unwrap();
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        std::fs::write(&first, "prefix-one-suffix-a").unwrap();
        std::fs::write(&second, "prefix-one-suffix-b").unwrap();

        assert_eq!(
            hash_file_contents(&first, Some(10)),
            hash_file_contents(&second, Some(10))
        );
        assert_ne!(
            hash_file_contents(&first, None),
            hash_file_contents(&second, None)
        );
    }

    #[test]
    fn prompt_cache_exposes_active_skill_envelopes_with_canonical_context() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill(
            &workspace_root,
            "my-skill",
            "My Skill",
            "Custom test skill",
            "# Instructions\nUse this skill when asked.",
        );

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please use $my-skill for this task")];

        let prompt = cache.build(Some(&user_input));
        assert_eq!(prompt.active_skills.len(), 1);

        let active_skill = &prompt.active_skills[0];
        let expected_root =
            std::fs::canonicalize(workspace_root.join(".alan/agent/skills/my-skill")).unwrap();
        assert_eq!(active_skill.metadata.id, "my-skill");
        assert_eq!(
            active_skill.metadata.package_id.as_deref(),
            Some("skill:my-skill")
        );
        assert_eq!(active_skill.metadata.path, expected_root.join("SKILL.md"));
        assert_eq!(
            active_skill.metadata.package_root.as_deref(),
            Some(expected_root.as_path())
        );
        assert_eq!(
            active_skill.metadata.resource_root.as_deref(),
            Some(expected_root.as_path())
        );
        assert!(matches!(
            active_skill.activation_reason,
            SkillActivationReason::ExplicitMention { .. }
        ));
        assert!(prompt.system_prompt.contains(&format!(
            "canonical_path: {}",
            expected_root.join("SKILL.md").display()
        )));
        assert!(
            prompt
                .system_prompt
                .contains(&format!("resource_root: {}", expected_root.display()))
        );
    }

    #[test]
    fn explicit_mention_overrides_always_active_activation_reason() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill(
            &workspace_root,
            "my-skill",
            "My Skill",
            "Custom test skill",
            "# Instructions\nUse this skill when asked.",
        );

        let mut capability_view = capability_view_for_workspace_root(&workspace_root);
        capability_view.apply_mount_overrides(&[PackageMount {
            package_id: "skill:my-skill".to_string(),
            mode: PackageMountMode::AlwaysActive,
        }]);
        let mut cache = PromptAssemblyCache::with_fixed_capability_view(
            capability_view,
            Vec::new(),
            SkillHostCapabilities::default(),
        );

        let mentioned = vec![ContentPart::text("please use $my-skill for this task")];
        let prompt = cache.build(Some(&mentioned));

        assert_eq!(prompt.active_skills.len(), 1);
        assert!(matches!(
            prompt.active_skills[0].activation_reason,
            SkillActivationReason::ExplicitMention { .. }
        ));
        assert!(
            prompt
                .system_prompt
                .contains("activation_reason: explicit_mention($my-skill)")
        );
    }

    #[test]
    fn explicit_alias_mention_activates_skill() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill_with_frontmatter(
            &workspace_root,
            "my-skill",
            r#"name: My Skill
description: Custom test skill
capabilities:
  triggers:
    explicit: ["Ship_It"]"#,
            "# Instructions\nUse this skill when asked.",
        );

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please use $ship-it for this task")];

        let prompt = cache.build(Some(&user_input));

        assert_eq!(prompt.active_skills.len(), 1);
        assert_eq!(prompt.active_skills[0].metadata.id, "my-skill");
        assert!(matches!(
            prompt.active_skills[0].activation_reason,
            SkillActivationReason::ExplicitMention { .. }
        ));
        assert!(
            prompt
                .system_prompt
                .contains("activation_reason: explicit_mention($ship-it)")
        );
    }

    #[test]
    fn explicit_alias_unavailable_messages_use_canonicalized_aliases() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let skill_dir = workspace_root.join(".alan/agent/skills/tool-heavy");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: Tool Heavy
description: Needs extra tools
capabilities:
  required_tools: ["missing_tool"]
  triggers:
    explicit: ["Ship_It"]
---

# Instructions
Use this skill when asked.
"#,
        )
        .unwrap();

        let mut cache = PromptAssemblyCache::with_fixed_capability_view(
            capability_view_for_workspace_root(&workspace_root),
            Vec::new(),
            SkillHostCapabilities::with_tools(["read_file"]).with_runtime_defaults(),
        );
        let mentioned = vec![ContentPart::text("please use $ship-it for this task")];
        let prompt = cache.build(Some(&mentioned));

        assert!(!prompt.system_prompt.contains("## Skill: Tool Heavy"));
        assert!(
            prompt
                .system_prompt
                .contains("Skill '$tool-heavy' is unavailable")
        );
        assert!(!prompt.system_prompt.contains("Skill '$ship-it' not found"));
        assert!(
            prompt
                .system_prompt
                .contains("missing required tools: missing_tool")
        );
    }

    #[test]
    fn keyword_trigger_activates_discoverable_skill() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill_with_frontmatter(
            &workspace_root,
            "my-skill",
            r#"name: My Skill
description: Custom test skill
capabilities:
  triggers:
    keywords: ["ship release"]"#,
            "# Instructions\nUse this skill when asked.",
        );

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please ship release for this workspace")];

        let prompt = cache.build(Some(&user_input));

        assert_eq!(prompt.active_skills.len(), 1);
        assert!(matches!(
            prompt.active_skills[0].activation_reason,
            SkillActivationReason::Keyword { .. }
        ));
        assert!(
            prompt
                .system_prompt
                .contains("activation_reason: keyword(ship release)")
        );
    }

    #[test]
    fn pattern_trigger_activates_discoverable_skill() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill_with_frontmatter(
            &workspace_root,
            "my-skill",
            r#"name: My Skill
description: Custom test skill
capabilities:
  triggers:
    patterns: ["PR\\s*#\\d+"]"#,
            "# Instructions\nUse this skill when asked.",
        );

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please prepare follow-up for PR #123")];

        let prompt = cache.build(Some(&user_input));

        assert_eq!(prompt.active_skills.len(), 1);
        assert!(matches!(
            prompt.active_skills[0].activation_reason,
            SkillActivationReason::Pattern { .. }
        ));
        assert!(
            prompt
                .system_prompt
                .contains(r#"activation_reason: pattern(PR\s*#\d+)"#)
        );
    }

    #[test]
    fn negative_keyword_suppresses_automatic_trigger_but_not_explicit_mention() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill_with_frontmatter(
            &workspace_root,
            "my-skill",
            r#"name: My Skill
description: Custom test skill
capabilities:
  triggers:
    keywords: ["release"]
    negative_keywords: ["dry run"]"#,
            "# Instructions\nUse this skill when asked.",
        );

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let suppressed = vec![ContentPart::text("do a release dry run first")];
        let explicit = vec![ContentPart::text(
            "do a release dry run first and use $my-skill",
        )];

        let suppressed_prompt = cache.build(Some(&suppressed));
        let explicit_prompt = cache.build(Some(&explicit));

        assert!(suppressed_prompt.active_skills.is_empty());
        assert_eq!(explicit_prompt.active_skills.len(), 1);
        assert!(matches!(
            explicit_prompt.active_skills[0].activation_reason,
            SkillActivationReason::ExplicitMention { .. }
        ));
    }

    #[test]
    fn prompt_cache_invalidates_when_skill_changes() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill(
            &workspace_root,
            "my-skill",
            "My Skill",
            "Custom test skill",
            "# Instructions\nUse this skill when asked.",
        );

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please use $my-skill for this task")];

        let first = cache.build(Some(&user_input));
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(
            workspace_root.join(".alan/agent/skills/my-skill/SKILL.md"),
            r#"---
name: My Skill
description: Custom test skill
---

# Instructions
Updated instructions.
"#,
        )
        .unwrap();
        let second = cache.build(Some(&user_input));

        assert!(first.system_prompt.contains("Use this skill when asked."));
        assert!(second.system_prompt.contains("Updated instructions."));
        assert!(!second.skills_cache_hit);
        assert_eq!(second.metrics.skills_misses, 2);
    }

    #[test]
    fn prompt_cache_invalidates_when_skill_contents_change_with_same_length() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();

        let initial = r#"---
name: My Skill
description: Custom test skill
---

# Instructions
ABCD
"#;
        let updated = r#"---
name: My Skill
description: Custom test skill
---

# Instructions
WXYZ
"#;
        assert_eq!(initial.len(), updated.len());

        let skill_path = workspace_root.join(".alan/agent/skills/my-skill/SKILL.md");
        std::fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        std::fs::write(&skill_path, initial).unwrap();

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please use $my-skill for this task")];

        let first = cache.build(Some(&user_input));
        std::fs::write(&skill_path, updated).unwrap();
        let second = cache.build(Some(&user_input));

        assert!(first.system_prompt.contains("# Instructions\nABCD"));
        assert!(second.system_prompt.contains("# Instructions\nWXYZ"));
        assert!(!second.skills_cache_hit);
    }

    #[test]
    fn prompt_cache_uses_disclosure_level2_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let skill_dir = workspace_root.join(".alan/agent/skills/my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: My Skill
description: Custom test skill
capabilities:
  disclosure:
    level2: details.md
---

# Instructions
Fallback instructions.
"#,
        )
        .unwrap();
        std::fs::write(skill_dir.join("details.md"), "Expanded instructions.").unwrap();

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please use $my-skill for this task")];

        let prompt = cache.build(Some(&user_input));

        assert!(prompt.system_prompt.contains("source: details.md"));
        assert!(prompt.system_prompt.contains("Expanded instructions."));
        assert!(!prompt.system_prompt.contains("Fallback instructions."));
    }

    #[test]
    fn prompt_cache_invalidates_when_disclosed_resource_changes() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let skill_dir = workspace_root.join(".alan/agent/skills/my-skill");
        let references_dir = skill_dir.join("references");
        std::fs::create_dir_all(&references_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: My Skill
description: Custom test skill
---

# Instructions
Read `references/guide.md` before acting.
"#,
        )
        .unwrap();

        let initial = "ALPHA";
        let updated = "OMEGA";
        assert_eq!(initial.len(), updated.len());
        std::fs::write(references_dir.join("guide.md"), initial).unwrap();

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please use $my-skill for this task")];

        let first = cache.build(Some(&user_input));
        std::fs::write(references_dir.join("guide.md"), updated).unwrap();
        let second = cache.build(Some(&user_input));

        assert!(first.system_prompt.contains("ALPHA"));
        assert!(second.system_prompt.contains("OMEGA"));
        assert!(!second.skills_cache_hit);
    }

    #[cfg(unix)]
    #[test]
    fn prompt_cache_invalidates_when_skill_symlink_is_retargeted() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        let skills_root = workspace_root.join(".alan/agent/skills");
        let pack_v1 = temp.path().join("pack-v1");
        let pack_v2 = temp.path().join("pack-v2");
        let linked_pack = skills_root.join("linked-pack");

        std::fs::create_dir_all(&skills_root).unwrap();
        std::fs::create_dir_all(pack_v1.join("my-skill")).unwrap();
        std::fs::create_dir_all(pack_v2.join("my-skill")).unwrap();
        std::fs::write(
            pack_v1.join("my-skill/SKILL.md"),
            r#"---
name: My Skill
description: Custom test skill
---

# Instructions
Version one.
"#,
        )
        .unwrap();
        std::fs::write(
            pack_v2.join("my-skill/SKILL.md"),
            r#"---
name: My Skill
description: Custom test skill
---

# Instructions
Version two.
"#,
        )
        .unwrap();
        symlink(&pack_v1, &linked_pack).unwrap();

        let mut cache = prompt_cache_for_workspace_root(&workspace_root, Vec::new());
        let user_input = vec![ContentPart::text("please use $my-skill for this task")];

        let first = cache.build(Some(&user_input));
        std::fs::remove_file(&linked_pack).unwrap();
        symlink(&pack_v2, &linked_pack).unwrap();
        let second = cache.build(Some(&user_input));

        assert!(first.system_prompt.contains("Version one."));
        assert!(second.system_prompt.contains("Version two."));
        assert!(!second.skills_cache_hit);
    }

    #[test]
    fn explicit_only_skills_are_mentionable_but_not_listed() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill(
            &workspace_root,
            "my-skill",
            "My Skill",
            "Custom test skill",
            "# Instructions\nUse this skill when asked.",
        );

        let mut capability_view = capability_view_for_workspace_root(&workspace_root);
        capability_view.apply_mount_overrides(&[PackageMount {
            package_id: "skill:my-skill".to_string(),
            mode: PackageMountMode::ExplicitOnly,
        }]);
        let mut cache = PromptAssemblyCache::with_fixed_capability_view(
            capability_view,
            Vec::new(),
            SkillHostCapabilities::default(),
        );

        let mentioned = vec![ContentPart::text("please use $my-skill for this task")];
        let prompt = cache.build(Some(&mentioned));
        assert!(!prompt.system_prompt.contains("**My Skill** ($my-skill)"));
        assert!(prompt.system_prompt.contains("## Skill: My Skill"));
    }

    #[test]
    fn internal_skills_are_hidden_from_catalog_and_activation() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        create_repo_skill(
            &workspace_root,
            "my-skill",
            "My Skill",
            "Custom test skill",
            "# Instructions\nUse this skill when asked.",
        );

        let mut capability_view = capability_view_for_workspace_root(&workspace_root);
        capability_view.apply_mount_overrides(&[PackageMount {
            package_id: "skill:my-skill".to_string(),
            mode: PackageMountMode::Internal,
        }]);
        let mut cache = PromptAssemblyCache::with_fixed_capability_view(
            capability_view,
            Vec::new(),
            SkillHostCapabilities::default(),
        );

        let mentioned = vec![ContentPart::text("please use $my-skill for this task")];
        let prompt = cache.build(Some(&mentioned));
        assert!(!prompt.system_prompt.contains("**My Skill** ($my-skill)"));
        assert!(!prompt.system_prompt.contains("## Skill: My Skill"));
        assert!(prompt.system_prompt.contains("Skill '$my-skill' not found"));
    }

    #[test]
    fn internal_skills_with_missing_tools_still_render_as_not_found() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let skill_dir = workspace_root.join(".alan/agent/skills/hidden-helper");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: Hidden Helper
description: Should stay hidden
capabilities:
  required_tools: ["missing_tool"]
---

# Instructions
Use this skill when asked.
"#,
        )
        .unwrap();

        let mut capability_view = capability_view_for_workspace_root(&workspace_root);
        capability_view.apply_mount_overrides(&[PackageMount {
            package_id: "skill:hidden-helper".to_string(),
            mode: PackageMountMode::Internal,
        }]);
        let mut cache = PromptAssemblyCache::with_fixed_capability_view(
            capability_view,
            Vec::new(),
            SkillHostCapabilities::with_tools(["read_file"]).with_runtime_defaults(),
        );

        let mentioned = vec![ContentPart::text("please use $hidden-helper for this task")];
        let prompt = cache.build(Some(&mentioned));

        assert!(!prompt.system_prompt.contains("## Skill: Hidden Helper"));
        assert!(
            !prompt
                .system_prompt
                .contains("Skill '$hidden-helper' is unavailable")
        );
        assert!(
            prompt
                .system_prompt
                .contains("Skill '$hidden-helper' not found")
        );
    }

    #[test]
    fn skills_with_missing_required_tools_are_reported_as_unavailable() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let skill_dir = workspace_root.join(".alan/agent/skills/tool-heavy");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: Tool Heavy
description: Needs extra tools
capabilities:
  required_tools: ["missing_tool"]
---

# Instructions
Use this skill when asked.
"#,
        )
        .unwrap();

        let mut cache = PromptAssemblyCache::with_fixed_capability_view(
            capability_view_for_workspace_root(&workspace_root),
            Vec::new(),
            SkillHostCapabilities::with_tools(["read_file"]).with_runtime_defaults(),
        );
        let mentioned = vec![ContentPart::text("please use $tool-heavy for this task")];
        let prompt = cache.build(Some(&mentioned));

        assert!(!prompt.system_prompt.contains("## Skill: Tool Heavy"));
        assert!(
            prompt
                .system_prompt
                .contains("Skill '$tool-heavy' is unavailable")
        );
        assert!(
            prompt
                .system_prompt
                .contains("missing required tools: missing_tool")
        );
    }

    #[test]
    fn prompt_cache_invalidates_when_host_capabilities_change() {
        let temp = tempfile::TempDir::new().unwrap();
        let workspace_root = temp.path().join("repo");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let skill_dir = workspace_root.join(".alan/agent/skills/dynamic-helper");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: Dynamic Helper
description: Needs a dynamic tool
capabilities:
  required_tools: ["custom_tool"]
---

# Instructions
Use this skill when asked.
"#,
        )
        .unwrap();

        let mut cache = PromptAssemblyCache::with_fixed_capability_view(
            capability_view_for_workspace_root(&workspace_root),
            Vec::new(),
            SkillHostCapabilities::with_tools(["read_file"]).with_runtime_defaults(),
        );
        let mentioned = vec![ContentPart::text(
            "please use $dynamic-helper for this task",
        )];

        let before = cache.build(Some(&mentioned));
        assert!(
            before
                .system_prompt
                .contains("Skill '$dynamic-helper' is unavailable")
        );

        let mut refreshed =
            SkillHostCapabilities::with_tools(["read_file"]).with_runtime_defaults();
        refreshed.extend_tools(["custom_tool"]);
        cache.set_host_capabilities(refreshed);

        let after = cache.build(Some(&mentioned));
        assert!(after.system_prompt.contains("## Skill: Dynamic Helper"));
        assert!(
            !after
                .system_prompt
                .contains("Skill '$dynamic-helper' is unavailable")
        );
    }
}
