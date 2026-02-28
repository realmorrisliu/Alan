//! Event Contract Tests - 确保服务端和客户端对事件类型的理解一致
//!
//! 这些测试验证：
//! 1. 服务端发送的关键事件能被客户端消费
//! 2. 客户端期望的事件类型服务端会发送
//! 3. 事件结构符合预期

use alan_protocol::{Event, EventEnvelope};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

/// 模拟客户端事件处理器的行为
/// 这代表了客户端（TUI/ask）实际处理事件的逻辑
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

    /// 模拟客户端处理事件的逻辑（基于 components.tsx 中的 switch 语句）
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
                // 其他事件类型，客户端可能忽略
            }
        }
    }

    fn has_received_message(&self) -> bool {
        !self.received_messages.is_empty()
    }
}

/// 契约测试：验证当 LLM 返回文本响应时，客户端能收到消息
/// 这是核心的用户可见行为契约
#[test]
fn contract_text_response_must_emit_displayable_event() {
    // 模拟 runtime 发送的事件序列（来自 turn_executor.rs）
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
        // 关键：服务端必须发送能被客户端显示的事件
        Event::TextDelta {
            chunk: "Hello, world!".to_string(),
            is_final: false,
        },
        Event::TextDelta {
            chunk: String::new(),
            is_final: true,
        },
        Event::TaskCompleted {
            summary: "Task completed".to_string(),
            results: json!({"status": "completed"}),
        },
        Event::TurnCompleted { summary: None },
    ];

    let mut client = MockClientEventHandler::new();

    for event in &events {
        let envelope = create_test_envelope(event.clone());
        client.handle_event(&envelope);
    }

    // 契约断言：客户端必须能显示消息给用户
    assert!(
        client.has_received_message(),
        "Contract violation: Client must receive at least one displayable message event \
         when LLM returns text response. \
         Make sure the runtime emits TextDelta events."
    );

    // 验证消息内容正确
    assert_eq!(client.received_messages.len(), 1); // non-empty TextDelta chunk
    assert!(
        client
            .received_messages
            .contains(&"Hello, world!".to_string())
    );
}

/// 契约测试：验证客户端能处理工具调用事件
#[test]
fn contract_tool_call_must_emit_tool_events() {
    let events = vec![
        Event::ToolCallStarted {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
        },
        Event::ToolCallCompleted {
            id: "call_1".to_string(),
            result_preview: Some("content loaded".to_string()),
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

/// 契约测试：验证空响应回退机制
#[test]
fn contract_empty_response_must_show_fallback_message() {
    // 当 LLM 返回空内容时，应该显示回退消息
    let events = vec![
        Event::TextDelta {
            chunk: "I apologize, but I couldn't generate a response.".to_string(),
            is_final: true,
        },
        Event::TaskCompleted {
            summary: "Turn completed with empty response fallback".to_string(),
            results: json!({"status": "completed", "fallback": "empty_response"}),
        },
        Event::TurnCompleted { summary: None },
    ];

    let mut client = MockClientEventHandler::new();

    for event in &events {
        let envelope = create_test_envelope(event.clone());
        client.handle_event(&envelope);
    }

    // 即使 LLM 返回空内容，用户也应该看到回退消息
    assert!(
        client.has_received_message(),
        "Contract violation: Empty response must show fallback message to user"
    );
}

/// 契约测试：验证必需事件的完整序列
#[test]
fn contract_turn_must_emit_complete_event_sequence() {
    // 定义一个完整 turn 的必需事件类型
    let required_event_types = vec!["turn_started", "thinking_delta", "turn_completed"];

    // 模拟一个完整的 turn 事件序列
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
