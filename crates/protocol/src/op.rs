//! User operation definitions (Submission Queue).
//!
//! These are the operations that users can submit to the agent.

use serde::{Deserialize, Serialize};

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

/// User answer for a structured input request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredInputAnswer {
    pub question_id: String,
    pub value: String,
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
    /// Start a new task with explicit domain selection (generic entrypoint)
    StartTask {
        /// Optional existing workspace ID to route this task to
        workspace_id: Option<String>,
        /// Optional domain identifier (e.g. sourcing, sales, ops)
        domain: Option<String>,
        /// Main user input for the task
        input: String,
        /// Optional attachment paths or URLs relevant to the task
        attachments: Vec<String>,
    },

    /// User confirms a checkpoint
    Confirm {
        /// ID of the checkpoint being confirmed
        checkpoint_id: String,
        /// User's choice
        choice: ConfirmChoice,
        /// Optional modifications if choice is Modify
        modifications: Option<String>,
    },

    /// User provides additional input
    UserInput {
        /// User's input content
        content: String,
    },

    /// User answers a previously requested structured input form.
    StructuredUserInput {
        /// Request id from `structured_user_input_requested`
        request_id: String,
        /// Structured answers keyed by question id
        answers: Vec<StructuredInputAnswer>,
    },

    /// Register or replace client-provided dynamic tools for this session.
    RegisterDynamicTools {
        /// Tool definitions exposed to the LLM for this session.
        tools: Vec<DynamicToolSpec>,
    },

    /// Return the result for a pending dynamic tool call.
    DynamicToolResult {
        /// Call id from `dynamic_tool_call_requested`
        call_id: String,
        /// Whether the dynamic tool call succeeded.
        success: bool,
        /// Tool result payload to inject as tool output.
        result: serde_json::Value,
    },

    /// Compact the current session context (manual trigger)
    Compact,

    /// Roll back the last N user turns from in-memory session context
    Rollback {
        /// Number of user turns to remove (must be >= 1)
        num_turns: u32,
    },

    /// Cancel the current task
    Cancel,
}

