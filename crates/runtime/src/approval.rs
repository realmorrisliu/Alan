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
