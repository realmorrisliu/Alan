//! System event definitions (Event Queue).
//!
//! These are events emitted by the agent to notify frontends.

use crate::op::{PlanItem, StructuredInputQuestion};
use serde::{Deserialize, Serialize};

/// Events emitted by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // ========================================================================
    // New unified events (Phase 3)
    // ========================================================================

    /// Streaming text output from the assistant.
    /// Replaces MessageDelta + MessageDeltaChunk.
    TextDelta {
        /// Incremental text chunk.
        chunk: String,
        /// Whether this is the final chunk of the current text stream.
        is_final: bool,
    },

    /// Streaming thinking/reasoning output.
    /// Replaces Thinking + ThinkingComplete + ReasoningDelta.
    ThinkingDelta {
        /// Incremental thinking text chunk.
        chunk: String,
        /// Whether this is the final chunk of the current thinking stream.
        is_final: bool,
    },

    /// Engine is suspended, waiting for external input.
    /// Unified replacement for ConfirmationRequired, StructuredUserInputRequested,
    /// and DynamicToolCallRequested. Client responds with Op::Resume.
    Yield {
        /// Unique request ID — client uses this in Op::Resume.
        request_id: String,
        /// Kind of yield — tells the client what UI to render.
        kind: YieldKind,
        /// Payload with details for the specific yield kind.
        payload: serde_json::Value,
    },

    // ========================================================================
    // Existing events (some deprecated, some kept as-is)
    // ========================================================================

    /// Start of a new logical user-initiated turn.
    TurnStarted {},

    /// End of the current logical turn.
    TurnCompleted {},

    /// Agent is thinking/processing.
    /// Deprecated: use ThinkingDelta instead.
    Thinking {
        /// Description of what the agent is thinking about
        message: String,
    },

    /// Thinking phase has completed.
    /// Deprecated: use ThinkingDelta { is_final: true } instead.
    ThinkingComplete {},

    /// Streaming reasoning content (think tags / reasoning tokens).
    /// Deprecated: use ThinkingDelta instead.
    ReasoningDelta {
        /// Incremental reasoning text chunk
        chunk: String,
        /// Whether this is the final chunk
        is_final: bool,
    },

    /// Streaming message content from the agent (complete message).
    /// Deprecated: use TextDelta instead.
    MessageDelta {
        /// Complete message content
        content: String,
    },

    /// Streaming message chunk for real-time typing effect.
    /// Deprecated: use TextDelta instead.
    MessageDeltaChunk {
        /// Incremental text chunk (can be a character, word, or sentence fragment)
        chunk: String,
        /// Whether this is the final chunk
        is_final: bool,
    },

    /// Agent requires user confirmation to proceed.
    /// Deprecated: use Yield { kind: YieldKind::Confirmation, .. } instead.
    ConfirmationRequired {
        /// Unique checkpoint ID
        checkpoint_id: String,
        /// Domain-defined checkpoint kind (stringly-typed to avoid protocol churn)
        checkpoint_type: String,
        /// Summary for the user
        summary: String,
        /// Detailed data for review
        details: serde_json::Value,
        /// Available options (e.g., ["approve", "modify", "reject"])
        options: Vec<String>,
    },

    /// Request structured user input (transport-level, not free-text only).
    /// Deprecated: use Yield { kind: YieldKind::StructuredInput, .. } instead.
    StructuredUserInputRequested {
        /// Request id used when client sends `structured_user_input`.
        request_id: String,
        /// Short title shown to the user.
        title: String,
        /// Prompt/context for the request.
        prompt: String,
        /// Questions with ids and optional choices.
        questions: Vec<StructuredInputQuestion>,
    },

    /// A tool call has started
    ToolCallStarted {
        /// Stable tool call id emitted by the LLM/tool loop
        call_id: String,
        /// Name of the tool being called
        tool_name: String,
        /// Arguments passed to the tool
        arguments: serde_json::Value,
    },

    /// A tool call has completed
    ToolCallCompleted {
        /// Stable tool call id emitted by the LLM/tool loop
        call_id: String,
        /// Name of the tool that completed
        tool_name: String,
        /// Result from the tool
        result: serde_json::Value,
        /// Whether the call was successful
        success: bool,
    },

    /// Task has been completed
    TaskCompleted {
        /// Summary of what was accomplished
        summary: String,
        /// Final results
        results: serde_json::Value,
    },

    /// Context was compacted into a summary
    ContextCompacted {},

    /// Transport-level plan synchronization event (e.g. from `todo_list` / planner tools).
    PlanUpdated {
        /// Optional summary/explanation of the plan change.
        explanation: Option<String>,
        /// Full normalized plan items.
        items: Vec<PlanItem>,
    },

    /// Session history was rolled back in-memory
    SessionRolledBack {
        /// Number of turns requested
        num_turns: u32,
        /// Number of messages removed from context
        removed_messages: usize,
    },

    /// Subscriber lagged behind the event stream and should replay from cursor.
    StreamLagged {
        /// Number of events dropped for this subscriber by the broadcast buffer.
        skipped: u64,
        /// Last successfully delivered event id for the subscriber, if any.
        replay_from_event_id: Option<String>,
    },

    /// An error occurred
    Error {
        /// Error message
        message: String,
        /// Whether the error is recoverable
        recoverable: bool,
    },

    /// Skills have been loaded for this turn
    SkillsLoaded {
        /// List of skill IDs that were loaded
        skill_ids: Vec<String>,
        /// Whether these were explicitly mentioned by user or auto-selected
        auto_selected: bool,
    },

    /// Dynamic tools were registered or replaced for this session.
    DynamicToolsRegistered { tool_names: Vec<String> },

    /// A client-provided dynamic tool must be executed out-of-process/by frontend.
    /// Deprecated: use Yield { kind: YieldKind::DynamicToolCall, .. } instead.
    DynamicToolCallRequested {
        call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
}

