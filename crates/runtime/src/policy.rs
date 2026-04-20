//! Policy engine for runtime tool decisions.
//!
//! This layer expresses decision semantics ("should we do this now?"),
//! while the current execution backend remains a best-effort host-side guard.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Builtin policy profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolicyProfile {
    /// Conservative profile: closer to current defaults.
    Conservative,
    /// Autonomous profile: fewer restrictions, keep only critical boundaries.
    #[default]
    Autonomous,
}

/// Policy decision action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Allow,
    Deny,
    Escalate,
}

fn default_action_allow() -> PolicyAction {
    PolicyAction::Allow
}

/// Rule loaded from policy file.
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyRule {
    /// Optional stable id for audit/reasoning.
    #[serde(default)]
    pub id: Option<String>,
    /// Tool name or "*".
    #[serde(default)]
    pub tool: Option<String>,
    /// Capability filter: read/write/network/unknown.
    #[serde(default)]
    pub capability: Option<String>,
    /// For bash: case-insensitive substring match against command.
    #[serde(default)]
    pub match_command: Option<String>,
    /// For file-oriented tools: normalized prefix match against common path arguments.
    #[serde(default)]
    pub match_path_prefix: Option<String>,
    /// Rule action.
    pub action: PolicyAction,
    /// Optional human-readable reason.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Policy file schema (`policy.yaml` inside an `AgentRoot`).
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyFile {
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
    #[serde(default = "default_action_allow")]
    pub default_action: PolicyAction,
}

/// Evaluation input.
pub struct PolicyContext<'a> {
    pub tool_name: &'a str,
    pub arguments: &'a serde_json::Value,
    pub capability: alan_protocol::ToolCapability,
    pub cwd: Option<&'a Path>,
}

/// Evaluation output with lightweight audit metadata.
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub action: PolicyAction,
    pub reason: Option<String>,
    pub rule_id: Option<String>,
    pub source: &'static str,
}

/// Runtime policy engine.
#[derive(Debug, Clone)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
    default_action: PolicyAction,
    source: &'static str,
}

impl PolicyEngine {
    pub fn for_profile(profile: PolicyProfile) -> Self {
        match profile {
            PolicyProfile::Conservative => Self {
                rules: vec![
                    PolicyRule {
                        id: Some("deny-network".to_string()),
                        tool: Some("*".to_string()),
                        capability: Some("network".to_string()),
                        match_command: None,
                        match_path_prefix: None,
                        action: PolicyAction::Deny,
                        reason: Some(
                            "network access is denied by conservative profile".to_string(),
                        ),
                    },
                    PolicyRule {
                        id: Some("review-write".to_string()),
                        tool: Some("*".to_string()),
                        capability: Some("write".to_string()),
                        match_command: None,
                        match_path_prefix: None,
                        action: PolicyAction::Escalate,
                        reason: Some("write operations require escalation".to_string()),
                    },
                    PolicyRule {
                        id: Some("review-unknown".to_string()),
                        tool: Some("*".to_string()),
                        capability: Some("unknown".to_string()),
                        match_command: None,
                        match_path_prefix: None,
                        action: PolicyAction::Escalate,
                        reason: Some("unknown capability requires escalation".to_string()),
                    },
                ],
                default_action: PolicyAction::Allow,
                source: "builtin_conservative",
            },
            PolicyProfile::Autonomous => Self {
                rules: vec![
                    PolicyRule {
                        id: Some("deny-rm-root".to_string()),
                        tool: Some("bash".to_string()),
                        capability: None,
                        match_command: Some("rm -rf /".to_string()),
                        match_path_prefix: None,
                        action: PolicyAction::Deny,
                        reason: Some("dangerous destructive command".to_string()),
                    },
                    PolicyRule {
                        id: Some("deny-filesystem-wipe".to_string()),
                        tool: Some("bash".to_string()),
                        capability: None,
                        match_command: Some("mkfs".to_string()),
                        match_path_prefix: None,
                        action: PolicyAction::Deny,
                        reason: Some("dangerous filesystem operation".to_string()),
                    },
                    PolicyRule {
                        id: Some("review-force-push".to_string()),
                        tool: Some("bash".to_string()),
                        capability: None,
                        match_command: Some("git push --force".to_string()),
                        match_path_prefix: None,
                        action: PolicyAction::Escalate,
                        reason: Some("force push requires escalation".to_string()),
                    },
                    PolicyRule {
                        id: Some("review-unknown".to_string()),
                        tool: Some("*".to_string()),
                        capability: Some("unknown".to_string()),
                        match_command: None,
                        match_path_prefix: None,
                        action: PolicyAction::Escalate,
                        reason: Some("unknown capability requires escalation".to_string()),
                    },
                ],
                default_action: PolicyAction::Allow,
                source: "builtin_autonomous",
            },
        }
    }

