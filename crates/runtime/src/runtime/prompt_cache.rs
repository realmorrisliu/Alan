use crate::prompts;
use crate::skills::{
    ResolvedCapabilityView, Skill, SkillContentSource, SkillHostCapabilities, SkillMetadata,
    SkillsRegistry, extract_mentions, format_skill_availability_issues, inject_skills,
    render_skill_not_found, render_skill_unavailable, render_skills_list,
    skill_availability_issues,
};
use crate::tape::{ContentPart, parts_to_text};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::fs::Metadata;
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
    pub metrics: PromptAssemblyMetrics,
    pub elapsed_ms: u128,
    pub skills_cache_hit: bool,
    pub persona_cache_hit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathFingerprint {
    path: PathBuf,
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
    fn capture(path: &Path, metadata: &Metadata, include_content_digest: bool) -> Self {
        Self {
            modified: metadata.modified().ok(),
            len: metadata.len(),
            content_digest: include_content_digest
                .then(|| hash_file_contents(path))
                .flatten(),
            platform: PlatformFingerprint::capture(metadata),
        }
    }
}

impl PathFingerprint {
    fn capture(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let state = match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                PathState::File(MetadataFingerprint::capture(&path, &metadata, true))
            }
            Ok(metadata) if metadata.is_dir() => {
                PathState::Directory(MetadataFingerprint::capture(&path, &metadata, false))
            }
            Ok(metadata) => PathState::Other(MetadataFingerprint::capture(&path, &metadata, false)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => PathState::Missing,
            Err(_) => PathState::Missing,
        };
        Self { path, state }
    }

    fn matches_current(&self) -> bool {
        Self::capture(self.path.clone()) == *self
    }
}

fn hash_file_contents(path: &Path) -> Option<[u8; 32]> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
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
    fn load(skill: &Skill) -> Self {
        let tracked_paths = skill_prompt_tracked_paths(skill)
            .into_iter()
            .map(PathFingerprint::capture)
            .collect();
        Self {
            tracked_paths,
            rendered: inject_skills(std::slice::from_ref(skill)),
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
        let mut always_active_skill_ids = BTreeSet::new();
        let mut unavailable_skill_messages = HashMap::new();

        for skill in registry.list_sorted().into_iter().cloned() {
            let availability_issues = skill_availability_issues(&skill, host_capabilities);
            if !availability_issues.is_empty() {
                unavailable_skill_messages.insert(
                    skill.id.clone(),
                    render_skill_unavailable(
                        &skill.id,
                        &format_skill_availability_issues(&availability_issues),
                    ),
                );
                continue;
            }
            if skill.mount_mode.is_catalog_visible() {
                listed_skills.push(skill.clone());
            }
            if skill.mount_mode.allows_explicit_activation() {
                mentionable_skill_ids.insert(skill.id.clone());
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

    fn render_domain_prompt(&mut self, user_input: Option<&[ContentPart]>) -> String {
        if !self.registry.errors().is_empty() {
            warn!(
                errors = self.registry.errors().len(),
                "Loaded skills with non-fatal parse/scan errors"
            );
        }

        let mut sections = Vec::new();
        if let Some(skills_list) = &self.skills_list {
            sections.push(skills_list.clone());
        }

        let mention_text = user_input.map(parts_to_text).unwrap_or_default();
        let mentioned_ids = extract_mentions(&mention_text);

        let mut active_ids = self.always_active_skill_ids.clone();
        for mention in &mentioned_ids {
            if self.mentionable_skill_ids.contains(mention) {
                active_ids.insert(mention.clone());
            }
        }

        let mut active_sections = Vec::new();
        for skill_id in &active_ids {
            match self.render_active_skill(skill_id) {
                Ok(rendered) => active_sections.push(rendered),
                Err(err) => {
                    warn!(skill_id = %skill_id, error = %err, "Failed to load active skill");
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
                if !self.mentionable_skill_ids.contains(mention.as_str()) {
                    if let Some(message) = self.unavailable_skill_messages.get(&mention) {
                        sections.push(message.clone());
                    } else {
                        sections.push(render_skill_not_found(&mention, &self.listed_skills));
                    }
                }
            }
        }

        sections.join("\n\n")
    }

    fn render_active_skill(
        &mut self,
        skill_id: &str,
    ) -> Result<String, crate::skills::SkillsError> {
        if let Some(cached) = self.active_skill_cache.get(skill_id)
            && cached.is_current()
        {
            return Ok(cached.rendered.clone());
        }

        let skill = self.registry.load_skill(&skill_id.to_string())?;
        let cached = CachedSkillRender::load(&skill);
        let rendered = cached.rendered.clone();
        self.active_skill_cache.insert(skill_id.to_string(), cached);
        Ok(rendered)
    }
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
        let (domain_prompt, skills_cache_hit) = self.domain_prompt_with_cache(user_input);
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
            metrics: self.metrics,
            elapsed_ms,
            skills_cache_hit,
            persona_cache_hit,
        }
    }

    fn domain_prompt_with_cache(&mut self, user_input: Option<&[ContentPart]>) -> (String, bool) {
        let Some(capability_view) = self.fixed_capability_view.as_ref() else {
            return (String::new(), true);
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
                    return (String::new(), false);
                }
            }
        }

        let domain_prompt = self
            .skills_snapshot
            .as_mut()
            .map(|snapshot| snapshot.render_domain_prompt(user_input))
            .unwrap_or_default();
        (domain_prompt, cache_hit)
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

fn skill_prompt_tracked_paths(skill: &Skill) -> Vec<PathBuf> {
    if matches!(skill.metadata.source, SkillContentSource::Embedded(_)) {
        return Vec::new();
    }

    let mut paths = vec![skill.metadata.path.clone()];
    if let Some(skill_dir) = skill.metadata.path.parent() {
        paths.push(skill_dir.join("scripts"));
        paths.push(skill_dir.join("references"));
        paths.push(skill_dir.join("assets"));
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::ensure_workspace_bootstrap_files_at;
    use crate::skills::{
        PackageMount, PackageMountMode, ScopedPackageDir, SkillHostCapabilities, SkillScope,
        default_builtin_package_mounts,
    };

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