/// Kind of Yield — tells the client what UI to render.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum YieldKind {
    /// User confirmation required (approve/modify/reject).
    Confirmation,
    /// Structured user input form.
    StructuredInput,
    /// Dynamic tool call that must be executed by the client.
    DynamicToolCall,
}

/// Event envelope used by server transports for stable cursors and replay.
///
/// The underlying event fields (including `"type"`) are flattened so existing
/// consumers can continue reading the same event payload shape while gaining
/// metadata like `event_id`, `turn_id`, and `item_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Stable session-scoped event id (monotonic sequence encoded as string).
    pub event_id: String,
    /// Numeric session-scoped event sequence.
    pub sequence: u64,
    /// Session id the event belongs to.
    pub session_id: String,
    /// Submission id that triggered this event, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submission_id: Option<String>,
    /// Coarse logical turn id (monotonic within session).
    pub turn_id: String,
    /// Stable item id (monotonic within a turn).
    pub item_id: String,
    /// Server timestamp in unix epoch milliseconds.
    pub timestamp_ms: u64,
    /// Wrapped event payload (flattened to preserve the `type` field).
    #[serde(flatten)]
    pub event: Event,
}

/// Format a checkpoint kind (e.g. `supplier_list`) for user-facing display.
pub fn format_checkpoint_kind_label(kind: &str) -> String {
    let trimmed = kind.trim();
    if trimmed.is_empty() {
        return "Unknown".to_string();
    }

    let words: Vec<String> = trimmed
        .split(|c: char| c == '_' || c == '-' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut out = String::new();
            out.extend(first.to_uppercase());
            out.push_str(&chars.as_str().to_lowercase());
            out
        })
        .collect();

    if words.is_empty() {
        "Unknown".to_string()
    } else {
        words.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_thinking_serialization() {
        let event = Event::Thinking {
            message: "Analyzing requirements".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("thinking"));
        assert!(json.contains("Analyzing requirements"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Thinking { message } => {
                assert_eq!(message, "Analyzing requirements");
            }
            _ => panic!("Expected Thinking variant"),
        }
    }

    #[test]
    fn test_event_turn_started_serialization() {
        let event = Event::TurnStarted {};

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("turn_started"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TurnStarted {} => {}
            _ => panic!("Expected TurnStarted variant"),
        }
    }

    #[test]
    fn test_event_turn_completed_serialization() {
        let event = Event::TurnCompleted {};

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("turn_completed"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TurnCompleted {} => {}
            _ => panic!("Expected TurnCompleted variant"),
        }
    }

    #[test]
    fn test_event_thinking_complete_serialization() {
        let event = Event::ThinkingComplete {};

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("thinking_complete"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ThinkingComplete {} => {}
            _ => panic!("Expected ThinkingComplete variant"),
        }
    }

    #[test]
    fn test_event_reasoning_delta_serialization() {
        let event = Event::ReasoningDelta {
            chunk: "Let me think about this".to_string(),
            is_final: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("reasoning_delta"));
        assert!(json.contains("Let me think about this"));
        assert!(json.contains("\"is_final\":false"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ReasoningDelta { chunk, is_final } => {
                assert_eq!(chunk, "Let me think about this");
                assert!(!is_final);
            }
            _ => panic!("Expected ReasoningDelta variant"),
        }
    }

    #[test]
    fn test_event_message_delta_serialization() {
        let event = Event::MessageDelta {
            content: "Hello, world!".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("message_delta"));
        assert!(json.contains("Hello, world!"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::MessageDelta { content } => {
                assert_eq!(content, "Hello, world!");
            }
            _ => panic!("Expected MessageDelta variant"),
        }
    }

    #[test]
    fn test_event_session_rolled_back_serialization() {
        let event = Event::SessionRolledBack {
            num_turns: 2,
            removed_messages: 5,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("session_rolled_back"));
        assert!(json.contains("\"num_turns\":2"));
        assert!(json.contains("\"removed_messages\":5"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::SessionRolledBack {
                num_turns,
                removed_messages,
            } => {
                assert_eq!(num_turns, 2);
                assert_eq!(removed_messages, 5);
            }
            _ => panic!("Expected SessionRolledBack variant"),
        }
    }

    #[test]
    fn test_event_stream_lagged_serialization() {
        let event = Event::StreamLagged {
            skipped: 12,
            replay_from_event_id: Some("evt_00000012".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("stream_lagged"));
        assert!(json.contains("\"skipped\":12"));
        assert!(json.contains("evt_00000012"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::StreamLagged {
                skipped,
                replay_from_event_id,
            } => {
                assert_eq!(skipped, 12);
                assert_eq!(replay_from_event_id.as_deref(), Some("evt_00000012"));
            }
            _ => panic!("Expected StreamLagged variant"),
        }
    }

    #[test]
    fn test_event_structured_user_input_requested_serialization() {
        let event = Event::StructuredUserInputRequested {
            request_id: "req-1".to_string(),
            title: "Need Details".to_string(),
            prompt: "Please clarify".to_string(),
            questions: vec![crate::op::StructuredInputQuestion {
                id: "team".to_string(),
                label: "Team".to_string(),
                prompt: "Which team?".to_string(),
                required: true,
                options: vec![crate::op::StructuredInputOption {
                    value: "sales".to_string(),
                    label: "Sales".to_string(),
                    description: None,
                }],
            }],
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("structured_user_input_requested"));
        assert!(json.contains("\"request_id\":\"req-1\""));

        let parsed: Event = serde_json::from_str(&json).unwrap();
        match parsed {
            Event::StructuredUserInputRequested {
                request_id,
                questions,
                ..
            } => {
                assert_eq!(request_id, "req-1");
                assert_eq!(questions.len(), 1);
                assert_eq!(questions[0].id, "team");
            }
            _ => panic!("Expected StructuredUserInputRequested"),
        }
    }

    #[test]
    fn test_event_plan_updated_serialization() {
        let event = Event::PlanUpdated {
            explanation: Some("sync".to_string()),
            items: vec![crate::op::PlanItem {
                id: "1".to_string(),
                content: "Do work".to_string(),
                status: crate::op::PlanItemStatus::InProgress,
            }],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("plan_updated"));
        assert!(json.contains("\"in_progress\""));
        let parsed: Event = serde_json::from_str(&json).unwrap();
        match parsed {
            Event::PlanUpdated { items, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].content, "Do work");
            }
            _ => panic!("Expected PlanUpdated"),
        }
    }

    #[test]
    fn test_event_dynamic_tool_call_requested_serialization() {
        let event = Event::DynamicToolCallRequested {
            call_id: "dyn-1".to_string(),
            tool_name: "lookup".to_string(),
            arguments: serde_json::json!({"id":"123"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("dynamic_tool_call_requested"));
        let parsed: Event = serde_json::from_str(&json).unwrap();
        match parsed {
            Event::DynamicToolCallRequested {
                call_id,
                tool_name,
                arguments,
            } => {
                assert_eq!(call_id, "dyn-1");
                assert_eq!(tool_name, "lookup");
                assert_eq!(arguments["id"], "123");
            }
            _ => panic!("Expected DynamicToolCallRequested"),
        }
    }

    #[test]
    fn test_event_envelope_serialization_flattens_event() {
        let envelope = EventEnvelope {
            event_id: "evt_00000001".to_string(),
            sequence: 1,
            session_id: "sess_1".to_string(),
            submission_id: Some("sub_1".to_string()),
            turn_id: "turn_000001".to_string(),
            item_id: "item_000001_0001".to_string(),
            timestamp_ms: 1_708_646_400_000,
            event: Event::Thinking {
                message: "planning".to_string(),
            },
        };

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains("\"event_id\":\"evt_00000001\""));
        assert!(json.contains("\"submission_id\":\"sub_1\""));
        assert!(json.contains("\"turn_id\":\"turn_000001\""));
        assert!(json.contains("\"type\":\"thinking\""));
        assert!(json.contains("\"message\":\"planning\""));

        let decoded: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.event_id, "evt_00000001");
        assert_eq!(decoded.submission_id.as_deref(), Some("sub_1"));
        match decoded.event {
            Event::Thinking { message } => assert_eq!(message, "planning"),
            _ => panic!("Expected Thinking"),
        }
    }

    #[test]
    fn test_event_message_delta_chunk_serialization() {
        let event = Event::MessageDeltaChunk {
            chunk: "He".to_string(),
            is_final: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("message_delta_chunk"));
        assert!(json.contains("He"));
        assert!(json.contains("\"is_final\":false"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::MessageDeltaChunk { chunk, is_final } => {
                assert_eq!(chunk, "He");
                assert!(!is_final);
            }
            _ => panic!("Expected MessageDeltaChunk variant"),
        }
    }

    #[test]
    fn test_event_confirmation_required_serialization() {
        let details = serde_json::json!({
            "suppliers": ["Supplier A", "Supplier B"],
            "count": 2
        });

        let event = Event::ConfirmationRequired {
            checkpoint_id: "cp-123".to_string(),
            checkpoint_type: "supplier_list".to_string(),
            summary: "Found 2 suppliers".to_string(),
            details,
            options: vec![
                "approve".to_string(),
                "modify".to_string(),
                "reject".to_string(),
            ],
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("confirmation_required"));
        assert!(json.contains("cp-123"));
        assert!(json.contains("supplier_list"));
        assert!(json.contains("Found 2 suppliers"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ConfirmationRequired {
                checkpoint_id,
                checkpoint_type,
                summary,
                options,
                ..
            } => {
                assert_eq!(checkpoint_id, "cp-123");
                assert_eq!(checkpoint_type, "supplier_list");
                assert_eq!(summary, "Found 2 suppliers");
                assert_eq!(options.len(), 3);
                assert_eq!(options[0], "approve");
            }
            _ => panic!("Expected ConfirmationRequired variant"),
        }
    }

    #[test]
    fn test_event_tool_call_started_serialization() {
        let arguments = serde_json::json!({
            "query": "electronics supplier"
        });

        let event = Event::ToolCallStarted {
            call_id: "call-1".to_string(),
            tool_name: "web_search".to_string(),
            arguments: arguments.clone(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool_call_started"));
        assert!(json.contains("web_search"));
        assert!(json.contains("electronics supplier"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ToolCallStarted {
                call_id,
                tool_name,
                arguments: args,
            } => {
                assert_eq!(call_id, "call-1");
                assert_eq!(tool_name, "web_search");
                assert_eq!(args, arguments);
            }
            _ => panic!("Expected ToolCallStarted variant"),
        }
    }

    #[test]
    fn test_event_tool_call_completed_serialization() {
        let result = serde_json::json!({
            "found": true,
            "count": 5
        });

        let event = Event::ToolCallCompleted {
            call_id: "call-1".to_string(),
            tool_name: "web_search".to_string(),
            result: result.clone(),
            success: true,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool_call_completed"));
        assert!(json.contains("\"success\":true"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ToolCallCompleted {
                call_id,
                tool_name,
                success,
                ..
            } => {
                assert_eq!(call_id, "call-1");
                assert_eq!(tool_name, "web_search");
                assert!(success);
            }
            _ => panic!("Expected ToolCallCompleted variant"),
        }
    }

    #[test]
    fn test_event_task_completed_serialization() {
        let results = serde_json::json!({
            "suppliers_contacted": 3,
            "rfqs_sent": 3
        });

        let event = Event::TaskCompleted {
            summary: "Task finished successfully".to_string(),
            results: results.clone(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("task_completed"));
        assert!(json.contains("Task finished successfully"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TaskCompleted { summary, .. } => {
                assert_eq!(summary, "Task finished successfully");
            }
            _ => panic!("Expected TaskCompleted variant"),
        }
    }

    #[test]
    fn test_event_error_serialization() {
        let event = Event::Error {
            message: "Something went wrong".to_string(),
            recoverable: true,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Something went wrong"));
        assert!(json.contains("\"recoverable\":true"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Error {
                message,
                recoverable,
            } => {
                assert_eq!(message, "Something went wrong");
                assert!(recoverable);
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_event_context_compacted_serialization() {
        let event = Event::ContextCompacted {};

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("context_compacted"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ContextCompacted {} => {}
            _ => panic!("Expected ContextCompacted variant"),
        }
    }

    #[test]
    fn test_error_not_recoverable() {
        let event = Event::Error {
            message: "Fatal error".to_string(),
            recoverable: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"recoverable\":false"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Error { recoverable, .. } => {
                assert!(!recoverable);
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_tool_call_completed_failure() {
        let result = serde_json::json!({
            "error": "API rate limit exceeded"
        });

        let event = Event::ToolCallCompleted {
            call_id: "call-2".to_string(),
            tool_name: "external_api".to_string(),
            result,
            success: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"success\":false"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ToolCallCompleted { success, .. } => {
                assert!(!success);
            }
            _ => panic!("Expected ToolCallCompleted variant"),
        }
    }

    #[test]
    fn test_event_skills_loaded_serialization() {
        let event = Event::SkillsLoaded {
            skill_ids: vec![
                "website-researcher".to_string(),
                "entity-extractor".to_string(),
            ],
            auto_selected: true,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("skills_loaded"));
        assert!(json.contains("website-researcher"));
        assert!(json.contains("entity-extractor"));
        assert!(json.contains("\"auto_selected\":true"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::SkillsLoaded {
                skill_ids,
                auto_selected,
            } => {
                assert_eq!(skill_ids.len(), 2);
                assert!(skill_ids.contains(&"website-researcher".to_string()));
                assert!(skill_ids.contains(&"entity-extractor".to_string()));
                assert!(auto_selected);
            }
            _ => panic!("Expected SkillsLoaded variant"),
        }
    }

    #[test]
    fn test_event_skills_loaded_explicit() {
        let event = Event::SkillsLoaded {
            skill_ids: vec!["supplier-researcher".to_string()],
            auto_selected: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"auto_selected\":false"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::SkillsLoaded {
                skill_ids,
                auto_selected,
            } => {
                assert_eq!(skill_ids, vec!["supplier-researcher"]);
                assert!(!auto_selected);
            }
            _ => panic!("Expected SkillsLoaded variant"),
        }
    }

    #[test]
    fn test_event_skills_loaded_empty() {
        let event = Event::SkillsLoaded {
            skill_ids: vec![],
            auto_selected: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("skills_loaded"));
        assert!(json.contains("\"skill_ids\":[]"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::SkillsLoaded {
                skill_ids,
                auto_selected,
            } => {
                assert!(skill_ids.is_empty());
                assert!(!auto_selected);
            }
            _ => panic!("Expected SkillsLoaded variant"),
        }
    }

    #[test]
    fn test_format_checkpoint_kind_label() {
        assert_eq!(
            format_checkpoint_kind_label("supplier_list"),
            "Supplier List"
        );
        assert_eq!(
            format_checkpoint_kind_label("send-approval"),
            "Send Approval"
        );
        assert_eq!(format_checkpoint_kind_label("  "), "Unknown");
    }

    // ========================================================================
    // Tests for new Phase 3 Event variants
    // ========================================================================

    #[test]
    fn test_event_text_delta_serialization() {
        let event = Event::TextDelta {
            chunk: "Hello ".to_string(),
            is_final: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("text_delta"));
        assert!(json.contains("Hello "));
        assert!(json.contains("\"is_final\":false"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TextDelta { chunk, is_final } => {
                assert_eq!(chunk, "Hello ");
                assert!(!is_final);
            }
            _ => panic!("Expected TextDelta variant"),
        }
    }

    #[test]
    fn test_event_text_delta_final() {
        let event = Event::TextDelta {
            chunk: String::new(),
            is_final: true,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TextDelta { is_final, .. } => assert!(is_final),
            _ => panic!("Expected TextDelta variant"),
        }
    }

    #[test]
    fn test_event_thinking_delta_serialization() {
        let event = Event::ThinkingDelta {
            chunk: "Let me think...".to_string(),
            is_final: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("thinking_delta"));
        assert!(json.contains("Let me think..."));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ThinkingDelta { chunk, is_final } => {
                assert_eq!(chunk, "Let me think...");
                assert!(!is_final);
            }
            _ => panic!("Expected ThinkingDelta variant"),
        }
    }

    #[test]
    fn test_event_yield_confirmation() {
        let event = Event::Yield {
            request_id: "cp-1".to_string(),
            kind: YieldKind::Confirmation,
            payload: serde_json::json!({
                "summary": "Approve file write?",
                "options": ["approve", "reject"]
            }),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("yield"));
        assert!(json.contains("confirmation"));
        assert!(json.contains("cp-1"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Yield {
                request_id,
                kind,
                payload,
            } => {
                assert_eq!(request_id, "cp-1");
                assert!(matches!(kind, YieldKind::Confirmation));
                assert!(payload["summary"].as_str().unwrap().contains("Approve"));
            }
            _ => panic!("Expected Yield variant"),
        }
    }

    #[test]
    fn test_event_yield_structured_input() {
        let event = Event::Yield {
            request_id: "req-1".to_string(),
            kind: YieldKind::StructuredInput,
            payload: serde_json::json!({"title": "Select options"}),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("structured_input"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Yield { kind, .. } => {
                assert!(matches!(kind, YieldKind::StructuredInput));
            }
            _ => panic!("Expected Yield variant"),
        }
    }

    #[test]
    fn test_event_yield_dynamic_tool_call() {
        let event = Event::Yield {
            request_id: "call-1".to_string(),
            kind: YieldKind::DynamicToolCall,
            payload: serde_json::json!({
                "tool_name": "custom_tool",
                "arguments": {"key": "value"}
            }),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("dynamic_tool_call"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Yield {
                request_id, kind, ..
            } => {
                assert_eq!(request_id, "call-1");
                assert!(matches!(kind, YieldKind::DynamicToolCall));
            }
            _ => panic!("Expected Yield variant"),
        }
    }
}
