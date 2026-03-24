//! Core types for the skills framework.

use semver::{BuildMetadata, Version};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Skill unique identifier (lowercase, hyphenated).
pub type SkillId = String;

/// Capability package unique identifier.
pub type CapabilityPackageId = String;

/// Optional skill sidecar filename.
pub const SKILL_SIDECAR_FILE: &str = "skill.yaml";
/// Optional package sidecar filename.
pub const PACKAGE_SIDECAR_FILE: &str = "package.yaml";

/// How a mounted capability package is exposed to the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PackageMountMode {
    /// Package is visible in the skills catalog and always injected as active instructions.
    AlwaysActive,
    /// Package is visible in the skills catalog and can be activated explicitly.
    #[default]
    Discoverable,
    /// Package is not listed in the catalog but can still be activated by explicit mention.
    ExplicitOnly,
    /// Package is mounted for the definition layer but hidden from the current skill runtime.
    Internal,
}

impl PackageMountMode {
    pub fn is_catalog_visible(self) -> bool {
        matches!(self, Self::AlwaysActive | Self::Discoverable)
    }

    pub fn is_active_by_default(self) -> bool {
        matches!(self, Self::AlwaysActive)
    }

    pub fn allows_explicit_activation(self) -> bool {
        !matches!(self, Self::Internal)
    }

    pub fn exposes_skills(self) -> bool {
        !matches!(self, Self::Internal)
    }
}

/// Skill scope determines precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillScope {
    /// Repository-level skills (highest priority).
    #[serde(rename = "repo")]
    Repo,
    /// User-level skills.
    #[serde(rename = "user")]
    User,
    /// Built-in first-party packages (lowest priority).
    #[serde(rename = "builtin", alias = "system")]
    Builtin,
}

impl SkillScope {
    /// Priority order: lower number = higher priority.
    pub fn priority(&self) -> u8 {
        match self {
            SkillScope::Repo => 0,
            SkillScope::User => 1,
            SkillScope::Builtin => 2,
        }
    }
}

/// Filesystem package discovery directory with its effective overlay scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedPackageDir {
    pub path: PathBuf,
    pub scope: SkillScope,
}

/// Skill content source.
#[derive(Debug, Clone)]
pub enum SkillContentSource {
    File(PathBuf),
    Embedded(&'static str),
}

impl Default for SkillContentSource {
    fn default() -> Self {
        Self::File(PathBuf::new())
    }
}

/// Portable skill exported by a capability package.
#[derive(Debug, Clone)]
pub struct PortableSkill {
    pub path: PathBuf,
    pub source: SkillContentSource,
}

/// Package-level resource directories exported by a capability package.
#[derive(Debug, Clone, Default)]
pub struct CapabilityPackageResources {
    pub scripts_dir: Option<PathBuf>,
    pub references_dir: Option<PathBuf>,
    pub assets_dir: Option<PathBuf>,
    pub viewers_dir: Option<PathBuf>,
}

impl CapabilityPackageResources {
    pub fn is_empty(&self) -> bool {
        self.scripts_dir.is_none()
            && self.references_dir.is_none()
            && self.assets_dir.is_none()
            && self.viewers_dir.is_none()
    }
}

/// Additional exports a capability package can expose beyond portable skills.
#[derive(Debug, Clone, Default)]
pub struct CapabilityPackageExports {
    pub child_agent_roots: Vec<PathBuf>,
    pub resources: CapabilityPackageResources,
}

impl CapabilityPackageExports {
    pub fn is_empty(&self) -> bool {
        self.child_agent_roots.is_empty() && self.resources.is_empty()
    }
}

/// Capability package available to an agent definition.
#[derive(Debug, Clone)]
pub struct CapabilityPackage {
    pub id: CapabilityPackageId,
    pub scope: SkillScope,
    pub root_dir: Option<PathBuf>,
    pub exports: CapabilityPackageExports,
    pub portable_skills: Vec<PortableSkill>,
}

/// Package mounted into the resolved capability view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageMount {
    #[serde(rename = "package", alias = "package_id")]
    pub package_id: CapabilityPackageId,
    #[serde(default)]
    pub mode: PackageMountMode,
}

/// Runtime-facing resolved capability view assembled from package sources.
#[derive(Debug, Clone, Default)]
pub struct ResolvedCapabilityView {
    pub package_dirs: Vec<ScopedPackageDir>,
    pub mounts: Vec<PackageMount>,
    pub packages: Vec<CapabilityPackage>,
    pub errors: Vec<SkillError>,
    pub tracked_paths: Vec<PathBuf>,
}

