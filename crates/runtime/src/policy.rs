//! Policy engine for runtime tool decisions.
//!
//! This layer expresses decision semantics ("should we do this now?"),
//! while the current sandbox backend remains a best-effort execution guard.

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
    pub capability: Option<alan_protocol::ToolCapability>,
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
                        action: PolicyAction::Escalate,
                        reason: Some("write operations require escalation".to_string()),
                    },
                    PolicyRule {
                        id: Some("review-unknown".to_string()),
                        tool: Some("*".to_string()),
                        capability: Some("unknown".to_string()),
                        match_command: None,
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
                        action: PolicyAction::Deny,
                        reason: Some("dangerous destructive command".to_string()),
                    },
                    PolicyRule {
                        id: Some("deny-filesystem-wipe".to_string()),
                        tool: Some("bash".to_string()),
                        capability: None,
                        match_command: Some("mkfs".to_string()),
                        action: PolicyAction::Deny,
                        reason: Some("dangerous filesystem operation".to_string()),
                    },
                    PolicyRule {
                        id: Some("review-force-push".to_string()),
                        tool: Some("bash".to_string()),
                        capability: None,
                        match_command: Some("git push --force".to_string()),
                        action: PolicyAction::Escalate,
                        reason: Some("force push requires escalation".to_string()),
                    },
                    PolicyRule {
                        id: Some("review-unknown".to_string()),
                        tool: Some("*".to_string()),
                        capability: Some("unknown".to_string()),
                        match_command: None,
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

    true
}

fn capability_label(capability: Option<alan_protocol::ToolCapability>) -> &'static str {
    match capability {
        Some(alan_protocol::ToolCapability::Read) => "read",
        Some(alan_protocol::ToolCapability::Write) => "write",
        Some(alan_protocol::ToolCapability::Network) => "network",
        None => "unknown",
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
            capability: Some(alan_protocol::ToolCapability::Network),
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
            capability: Some(alan_protocol::ToolCapability::Network),
        });
        assert_eq!(decision.action, PolicyAction::Allow);
    }

    #[test]
    fn autonomous_denies_dangerous_bash() {
        let engine = PolicyEngine::for_profile(PolicyProfile::Autonomous);
        let decision = engine.evaluate(PolicyContext {
            tool_name: "bash",
            arguments: &json!({"command":"rm -rf / --no-preserve-root"}),
            capability: Some(alan_protocol::ToolCapability::Write),
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
            capability: Some(alan_protocol::ToolCapability::Read),
        });
        assert_eq!(decision.action, PolicyAction::Deny);
        assert_eq!(decision.rule_id.as_deref(), Some("deny-read-file"));
        assert_eq!(decision.source, "workspace_policy_file");
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
