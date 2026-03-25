//! Skills injector for adding skill content to prompts.

use crate::skills::types::*;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

const MAX_DISCLOSED_RESOURCE_COUNT: usize = 8;
const MAX_DISCLOSED_RESOURCE_CHARS: usize = 4000;
const MAX_DISCLOSED_RESOURCE_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone)]
pub struct RenderedActiveSkillPrompt {
    pub rendered: String,
    pub tracked_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct DisclosedSkillPrompt {
    level2: DisclosedLevel2Content,
    resources: Vec<DisclosedSkillResource>,
}

#[derive(Debug, Clone)]
struct DisclosedLevel2Content {
    source_display: String,
    body: String,
    tracked_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct DisclosedSkillResource {
    kind: SkillResourceKind,
    display_path: String,
    tracked_path: PathBuf,
    content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SkillResourceKind {
    Reference,
    Script,
    Asset,
}

impl SkillResourceKind {
    fn label(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::Script => "script",
            Self::Asset => "asset",
        }
    }

    fn default_dir(self) -> &'static str {
        match self {
            Self::Reference => "references",
            Self::Script => "scripts",
            Self::Asset => "assets",
        }
    }
}

/// Extract skill mentions from user input.
/// Supports `$skill-name` or `$skill_name` format.
pub fn extract_mentions(input: &str) -> Vec<SkillId> {
    let mut mentions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '$' {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < chars.len() {
            let c = chars[j];
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                j += 1;
            } else {
                break;
            }
        }

        if j > i + 1 {
            let raw: String = chars[i + 1..j].iter().collect();
            let id = name_to_id(&raw);
            if seen.insert(id.clone()) {
                mentions.push(id);
            }
        }

        i = j;
    }

    mentions
}

/// Inject skill content into a prompt.
pub fn inject_skills(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut sections = Vec::new();

    for skill in skills {
        let envelope = ActiveSkillEnvelope::available(
            skill.metadata.clone(),
            SkillActivationReason::AlwaysActiveMount,
        );
        sections.push(inject_active_skill(skill, &envelope));
    }

    sections.join("\n\n")
}

/// Inject one active skill using the structured runtime envelope.
pub fn inject_active_skill(skill: &Skill, envelope: &ActiveSkillEnvelope) -> String {
    render_active_skill_prompt(skill, envelope).rendered
}

/// Render one active skill prompt together with the exact files it depends on.
pub fn render_active_skill_prompt(
    skill: &Skill,
    envelope: &ActiveSkillEnvelope,
) -> RenderedActiveSkillPrompt {
    let runtime_context = format_active_skill_context(envelope);
    let disclosed = disclose_skill_prompt(skill, envelope);
    let resources = format_disclosed_resources(&disclosed.resources);
    let rendered = format!(
        r#"## Skill: {}

{runtime_context}

### Active Instructions
source: {}

{}

{resources}

---"#,
        skill.metadata.name,
        disclosed.level2.source_display,
        disclosed.level2.body,
        runtime_context = runtime_context,
        resources = resources
    );

    let mut tracked_paths = disclosed.level2.tracked_paths.clone();
    tracked_paths.extend(
        disclosed
            .resources
            .iter()
            .map(|resource| resource.tracked_path.clone()),
    );
    dedupe_paths(&mut tracked_paths);

    RenderedActiveSkillPrompt {
        rendered,
        tracked_paths,
    }
}

fn format_active_skill_context(envelope: &ActiveSkillEnvelope) -> String {
    let mut lines = vec![
        "### Alan Runtime Context".to_string(),
        format!("skill_id: {}", envelope.metadata.id),
        format!(
            "package_id: {}",
            envelope.metadata.package_id.as_deref().unwrap_or("<none>")
        ),
        format!(
            "mount_mode: {}",
            render_mount_mode(envelope.metadata.mount_mode)
        ),
        format!("canonical_path: {}", envelope.metadata.path.display()),
        format!(
            "package_root: {}",
            render_optional_path(envelope.metadata.package_root())
        ),
        format!(
            "resource_root: {}",
            render_optional_path(envelope.metadata.resource_root())
        ),
        format!("availability: {}", envelope.availability.render_label()),
        format!(
            "activation_reason: {}",
            envelope.activation_reason.render_label()
        ),
    ];

    if envelope.metadata.resource_root().is_some() {
        lines.push(
            "Resolve relative skill resource references against `resource_root`.".to_string(),
        );
    }

    lines.join("\n")
}