/// Skill metadata loaded at startup (lightweight).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: SkillId,
    pub package_id: Option<CapabilityPackageId>,
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
    /// Canonical path to the skill's `SKILL.md`.
    pub path: PathBuf,
    /// Canonical package root that exported this skill, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_root: Option<PathBuf>,
    /// Canonical resource root for resolving relative skill references.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_root: Option<PathBuf>,
    pub scope: SkillScope,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Skill capabilities (optional)
    #[serde(skip)]
    pub capabilities: Option<SkillCapabilities>,
    /// Compatibility requirements declared by the skill.
    #[serde(skip, default)]
    pub compatibility: SkillCompatibility,
    /// Skill content location.
    #[serde(skip, default)]
    pub source: SkillContentSource,
    /// How the resolved package mount exposes this skill to the runtime.
    #[serde(skip, default)]
    pub mount_mode: PackageMountMode,
    /// Alan-native runtime/UI metadata loaded from optional sidecars.
    #[serde(skip, default)]
    pub alan_metadata: AlanSkillRuntimeMetadata,
}

impl SkillMetadata {
    pub fn package_root(&self) -> Option<&Path> {
        self.package_root.as_deref()
    }

    pub fn resource_root(&self) -> Option<&Path> {
        self.resource_root
            .as_deref()
            .or_else(|| self.package_root())
    }

    pub fn apply_sidecar_metadata(
        &mut self,
        package_defaults: Option<&AlanSkillSidecar>,
        skill_sidecar: Option<&AlanSkillSidecar>,
    ) -> Result<(), SkillsError> {
        if let Some(defaults) = package_defaults {
            self.apply_skill_sidecar(defaults);
        }
        if let Some(sidecar) = skill_sidecar {
            self.apply_skill_sidecar(sidecar);
        }

        if let Some(capabilities) = self.capabilities.as_ref() {
            validate_capabilities(capabilities)?;
        }
        Ok(())
    }

    fn apply_skill_sidecar(&mut self, sidecar: &AlanSkillSidecar) {
        if !sidecar.capabilities.is_empty() {
            self.capabilities
                .get_or_insert_with(SkillCapabilities::default)
                .apply_overlay(&sidecar.capabilities);
        }
        if !sidecar.compatibility.is_empty() {
            self.compatibility.apply_overlay(&sidecar.compatibility);
        }
        if !sidecar.runtime.is_empty() {
            self.alan_metadata.apply_overlay(&sidecar.runtime);
        }
    }
}

/// Full skill content loaded on demand.
pub struct Skill {
    pub metadata: SkillMetadata,
    /// SKILL.md body content (without frontmatter).
    pub content: String,
    /// Parsed frontmatter.
    pub frontmatter: SkillFrontmatter,
}

/// YAML frontmatter in SKILL.md.
#[derive(Debug, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub metadata: FrontmatterMetadata,
    /// Skill capabilities
    #[serde(default)]
    pub capabilities: SkillCapabilities,
    /// Compatibility requirements
    #[serde(default)]
    pub compatibility: SkillCompatibility,
}

/// Optional metadata in frontmatter.
#[derive(Debug, Default, Deserialize)]
pub struct FrontmatterMetadata {
    #[serde(rename = "short-description")]
    pub short_description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Skill capabilities declaration (from SKILL.md frontmatter)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillCapabilities {
    /// Required tools - must be available for skill to function
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Optional tools - enhance functionality but not required
    #[serde(default)]
    pub optional_tools: Vec<String>,
    /// Applicable domains (empty = universal)
    #[serde(default)]
    pub domains: Vec<String>,
    /// Trigger conditions for automatic skill selection
    #[serde(default)]
    pub triggers: SkillTriggers,
    /// Progressive disclosure configuration (Level 3 resources)
    #[serde(default)]
    pub disclosure: DisclosureConfig,
}

impl SkillCapabilities {
    pub fn apply_overlay(&mut self, overlay: &SkillCapabilitiesOverlay) {
        if let Some(required_tools) = overlay.required_tools.as_ref() {
            self.required_tools = required_tools.clone();
        }
        if let Some(optional_tools) = overlay.optional_tools.as_ref() {
            self.optional_tools = optional_tools.clone();
        }
        if let Some(domains) = overlay.domains.as_ref() {
            self.domains = domains.clone();
        }
        if let Some(triggers) = overlay.triggers.as_ref() {
            self.triggers = triggers.clone();
        }
        if let Some(disclosure) = overlay.disclosure.as_ref() {
            self.disclosure = disclosure.clone();
        }
    }
}

/// Partial capabilities overlay loaded from optional Alan sidecars.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillCapabilitiesOverlay {
    #[serde(default)]
    pub required_tools: Option<Vec<String>>,
    #[serde(default)]
    pub optional_tools: Option<Vec<String>>,
    #[serde(default)]
    pub domains: Option<Vec<String>>,
    #[serde(default)]
    pub triggers: Option<SkillTriggers>,
    #[serde(default)]
    pub disclosure: Option<DisclosureConfig>,
}

