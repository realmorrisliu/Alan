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
    capability: alan_protocol::ToolCapability,
    current_cwd: Option<&std::path::Path>,
) -> ToolPolicyDecision {
    let sandbox_backend = crate::tools::Sandbox::backend_name_static();
    if let Some(reason) = bash_shape_preflight_reason(tool_name, arguments) {
        return ToolPolicyDecision::Forbidden {
            reason: reason.clone(),
            audit: alan_protocol::ToolDecisionAudit {
                policy_source: "sandbox_preflight".to_string(),
                rule_id: None,
                action: "deny".to_string(),
                reason: Some(reason),
                capability: capability_label(capability).to_string(),
                sandbox_backend: sandbox_backend.to_string(),
            },
        };
    }

    let policy_decision = policy_engine.evaluate(crate::policy::PolicyContext {
        tool_name,
        arguments,
        capability,
        cwd: current_cwd,
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

fn bash_shape_preflight_reason(tool_name: &str, arguments: &serde_json::Value) -> Option<String> {
    if tool_name != "bash" {
        return None;
    }

    let command = arguments
        .get("command")
        .and_then(serde_json::Value::as_str)?;
    crate::tools::Sandbox::bash_preflight_reason(command)
}

pub(super) fn capability_label(capability: alan_protocol::ToolCapability) -> &'static str {
    match capability {
        alan_protocol::ToolCapability::Read => "read",
        alan_protocol::ToolCapability::Write => "write",
        alan_protocol::ToolCapability::Network => "network",
        alan_protocol::ToolCapability::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;
    use tempfile::TempDir;

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
            alan_protocol::ToolCapability::Unknown,
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
            alan_protocol::ToolCapability::Network,
            None,
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
    fn test_bash_shape_preflight_blocks_unsupported_wrapper_before_policy_allow() {
        let policy =
            crate::policy::PolicyEngine::for_profile(crate::policy::PolicyProfile::Autonomous);
        let result = evaluate_tool_policy(
            &policy,
            &alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: None,
            },
            "bash",
            &json!({"command":"bash -lc 'rg TODO src'"}),
            alan_protocol::ToolCapability::Unknown,
            None,
        );
        match result {
            ToolPolicyDecision::Forbidden { reason, audit } => {
                assert!(
                    reason.contains("rejects nested command evaluators")
                        || reason.contains("rejects shell wrappers")
                );
                assert_eq!(audit.policy_source, "sandbox_preflight");
                assert_eq!(audit.action, "deny");
            }
            other => panic!("expected preflight denial, got {:?}", other),
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
            alan_protocol::ToolCapability::Write,
            None,
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
    fn test_tool_policy_uses_current_cwd_for_relative_path_prefix_matching() {
        let tmp = TempDir::new().unwrap();
        let policy_dir = tmp.path().join("workspace-alan");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("policy.yaml"),
            r#"
rules:
  - id: review-deploy
    tool: "*"
    capability: write
    match_path_prefix: "deploy/"
    action: escalate
    reason: deploy config updates require escalation
default_action: allow
"#,
        )
        .unwrap();
        let policy = crate::policy::PolicyEngine::load_or_profile(
            Some(policy_dir.as_path()),
            crate::policy::PolicyProfile::Autonomous,
        );
        let result = evaluate_tool_policy(
            &policy,
            &alan_protocol::GovernanceConfig {
                profile: alan_protocol::GovernanceProfile::Autonomous,
                policy_path: None,
            },
            "write_file",
            &json!({"path":"../deploy/prod.yaml","content":"version = 2"}),
            alan_protocol::ToolCapability::Write,
            Some(Path::new("/workspace/repo/src")),
        );
        match result {
            ToolPolicyDecision::Escalate { audit, .. } => {
                assert_eq!(audit.action, "escalate");
                assert_eq!(audit.rule_id.as_deref(), Some("review-deploy"));
            }
            other => panic!("expected escalation, got {:?}", other),
        }
    }
}
