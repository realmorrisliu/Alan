use crate::approval::ToolApprovalCacheKey;
use serde_json::json;
use sha2::{Digest, Sha256};

const SANDBOX_BACKEND: &str = "workspace_path_guard";

#[derive(Debug, Clone)]
pub(super) enum ToolPolicyDecision {
    Allow {
        audit: alan_protocol::ToolDecisionAudit,
    },
    Escalate {
        summary: String,
        details: serde_json::Value,
        audit: alan_protocol::ToolDecisionAudit,
    },
    Forbidden {
        reason: String,
        audit: alan_protocol::ToolDecisionAudit,
    },
}

pub(super) fn evaluate_tool_policy(
    policy_engine: &crate::policy::PolicyEngine,
    governance: &alan_protocol::GovernanceConfig,
    tool_name: &str,
    arguments: &serde_json::Value,
    capability: Option<alan_protocol::ToolCapability>,
) -> ToolPolicyDecision {
    let policy_decision = policy_engine.evaluate(crate::policy::PolicyContext {
        tool_name,
        arguments,
        capability,
    });
    let capability_kind = capability_label(capability).to_string();
    let policy_source = policy_decision.source.to_string();
    let rule_id = policy_decision.rule_id.clone();
    let policy_reason = policy_decision.reason.clone();

    match policy_decision.action {
        crate::policy::PolicyAction::Allow => ToolPolicyDecision::Allow {
            audit: alan_protocol::ToolDecisionAudit {
                policy_source: policy_source.clone(),
                rule_id: rule_id.clone(),
                action: "allow".to_string(),
                reason: policy_reason.clone(),
                capability: capability_kind,
                sandbox_backend: SANDBOX_BACKEND.to_string(),
            },
        },
        crate::policy::PolicyAction::Deny => ToolPolicyDecision::Forbidden {
            reason: policy_reason
                .clone()
                .unwrap_or_else(|| format!("Tool '{}' denied by policy", tool_name)),
            audit: alan_protocol::ToolDecisionAudit {
                policy_source: policy_source.clone(),
                rule_id: rule_id.clone(),
                action: "deny".to_string(),
                reason: policy_reason.clone(),
                capability: capability_kind,
                sandbox_backend: SANDBOX_BACKEND.to_string(),
            },
        },
        crate::policy::PolicyAction::Escalate => ToolPolicyDecision::Escalate {
            summary: format!("Escalate tool call '{}'? ", tool_name)
                .trim()
                .to_string(),
            details: json!({
                "kind": "tool_escalation",
                "tool_name": tool_name,
                "arguments": arguments,
                "capability": capability_label(capability),
                "governance": governance,
                "policy": {
                    "source": policy_source,
                    "rule_id": rule_id,
                    "reason": policy_reason,
                    "action": "escalate"
                },
                "sandbox_backend": SANDBOX_BACKEND
            }),
            audit: alan_protocol::ToolDecisionAudit {
                policy_source,
                rule_id,
                action: "escalate".to_string(),
                reason: policy_reason,
                capability: capability_kind,
                sandbox_backend: SANDBOX_BACKEND.to_string(),
            },
        },
    }
}

pub(super) fn capability_label(capability: Option<alan_protocol::ToolCapability>) -> &'static str {
    match capability {
        Some(alan_protocol::ToolCapability::Read) => "read",
        Some(alan_protocol::ToolCapability::Write) => "write",
        Some(alan_protocol::ToolCapability::Network) => "network",
        None => "unknown",
    }
}

pub(super) fn tool_approval_cache_key(
    tool_name: &str,
    capability: Option<alan_protocol::ToolCapability>,
    governance: &alan_protocol::GovernanceConfig,
    dynamic_tool_spec: Option<&alan_protocol::DynamicToolSpec>,
    arguments: &serde_json::Value,
) -> ToolApprovalCacheKey {
    let governance_profile = match governance.profile {
        alan_protocol::GovernanceProfile::Conservative => "conservative",
        alan_protocol::GovernanceProfile::Autonomous => "autonomous",
    };
    ToolApprovalCacheKey {
        tool_name: tool_name.to_string(),
        capability: capability_label(capability).to_string(),
        governance_profile: governance_profile.to_string(),
        dynamic_tool_spec_fingerprint: dynamic_tool_spec.map(dynamic_tool_spec_fingerprint),
        arguments_fingerprint: if !matches!(capability, Some(alan_protocol::ToolCapability::Read)) {
            Some(json_value_fingerprint(arguments))
        } else {
            None
        },
    }
}

fn dynamic_tool_spec_fingerprint(spec: &alan_protocol::DynamicToolSpec) -> String {
    let encoded = serde_json::to_vec(spec).unwrap_or_default();
    let digest = Sha256::digest(&encoded);
    format!("{digest:x}")
}