impl SkillCapabilitiesOverlay {
    pub fn is_empty(&self) -> bool {
        self.required_tools.is_none()
            && self.optional_tools.is_none()
            && self.domains.is_none()
            && self.triggers.is_none()
            && self.disclosure.is_none()
    }
}

/// Skill trigger conditions
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillTriggers {
    /// Explicit trigger words (e.g., alternative names besides $skill-id)
    #[serde(default)]
    pub explicit: Vec<String>,
    /// Keywords for simple substring matching
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Regex patterns for advanced matching
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Semantic description for LLM-based triggering
    #[serde(default)]
    pub semantic: Option<String>,
    /// Negative keywords - if matched, skill should not trigger
    #[serde(default)]
    pub negative_keywords: Vec<String>,
}

/// Progressive disclosure configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DisclosureConfig {
    /// Level 2 content file (default: SKILL.md)
    #[serde(default = "default_level2")]
    pub level2: String,
    /// Level 3 resources (loaded on demand)
    #[serde(default)]
    pub level3: Level3Resources,
}

fn default_level2() -> String {
    "SKILL.md".to_string()
}

/// Level 3 resources configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Level3Resources {
    /// Reference documents (markdown, etc.)
    #[serde(default)]
    pub references: Vec<String>,
    /// Executable scripts
    #[serde(default)]
    pub scripts: Vec<String>,
    /// Template and resource files
    #[serde(default)]
    pub assets: Vec<String>,
}

/// Skill compatibility declaration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillCompatibility {
    /// Minimum version required
    #[serde(default)]
    pub min_version: Option<String>,
    /// Environment requirements description
    #[serde(default)]
    pub requirements: Option<String>,
}

impl SkillCompatibility {
    pub fn apply_overlay(&mut self, overlay: &SkillCompatibilityOverlay) {
        if let Some(min_version) = overlay.min_version.as_ref() {
            self.min_version = Some(min_version.clone());
        }
        if let Some(requirements) = overlay.requirements.as_ref() {
            self.requirements = Some(requirements.clone());
        }
    }
}

/// Partial compatibility overlay loaded from optional Alan sidecars.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SkillCompatibilityOverlay {
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub requirements: Option<String>,
}

impl SkillCompatibilityOverlay {
    pub fn is_empty(&self) -> bool {
        self.min_version.is_none() && self.requirements.is_none()
    }
}

/// Alan-native UI metadata for a skill.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AlanSkillUiMetadata {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

impl AlanSkillUiMetadata {
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.category.is_none()
    }

    pub fn apply_overlay(&mut self, overlay: &Self) {
        if let Some(title) = overlay.title.as_ref() {
            self.title = Some(title.clone());
        }
        if let Some(category) = overlay.category.as_ref() {
            self.category = Some(category.clone());
        }
    }
}

/// Alan-native runtime metadata for a skill.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AlanSkillRuntimeMetadata {
    #[serde(default)]
    pub permission_hints: Vec<String>,
    #[serde(default)]
    pub ui: AlanSkillUiMetadata,
}

impl AlanSkillRuntimeMetadata {
    pub fn is_empty(&self) -> bool {
        self.permission_hints.is_empty() && self.ui.is_empty()
    }

    pub fn apply_overlay(&mut self, overlay: &Self) {
        for hint in &overlay.permission_hints {
            if !self.permission_hints.contains(hint) {
                self.permission_hints.push(hint.clone());
            }
        }
        self.ui.apply_overlay(&overlay.ui);
    }
}

/// Optional Alan-native skill sidecar content.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AlanSkillSidecar {
    #[serde(default)]
    pub capabilities: SkillCapabilitiesOverlay,
    #[serde(default)]
    pub compatibility: SkillCompatibilityOverlay,
    #[serde(default)]
    pub runtime: AlanSkillRuntimeMetadata,
}

