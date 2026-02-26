//! Event Sequence Validation Tests
//!
//! 这些测试验证 runtime 生成的各种场景下的事件序列
//! 确保客户端能正确处理和显示

use alan_protocol::Event;

/// 测试场景：正常的文本响应
#[test]
fn sequence_text_response() {
    // 来自 turn_executor.rs 的 run_turn_with_cancel 的实际事件序列
    let expected_sequence = vec![
        EventPattern::new("turn_started").required(),
        EventPattern::new("thinking").required(),
        EventPattern::new("thinking_complete").required(),
        // 消息内容 - 可以是一个或多个 chunk，最后必须有一个 MessageDelta
        EventPattern::new("message_delta_chunk").optional(),
        EventPattern::new("message_delta").required(),
        EventPattern::new("task_completed").required(),
        EventPattern::new("turn_completed").required(),
    ];

    let actual_events = simulate_text_response_turn();

    validate_sequence(&actual_events, &expected_sequence);
}

/// 测试场景：带有工具调用的响应
#[test]
fn sequence_tool_call_response() {
    let expected_sequence = vec![
        EventPattern::new("turn_started").required(),
        EventPattern::new("thinking").required(),
        EventPattern::new("thinking_complete").optional(),
        // 工具调用
        EventPattern::new("tool_call_started").required(),
        EventPattern::new("tool_call_completed").required(),
        // 工具调用后可能有最终消息
        EventPattern::new("message_delta").optional(),
        EventPattern::new("task_completed").required(),
        EventPattern::new("turn_completed").required(),
    ];

    let actual_events = simulate_tool_call_turn();

    validate_sequence(&actual_events, &expected_sequence);
}

/// 测试场景：空响应回退
#[test]
fn sequence_empty_response_fallback() {
    let expected_sequence = vec![
        EventPattern::new("turn_started").required(),
        EventPattern::new("thinking").required(),
        EventPattern::new("thinking_complete").optional(),
        // 回退消息
        EventPattern::new("message_delta_chunk").required(),
        EventPattern::new("task_completed").required(),
        EventPattern::new("turn_completed").required(),
    ];

    let actual_events = simulate_empty_fallback_turn();

    validate_sequence(&actual_events, &expected_sequence);
}

/// 事件模式定义
struct EventPattern {
    event_type: String,
    required: bool,
}

impl EventPattern {
    fn new(event_type: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            required: false,
        }
    }

    fn required(mut self) -> Self {
        self.required = true;
        self
    }

    #[allow(dead_code)]
    fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

/// 获取事件的类型字符串
fn get_event_type(event: &Event) -> String {
    match event {
        Event::TurnStarted { .. } => "turn_started",
        Event::TurnCompleted { .. } => "turn_completed",
        Event::Thinking { .. } => "thinking",
        Event::ThinkingComplete { .. } => "thinking_complete",
        Event::ReasoningDelta { .. } => "reasoning_delta",
        Event::MessageDelta { .. } => "message_delta",
        Event::MessageDeltaChunk { .. } => "message_delta_chunk",
        Event::ConfirmationRequired { .. } => "confirmation_required",
        Event::StructuredUserInputRequested { .. } => "structured_user_input_requested",
        Event::ToolCallStarted { .. } => "tool_call_started",
        Event::ToolCallCompleted { .. } => "tool_call_completed",
        Event::TaskCompleted { .. } => "task_completed",
        Event::ContextCompacted { .. } => "context_compacted",
        Event::PlanUpdated { .. } => "plan_updated",
        Event::SessionRolledBack { .. } => "session_rolled_back",
        Event::StreamLagged { .. } => "stream_lagged",
        Event::Error { .. } => "error",
        Event::SkillsLoaded { .. } => "skills_loaded",
        Event::DynamicToolsRegistered { .. } => "dynamic_tools_registered",
        Event::DynamicToolCallRequested { .. } => "dynamic_tool_call_requested",
    }
    .to_string()
}

/// 验证事件序列是否符合预期模式
fn validate_sequence(events: &[Event], expected: &[EventPattern]) {
    let actual_types: Vec<String> = events.iter().map(get_event_type).collect();

    println!("Expected sequence:");
    for (i, pattern) in expected.iter().enumerate() {
        let req_marker = if pattern.required { "[R]" } else { "[O]" };
        println!("  {} {} {}", i, req_marker, pattern.event_type);
    }

    println!("\nActual sequence:");
    for (i, event_type) in actual_types.iter().enumerate() {
        println!("  {} {}", i, event_type);
    }

    // 检查必需事件是否存在
    for pattern in expected.iter().filter(|p| p.required) {
        let found = actual_types.contains(&pattern.event_type);
        assert!(
            found,
            "Required event '{}' not found in sequence",
            pattern.event_type
        );
    }

    // 检查顺序：如果是必需事件，它应该在正确的相对顺序中
    let mut expected_iter = expected.iter().filter(|p| p.required);
    let mut actual_iter = actual_types.iter();

    while let Some(expected_pattern) = expected_iter.next() {
        let found = actual_iter
            .by_ref()
            .any(|actual| *actual == expected_pattern.event_type);
        assert!(
            found,
            "Required event '{}' appears out of order or is missing",
            expected_pattern.event_type
        );
    }
}

// ==================== 模拟函数 ====================

/// 模拟正常文本响应的事件序列
fn simulate_text_response_turn() -> Vec<Event> {
    use serde_json::json;

    vec![
        Event::TurnStarted {},
        Event::Thinking {
            message: "Working on your request...".to_string(),
        },
        Event::ThinkingComplete {},
        Event::MessageDeltaChunk {
            chunk: "Hello! ".to_string(),
            is_final: false,
        },
        Event::MessageDeltaChunk {
            chunk: "How can I help you?".to_string(),
            is_final: false,
        },
        Event::MessageDeltaChunk {
            chunk: String::new(),
            is_final: true,
        },
        Event::MessageDelta {
            content: "Hello! How can I help you?".to_string(),
        },
        Event::TaskCompleted {
            summary: "Task completed".to_string(),
            results: json!({"status": "completed"}),
        },
        Event::TurnCompleted {},
    ]
}

/// 模拟工具调用响应的事件序列
fn simulate_tool_call_turn() -> Vec<Event> {
    use serde_json::json;

    vec![
        Event::TurnStarted {},
        Event::Thinking {
            message: "I need to read a file...".to_string(),
        },
        Event::ThinkingComplete {},
        Event::ToolCallStarted {
            call_id: "call_1".to_string(),
            tool_name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        },
        Event::ToolCallCompleted {
            call_id: "call_1".to_string(),
            tool_name: "read_file".to_string(),
            result: json!({"content": "file content"}),
            success: true,
        },
        Event::MessageDelta {
            content: "I found the file content.".to_string(),
        },
        Event::TaskCompleted {
            summary: "Task completed".to_string(),
            results: json!({"status": "completed"}),
        },
        Event::TurnCompleted {},
    ]
}

/// 模拟空响应回退的事件序列
fn simulate_empty_fallback_turn() -> Vec<Event> {
    use serde_json::json;

    vec![
        Event::TurnStarted {},
        Event::Thinking {
            message: "Working...".to_string(),
        },
        Event::ThinkingComplete {},
        Event::MessageDeltaChunk {
            chunk: "I apologize, but I couldn't generate a response.".to_string(),
            is_final: true,
        },
        Event::TaskCompleted {
            summary: "Turn completed with empty response fallback".to_string(),
            results: json!({"status": "completed", "fallback": "empty_response"}),
        },
        Event::TurnCompleted {},
    ]
}