    pub fn load_for_governance(
        workspace_alan_dir: Option<&Path>,
        governance: &alan_protocol::GovernanceConfig,
    ) -> Self {
        Self::load_for_governance_with_default_policy_path(workspace_alan_dir, None, governance)
    }

    pub fn load_for_governance_with_default_policy_path(
        workspace_alan_dir: Option<&Path>,
        default_policy_path: Option<&Path>,
        governance: &alan_protocol::GovernanceConfig,
    ) -> Self {
        let profile: PolicyProfile = governance.profile.into();
        let Some(policy_path) = governance.policy_path.as_deref() else {
            return Self::load_or_profile_with_default_policy_path(default_policy_path, profile);
        };

        let resolved = resolve_policy_path(workspace_alan_dir, Path::new(policy_path));
        match load_policy_file(&resolved) {
            Ok(policy_file) => Self {
                rules: policy_file.rules,
                default_action: policy_file.default_action,
                source: "governance_policy_file",
            },
            Err(err) => {
                tracing::warn!(
                    path = %resolved.display(),
                    error = %err,
                    "Failed to parse governance policy file, falling back to builtin profile"
                );
                Self::for_profile(profile)
            }
        }
    }

    pub fn load_or_profile(workspace_alan_dir: Option<&Path>, profile: PolicyProfile) -> Self {
        Self::load_or_profile_with_default_policy_path(
            workspace_alan_dir.map(workspace_policy_path).as_deref(),
            profile,
        )
    }

    pub fn load_or_profile_with_default_policy_path(
        default_policy_path: Option<&Path>,
        profile: PolicyProfile,
    ) -> Self {
        let Some(policy_path) = default_policy_path else {
            return Self::for_profile(profile);
        };

        if !policy_path.exists() {
            return Self::for_profile(profile);
        }

        match load_policy_file(policy_path) {
            Ok(policy_file) => Self {
                rules: policy_file.rules,
                default_action: policy_file.default_action,
                source: "workspace_policy_file",
            },
            Err(err) => {
                tracing::warn!(
                    path = %policy_path.display(),
                    error = %err,
                    "Failed to parse policy file, falling back to builtin profile"
                );
                Self::for_profile(profile)
            }
        }
    }

    pub fn evaluate(&self, ctx: PolicyContext<'_>) -> PolicyDecision {
        for rule in &self.rules {
            if rule_matches(rule, &ctx) {
                return PolicyDecision {
                    action: rule.action,
                    reason: rule.reason.clone(),
                    rule_id: rule.id.clone(),
                    source: self.source,
                };
            }
        }
        PolicyDecision {
            action: self.default_action,
            reason: None,
            rule_id: None,
            source: self.source,
        }
    }
}

fn workspace_policy_path(workspace_alan_dir: &Path) -> PathBuf {
    workspace_alan_dir.join("policy.yaml")
}

fn resolve_policy_path(workspace_alan_dir: Option<&Path>, raw_path: &Path) -> PathBuf {
    if raw_path.is_absolute() {
        return raw_path.to_path_buf();
    }
    if let Some(base) = workspace_alan_dir {
        if let Some(stripped) = strip_dot_alan_prefix(raw_path) {
            return base.join(stripped);
        }
        return base.join(raw_path);
    }
    raw_path.to_path_buf()
}

fn strip_dot_alan_prefix(path: &Path) -> Option<&Path> {
    let mut components = path.components();
    match components.next() {
        Some(Component::CurDir) => match components.next() {
            Some(Component::Normal(name)) if name == std::ffi::OsStr::new(".alan") => {
                Some(components.as_path())
            }
            _ => None,
        },
        Some(Component::Normal(name)) if name == std::ffi::OsStr::new(".alan") => {
            Some(components.as_path())
        }
        _ => None,
    }
}

