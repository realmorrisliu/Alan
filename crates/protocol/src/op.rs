//! User operation definitions (Submission Queue).
//!
//! These are the operations that users can submit to the agent.

use serde::{Deserialize, Serialize};

use crate::ContentPart;

/// Tool execution approval policy for a session/runtime.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    /// Prompt the user before risky tool calls (network/write).
    #[default]
    OnRequest,
    /// Never prompt; execute allowed tools directly.
    Never,
}

/// Coarse sandbox mode used by the runtime tool policy layer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    /// Read-only/no-network tool policy. Blocks write + network tools.
    ReadOnly,
    /// Allows write tools but blocks network tools.
    #[default]
    WorkspaceWrite,
    /// Allows all tool capability classes.
    DangerFullAccess,
}

/// Coarse capability class for tool policy decisions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapability {
    Read,
    Write,
    Network,
}

/// Status for a plan item in transport-level progress updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanItemStatus {
    Pending,
    InProgress,
    Completed,
}

/// Transport-level plan item for UI synchronization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanItem {
    pub id: String,
    pub content: String,
    pub status: PlanItemStatus,
}

/// Option for a structured user-input question.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredInputOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Structured question shown to the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredInputQuestion {
    pub id: String,
    pub label: String,
    pub prompt: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<StructuredInputOption>,
}

/// Session-scoped dynamic tool definition provided by the client/frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<ToolCapability>,
}

/// User-submitted operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Op {
    // ========================================================================
    // New unified operations (Phase 2)
    // ========================================================================
    /// Start a new reasoning turn.
    /// This is a user-initiated conversation turn with full context metadata.
    Turn {
        /// Content parts for the turn input.
        parts: Vec<ContentPart>,
        /// Optional turn context metadata.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context: Option<TurnContext>,
    },

    /// Append user input within an existing turn (steering message).
    /// The engine should not reset state, but buffer or inject the input.
    Input {
        /// User's input content parts.
        parts: Vec<ContentPart>,
    },

    /// Resume a suspended Yield request.
    /// Unified replacement for Confirm, StructuredUserInput, DynamicToolResult.
    Resume {
        /// The request_id from the corresponding Yield event.
        request_id: String,
        /// Resume payload content.
        content: Vec<ContentPart>,
    },

    /// Interrupt current execution.
    Interrupt,

    /// Register or replace client-provided dynamic tools for this session.
    RegisterDynamicTools {
        /// Tool definitions exposed to the LLM for this session.
        tools: Vec<DynamicToolSpec>,
    },

    /// Compact the current session context (manual trigger)
    Compact,

    /// Roll back the last N user turns from in-memory session context
    Rollback {
        /// Number of user turns to remove (must be >= 1)
        turns: u32,
    },
}

/// Turn context metadata — attached to Turn ops.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnContext {
    /// Optional workspace ID to route this turn to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// Optional domain identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// A submission wrapping an operation with an ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    /// Unique submission ID
    pub id: String,
    /// The operation being submitted
    pub op: Op,
}