fn disclose_skill_prompt(skill: &Skill, envelope: &ActiveSkillEnvelope) -> DisclosedSkillPrompt {
    let disclosure = skill_disclosure_config(skill);
    let base_dir = disclosure_base_dir(skill, envelope);
    let level2 = load_level2_content(skill, &disclosure, base_dir.as_deref());
    let resources = collect_disclosed_resources(&level2.body, &disclosure, base_dir.as_deref());

    DisclosedSkillPrompt { level2, resources }
}

fn skill_disclosure_config(skill: &Skill) -> DisclosureConfig {
    skill
        .metadata
        .capabilities
        .as_ref()
        .map(|capabilities| capabilities.disclosure.clone())
        .unwrap_or_else(|| skill.frontmatter.capabilities.disclosure.clone())
}

fn disclosure_base_dir(skill: &Skill, envelope: &ActiveSkillEnvelope) -> Option<PathBuf> {
    envelope
        .metadata
        .resource_root()
        .or_else(|| skill.metadata.path.parent())
        .map(Path::to_path_buf)
}

fn load_level2_content(
    skill: &Skill,
    disclosure: &DisclosureConfig,
    base_dir: Option<&Path>,
) -> DisclosedLevel2Content {
    let mut tracked_paths = vec![skill.metadata.path.clone()];

    let requested = disclosure.level2.trim();
    if requested.is_empty() || requested == "SKILL.md" {
        return fallback_level2_content(skill, tracked_paths);
    }

    let Some(base_dir) = base_dir else {
        return fallback_level2_content(skill, tracked_paths);
    };
    let Some((display_path, path)) = resolve_relative_path(base_dir, requested) else {
        return fallback_level2_content(skill, tracked_paths);
    };

    tracked_paths.push(path.clone());
    if path == skill.metadata.path {
        return DisclosedLevel2Content {
            source_display: display_path,
            body: skill.content.clone(),
            tracked_paths,
        };
    }

    let Ok(content) = std::fs::read_to_string(&path) else {
        return fallback_level2_content(skill, tracked_paths);
    };

    DisclosedLevel2Content {
        source_display: display_path,
        body: strip_frontmatter_if_present(content),
        tracked_paths,
    }
}

fn fallback_level2_content(skill: &Skill, tracked_paths: Vec<PathBuf>) -> DisclosedLevel2Content {
    DisclosedLevel2Content {
        source_display: "SKILL.md".to_string(),
        body: skill.content.clone(),
        tracked_paths,
    }
}

fn collect_disclosed_resources(
    level2_body: &str,
    disclosure: &DisclosureConfig,
    base_dir: Option<&Path>,
) -> Vec<DisclosedSkillResource> {
    let Some(base_dir) = base_dir else {
        return Vec::new();
    };

    let mut resources = BTreeMap::new();

    for entry in &disclosure.level3.references {
        add_declared_resource(
            &mut resources,
            base_dir,
            SkillResourceKind::Reference,
            entry,
        );
    }
    for entry in &disclosure.level3.scripts {
        add_declared_resource(&mut resources, base_dir, SkillResourceKind::Script, entry);
    }
    for entry in &disclosure.level3.assets {
        add_declared_resource(&mut resources, base_dir, SkillResourceKind::Asset, entry);
    }

    for reference in extract_resource_references(level2_body) {
        add_prefixed_resource(&mut resources, base_dir, &reference);
    }

    resources
        .into_values()
        .take(MAX_DISCLOSED_RESOURCE_COUNT)
        .collect()
}

fn add_declared_resource(
    resources: &mut BTreeMap<String, DisclosedSkillResource>,
    base_dir: &Path,
    kind: SkillResourceKind,
    entry: &str,
) {
    let Some((display_path, path)) = resolve_resource_entry(base_dir, kind, entry) else {
        return;
    };
    resources
        .entry(display_path.clone())
        .or_insert_with(|| DisclosedSkillResource {
            kind,
            display_path,
            tracked_path: path.clone(),
            content: load_resource_content(&path),
        });
}