/// Optional Alan-native package sidecar content.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AlanPackageSidecar {
    #[serde(default)]
    pub skill_defaults: AlanSkillSidecar,
}

/// Host/runtime capability context used to decide if a skill is runnable now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillHostCapabilities {
    pub alan_version: String,
    pub tools: BTreeSet<String>,
}

impl Default for SkillHostCapabilities {
    fn default() -> Self {
        Self {
            alan_version: env!("CARGO_PKG_VERSION").to_string(),
            tools: BTreeSet::new(),
        }
    }
}

impl SkillHostCapabilities {
    pub fn with_tools<I, S>(tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tools: tools.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    pub fn extend_tools<I, S>(&mut self, tools: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tools.extend(tools.into_iter().map(Into::into));
    }

    pub fn with_runtime_defaults(mut self) -> Self {
        self.extend_tools(["request_confirmation", "request_user_input", "update_plan"]);
        self
    }
}

/// Reason a skill is not currently runnable in the active host/runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillAvailabilityIssue {
    MissingRequiredTools(Vec<String>),
    MinVersionNotMet { required: String, current: String },
    InvalidMinVersion(String),
}

impl std::fmt::Display for SkillAvailabilityIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillAvailabilityIssue::MissingRequiredTools(tools) => {
                write!(f, "missing required tools: {}", tools.join(", "))
            }
            SkillAvailabilityIssue::MinVersionNotMet { required, current } => {
                write!(f, "requires Alan >= {required} (current: {current})")
            }
            SkillAvailabilityIssue::InvalidMinVersion(version) => {
                write!(f, "invalid compatibility.min_version: {version}")
            }
        }
    }
}

/// Skill dependency validation error
#[derive(Debug, Clone)]
pub struct SkillDependencyError {
    pub skill_id: SkillId,
    pub missing_tools: Vec<String>,
    pub message: String,
}

impl std::fmt::Display for SkillDependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Skill '{}' missing required tools: {:?}",
            self.skill_id, self.missing_tools
        )
    }
}

impl std::error::Error for SkillDependencyError {}

/// Runtime-facing availability state for a selected skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SkillAvailabilityState {
    Available,
    Unavailable { issues: Vec<SkillAvailabilityIssue> },
}

impl SkillAvailabilityState {
    pub fn available() -> Self {
        Self::Available
    }

    pub fn from_issues(issues: Vec<SkillAvailabilityIssue>) -> Self {
        if issues.is_empty() {
            Self::Available
        } else {
            Self::Unavailable { issues }
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    pub fn render_label(&self) -> String {
        match self {
            Self::Available => "available".to_string(),
            Self::Unavailable { issues } => {
                format!("unavailable ({})", format_skill_availability_issues(issues))
            }
        }
    }
}

/// Why a skill was activated for the current turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillActivationReason {
    AlwaysActiveMount,
    ExplicitMention { mention: String },
    Keyword { keyword: String },
    Pattern { pattern: String },
}

impl SkillActivationReason {
    pub fn cache_key_fragment(&self) -> String {
        match self {
            Self::AlwaysActiveMount => "always_active_mount".to_string(),
            Self::ExplicitMention { mention } => format!("explicit:{mention}"),
            Self::Keyword { keyword } => format!("keyword:{keyword}"),
            Self::Pattern { pattern } => format!("pattern:{pattern}"),
        }
    }

    pub fn render_label(&self) -> String {
        match self {
            Self::AlwaysActiveMount => "always_active_mount".to_string(),
            Self::ExplicitMention { mention } => format!("explicit_mention(${mention})"),
            Self::Keyword { keyword } => format!("keyword({keyword})"),
            Self::Pattern { pattern } => format!("pattern({pattern})"),
        }
    }
}

/// Structured runtime envelope for each selected active skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSkillEnvelope {
    pub metadata: SkillMetadata,
    pub availability: SkillAvailabilityState,
    pub activation_reason: SkillActivationReason,
}

impl ActiveSkillEnvelope {
    pub fn available(metadata: SkillMetadata, activation_reason: SkillActivationReason) -> Self {
        Self {
            metadata,
            availability: SkillAvailabilityState::available(),
            activation_reason,
        }
    }

    pub fn with_issues(
        metadata: SkillMetadata,
        activation_reason: SkillActivationReason,
        issues: Vec<SkillAvailabilityIssue>,
    ) -> Self {
        Self {
            metadata,
            availability: SkillAvailabilityState::from_issues(issues),
            activation_reason,
        }
    }

