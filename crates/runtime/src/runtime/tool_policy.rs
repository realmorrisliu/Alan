use crate::approval::ToolApprovalCacheKey;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(super) enum ToolPolicyDecision {
    Allow,
    RequireApproval {
        summary: String,
        details: serde_json::Value,
    },
    Forbidden {
        reason: String,
    },
}

pub(super) fn evaluate_tool_policy(
    approval_policy: alan_protocol::ApprovalPolicy,
    sandbox_mode: alan_protocol::SandboxMode,
    tool_name: &str,
    arguments: &serde_json::Value,
    capability: Option<alan_protocol::ToolCapability>,
) -> ToolPolicyDecision {
    let unknown_capability = capability.is_none();
    let sandbox_forbidden = match (sandbox_mode, capability) {
        (alan_protocol::SandboxMode::DangerFullAccess, _) => false,
        (_, None) => !matches!(approval_policy, alan_protocol::ApprovalPolicy::OnRequest),
        (alan_protocol::SandboxMode::WorkspaceWrite, Some(alan_protocol::ToolCapability::Network)) => true,
        (
            alan_protocol::SandboxMode::ReadOnly,
            Some(alan_protocol::ToolCapability::Write | alan_protocol::ToolCapability::Network),
        ) => true,
        _ => false,
    };

    if sandbox_forbidden {
        let capability_label = capability_label(capability);
        return ToolPolicyDecision::Forbidden {
            reason: format!(
                "Tool '{}' ({}) is blocked by sandbox_mode={:?}",
                tool_name, capability_label, sandbox_mode
            ),
        };
    }

    let needs_approval = matches!(approval_policy, alan_protocol::ApprovalPolicy::OnRequest)
        && !matches!(capability, Some(alan_protocol::ToolCapability::Read));
    if needs_approval {
        return ToolPolicyDecision::RequireApproval {
            summary: format!("Approve tool call '{}'? ", tool_name)
                .trim()
                .to_string(),
            details: json!({
                "kind": "tool_approval",
                "tool_name": tool_name,
                "arguments": arguments,
                "capability": capability_label(capability),
                "unknown_capability_requires_explicit_approval": unknown_capability,
                "approval_policy": approval_policy,
                "sandbox_mode": sandbox_mode
            }),
        };
    }

    ToolPolicyDecision::Allow
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
    sandbox_mode: alan_protocol::SandboxMode,
    dynamic_tool_spec: Option<&alan_protocol::DynamicToolSpec>,
    arguments: &serde_json::Value,
) -> ToolApprovalCacheKey {
    let sandbox = match sandbox_mode {
        alan_protocol::SandboxMode::ReadOnly => "read_only",
        alan_protocol::SandboxMode::WorkspaceWrite => "workspace_write",
        alan_protocol::SandboxMode::DangerFullAccess => "danger_full_access",
    };
    ToolApprovalCacheKey {
        tool_name: tool_name.to_string(),
        capability: capability_label(capability).to_string(),
        sandbox: sandbox.to_string(),
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
    fn test_unknown_capability_is_blocked_outside_full_access() {
        let result = evaluate_tool_policy(
            alan_protocol::ApprovalPolicy::Never,
            alan_protocol::SandboxMode::WorkspaceWrite,
            "dynamic_tool",
            &json!({}),
            None,
        );
        match result {
            ToolPolicyDecision::Forbidden { reason } => assert!(reason.contains("unknown")),
            other => panic!("expected forbidden, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown_capability_requires_approval_when_on_request() {
        let result = evaluate_tool_policy(
            alan_protocol::ApprovalPolicy::OnRequest,
            alan_protocol::SandboxMode::WorkspaceWrite,
            "dynamic_tool",
            &json!({"id":"123"}),
            None,
        );
        match result {
            ToolPolicyDecision::RequireApproval { details, .. } => {
                assert_eq!(details["capability"], "unknown");
                assert_eq!(
                    details["unknown_capability_requires_explicit_approval"],
                    true
                );
            }
            other => panic!("expected approval, got {:?}", other),
        }
    }

    #[test]
    fn test_network_capability_requires_approval_under_on_request() {
        let result = evaluate_tool_policy(
            alan_protocol::ApprovalPolicy::OnRequest,
            alan_protocol::SandboxMode::DangerFullAccess,
            "web_search",
            &json!({"query":"rust"}),
            Some(alan_protocol::ToolCapability::Network),
        );
        match result {
            ToolPolicyDecision::RequireApproval { details, .. } => {
                assert_eq!(details["capability"], "network");
            }
            other => panic!("expected approval, got {:?}", other),
        }
    }

    #[test]
    fn test_tool_approval_cache_key_is_stable() {
        let key = tool_approval_cache_key(
            "web_search",
            Some(alan_protocol::ToolCapability::Network),
            alan_protocol::SandboxMode::WorkspaceWrite,
            None,
            &json!({"query":"rust"}),
        );
        assert_eq!(key.tool_name, "web_search");
        assert_eq!(key.capability, "network");
        assert_eq!(key.sandbox, "workspace_write");
        assert!(key.arguments_fingerprint.is_some());
    }

    #[test]
    fn test_builtin_tool_approval_cache_key_changes_with_arguments() {
        let key_a = tool_approval_cache_key(
            "web_search",
            Some(alan_protocol::ToolCapability::Network),
            alan_protocol::SandboxMode::WorkspaceWrite,
            None,
            &json!({"query":"rust"}),
        );
        let key_b = tool_approval_cache_key(
            "web_search",
            Some(alan_protocol::ToolCapability::Network),
            alan_protocol::SandboxMode::WorkspaceWrite,
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

        let key_v1 = tool_approval_cache_key(
            "dyn_tool",
            Some(alan_protocol::ToolCapability::Network),
            alan_protocol::SandboxMode::WorkspaceWrite,
            Some(&spec_v1),
            &json!({"id":"1"}),
        );
        let key_v2 = tool_approval_cache_key(
            "dyn_tool",
            Some(alan_protocol::ToolCapability::Network),
            alan_protocol::SandboxMode::WorkspaceWrite,
            Some(&spec_v2),
            &json!({"id":"1"}),
        );

        assert_ne!(key_v1.dynamic_tool_spec_fingerprint, key_v2.dynamic_tool_spec_fingerprint);
        assert!(key_v1.dynamic_tool_spec_fingerprint.is_some());
        assert!(key_v2.dynamic_tool_spec_fingerprint.is_some());
    }
}