fn add_prefixed_resource(
    resources: &mut BTreeMap<String, DisclosedSkillResource>,
    base_dir: &Path,
    entry: &str,
) {
    let Some((kind, display_path, path)) = resolve_prefixed_resource_entry(base_dir, entry) else {
        return;
    };
    resources
        .entry(display_path.clone())
        .or_insert_with(|| DisclosedSkillResource {
            kind,
            display_path,
            tracked_path: path.clone(),
            content: load_resource_content(&path),
        });
}

fn resolve_resource_entry(
    base_dir: &Path,
    kind: SkillResourceKind,
    entry: &str,
) -> Option<(String, PathBuf)> {
    let relative = sanitize_relative_path(entry)?;
    let relative = if relative.starts_with(kind.default_dir()) {
        relative
    } else {
        PathBuf::from(kind.default_dir()).join(relative)
    };
    let display_path = relative.display().to_string();
    let path = resolve_relative_under_root(base_dir, &relative)?;
    Some((display_path, path))
}

fn resolve_prefixed_resource_entry(
    base_dir: &Path,
    entry: &str,
) -> Option<(SkillResourceKind, String, PathBuf)> {
    let relative = sanitize_relative_path(entry)?;
    let first = relative.components().next()?.as_os_str().to_str()?;
    let kind = match first {
        "references" => SkillResourceKind::Reference,
        "scripts" => SkillResourceKind::Script,
        "assets" => SkillResourceKind::Asset,
        _ => return None,
    };
    let display_path = relative.display().to_string();
    let path = resolve_relative_under_root(base_dir, &relative)?;
    Some((kind, display_path, path))
}

fn resolve_relative_path(base_dir: &Path, entry: &str) -> Option<(String, PathBuf)> {
    let relative = sanitize_relative_path(entry)?;
    let display_path = relative.display().to_string();
    let path = resolve_relative_under_root(base_dir, &relative)?;
    Some((display_path, path))
}

fn resolve_relative_under_root(root: &Path, relative: &Path) -> Option<PathBuf> {
    let candidate = root.join(relative);
    let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    match std::fs::canonicalize(&candidate) {
        Ok(path) if path.starts_with(&canonical_root) => Some(path),
        Ok(_) => None,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Some(candidate),
        Err(_) => None,
    }
}