    pub fn cache_key(&self) -> String {
        format!(
            "{}::{}",
            self.metadata.id,
            self.activation_reason.cache_key_fragment()
        )
    }
}

/// Skill resources (scripts, references, assets).
#[derive(Debug, Default)]
pub struct SkillResources {
    pub scripts: Vec<PathBuf>,
    pub references: Vec<PathBuf>,
    pub assets: Vec<PathBuf>,
}

/// Skill loading error (non-fatal).
#[derive(Debug, Clone)]
pub struct SkillError {
    pub path: PathBuf,
    pub message: String,
}

/// Skill load outcome with errors.
#[derive(Debug, Clone, Default)]
pub struct SkillLoadOutcome {
    pub skills: Vec<SkillMetadata>,
    pub errors: Vec<SkillError>,
    pub tracked_paths: Vec<PathBuf>,
}

impl SkillLoadOutcome {
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

/// Skill loading error.
#[derive(Debug, thiserror::Error)]
pub enum SkillsError {
    #[error("IO error: {0}")]
    Io(#[source] std::io::Error),
    #[error("Missing or invalid YAML frontmatter")]
    MissingFrontmatter,
    #[error("Invalid YAML: {0}")]
    InvalidYaml(#[source] serde_yaml::Error),
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
    #[error("Skill not found: {0}")]
    NotFound(SkillId),
    #[error("Skill name exceeds maximum length of {max} characters (got {actual})")]
    NameTooLong { max: usize, actual: usize },
    #[error("Skill description exceeds maximum length of {max} characters (got {actual})")]
    DescriptionTooLong { max: usize, actual: usize },
    #[error("Short description exceeds maximum length of {max} characters (got {actual})")]
    ShortDescriptionTooLong { max: usize, actual: usize },
    #[error("Invalid capabilities declaration: {0}")]
    InvalidCapabilities(String),
}

impl From<std::io::Error> for SkillsError {
    fn from(e: std::io::Error) -> Self {
        SkillsError::Io(e)
    }
}

impl From<serde_yaml::Error> for SkillsError {
    fn from(e: serde_yaml::Error) -> Self {
        SkillsError::InvalidYaml(e)
    }
}

/// Extract YAML frontmatter from markdown content.
/// Returns (frontmatter_yaml, body) if successful.
pub fn extract_frontmatter(content: &str) -> Option<(String, String)> {
    let mut lines = content.lines();

    // Must start with ---
    let first = lines.next()?;
    if first.trim() != "---" {
        return None;
    }

    let mut frontmatter_lines = Vec::new();
    let mut found_end = false;

    for line in lines.by_ref() {
        if line.trim() == "---" {
            found_end = true;
            break;
        }
        frontmatter_lines.push(line);
    }

    if !found_end || frontmatter_lines.is_empty() {
        return None;
    }

    let body = lines.collect::<Vec<_>>().join("\n");
    Some((frontmatter_lines.join("\n"), body))
}

/// Convert skill name to valid ID.
pub fn name_to_id(name: &str) -> SkillId {
    name.to_lowercase().replace(" ", "-").replace("_", "-")
}

/// Load skill resources from directory.
pub fn load_skill_resources(skill_dir: &Path) -> SkillResources {
    let mut resources = SkillResources::default();

    // Scan scripts/
    let scripts_dir = skill_dir.join("scripts");
    if scripts_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&scripts_dir)
    {
        resources.scripts = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
    }

    // Scan references/
    let refs_dir = skill_dir.join("references");
    if refs_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&refs_dir)
    {
        resources.references = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
    }

    // Scan assets/
    let assets_dir = skill_dir.join("assets");
    if assets_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&assets_dir)
    {
        resources.assets = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
    }

    resources
}

/// Read a reference file content.
pub fn read_reference(skill_dir: &Path, name: &str) -> Option<String> {
    let path = skill_dir.join("references").join(name);
    std::fs::read_to_string(path).ok()
}

/// Maximum allowed length for skill name.
pub const MAX_NAME_LEN: usize = 64;
/// Maximum allowed length for skill description.
pub const MAX_DESCRIPTION_LEN: usize = 1024;
/// Maximum allowed length for skill short description.
pub const MAX_SHORT_DESCRIPTION_LEN: usize = MAX_DESCRIPTION_LEN;

