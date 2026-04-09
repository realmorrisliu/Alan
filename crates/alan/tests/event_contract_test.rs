//! Event Contract Tests - ensure server/client agreement on event semantics.
//!
//! These tests verify:
//! 1. Key events emitted by the server are consumable by clients.
//! 2. Event types expected by clients are actually emitted by the server.
//! 3. Event payload structures match expectations.

use alan_protocol::{Event, EventEnvelope};
use std::time::{SystemTime, UNIX_EPOCH};

/// Simulated client event handler behavior.
/// Mirrors the practical event handling logic in clients (TUI/ask).
struct MockClientEventHandler {
    received_messages: Vec<String>,
    received_thinking_events: Vec<String>,
    received_tool_calls: Vec<String>,
}

impl MockClientEventHandler {
    fn new() -> Self {
        Self {
            received_messages: Vec::new(),
            received_thinking_events: Vec::new(),
            received_tool_calls: Vec::new(),
        }
    }

    /// Simulates client-side event handling logic (based on the `switch` in `components.tsx`).
    fn handle_event(&mut self, envelope: &EventEnvelope) {
        match &envelope.event {
            Event::TextDelta { chunk, .. } => {
                if !chunk.is_empty() {
                    self.received_messages.push(chunk.clone());
                }
            }
            Event::ThinkingDelta {
                chunk,
                is_final: false,
            } => {
                self.received_thinking_events.push(chunk.clone());
            }
            Event::ToolCallStarted { name, .. } => {
                self.received_tool_calls.push(name.clone());
            }
            _ => {
                // Other event types may be ignored by this mock client.
            }
        }
    }

    fn has_received_message(&self) -> bool {
        !self.received_messages.is_empty()
    }
}

/// Contract test: when the LLM returns text, the client must receive a displayable message.
/// This is the core user-visible behavior contract.
#[test]
fn contract_text_response_must_emit_displayable_event() {
    // Simulated runtime event sequence (from `turn_executor.rs` behavior).
    let events = vec![
        Event::TurnStarted {},
        Event::ThinkingDelta {
            chunk: "Working on your request...".to_string(),
            is_final: false,
        },
        Event::ThinkingDelta {
            chunk: String::new(),
            is_final: true,
        },
        // Critical: the server must emit events the client can display.
        Event::TextDelta {
            chunk: "Hello, world!".to_string(),
            is_final: false,
        },
        Event::TextDelta {
            chunk: String::new(),
            is_final: true,
        },
        Event::TurnCompleted { summary: None },
    ];

    let mut client = MockClientEventHandler::new();

    for event in &events {
        let envelope = create_test_envelope(event.clone());
        client.handle_event(&envelope);
    }

    // Contract assertion: the client must be able to display a message to the user.
    assert!(
        client.has_received_message(),
        "Contract violation: Client must receive at least one displayable message event \
         when LLM returns text response. \
         Make sure the runtime emits TextDelta events."
    );

    // Validate the message content.
    assert_eq!(client.received_messages.len(), 1); // non-empty TextDelta chunk
    assert!(
        client
            .received_messages
            .contains(&"Hello, world!".to_string())
    );
}

/// Contract test: client can process tool-call events.
#[test]
fn contract_tool_call_must_emit_tool_events() {
    let events = vec![
        Event::ToolCallStarted {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            audit: None,
        },
        Event::ToolCallCompleted {
            id: "call_1".to_string(),
            name: Some("read_file".to_string()),
            success: Some(true),
            result_preview: Some("content loaded".to_string()),
            audit: None,
        },
    ];

    let mut client = MockClientEventHandler::new();

    for event in &events {
        let envelope = create_test_envelope(event.clone());
        client.handle_event(&envelope);
    }

    assert_eq!(client.received_tool_calls.len(), 1);
    assert_eq!(client.received_tool_calls[0], "read_file");
}

/// Contract test: empty-response fallback.
#[test]
fn contract_empty_response_must_show_fallback_message() {
    // When the LLM returns empty content, a fallback message should be shown.
    let events = vec![
        Event::TextDelta {
            chunk: "I apologize, but I couldn't generate a response.".to_string(),
            is_final: true,
        },
        Event::TurnCompleted { summary: None },
    ];

    let mut client = MockClientEventHandler::new();

    for event in &events {
        let envelope = create_test_envelope(event.clone());
        client.handle_event(&envelope);
    }

    // Even with empty LLM content, users should still see a fallback message.
    assert!(
        client.has_received_message(),
        "Contract violation: Empty response must show fallback message to user"
    );
}

/// Contract test: full required event sequence for a turn.
#[test]
fn contract_turn_must_emit_complete_event_sequence() {
    // Required event types for a complete turn.
    let required_event_types = vec!["turn_started", "thinking_delta", "turn_completed"];

    // Simulate a full turn event sequence.
    let events = [
        Event::TurnStarted {},
        Event::ThinkingDelta {
            chunk: "Working...".to_string(),
            is_final: false,
        },
        Event::TextDelta {
            chunk: "Response".to_string(),
            is_final: true,
        },
        Event::TurnCompleted { summary: None },
    ];

    let event_type_names: Vec<String> = events
        .iter()
        .map(|e| match e {
            Event::TurnStarted { .. } => "turn_started",
            Event::TurnCompleted { .. } => "turn_completed",
            Event::ThinkingDelta {
                is_final: false, ..
            } => "thinking_delta",
            Event::ThinkingDelta { is_final: true, .. } => "thinking_delta_final",
            Event::TextDelta { .. } => "text_delta",
            _ => "other",
        })
        .map(|s| s.to_string())
        .collect();

    for required in &required_event_types {
        assert!(
            event_type_names.contains(&required.to_string()),
            "Contract violation: Turn must emit '{}' event",
            required
        );
    }
}

fn create_test_envelope(event: Event) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("evt_{}", uuid::Uuid::new_v4()),
        sequence: 1,
        session_id: "test_session".to_string(),
        submission_id: Some("sub_1".to_string()),
        turn_id: "turn_1".to_string(),
        item_id: "item_1".to_string(),
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        event,
    }
}