fn sanitize_relative_path(entry: &str) -> Option<PathBuf> {
    let trimmed = entry.trim().trim_matches(|c| matches!(c, '`' | '"' | '\''));
    let trimmed = trimmed.split(['#', '?']).next()?.trim();
    if trimmed.is_empty() || trimmed.contains("://") {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    (!normalized.as_os_str().is_empty()).then_some(normalized)
}

fn extract_resource_references(content: &str) -> Vec<String> {
    static RESOURCE_REF_RE: OnceLock<Regex> = OnceLock::new();
    let regex = RESOURCE_REF_RE.get_or_init(|| {
        Regex::new(r"(references|scripts|assets)/[A-Za-z0-9][A-Za-z0-9._/\-]*").unwrap()
    });

    let mut references = BTreeSet::new();
    for capture in regex.find_iter(content) {
        references.insert(capture.as_str().to_string());
    }
    references.into_iter().collect()
}

fn load_resource_content(path: &Path) -> Option<String> {
    let mut reader = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    let bytes_read = reader
        .by_ref()
        .take(MAX_DISCLOSED_RESOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    let truncated_by_bytes = bytes_read as u64 > MAX_DISCLOSED_RESOURCE_BYTES;
    if truncated_by_bytes {
        bytes.truncate(MAX_DISCLOSED_RESOURCE_BYTES as usize);
    }

    let valid_len = match std::str::from_utf8(&bytes) {
        Ok(_) => bytes.len(),
        Err(err) if truncated_by_bytes && err.error_len().is_none() && err.valid_up_to() > 0 => {
            err.valid_up_to()
        }
        Err(_) => return None,
    };
    bytes.truncate(valid_len);

    let content = String::from_utf8(bytes).ok()?;
    Some(truncate_resource_content(content, truncated_by_bytes))
}

fn truncate_resource_content(content: String, truncated_by_bytes: bool) -> String {
    let total_chars = content.chars().count();
    if !truncated_by_bytes && total_chars <= MAX_DISCLOSED_RESOURCE_CHARS {
        return content;
    }

    let truncated: String = content.chars().take(MAX_DISCLOSED_RESOURCE_CHARS).collect();
    let visible_chars = truncated.chars().count();
    let notice = if truncated_by_bytes {
        format!(
            "truncated after {visible_chars} chars from a file that exceeded {MAX_DISCLOSED_RESOURCE_BYTES} bytes"
        )
    } else {
        format!("truncated after {visible_chars} chars from {total_chars}")
    };
    format!("{truncated}\n...[{notice}]")
}

fn strip_frontmatter_if_present(content: String) -> String {
    extract_frontmatter(&content)
        .map(|(_, body)| body)
        .unwrap_or(content)
}

fn format_disclosed_resources(resources: &[DisclosedSkillResource]) -> String {
    if resources.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        "### Disclosed Resources".to_string(),
        "Only resources referenced by the active instructions or declared disclosure metadata are expanded here.".to_string(),
    ];

    for resource in resources {
        lines.push(format!(
            "#### {}: {}",
            resource.kind.label(),
            resource.display_path
        ));
        if let Some(content) = resource.content.as_ref() {
            lines.push(render_fenced_resource_content(
                &resource.display_path,
                content,
            ));
        } else {
            lines.push(
                "Binary or unreadable resource; resolve it from `resource_root` if deeper inspection is needed."
                    .to_string(),
            );
        }
    }

    lines.join("\n")
}

fn guess_code_fence(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("md") => "md",
        Some("rs") => "rust",
        Some("sh") => "bash",
        Some("py") => "python",
        Some("json") => "json",
        Some("yaml" | "yml") => "yaml",
        Some("toml") => "toml",
        Some("js") => "javascript",
        Some("ts") => "typescript",
        Some("html") => "html",
        Some("css") => "css",
        _ => "text",
    }
}

fn render_fenced_resource_content(path: &str, content: &str) -> String {
    let language = guess_code_fence(path);
    let fence = "`".repeat(fence_length(content));
    format!("{fence}{language}\n{content}\n{fence}")
}

fn fence_length(content: &str) -> usize {
    longest_backtick_run(content).max(3) + 1
}

fn longest_backtick_run(content: &str) -> usize {
    let mut longest = 0;
    let mut current = 0;

    for ch in content.chars() {
        if ch == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }

    longest
}

fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert(path.clone()));
}