/// Validates skill metadata fields and returns appropriate error for invalid values.
pub fn validate_skill_metadata(
    name: &str,
    description: &str,
    short_description: Option<&str>,
) -> Result<(), SkillsError> {
    // Validate name
    if name.trim().is_empty() {
        return Err(SkillsError::MissingField("name"));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(SkillsError::NameTooLong {
            max: MAX_NAME_LEN,
            actual: name.len(),
        });
    }

    // Validate description
    if description.trim().is_empty() {
        return Err(SkillsError::MissingField("description"));
    }
    if description.len() > MAX_DESCRIPTION_LEN {
        return Err(SkillsError::DescriptionTooLong {
            max: MAX_DESCRIPTION_LEN,
            actual: description.len(),
        });
    }

    // Validate short description
    if let Some(short) = short_description
        && short.len() > MAX_SHORT_DESCRIPTION_LEN
    {
        return Err(SkillsError::ShortDescriptionTooLong {
            max: MAX_SHORT_DESCRIPTION_LEN,
            actual: short.len(),
        });
    }

    Ok(())
}

/// Validates skill capabilities declaration.
/// Returns Ok(()) if valid, Err otherwise.
pub fn validate_capabilities(cap: &SkillCapabilities) -> Result<(), SkillsError> {
    // Validate tool names (should not contain spaces or special chars)
    for tool in &cap.required_tools {
        if tool.contains(' ') || tool.contains('<') || tool.contains('>') {
            return Err(SkillsError::InvalidCapabilities(format!(
                "Invalid tool name: {}",
                tool
            )));
        }
    }

    for tool in &cap.optional_tools {
        if tool.contains(' ') || tool.contains('<') || tool.contains('>') {
            return Err(SkillsError::InvalidCapabilities(format!(
                "Invalid tool name: {}",
                tool
            )));
        }
    }

    // Validate regex patterns
    for pattern in &cap.triggers.patterns {
        if let Err(e) = regex::Regex::new(&format!("(?i){}", pattern)) {
            return Err(SkillsError::InvalidCapabilities(format!(
                "Invalid regex pattern '{}': {}",
                pattern, e
            )));
        }
    }

    Ok(())
}

pub fn skill_availability_issues(
    metadata: &SkillMetadata,
    host_capabilities: &SkillHostCapabilities,
) -> Vec<SkillAvailabilityIssue> {
    let mut issues = Vec::new();

    if let Some(capabilities) = metadata.capabilities.as_ref() {
        let missing_tools: Vec<String> = capabilities
            .required_tools
            .iter()
            .filter(|tool| !host_capabilities.tools.contains(tool.as_str()))
            .cloned()
            .collect();
        if !missing_tools.is_empty() {
            issues.push(SkillAvailabilityIssue::MissingRequiredTools(missing_tools));
        }
    }

    if let Some(required) = metadata.compatibility.min_version.as_deref() {
        match (
            parse_semver_version(required),
            parse_semver_version(&host_capabilities.alan_version),
        ) {
            (Some(required_version), Some(current_version)) => {
                if current_version < required_version {
                    issues.push(SkillAvailabilityIssue::MinVersionNotMet {
                        required: required.to_string(),
                        current: host_capabilities.alan_version.clone(),
                    });
                }
            }
            _ => issues.push(SkillAvailabilityIssue::InvalidMinVersion(
                required.to_string(),
            )),
        }
    }

    issues
}

pub fn is_skill_available(
    metadata: &SkillMetadata,
    host_capabilities: &SkillHostCapabilities,
) -> bool {
    skill_availability_issues(metadata, host_capabilities).is_empty()
}

