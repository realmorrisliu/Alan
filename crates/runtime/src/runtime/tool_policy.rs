use serde_json::json;

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
    let sandbox_backend = crate::tools::Sandbox::backend_name_static();
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
                sandbox_backend: sandbox_backend.to_string(),
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
                sandbox_backend: sandbox_backend.to_string(),
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
                "sandbox_backend": sandbox_backend
            }),
            audit: alan_protocol::ToolDecisionAudit {
                policy_source,
                rule_id,
                action: "escalate".to_string(),
                reason: policy_reason,
                capability: capability_kind,
                sandbox_backend: sandbox_backend.to_string(),
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
}
