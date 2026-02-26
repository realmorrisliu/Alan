//! Tool approval and pending interactive request types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Cache key for a tool approval decision scoped to a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolApprovalCacheKey {
    pub tool_name: String,
    pub capability: String,
    pub sandbox: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_tool_spec_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments_fingerprint: Option<String>,
}

impl fmt::Display for ToolApprovalCacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(self) {
            Ok(encoded) => f.write_str(&encoded),
            Err(_) => write!(
                f,
                "tool={} capability={} sandbox={}",
                self.tool_name, self.capability, self.sandbox
            ),
        }
    }
}

/// Represents the decision for a tool approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolApprovalDecision {
    /// The tool is approved for the entire session.
    ApprovedForSession,
}

#[derive(Debug, Clone)]
pub struct PendingStructuredInputRequest {
    pub request_id: String,
    pub title: String,
    pub prompt: String,
    pub questions: Vec<alan_protocol::StructuredInputQuestion>,
}

#[derive(Debug, Clone)]
pub struct PendingConfirmation {
    pub checkpoint_id: String,
    pub checkpoint_type: String,
    pub summary: String,
    pub details: serde_json::Value,
    pub options: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PendingDynamicToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_approval_cache_key_display() {
        let key = ToolApprovalCacheKey {
            tool_name: "read_file".to_string(),
            capability: "read".to_string(),
            sandbox: "strict".to_string(),
            dynamic_tool_spec_fingerprint: None,
            arguments_fingerprint: None,
        };
        let display = format!("{}", key);
        assert!(display.contains("read_file"));
        assert!(display.contains("read"));
    }

    #[test]
    fn test_tool_approval_cache_key_with_fingerprints() {
        let key = ToolApprovalCacheKey {
            tool_name: "bash".to_string(),
            capability: "exec".to_string(),
            sandbox: "strict".to_string(),
            dynamic_tool_spec_fingerprint: Some("abc123".to_string()),
            arguments_fingerprint: Some("def456".to_string()),
        };
        let json = serde_json::to_string(&key).unwrap();
        assert!(json.contains("bash"));
        assert!(json.contains("abc123"));
        assert!(json.contains("def456"));
    }

    #[test]
    fn test_tool_approval_cache_key_serde_roundtrip() {
        let key = ToolApprovalCacheKey {
            tool_name: "write_file".to_string(),
            capability: "write".to_string(),
            sandbox: "permissive".to_string(),
            dynamic_tool_spec_fingerprint: Some("fp1".to_string()),
            arguments_fingerprint: Some("fp2".to_string()),
        };
        let json = serde_json::to_string(&key).unwrap();
        let decoded: ToolApprovalCacheKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key.tool_name, decoded.tool_name);
        assert_eq!(key.capability, decoded.capability);
        assert_eq!(key.sandbox, decoded.sandbox);
        assert_eq!(
            key.dynamic_tool_spec_fingerprint,
            decoded.dynamic_tool_spec_fingerprint
        );
        assert_eq!(key.arguments_fingerprint, decoded.arguments_fingerprint);
    }

    #[test]
    fn test_tool_approval_decision_serde() {
        let decision = ToolApprovalDecision::ApprovedForSession;
        let json = serde_json::to_string(&decision).unwrap();
        assert_eq!(json, "\"ApprovedForSession\"");

        let decoded: ToolApprovalDecision = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ToolApprovalDecision::ApprovedForSession));
    }

    #[test]
    fn test_pending_structured_input_request_creation() {
        let request = PendingStructuredInputRequest {
            request_id: "req-123".to_string(),
            title: "Test Title".to_string(),
            prompt: "Test Prompt".to_string(),
            questions: vec![alan_protocol::StructuredInputQuestion {
                id: "q1".to_string(),
                label: "Question 1".to_string(),
                prompt: "What is your name?".to_string(),
                required: true,
                options: vec![],
            }],
        };
        assert_eq!(request.request_id, "req-123");
        assert_eq!(request.title, "Test Title");
        assert_eq!(request.questions.len(), 1);
    }

    #[test]
    fn test_pending_confirmation_creation() {
        let pending = PendingConfirmation {
            checkpoint_id: "chk-123".to_string(),
            checkpoint_type: "tool_approval".to_string(),
            summary: "Approve file write?".to_string(),
            details: serde_json::json!({"path": "/test/file.txt"}),
            options: vec!["approve".to_string(), "reject".to_string()],
        };
        assert_eq!(pending.checkpoint_id, "chk-123");
        assert_eq!(pending.checkpoint_type, "tool_approval");
        assert_eq!(pending.options.len(), 2);
    }

    #[test]
    fn test_pending_dynamic_tool_call_creation() {
        let call = PendingDynamicToolCall {
            call_id: "call-123".to_string(),
            tool_name: "custom_tool".to_string(),
            arguments: serde_json::json!({"arg1": "value1"}),
        };
        assert_eq!(call.call_id, "call-123");
        assert_eq!(call.tool_name, "custom_tool");
    }
}