pub fn format_skill_availability_issues(issues: &[SkillAvailabilityIssue]) -> String {
    issues
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn parse_semver_version(version: &str) -> Option<Version> {
    let mut version = Version::parse(version).ok()?;
    version.build = BuildMetadata::EMPTY;
    Some(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let content = r#"---
name: test-skill
description: A test skill
---

# Body content

This is the body.
"#;

        let (frontmatter, body) = extract_frontmatter(content).unwrap();
        assert!(frontmatter.contains("name: test-skill"));
        assert!(body.contains("# Body content"));
    }

    #[test]
    fn test_extract_frontmatter_no_start_marker() {
        // Content without --- at start
        let content = "Just some content without frontmatter";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn test_extract_frontmatter_no_end_marker() {
        // Content with start marker but no end marker
        let content = r#"---
name: test-skill
description: A test skill

# Body content"#;
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn test_name_to_id() {
        assert_eq!(name_to_id("Supplier Evaluation"), "supplier-evaluation");
        assert_eq!(name_to_id("RFQ_Generator"), "rfq-generator");
        assert_eq!(name_to_id("test skill"), "test-skill");
        assert_eq!(name_to_id("Mixed_Case-Name Here"), "mixed-case-name-here");
        assert_eq!(name_to_id("UPPER CASE"), "upper-case");
        assert_eq!(name_to_id("lower case"), "lower-case");
        assert_eq!(name_to_id(""), "");
    }

    #[test]
    fn test_skill_scope_priority() {
        assert!(SkillScope::Repo.priority() < SkillScope::User.priority());
        assert!(SkillScope::User.priority() < SkillScope::Builtin.priority());
        assert_eq!(SkillScope::Repo.priority(), 0);
        assert_eq!(SkillScope::User.priority(), 1);
        assert_eq!(SkillScope::Builtin.priority(), 2);
    }

    #[test]
    fn test_skill_scope_serde() {
        // Test serialization/deserialization of SkillScope
        let repo = serde_json::to_string(&SkillScope::Repo).unwrap();
        assert_eq!(repo, "\"repo\"");

        let user: SkillScope = serde_json::from_str("\"user\"").unwrap();
        assert!(matches!(user, SkillScope::User));

        let builtin = serde_json::to_string(&SkillScope::Builtin).unwrap();
        assert_eq!(builtin, "\"builtin\"");

        let legacy_system: SkillScope = serde_json::from_str("\"system\"").unwrap();
        assert!(matches!(legacy_system, SkillScope::Builtin));
    }

    #[test]
    fn test_package_mount_mode_serde_and_helpers() {
        let mode: PackageMountMode = serde_json::from_str("\"explicit_only\"").unwrap();
        assert_eq!(mode, PackageMountMode::ExplicitOnly);
        assert!(!mode.is_catalog_visible());
        assert!(mode.allows_explicit_activation());
        assert!(!mode.is_active_by_default());

        let internal: PackageMountMode = serde_json::from_str("\"internal\"").unwrap();
        assert_eq!(internal, PackageMountMode::Internal);
        assert!(!internal.exposes_skills());
    }

    #[test]
    fn test_skill_availability_tracks_tools_and_min_version() {
        let metadata = SkillMetadata {
            id: "test-skill".to_string(),
            package_id: Some("skill:test-skill".to_string()),
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            short_description: None,
            path: PathBuf::from("/tmp/test-skill/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::Repo,
            tags: vec![],
            capabilities: Some(SkillCapabilities {
                required_tools: vec!["read_file".to_string()],
                ..Default::default()
            }),
            compatibility: SkillCompatibility {
                min_version: Some("0.2.0".to_string()),
                requirements: None,
            },
            source: SkillContentSource::File(PathBuf::from("/tmp/test-skill/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
            alan_metadata: Default::default(),
        };

        let unavailable = skill_availability_issues(
            &metadata,
            &SkillHostCapabilities::default().with_runtime_defaults(),
        );
        assert_eq!(unavailable.len(), 2);
        assert!(!is_skill_available(
            &metadata,
            &SkillHostCapabilities::default().with_runtime_defaults()
        ));

        let available_host =
            SkillHostCapabilities::with_tools(["read_file"]).with_runtime_defaults();
        let issues = skill_availability_issues(
            &SkillMetadata {
                compatibility: SkillCompatibility {
                    min_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    ..metadata.compatibility.clone()
                },
                ..metadata
            },
            &available_host,
        );
        assert!(issues.is_empty());
    }

    #[test]
    fn test_skill_host_capabilities_runtime_defaults_include_virtual_tools() {
        let capabilities = SkillHostCapabilities::default().with_runtime_defaults();
        assert!(capabilities.tools.contains("request_confirmation"));
        assert!(capabilities.tools.contains("request_user_input"));
        assert!(capabilities.tools.contains("update_plan"));
    }

    #[test]
    fn test_skill_availability_respects_semver_prerelease_ordering() {
        let metadata = SkillMetadata {
            id: "test-skill".to_string(),
            package_id: Some("skill:test-skill".to_string()),
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            short_description: None,
            path: PathBuf::from("/tmp/test-skill/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::Repo,
            tags: vec![],
            capabilities: None,
            compatibility: SkillCompatibility {
                min_version: Some("1.2.3".to_string()),
                requirements: None,
            },
            source: SkillContentSource::File(PathBuf::from("/tmp/test-skill/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
            alan_metadata: Default::default(),
        };
        let host_capabilities = SkillHostCapabilities {
            alan_version: "1.2.3-alpha.1".to_string(),
            ..SkillHostCapabilities::default()
        }
        .with_runtime_defaults();

        let issues = skill_availability_issues(&metadata, &host_capabilities);
        assert_eq!(
            issues,
            vec![SkillAvailabilityIssue::MinVersionNotMet {
                required: "1.2.3".to_string(),
                current: "1.2.3-alpha.1".to_string(),
            }]
        );
    }

    #[test]
    fn test_skill_availability_accepts_semver_build_metadata() {
        let metadata = SkillMetadata {
            id: "test-skill".to_string(),
            package_id: Some("skill:test-skill".to_string()),
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            short_description: None,
            path: PathBuf::from("/tmp/test-skill/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::Repo,
            tags: vec![],
            capabilities: None,
            compatibility: SkillCompatibility {
                min_version: Some("1.2.3+build.5".to_string()),
                requirements: None,
            },
            source: SkillContentSource::File(PathBuf::from("/tmp/test-skill/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
            alan_metadata: Default::default(),
        };
        let host_capabilities = SkillHostCapabilities {
            alan_version: "1.2.3".to_string(),
            ..SkillHostCapabilities::default()
        }
        .with_runtime_defaults();

        let issues = skill_availability_issues(&metadata, &host_capabilities);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_load_skill_resources() {
        let temp = std::env::temp_dir().join(format!("skill_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();

        let skill_dir = temp.join("test-skill");

        // Create scripts directory with files
        let scripts_dir = skill_dir.join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("helper.sh"), "#!/bin/bash").unwrap();
        std::fs::write(scripts_dir.join("tool.py"), "#!/usr/bin/env python3").unwrap();

        // Create references directory with files
        let refs_dir = skill_dir.join("references");
        std::fs::create_dir_all(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("guide.md"), "# Guide").unwrap();

        // Create assets directory with files
        let assets_dir = skill_dir.join("assets");
        std::fs::create_dir_all(&assets_dir).unwrap();
        std::fs::write(assets_dir.join("template.txt"), "Template").unwrap();

        let resources = load_skill_resources(&skill_dir);

        assert_eq!(resources.scripts.len(), 2);
        assert_eq!(resources.references.len(), 1);
        assert_eq!(resources.assets.len(), 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_read_reference() {
        let temp = std::env::temp_dir().join(format!("ref_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();

        let refs_dir = temp.join("references");
        std::fs::create_dir_all(&refs_dir).unwrap();
        std::fs::write(
            refs_dir.join("guide.md"),
            "# Reference Guide\n\nContent here.",
        )
        .unwrap();

        let content = read_reference(&temp, "guide.md");
        assert_eq!(
            content,
            Some("# Reference Guide\n\nContent here.".to_string())
        );

        // Non-existent reference
        let not_found = read_reference(&temp, "nonexistent.md");
        assert_eq!(not_found, None);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_skills_error_display() {
        let err = SkillsError::MissingField("name");
        assert!(err.to_string().contains("name"));

        let err = SkillsError::MissingFrontmatter;
        assert!(err.to_string().contains("frontmatter"));

        let err = SkillsError::NotFound("test-skill".to_string());
        assert!(err.to_string().contains("test-skill"));

        let err = SkillsError::NameTooLong {
            max: 64,
            actual: 100,
        };
        assert!(err.to_string().contains("64"));
        assert!(err.to_string().contains("100"));
    }

    #[test]
    fn test_skill_metadata_serde() {
        // Test serialization/deserialization of SkillMetadata
        let metadata = SkillMetadata {
            id: "test-skill".to_string(),
            package_id: None,
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            short_description: Some("Short".to_string()),
            path: PathBuf::from("/test/SKILL.md"),
            package_root: None,
            resource_root: None,
            scope: SkillScope::Repo,
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            capabilities: None,
            compatibility: Default::default(),
            source: SkillContentSource::File(PathBuf::from("/test/SKILL.md")),
            mount_mode: PackageMountMode::Discoverable,
            alan_metadata: Default::default(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("test-skill"));
        assert!(json.contains("Test Skill"));

        let deserialized: SkillMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, metadata.id);
        assert_eq!(deserialized.name, metadata.name);
        assert_eq!(deserialized.scope, metadata.scope);
    }
}