/// Choices for confirmation checkpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmChoice {
    /// Approve and proceed
    Approve,
    /// Request modifications
    Modify,
    /// Reject and stop
    Reject,
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
    fn test_op_serialization_start_task() {
        let op = Op::StartTask {
            workspace_id: Some("workspace-123".to_string()),
            domain: Some("sales".to_string()),
            input: "Find fintech prospects".to_string(),
            attachments: vec!["brief.md".to_string(), "https://example.com".to_string()],
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("start_task"));
        assert!(json.contains("workspace-123"));
        assert!(json.contains("sales"));
        assert!(json.contains("Find fintech prospects"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::StartTask {
                workspace_id,
                domain,
                input,
                attachments,
            } => {
                assert_eq!(workspace_id, Some("workspace-123".to_string()));
                assert_eq!(domain, Some("sales".to_string()));
                assert_eq!(input, "Find fintech prospects");
                assert_eq!(attachments.len(), 2);
            }
            _ => panic!("Expected StartTask variant"),
        }
    }

    #[test]
    fn test_op_serialization_confirm() {
        let op = Op::Confirm {
            checkpoint_id: "cp-123".to_string(),
            choice: ConfirmChoice::Approve,
            modifications: None,
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("confirm"));
        assert!(json.contains("cp-123"));
        assert!(json.contains("approve"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Confirm {
                checkpoint_id,
                choice,
                modifications,
            } => {
                assert_eq!(checkpoint_id, "cp-123");
                assert!(matches!(choice, ConfirmChoice::Approve));
                assert!(modifications.is_none());
            }
            _ => panic!("Expected Confirm variant"),
        }
    }

    #[test]
    fn test_op_serialization_confirm_with_modifications() {
        let op = Op::Confirm {
            checkpoint_id: "cp-456".to_string(),
            choice: ConfirmChoice::Modify,
            modifications: Some("Change quantity to 100".to_string()),
        };

        let json = serde_json::to_string(&op).unwrap();
        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Confirm {
                choice,
                modifications,
                ..
            } => {
                assert!(matches!(choice, ConfirmChoice::Modify));
                assert_eq!(modifications, Some("Change quantity to 100".to_string()));
            }
            _ => panic!("Expected Confirm variant"),
        }
    }

    #[test]
    fn test_confirm_choice_reject() {
        let op = Op::Confirm {
            checkpoint_id: "cp-789".to_string(),
            choice: ConfirmChoice::Reject,
            modifications: None,
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("reject"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Confirm { choice, .. } => {
                assert!(matches!(choice, ConfirmChoice::Reject));
            }
            _ => panic!("Expected Confirm variant"),
        }
    }

    #[test]
    fn test_op_serialization_user_input() {
        let op = Op::UserInput {
            content: "Here is more info".to_string(),
        };

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("user_input"));
        assert!(json.contains("Here is more info"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::UserInput { content } => {
                assert_eq!(content, "Here is more info");
            }
            _ => panic!("Expected UserInput variant"),
        }
    }

    #[test]
    fn test_op_serialization_cancel() {
        let op = Op::Cancel;

        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("cancel"));

        let deserialized: Op = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Op::Cancel));
    }

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
        let op = Op::Rollback { num_turns: 2 };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("rollback"));
        assert!(json.contains("\"num_turns\":2"));
        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::Rollback { num_turns } => assert_eq!(num_turns, 2),
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
        let op = Op::Cancel;
        let submission = Submission::new(op.clone());

        assert!(!submission.id.is_empty());
        assert!(matches!(submission.op, Op::Cancel));
    }

    #[test]
    fn test_submission_serialization() {
        let submission = Submission::with_id("test-id-123", Op::Cancel);

        let json = serde_json::to_string(&submission).unwrap();
        assert!(json.contains("test-id-123"));
        assert!(json.contains("cancel"));

        let deserialized: Submission = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id-123");
        assert!(matches!(deserialized.op, Op::Cancel));
    }

    #[test]
    fn test_start_task_minimal_fields() {
        let op = Op::StartTask {
            workspace_id: None,
            domain: None,
            input: "Just a prompt".to_string(),
            attachments: vec![],
        };

        let json = serde_json::to_string(&op).unwrap();
        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::StartTask {
                workspace_id,
                domain,
                attachments,
                ..
            } => {
                assert!(workspace_id.is_none());
                assert!(domain.is_none());
                assert!(attachments.is_empty());
            }
            _ => panic!("Expected StartTask variant"),
        }
    }

    #[test]
    fn test_start_task_with_multiple_attachments() {
        let op = Op::StartTask {
            workspace_id: None,
            domain: Some("sourcing".to_string()),
            input: "Multi-attachment test".to_string(),
            attachments: vec![
                "doc1.pdf".to_string(),
                "doc2.pdf".to_string(),
                "doc3.pdf".to_string(),
            ],
        };

        let json = serde_json::to_string(&op).unwrap();
        let deserialized: Op = serde_json::from_str(&json).unwrap();
        match deserialized {
            Op::StartTask { attachments, .. } => {
                assert_eq!(attachments.len(), 3);
                assert_eq!(attachments[0], "doc1.pdf");
                assert_eq!(attachments[2], "doc3.pdf");
            }
            _ => panic!("Expected StartTask variant"),
        }
    }

    #[test]
    fn test_op_serialization_structured_user_input() {
        let op = Op::StructuredUserInput {
            request_id: "req-1".to_string(),
            answers: vec![StructuredInputAnswer {
                question_id: "team".to_string(),
                value: "sales".to_string(),
            }],
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("structured_user_input"));
        let parsed: Op = serde_json::from_str(&json).unwrap();
        match parsed {
            Op::StructuredUserInput {
                request_id,
                answers,
            } => {
                assert_eq!(request_id, "req-1");
                assert_eq!(answers.len(), 1);
                assert_eq!(answers[0].question_id, "team");
            }
            _ => panic!("Expected StructuredUserInput"),
        }
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

    #[test]
    fn test_op_serialization_dynamic_tool_result() {
        let op = Op::DynamicToolResult {
            call_id: "dyn-1".to_string(),
            success: true,
            result: serde_json::json!({"ok": true}),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("dynamic_tool_result"));
        let parsed: Op = serde_json::from_str(&json).unwrap();
        match parsed {
            Op::DynamicToolResult {
                call_id,
                success,
                result,
            } => {
                assert_eq!(call_id, "dyn-1");
                assert!(success);
                assert_eq!(result["ok"], true);
            }
            _ => panic!("Expected DynamicToolResult"),
        }
    }
}