fn json_value_fingerprint(value: &serde_json::Value) -> String {
    let encoded = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(&encoded);
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_conservative_unknown_capability_escalates() {
        let policy =
            crate::policy::PolicyEngine::for_profile(crate::policy::PolicyProfile::Conservative);
        let result = evaluate_tool_policy(
            &policy,
            &alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Conservative,
                policy_path: None,
            },
            "dynamic_tool",
            &json!({"id":"123"}),
            None,
        );
        match result {
            ToolPolicyDecision::Escalate { details, .. } => {
                assert_eq!(details["capability"], "unknown");
                assert_eq!(details["policy"]["action"], "escalate");
            }
            other => panic!("expected escalation, got {:?}", other),
        }
    }

    #[test]
    fn test_autonomous_network_is_allowed_by_default() {
        let policy =
            crate::policy::PolicyEngine::for_profile(crate::policy::PolicyProfile::Autonomous);
        let result = evaluate_tool_policy(
            &policy,
            &alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: None,
            },
            "bash",
            &json!({"query":"rust"}),
            Some(alan_protocol::ToolCapability::Network),
        );
        match result {
            ToolPolicyDecision::Allow { audit } => {
                assert_eq!(audit.action, "allow");
                assert_eq!(audit.capability, "network");
            }
            other => panic!("expected allow, got {:?}", other),
        }
    }

    #[test]
    fn test_conservative_write_escalates() {
        let policy =
            crate::policy::PolicyEngine::for_profile(crate::policy::PolicyProfile::Conservative);
        let result = evaluate_tool_policy(
            &policy,
            &alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Conservative,
                policy_path: None,
            },
            "write_file",
            &json!({"path":"a.txt","content":"x"}),
            Some(alan_protocol::ToolCapability::Write),
        );
        match result {
            ToolPolicyDecision::Escalate { audit, .. } => {
                assert_eq!(audit.action, "escalate");
                assert_eq!(audit.capability, "write");
            }
            other => panic!("expected escalation, got {:?}", other),
        }
    }

    #[test]
    fn test_tool_approval_cache_key_is_stable() {
        let key = tool_approval_cache_key(
            "web_search",
            Some(alan_protocol::ToolCapability::Network),
            &alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: None,
            },
            None,
            &json!({"query":"rust"}),
        );
        assert_eq!(key.tool_name, "web_search");
        assert_eq!(key.capability, "network");
        assert_eq!(key.governance_profile, "autonomous");
        assert!(key.arguments_fingerprint.is_some());
    }

    #[test]
    fn test_builtin_tool_approval_cache_key_changes_with_arguments() {
        let governance = alan_protocol::GovernanceConfig {
            profile: alan_protocol::GovernanceProfile::Autonomous,
            policy_path: None,
        };
        let key_a = tool_approval_cache_key(
            "web_search",
            Some(alan_protocol::ToolCapability::Network),
            &governance,
            None,
            &json!({"query":"rust"}),
        );
        let key_b = tool_approval_cache_key(
            "web_search",
            Some(alan_protocol::ToolCapability::Network),
            &governance,
            None,
            &json!({"query":"golang"}),
        );
        assert_ne!(key_a.arguments_fingerprint, key_b.arguments_fingerprint);
    }

    #[test]
    fn test_dynamic_tool_approval_cache_key_changes_with_spec() {
        let spec_v1 = alan_protocol::DynamicToolSpec {
            name: "dyn_tool".to_string(),
            description: "v1".to_string(),
            parameters: json!({"type": "object"}),
            capability: Some(alan_protocol::ToolCapability::Network),
        };
        let spec_v2 = alan_protocol::DynamicToolSpec {
            description: "v2".to_string(),
            ..spec_v1.clone()
        };

        let governance = alan_protocol::GovernanceConfig {
            profile: alan_protocol::GovernanceProfile::Autonomous,
            policy_path: None,
        };
        let key_v1 = tool_approval_cache_key(
            "dyn_tool",
            Some(alan_protocol::ToolCapability::Network),
            &governance,
            Some(&spec_v1),
            &json!({"id":"1"}),
        );
        let key_v2 = tool_approval_cache_key(
            "dyn_tool",
            Some(alan_protocol::ToolCapability::Network),
            &governance,
            Some(&spec_v2),
            &json!({"id":"1"}),
        );

        assert_ne!(
            key_v1.dynamic_tool_spec_fingerprint,
            key_v2.dynamic_tool_spec_fingerprint
        );
        assert!(key_v1.dynamic_tool_spec_fingerprint.is_some());
        assert!(key_v2.dynamic_tool_spec_fingerprint.is_some());
    }
}