fn load_policy_file(path: &Path) -> anyhow::Result<PolicyFile> {
    let content = std::fs::read_to_string(path)?;
    let policy = serde_yaml::from_str::<PolicyFile>(&content)?;
    Ok(policy)
}

fn rule_matches(rule: &PolicyRule, ctx: &PolicyContext<'_>) -> bool {
    if let Some(tool) = rule.tool.as_deref()
        && tool != "*"
        && tool != ctx.tool_name
    {
        return false;
    }

    if let Some(capability) = rule.capability.as_deref()
        && capability != capability_label(ctx.capability)
    {
        return false;
    }

    if let Some(pattern) = rule.match_command.as_deref() {
        if ctx.tool_name != "bash" {
            return false;
        }
        let command = ctx
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        if !command.contains(&pattern.to_lowercase()) {
            return false;
        }
    }

    if let Some(path_prefix) = rule.match_path_prefix.as_deref()
        && !arguments_match_path_prefix(ctx.arguments, path_prefix, ctx.cwd)
    {
        return false;
    }

    true
}

fn arguments_match_path_prefix(
    arguments: &serde_json::Value,
    path_prefix: &str,
    current_cwd: Option<&Path>,
) -> bool {
    let normalized_prefix = normalize_path_match_value(path_prefix);

    collect_path_candidates(arguments, current_cwd)
        .into_iter()
        .any(|candidate| candidate.matches_prefix(&normalized_prefix))
}

fn collect_path_candidates(
    arguments: &serde_json::Value,
    current_cwd: Option<&Path>,
) -> Vec<NormalizedPathMatchValue> {
    const PATH_KEYS: &[&str] = &["path", "paths", "directory", "cwd", "workspace_root"];
    const BASE_PATH_KEYS: &[&str] = &["directory", "cwd", "workspace_root"];

    let Some(object) = arguments.as_object() else {
        return Vec::new();
    };

    let raw_candidates = collect_path_values(object, PATH_KEYS);
    let mut candidates: Vec<_> = raw_candidates
        .iter()
        .copied()
        .map(normalize_path_match_value)
        .collect();
    let base_candidates = collect_base_path_candidates(object, current_cwd, BASE_PATH_KEYS);

    for raw_candidate in raw_candidates {
        let normalized_candidate = normalize_path_match_value(raw_candidate);
        if normalized_candidate.is_absolute {
            continue;
        }
        for base_candidate in &base_candidates {
            candidates.push(normalized_candidate.resolved_against(base_candidate));
        }
    }

    candidates
}

fn collect_path_values<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Vec<&'a str> {
    let mut candidates = Vec::new();
    for key in keys {
        let Some(value) = object.get(*key) else {
            continue;
        };
        match value {
            serde_json::Value::String(path) => candidates.push(path.as_str()),
            serde_json::Value::Array(paths) => {
                for path in paths.iter().filter_map(serde_json::Value::as_str) {
                    candidates.push(path);
                }
            }
            _ => {}
        }
    }

    candidates
}

fn collect_base_path_candidates(
    object: &serde_json::Map<String, serde_json::Value>,
    current_cwd: Option<&Path>,
    keys: &[&str],
) -> Vec<NormalizedPathMatchValue> {
    let mut candidates: Vec<_> = collect_path_values(object, keys)
        .into_iter()
        .map(normalize_path_match_value)
        .collect();
    if let Some(cwd) = current_cwd {
        candidates.push(normalize_path_match_value(&cwd.to_string_lossy()));
    }
    candidates
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedPathMatchValue {
    is_absolute: bool,
    has_trailing_separator: bool,
    segments: Vec<String>,
}

impl NormalizedPathMatchValue {
    fn matches_prefix(&self, prefix: &Self) -> bool {
        let normalized_prefix = prefix.render_relative();
        if normalized_prefix.is_empty() {
            return false;
        }

        if prefix.is_absolute {
            return self.is_absolute
                && path_prefix_matches(
                    &self.render_absolute(),
                    &prefix.render_absolute(),
                    prefix.has_trailing_separator,
                );
        }

        if self.is_absolute {
            return self.relative_tails().into_iter().any(|candidate| {
                path_prefix_matches(
                    &candidate,
                    &normalized_prefix,
                    prefix.has_trailing_separator,
                )
            });
        }

        path_prefix_matches(
            &self.render_relative(),
            &normalized_prefix,
            prefix.has_trailing_separator,
        )
    }

    fn render_absolute(&self) -> String {
        if self.segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.render_relative())
        }
    }

    fn render_relative(&self) -> String {
        self.segments.join("/")
    }

    fn relative_tails(&self) -> Vec<String> {
        (0..self.segments.len())
            .map(|index| self.segments[index..].join("/"))
            .collect()
    }

    fn resolved_against(&self, base: &Self) -> Self {
        if self.is_absolute {
            return self.clone();
        }

        let mut combined = if base.is_absolute {
            PathBuf::from(base.render_absolute())
        } else {
            PathBuf::from(base.render_relative())
        };
        combined.push(self.render_relative());
        normalize_path_match_value(&combined.to_string_lossy())
    }
}

