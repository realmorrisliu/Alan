//! System event definitions (Event Queue).
//!
//! These are events emitted by the agent to notify frontends.

use crate::op::PlanItem;
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
    /// Replaces Thinking + ThinkingComplete.
    ThinkingDelta {
        /// Incremental thinking text chunk.
        chunk: String,
        /// Whether this is the final chunk of the current thinking stream.
        is_final: bool,
    },

    /// Engine is suspended, waiting for external input.
    /// Unified replacement for ConfirmationRequired, StructuredUserInputRequested,
    /// and DynamicToolRequested. Client responds with Op::Resume.
    Yield {
        /// Unique request ID — client uses this in Op::Resume.
        request_id: String,
        /// Kind of yield — tells the client what UI to render.
        kind: YieldKind,
        /// Payload with details for the specific yield kind.
        payload: serde_json::Value,
    },

    // ========================================================================
    // Core events
    // ========================================================================
    /// Start of a new logical user-initiated turn.
    TurnStarted {},

    /// End of the current logical turn.
    TurnCompleted {
        /// Optional summary for the completed turn.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },

    /// A tool call has started
    ToolCallStarted {
        /// Stable tool call id emitted by the LLM/tool loop
        id: String,
        /// Name of the tool being called
        name: String,
    },

    /// A tool call has completed
    ToolCallCompleted {
        /// Stable tool call id emitted by the LLM/tool loop
        id: String,
        /// Human-readable preview of the tool result.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result_preview: Option<String>,
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
    DynamicTool,
    /// Extensible custom yield kind.
    Custom(String),
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let event = Event::TurnCompleted { summary: None };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("turn_completed"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::TurnCompleted { summary } => assert!(summary.is_none()),
            _ => panic!("Expected TurnCompleted variant"),
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
    fn test_event_tool_call_started_serialization() {
        let event = Event::ToolCallStarted {
            id: "call-1".to_string(),
            name: "web_search".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool_call_started"));
        assert!(json.contains("web_search"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ToolCallStarted { id, name } => {
                assert_eq!(id, "call-1");
                assert_eq!(name, "web_search");
            }
            _ => panic!("Expected ToolCallStarted variant"),
        }
    }

    #[test]
    fn test_event_tool_call_completed_serialization() {
        let event = Event::ToolCallCompleted {
            id: "call-1".to_string(),
            result_preview: Some("5 records".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool_call_completed"));
        assert!(json.contains("5 records"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ToolCallCompleted { id, result_preview } => {
                assert_eq!(id, "call-1");
                assert_eq!(result_preview.as_deref(), Some("5 records"));
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
        let event = Event::ToolCallCompleted {
            id: "call-2".to_string(),
            result_preview: Some("error: API rate limit exceeded".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("error: API rate limit exceeded"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::ToolCallCompleted { result_preview, .. } => {
                assert_eq!(
                    result_preview.as_deref(),
                    Some("error: API rate limit exceeded")
                );
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
    fn test_event_yield_dynamic_tool() {
        let event = Event::Yield {
            request_id: "call-1".to_string(),
            kind: YieldKind::DynamicTool,
            payload: serde_json::json!({
                "tool_name": "custom_tool",
                "arguments": {"key": "value"}
            }),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("dynamic_tool"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Yield {
                request_id, kind, ..
            } => {
                assert_eq!(request_id, "call-1");
                assert!(matches!(kind, YieldKind::DynamicTool));
            }
            _ => panic!("Expected Yield variant"),
        }
    }

    #[test]
    fn test_event_yield_custom_kind() {
        let event = Event::Yield {
            request_id: "custom-1".to_string(),
            kind: YieldKind::Custom("human_review".to_string()),
            payload: serde_json::json!({"note": "needs review"}),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("human_review"));

        let deserialized: Event = serde_json::from_str(&json).unwrap();
        match deserialized {
            Event::Yield {
                request_id, kind, ..
            } => {
                assert_eq!(request_id, "custom-1");
                assert!(matches!(kind, YieldKind::Custom(ref s) if s == "human_review"));
            }
            _ => panic!("Expected Yield variant"),
        }
    }
}