impl Submission {
    /// Create a new submission with a generated UUID
    pub fn new(op: Op) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            op,
        }
    }

    /// Create a new submission with a specific ID (useful for testing)
    #[cfg(test)]
    pub fn with_id(id: &str, op: Op) -> Self {
        Self {
            id: id.to_string(),
            op,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_serialization_compact() {
        let op = Op::Compact;
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("compact"));
        let deserialized: Op = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Op::Compact));
    }

    #[test]
    fn test_op_serialization_rollback() {
        let op = Op::Rollback { turns: 2 };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("rollback"));
        assert!(json.contains("\"turns\":2"));
        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Rollback { turns } => assert_eq!(turns, 2),
            _ => panic!("Expected Rollback variant"),
        }
    }

    #[test]
    fn test_approval_policy_serialization() {
        let json = serde_json::to_string(&ApprovalPolicy::OnRequest).unwrap();
        assert_eq!(json, "\"on_request\"");
        let parsed: ApprovalPolicy = serde_json::from_str("\"never\"").unwrap();
        assert_eq!(parsed, ApprovalPolicy::Never);
    }

    #[test]
    fn test_sandbox_mode_serialization() {
        let json = serde_json::to_string(&SandboxMode::WorkspaceWrite).unwrap();
        assert_eq!(json, "\"workspace_write\"");
        let parsed: SandboxMode = serde_json::from_str("\"read_only\"").unwrap();
        assert_eq!(parsed, SandboxMode::ReadOnly);
    }

    #[test]
    fn test_submission_new() {
        let op = Op::Interrupt;
        let submission = Submission::new(op.clone());

        assert!(!submission.id.is_empty());
        assert!(matches!(submission.op, Op::Interrupt));
    }

    #[test]
    fn test_submission_serialization() {
        let submission = Submission::with_id("test-id-123", Op::Interrupt);

        let json = serde_json::to_string(&submission).unwrap();
        assert!(json.contains("test-id-123"));
        assert!(json.contains("interrupt"));

        let deserialized: Submission = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id-123");
        assert!(matches!(deserialized.op, Op::Interrupt));
    }

    #[test]
    fn test_op_serialization_register_dynamic_tools() {
        let op = Op::RegisterDynamicTools {
            tools: vec![DynamicToolSpec {
                name: "lookup_ticket".to_string(),
                description: "Lookup ticket".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": { "id": { "type": "string" } },
                    "required": ["id"]
                }),
                capability: None,
            }],
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("register_dynamic_tools"));
        let parsed: Op = serde_json::from_str(&json).unwrap();
        match parsed {
            Op::RegisterDynamicTools { tools } => {
                assert_eq!(tools.len(), 1);
                assert_eq!(tools[0].name, "lookup_ticket");
            }
            _ => panic!("Expected RegisterDynamicTools"),
        }
    }

    #[test]
    fn test_register_dynamic_tools_legacy_payload_without_capability_is_compatible() {
        let json = r#"{"type":"register_dynamic_tools","tools":[{"name":"lookup_ticket","description":"Lookup ticket","parameters":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}}]}"#;

        let parsed: Op = serde_json::from_str(json).unwrap();
        match parsed {
            Op::RegisterDynamicTools { tools } => {
                assert_eq!(tools.len(), 1);
                assert_eq!(tools[0].capability, None);
            }
            _ => panic!("Expected RegisterDynamicTools"),
        }
    }

    // ========================================================================
    // Tests for new Phase 2 Op variants
    // ========================================================================

    #[test]
    fn test_op_serialization_turn() {
        let op = Op::Turn {
            parts: vec![ContentPart::text("Hello agent")],
            context: Some(TurnContext {
                workspace_id: Some("ws-1".to_string()),
                domain: Some("sales".to_string()),
            }),
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("turn"));
        assert!(json.contains("Hello agent"));
        assert!(json.contains("ws-1"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Turn { parts, context } => {
                assert_eq!(parts.len(), 1);
                assert_eq!(parts[0].as_text(), Some("Hello agent"));
                let ctx = context.unwrap();
                assert_eq!(ctx.workspace_id, Some("ws-1".to_string()));
                assert_eq!(ctx.domain, Some("sales".to_string()));
            }
            _ => panic!("Expected Turn variant"),
        }
    }

    #[test]
    fn test_op_serialization_turn_minimal() {
        let op = Op::Turn {
            parts: vec![ContentPart::text("Hi")],
            context: None,
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("turn"));
        assert!(!json.contains("context")); // None should be skipped

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Turn { parts, context } => {
                assert_eq!(parts[0].as_text(), Some("Hi"));
                assert!(context.is_none());
            }
            _ => panic!("Expected Turn variant"),
        }
    }

    #[test]
    fn test_op_serialization_input() {
        let op = Op::Input {
            parts: vec![ContentPart::text("follow up")],
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("input"));
        assert!(json.contains("follow up"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Input { parts } => assert_eq!(parts[0].as_text(), Some("follow up")),
            _ => panic!("Expected Input variant"),
        }
    }

    #[test]
    fn test_op_serialization_resume() {
        let op = Op::Resume {
            request_id: "yield-123".to_string(),
            content: vec![ContentPart::structured(
                serde_json::json!({"choice": "approve"}),
            )],
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("resume"));
        assert!(json.contains("yield-123"));
        assert!(json.contains("\"content\""));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Resume {
                request_id,
                content,
            } => {
                assert_eq!(request_id, "yield-123");
                assert_eq!(content.len(), 1);
                match &content[0] {
                    ContentPart::Structured { data } => assert_eq!(data["choice"], "approve"),
                    _ => panic!("Expected structured resume content"),
                }
            }
            _ => panic!("Expected Resume variant"),
        }
    }

    #[test]
    fn test_op_serialization_interrupt() {
        let op = Op::Interrupt;

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("interrupt"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Op::Interrupt));
    }
}