fn normalize_path_match_value(value: &str) -> NormalizedPathMatchValue {
    let normalized_separators = value.trim().replace('\\', "/");
    let has_trailing_separator = normalized_separators.ends_with('/');
    let path = Path::new(normalized_separators.as_str());
    let mut is_absolute = path.is_absolute();
    let mut segments = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                is_absolute = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(segments.last().map(String::as_str), Some(segment) if segment != "..") {
                    segments.pop();
                } else if !is_absolute {
                    segments.push("..".to_string());
                }
            }
            Component::Normal(segment) => {
                segments.push(segment.to_string_lossy().into_owned());
            }
        }
    }

    NormalizedPathMatchValue {
        is_absolute,
        has_trailing_separator,
        segments,
    }
}

fn path_prefix_matches(candidate: &str, prefix: &str, require_component_boundary: bool) -> bool {
    if prefix.is_empty() {
        return false;
    }

    if require_component_boundary {
        candidate == prefix
            || candidate
                .strip_prefix(prefix)
                .is_some_and(|remaining| remaining.starts_with('/'))
    } else {
        candidate.starts_with(prefix)
    }
}

fn capability_label(capability: alan_protocol::ToolCapability) -> &'static str {
    match capability {
        alan_protocol::ToolCapability::Read => "read",
        alan_protocol::ToolCapability::Write => "write",
        alan_protocol::ToolCapability::Network => "network",
        alan_protocol::ToolCapability::Unknown => "unknown",
    }
}

impl From<alan_protocol::GovernanceProfile> for PolicyProfile {
    fn from(value: alan_protocol::GovernanceProfile) -> Self {
        match value {
            alan_protocol::GovernanceProfile::Autonomous => PolicyProfile::Autonomous,
            alan_protocol::GovernanceProfile::Conservative => PolicyProfile::Conservative,
        }
    }
}