fn render_optional_path(path: Option<&Path>) -> String {
    path.map(|value| value.display().to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

fn render_mount_mode(mode: PackageMountMode) -> &'static str {
    match mode {
        PackageMountMode::AlwaysActive => "always_active",
        PackageMountMode::Discoverable => "discoverable",
        PackageMountMode::ExplicitOnly => "explicit_only",
        PackageMountMode::Internal => "internal",
    }
}

/// Build a prompt with injected skills.
pub fn build_prompt_with_skills(user_input: &str, skills: &[Skill]) -> String {
    if skills.is_empty() {
        return user_input.to_string();
    }

    let skill_context = inject_skills(skills);

    format!(
        r#"{skill_context}

## User Request

{user_input}"#
    )
}

/// Render a list of available skills for the system prompt.
/// This helps the LLM know which skills are available for automatic triggering.
pub fn render_skills_list(skills: &[SkillMetadata]) -> Option<String> {
    if skills.is_empty() {
        return None;
    }

    let mut lines = vec![
        "## Available Skills".to_string(),
        "You have access to the following skills. When appropriate, you can use them by referencing their trigger word ($skill-name) or let the user know they are available:".to_string(),
        String::new(),
    ];

    for skill in skills {
        let desc = skill
            .short_description
            .as_ref()
            .unwrap_or(&skill.description);
        lines.push(format!("- **{}** (${}): {}", skill.name, skill.id, desc));
    }

    lines.push(String::new());
    lines.push(
        "When you decide to use a skill, announce it briefly and follow its instructions."
            .to_string(),
    );

    Some(lines.join("\n"))
}

/// Render a skill not found message.
pub fn render_skill_not_found(mention: &str, available: &[SkillMetadata]) -> String {
    let mut msg = format!("Skill '${}' not found. ", mention);

    // Suggest similar skills
    let similar: Vec<_> = available
        .iter()
        .filter(|s| s.id.contains(mention) || mention.contains(&s.id))
        .take(3)
        .collect();

    if !similar.is_empty() {
        msg.push_str("Did you mean: ");
        let names: Vec<_> = similar.iter().map(|s| format!("${}", s.id)).collect();
        msg.push_str(&names.join(", "));
        msg.push('?');
    } else {
        msg.push_str("Use `/skills` to see available skills.");
    }

    msg
}

/// Render a skill unavailable message with concrete host/runtime requirements.
pub fn render_skill_unavailable(mention: &str, reasons: &str) -> String {
    format!("Skill '${mention}' is unavailable in this runtime: {reasons}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mentions() {
        assert_eq!(
            extract_mentions("Use $test-skill for testing"),
            vec!["test-skill"]
        );

        assert_eq!(extract_mentions("Run $my_skill please"), vec!["my-skill"]);

        // Multiple mentions
        let mentions = extract_mentions("$skill-a and $skill-b");
        assert_eq!(mentions, vec!["skill-a", "skill-b"]);

        // With punctuation at end
        assert_eq!(extract_mentions("Use $skill-name."), vec!["skill-name"]);

        // No mentions
        assert!(extract_mentions("Plain text without mentions").is_empty());
    }

    #[test]
    fn test_inject_skills() {
        let skill = Skill {
            metadata: SkillMetadata {
                id: "test".to_string(),
                package_id: None,
                name: "Test Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/tmp/test/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/tmp/test/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
            content: "# Instructions\n\nDo this and that.".to_string(),
            frontmatter: SkillFrontmatter {
                name: "Test Skill".to_string(),
                description: "A test".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let injected = inject_skills(&[skill]);
        assert!(injected.contains("## Skill: Test Skill"));
        assert!(injected.contains("# Instructions"));
    }

    #[test]
    fn test_build_prompt_with_skills() {
        let skill = Skill {
            metadata: SkillMetadata {
                id: "eval".to_string(),
                package_id: None,
                name: "Evaluation".to_string(),
                description: "Eval".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/tmp/eval/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/tmp/eval/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
            content: "Follow these steps.".to_string(),
            frontmatter: SkillFrontmatter {
                name: "Evaluation".to_string(),
                description: "Eval".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let prompt = build_prompt_with_skills("Evaluate this", &[skill]);
        assert!(prompt.contains("Follow these steps"));
        assert!(prompt.contains("Evaluate this"));
    }

    #[test]
    fn test_render_skills_list() {
        let skills = vec![
            SkillMetadata {
                id: "skill-a".to_string(),
                package_id: None,
                name: "Skill A".to_string(),
                description: "Does A".to_string(),
                short_description: Some("Short A".to_string()),
                path: std::path::PathBuf::from("/a/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/a/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
            SkillMetadata {
                id: "skill-b".to_string(),
                package_id: None,
                name: "Skill B".to_string(),
                description: "Does B".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/b/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::Repo,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/b/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
        ];

        let list = render_skills_list(&skills).unwrap();
        assert!(list.contains("**Skill A** ($skill-a): Short A"));
        assert!(list.contains("**Skill B** ($skill-b): Does B"));
        assert!(list.contains("Available Skills"));
    }

    #[test]
    fn test_extract_mentions_edge_cases() {
        // Empty input
        assert!(extract_mentions("").is_empty());

        // Only $ sign
        assert!(extract_mentions("$").is_empty());

        // $ at end
        assert!(extract_mentions("text $").is_empty());

        // Duplicate mentions (should dedupe)
        assert_eq!(extract_mentions("$skill-a and $skill-a"), vec!["skill-a"]);

        // Underscore separator (converted to hyphen)
        assert_eq!(extract_mentions("$skill_name"), vec!["skill-name"]);
    }

    #[test]
    fn test_extract_mentions_multiple_same_and_different() {
        // Multiple skills with duplicates in various positions
        let mentions = extract_mentions("$skill-a $skill-b $skill-a $skill-c $skill-b");
        assert_eq!(mentions, vec!["skill-a", "skill-b", "skill-c"]);
    }

    #[test]
    fn test_extract_mentions_with_numbers() {
        assert_eq!(extract_mentions("Use $skill-123"), vec!["skill-123"]);
        assert_eq!(extract_mentions("$test-v2.0"), vec!["test-v2"]);
    }

    #[test]
    fn test_inject_skills_empty() {
        let result = inject_skills(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_inject_skills_with_resources() {
        let temp = tempfile::tempdir().unwrap();
        let skill_dir = temp.path().join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::create_dir(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir(skill_dir.join("references")).unwrap();
        std::fs::create_dir(skill_dir.join("assets")).unwrap();

        // Create resource files
        std::fs::write(skill_dir.join("scripts/test.sh"), "#!/bin/bash").unwrap();
        std::fs::write(skill_dir.join("references/ref.md"), "# Reference").unwrap();
        std::fs::write(skill_dir.join("assets/logo.png"), [0_u8, 159, 146, 150]).unwrap();

        let skill = Skill {
            metadata: SkillMetadata {
                id: "test-res".to_string(),
                package_id: None,
                name: "Test Resource Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: skill_dir.join("SKILL.md"),
                package_root: Some(skill_dir.clone()),
                resource_root: Some(skill_dir.clone()),
                scope: SkillScope::User,
                tags: vec![],
                capabilities: Some(SkillCapabilities {
                    disclosure: DisclosureConfig {
                        level3: Level3Resources {
                            assets: vec!["logo.png".to_string()],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                compatibility: Default::default(),
                source: SkillContentSource::File(skill_dir.join("SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
            content: "Read `references/ref.md` before running `scripts/test.sh`.".to_string(),
            frontmatter: SkillFrontmatter {
                name: "Test Resource Skill".to_string(),
                description: "A test".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let injected = inject_skills(&[skill]);
        assert!(injected.contains("## Skill: Test Resource Skill"));
        assert!(injected.contains("### Alan Runtime Context"));
        assert!(injected.contains("### Disclosed Resources"));
        assert!(injected.contains("#### script: scripts/test.sh"));
        assert!(injected.contains("#!/bin/bash"));
        assert!(injected.contains("#### reference: references/ref.md"));
        assert!(injected.contains("# Reference"));
        assert!(injected.contains("#### asset: assets/logo.png"));
        assert!(injected.contains("Binary or unreadable resource"));
    }

    #[test]
    fn test_inject_skills_uses_custom_level2_file() {
        let temp = tempfile::tempdir().unwrap();
        let skill_dir = temp.path().join("test-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("details.md"), "Expanded instructions.").unwrap();

        let skill = Skill {
            metadata: SkillMetadata {
                id: "test-res".to_string(),
                package_id: None,
                name: "Test Resource Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: skill_dir.join("SKILL.md"),
                package_root: Some(skill_dir.clone()),
                resource_root: Some(skill_dir.clone()),
                scope: SkillScope::User,
                tags: vec![],
                capabilities: Some(SkillCapabilities {
                    disclosure: DisclosureConfig {
                        level2: "details.md".to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                compatibility: Default::default(),
                source: SkillContentSource::File(skill_dir.join("SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
            content: "Fallback instructions.".to_string(),
            frontmatter: SkillFrontmatter {
                name: "Test Resource Skill".to_string(),
                description: "A test".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let injected = inject_skills(&[skill]);
        assert!(injected.contains("source: details.md"));
        assert!(injected.contains("Expanded instructions."));
        assert!(!injected.contains("Fallback instructions."));
    }

    #[test]
    fn test_load_resource_content_caps_large_files_by_byte_budget() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("large.txt");
        std::fs::write(
            &path,
            "a".repeat(MAX_DISCLOSED_RESOURCE_BYTES as usize + 1024),
        )
        .unwrap();

        let content = load_resource_content(&path).unwrap();

        assert!(content.starts_with('a'));
        assert!(content.contains(&format!("exceeded {MAX_DISCLOSED_RESOURCE_BYTES} bytes")));
    }

    #[test]
    fn test_format_disclosed_resources_uses_safe_fence_for_embedded_backticks() {
        let rendered = format_disclosed_resources(&[DisclosedSkillResource {
            kind: SkillResourceKind::Reference,
            display_path: "references/ref.md".to_string(),
            tracked_path: PathBuf::from("/tmp/ref.md"),
            content: Some("before\n```md\ninside\n```\nafter".to_string()),
        }]);

        assert!(rendered.contains("````md"));
        assert!(rendered.contains("\n````"));
    }

    #[test]
    fn test_inject_skills_no_parent_path() {
        // Test the edge case where skill path has no parent
        let skill = Skill {
            metadata: SkillMetadata {
                id: "no-parent".to_string(),
                package_id: None,
                name: "No Parent".to_string(),
                description: "Test".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("SKILL.md"), // No parent
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
            content: "Content".to_string(),
            frontmatter: SkillFrontmatter {
                name: "No Parent".to_string(),
                description: "Test".to_string(),
                metadata: Default::default(),
                capabilities: Default::default(),
                compatibility: Default::default(),
            },
        };

        let injected = inject_skills(&[skill]);
        assert!(injected.contains("## Skill: No Parent"));
        // Should not panic and should not have Resources section
        assert!(!injected.contains("### Resources"));
    }

    #[test]
    fn test_build_prompt_with_skills_empty() {
        let prompt = build_prompt_with_skills("Just user input", &[]);
        assert_eq!(prompt, "Just user input");
    }

    #[test]
    fn test_render_skills_list_empty() {
        assert!(render_skills_list(&[]).is_none());
    }

    #[test]
    fn test_render_skill_not_found_with_similar() {
        let available = vec![
            SkillMetadata {
                id: "test-skill".to_string(),
                package_id: None,
                name: "Test Skill".to_string(),
                description: "A test".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/test/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/test/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
            SkillMetadata {
                id: "testing".to_string(),
                package_id: None,
                name: "Testing".to_string(),
                description: "Testing skill".to_string(),
                short_description: None,
                path: std::path::PathBuf::from("/testing/SKILL.md"),
                package_root: None,
                resource_root: None,
                scope: SkillScope::User,
                tags: vec![],
                capabilities: None,
                compatibility: Default::default(),
                source: SkillContentSource::File(std::path::PathBuf::from("/testing/SKILL.md")),
                mount_mode: PackageMountMode::Discoverable,
                alan_metadata: Default::default(),
            },
        ];

        let msg = render_skill_not_found("test", &available);
        assert!(msg.contains("Skill '$test' not found"));
        assert!(msg.contains("Did you mean:"));
        assert!(msg.contains("$test-skill"));
    }

    #[test]
    fn test_render_skill_not_found_no_similar() {
        let available = vec![SkillMetadata {
            id: "other".to_string(),
            package_id: None,
            name: "Other".to_string(),
            description: "Other skill".to_string(),
            short_description: None,
            path: std::path::PathBuf::from("/other/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::User,
            tags: vec![],
            capabilities: None,
            compatibility: Default::default(),
            source: SkillContentSource::File(std::path::PathBuf::from("/other/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
            alan_metadata: Default::default(),
        }];

        let msg = render_skill_not_found("xyz", &available);
        assert!(msg.contains("Skill '$xyz' not found"));
        assert!(msg.contains("Use `/skills` to see available skills"));
        assert!(!msg.contains("Did you mean:"));
    }

    #[test]
    fn test_render_skill_not_found_partial_match() {
        // Test when the mention contains the skill id
        let available = vec![SkillMetadata {
            id: "rust".to_string(),
            package_id: None,
            name: "Rust".to_string(),
            description: "Rust skill".to_string(),
            short_description: None,
            path: std::path::PathBuf::from("/rust/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::User,
            tags: vec![],
            capabilities: None,
            compatibility: Default::default(),
            source: SkillContentSource::File(std::path::PathBuf::from("/rust/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
            alan_metadata: Default::default(),
        }];

        // "rustacean" contains "rust"
        let msg = render_skill_not_found("rustacean", &available);
        assert!(msg.contains("Did you mean:"));
        assert!(msg.contains("$rust"));
    }
}