impl From<PolicyProfile> for alan_protocol::GovernanceProfile {
    fn from(value: PolicyProfile) -> Self {
        match value {
            PolicyProfile::Autonomous => alan_protocol::GovernanceProfile::Autonomous,
            PolicyProfile::Conservative => alan_protocol::GovernanceProfile::Conservative,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn conservative_denies_network() {
        let engine = PolicyEngine::for_profile(PolicyProfile::Conservative);
        let decision = engine.evaluate(PolicyContext {
            tool_name: "bash",
            arguments: &json!({"command":"curl https://example.com"}),
            capability: alan_protocol::ToolCapability::Network,
            cwd: None,
        });
        assert_eq!(decision.action, PolicyAction::Deny);
        assert_eq!(decision.rule_id.as_deref(), Some("deny-network"));
    }

    #[test]
    fn autonomous_allows_network_by_default() {
        let engine = PolicyEngine::for_profile(PolicyProfile::Autonomous);
        let decision = engine.evaluate(PolicyContext {
            tool_name: "bash",
            arguments: &json!({"command":"curl https://example.com"}),
            capability: alan_protocol::ToolCapability::Network,
            cwd: None,
        });
        assert_eq!(decision.action, PolicyAction::Allow);
    }

    #[test]
    fn autonomous_denies_dangerous_bash() {
        let engine = PolicyEngine::for_profile(PolicyProfile::Autonomous);
        let decision = engine.evaluate(PolicyContext {
            tool_name: "bash",
            arguments: &json!({"command":"rm -rf / --no-preserve-root"}),
            capability: alan_protocol::ToolCapability::Write,
            cwd: None,
        });
        assert_eq!(decision.action, PolicyAction::Deny);
        assert_eq!(decision.rule_id.as_deref(), Some("deny-rm-root"));
    }

    #[test]
    fn load_workspace_policy_file_overrides_builtin() {
        let tmp = TempDir::new().unwrap();
        let policy_dir = tmp.path().join("workspace-alan");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("policy.yaml"),
            r#"
rules:
  - id: deny-read-file
    tool: read_file
    action: deny
    reason: no reads
default_action: allow
"#,
        )
        .unwrap();

        let engine =
            PolicyEngine::load_or_profile(Some(policy_dir.as_path()), PolicyProfile::Autonomous);
        let decision = engine.evaluate(PolicyContext {
            tool_name: "read_file",
            arguments: &json!({}),
            capability: alan_protocol::ToolCapability::Read,
            cwd: None,
        });
        assert_eq!(decision.action, PolicyAction::Deny);
        assert_eq!(decision.rule_id.as_deref(), Some("deny-read-file"));
        assert_eq!(decision.source, "workspace_policy_file");
    }

    #[test]
    fn policy_rule_match_path_prefix_matches_write_path() {
        let engine = PolicyEngine {
            rules: vec![PolicyRule {
                id: Some("review-workflows".to_string()),
                tool: Some("write_file".to_string()),
                capability: Some("write".to_string()),
                match_command: None,
                match_path_prefix: Some(".github/workflows/".to_string()),
                action: PolicyAction::Escalate,
                reason: Some("workflow edits require escalation".to_string()),
            }],
            default_action: PolicyAction::Allow,
            source: "test",
        };

        let decision = engine.evaluate(PolicyContext {
            tool_name: "write_file",
            arguments: &json!({"path":"./.github/workflows/release.yml","content":"name: release"}),
            capability: alan_protocol::ToolCapability::Write,
            cwd: None,
        });

        assert_eq!(decision.action, PolicyAction::Escalate);
        assert_eq!(decision.rule_id.as_deref(), Some("review-workflows"));
    }

    #[test]
    fn policy_rule_match_path_prefix_matches_paths_array() {
        let engine = PolicyEngine {
            rules: vec![PolicyRule {
                id: Some("review-deploy".to_string()),
                tool: Some("*".to_string()),
                capability: Some("write".to_string()),
                match_command: None,
                match_path_prefix: Some("deploy/".to_string()),
                action: PolicyAction::Escalate,
                reason: Some("deploy config updates require escalation".to_string()),
            }],
            default_action: PolicyAction::Allow,
            source: "test",
        };

        let decision = engine.evaluate(PolicyContext {
            tool_name: "edit_file",
            arguments: &json!({"paths":["src/lib.rs","deploy/prod.yaml"]}),
            capability: alan_protocol::ToolCapability::Write,
            cwd: None,
        });

        assert_eq!(decision.action, PolicyAction::Escalate);
        assert_eq!(decision.rule_id.as_deref(), Some("review-deploy"));
    }

    #[test]
    fn policy_rule_match_path_prefix_matches_absolute_write_path() {
        let engine = PolicyEngine {
            rules: vec![PolicyRule {
                id: Some("review-workflows".to_string()),
                tool: Some("write_file".to_string()),
                capability: Some("write".to_string()),
                match_command: None,
                match_path_prefix: Some(".github/workflows/".to_string()),
                action: PolicyAction::Escalate,
                reason: Some("workflow edits require escalation".to_string()),
            }],
            default_action: PolicyAction::Allow,
            source: "test",
        };

        let decision = engine.evaluate(PolicyContext {
            tool_name: "write_file",
            arguments: &json!({
                "path":"/workspace/repo/.github/workflows/release.yml",
                "content":"name: release"
            }),
            capability: alan_protocol::ToolCapability::Write,
            cwd: None,
        });

        assert_eq!(decision.action, PolicyAction::Escalate);
        assert_eq!(decision.rule_id.as_deref(), Some("review-workflows"));
    }

    #[test]
    fn policy_rule_match_path_prefix_matches_parent_traversal_path() {
        let engine = PolicyEngine {
            rules: vec![PolicyRule {
                id: Some("review-deploy".to_string()),
                tool: Some("*".to_string()),
                capability: Some("write".to_string()),
                match_command: None,
                match_path_prefix: Some("deploy/".to_string()),
                action: PolicyAction::Escalate,
                reason: Some("deploy config updates require escalation".to_string()),
            }],
            default_action: PolicyAction::Allow,
            source: "test",
        };

        let decision = engine.evaluate(PolicyContext {
            tool_name: "edit_file",
            arguments: &json!({"path":"tmp/../deploy/prod.yaml"}),
            capability: alan_protocol::ToolCapability::Write,
            cwd: None,
        });

        assert_eq!(decision.action, PolicyAction::Escalate);
        assert_eq!(decision.rule_id.as_deref(), Some("review-deploy"));
    }

    #[test]
    fn policy_rule_match_path_prefix_matches_parent_traversal_against_current_cwd() {
        let engine = PolicyEngine {
            rules: vec![PolicyRule {
                id: Some("review-deploy".to_string()),
                tool: Some("*".to_string()),
                capability: Some("write".to_string()),
                match_command: None,
                match_path_prefix: Some("deploy/".to_string()),
                action: PolicyAction::Escalate,
                reason: Some("deploy config updates require escalation".to_string()),
            }],
            default_action: PolicyAction::Allow,
            source: "test",
        };

        let decision = engine.evaluate(PolicyContext {
            tool_name: "edit_file",
            arguments: &json!({"path":"../deploy/prod.yaml"}),
            capability: alan_protocol::ToolCapability::Write,
            cwd: Some(Path::new("/workspace/repo/src")),
        });

        assert_eq!(decision.action, PolicyAction::Escalate);
        assert_eq!(decision.rule_id.as_deref(), Some("review-deploy"));
    }

    #[test]
    fn load_workspace_policy_file_supports_match_path_prefix() {
        let tmp = TempDir::new().unwrap();
        let policy_dir = tmp.path().join("workspace-alan");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("policy.yaml"),
            r#"
rules:
  - id: review-credentials
    tool: read_file
    capability: read
    match_path_prefix: ".env"
    action: escalate
    reason: credential reads require escalation
default_action: allow
"#,
        )
        .unwrap();

        let engine =
            PolicyEngine::load_or_profile(Some(policy_dir.as_path()), PolicyProfile::Autonomous);
        let decision = engine.evaluate(PolicyContext {
            tool_name: "read_file",
            arguments: &json!({"path":".env.production"}),
            capability: alan_protocol::ToolCapability::Read,
            cwd: None,
        });

        assert_eq!(decision.action, PolicyAction::Escalate);
        assert_eq!(decision.rule_id.as_deref(), Some("review-credentials"));
        assert_eq!(decision.source, "workspace_policy_file");
    }

    #[test]
    fn autonomous_escalates_unknown_capability() {
        let engine = PolicyEngine::for_profile(PolicyProfile::Autonomous);
        let decision = engine.evaluate(PolicyContext {
            tool_name: "bash",
            arguments: &json!({"command":"python3 script.py"}),
            capability: alan_protocol::ToolCapability::Unknown,
            cwd: None,
        });
        assert_eq!(decision.action, PolicyAction::Escalate);
        assert_eq!(decision.rule_id.as_deref(), Some("review-unknown"));
    }

    #[test]
    fn resolve_policy_path_strips_dot_alan_prefix() {
        let tmp = TempDir::new().unwrap();
        let alan_dir = tmp.path().join(".alan");
        let resolved = resolve_policy_path(
            Some(alan_dir.as_path()),
            Path::new(".alan/agent/policy.yaml"),
        );
        assert_eq!(resolved, alan_dir.join("agent/policy.yaml"));
    }

    #[test]
    fn resolve_policy_path_strips_curdir_dot_alan_prefix() {
        let tmp = TempDir::new().unwrap();
        let alan_dir = tmp.path().join(".alan");
        let resolved = resolve_policy_path(
            Some(alan_dir.as_path()),
            Path::new("./.alan/agent/policy.yaml"),
        );
        assert_eq!(resolved, alan_dir.join("agent/policy.yaml"));
    }

    #[test]
    fn resolve_policy_path_keeps_regular_relative_path() {
        let tmp = TempDir::new().unwrap();
        let alan_dir = tmp.path().join(".alan");
        let resolved = resolve_policy_path(Some(alan_dir.as_path()), Path::new("policy.yaml"));
        assert_eq!(resolved, alan_dir.join("policy.yaml"));
    }
}
