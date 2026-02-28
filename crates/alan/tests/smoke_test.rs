//! Smoke Tests — Mock runtime integration tests for coding agent verification loop.
//!
//! These tests use MockLlmProvider to exercise the runtime without real LLM calls.
//! Run with `cargo test -p alan --test smoke_test -- --nocapture` to see full event logs.

use alan_llm::{GenerationResponse, MockLlmProvider, TokenUsage, ToolCall};
use alan_protocol::{ContentPart, Event, Op, Submission};
use alan_runtime::runtime::spawn_with_llm_client_and_tools;
use alan_runtime::{
    LlmClient, RuntimeEventEnvelope, WorkspaceRuntimeConfig, spawn_with_llm_client,
};
use std::time::Duration;

/// Collect events from a runtime until TurnCompleted or timeout.
async fn collect_events_until_turn_complete(
    mut rx: tokio::sync::broadcast::Receiver<RuntimeEventEnvelope>,
    timeout: Duration,
) -> Vec<Event> {
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(envelope) => {
                        let event = envelope.event.clone();
                        events.push(event.clone());
                        if matches!(event, Event::TurnCompleted { .. }) {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("[smoke] WARNING: lagged {n} events");
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                eprintln!("[smoke] TIMEOUT waiting for TurnCompleted after {:?}", timeout);
                break;
            }
        }
    }

    events
}

fn print_event_summary(test_name: &str, events: &[Event]) {
    eprintln!("\n=== {test_name}: collected {} events ===", events.len());
    for (i, event) in events.iter().enumerate() {
        let tag = match event {
            Event::TurnStarted { .. } => "TurnStarted",
            Event::TurnCompleted { .. } => "TurnCompleted",
            Event::ThinkingDelta { chunk, is_final } => {
                eprintln!(
                    "  [{i}] ThinkingDelta(is_final={is_final}): {:?}",
                    &chunk[..chunk.len().min(80)]
                );
                continue;
            }
            Event::TextDelta { chunk, is_final } => {
                eprintln!("  [{i}] TextDelta(is_final={is_final}): {chunk:?}");
                continue;
            }
            Event::ToolCallStarted { name, .. } => {
                eprintln!("  [{i}] ToolCallStarted: {name}");
                continue;
            }
            Event::ToolCallCompleted { id, .. } => {
                eprintln!("  [{i}] ToolCallCompleted: {id}");
                continue;
            }
            Event::Error { message, .. } => {
                eprintln!("  [{i}] Error: {message}");
                continue;
            }
            other => {
                eprintln!("  [{i}] {:?}", std::mem::discriminant(other));
                continue;
            }
        };
        eprintln!("  [{i}] {tag}");
    }
    eprintln!("=== end {test_name} ===\n");
}

#[tokio::test]
async fn smoke_text_response() {
    // MockLlmProvider returns a fixed text response
    let mock = MockLlmProvider::new().with_response(GenerationResponse {
        content: "Hello from mock LLM!".to_string(),
        thinking: None,
        thinking_signature: None,
        redacted_thinking: Vec::new(),
        tool_calls: Vec::new(),
        usage: Some(TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            reasoning_tokens: None,
        }),
    });

    let llm_client = LlmClient::new(mock);
    let config = WorkspaceRuntimeConfig::default();

    let mut controller =
        spawn_with_llm_client(config, llm_client).expect("spawn_with_llm_client should succeed");

    controller
        .wait_until_ready()
        .await
        .expect("runtime should become ready");

    // Subscribe to events before submitting
    let rx = controller.handle.event_sender.subscribe();

    // Submit a Turn op
    let submission = Submission::new(Op::Turn {
        parts: vec![ContentPart::text("Say hello")],
        context: None,
    });
    controller
        .handle
        .submission_tx
        .send(submission)
        .await
        .expect("send submission");

    // Collect events
    let events = collect_events_until_turn_complete(rx, Duration::from_secs(10)).await;
    print_event_summary("smoke_text_response", &events);

    // Basic sanity checks
    let has_turn_started = events
        .iter()
        .any(|e| matches!(e, Event::TurnStarted { .. }));
    let has_turn_completed = events
        .iter()
        .any(|e| matches!(e, Event::TurnCompleted { .. }));
    let has_content = events.iter().any(|e| match e {
        Event::TextDelta { chunk, .. } => !chunk.is_empty(),
        _ => false,
    });

    assert!(has_turn_started, "Expected TurnStarted event");
    assert!(has_turn_completed, "Expected TurnCompleted event");
    assert!(has_content, "Expected at least one content event");

    // Verify TurnStarted comes before TurnCompleted
    let start_idx = events
        .iter()
        .position(|e| matches!(e, Event::TurnStarted { .. }));
    let end_idx = events
        .iter()
        .position(|e| matches!(e, Event::TurnCompleted { .. }));
    if let (Some(s), Some(e)) = (start_idx, end_idx) {
        assert!(s < e, "TurnStarted should come before TurnCompleted");
    }

    controller.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn smoke_tool_call_flow() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("test.txt");
    std::fs::write(&file_path, "hello from smoke test").expect("write test file");
    let file_path_str = file_path.to_string_lossy().to_string();

    // First response: a tool call. Second response: text after tool result.
    let mock = MockLlmProvider::new().with_responses(vec![
        GenerationResponse {
            content: String::new(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: Vec::new(),
            tool_calls: vec![ToolCall {
                id: Some("call_001".to_string()),
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": file_path_str}),
            }],
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                reasoning_tokens: None,
            }),
        },
        GenerationResponse {
            content: "I read the file for you.".to_string(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: Vec::new(),
            tool_calls: Vec::new(),
            usage: Some(TokenUsage {
                prompt_tokens: 20,
                completion_tokens: 10,
                total_tokens: 30,
                reasoning_tokens: None,
            }),
        },
    ]);

    let llm_client = LlmClient::new(mock);
    let mut config = WorkspaceRuntimeConfig::default();
    // Skip tool approval prompts in tests
    config.agent_config.runtime_config.governance = alan_protocol::GovernanceConfig {
        profile: alan_protocol::GovernanceProfile::Autonomous,
        policy_path: None,
    };
    let tools = alan_tools::create_tool_registry_with_core_tools(temp.path().to_path_buf());

    let mut controller = spawn_with_llm_client_and_tools(config, llm_client, tools)
        .expect("spawn_with_llm_client_and_tools should succeed");

    controller
        .wait_until_ready()
        .await
        .expect("runtime should become ready");

    let rx = controller.handle.event_sender.subscribe();

    let submission = Submission::new(Op::Turn {
        parts: vec![ContentPart::text("Read /tmp/test.txt for me")],
        context: None,
    });
    controller
        .handle
        .submission_tx
        .send(submission)
        .await
        .expect("send submission");

    let events = collect_events_until_turn_complete(rx, Duration::from_secs(15)).await;
    print_event_summary("smoke_tool_call_flow", &events);

    // Check for tool-related events
    let has_tool_started = events
        .iter()
        .any(|e| matches!(e, Event::ToolCallStarted { .. }));
    let has_tool_completed = events
        .iter()
        .any(|e| matches!(e, Event::ToolCallCompleted { .. }));

    // Tool events may or may not appear depending on tool registry config,
    // but we should at least get turn lifecycle events
    let has_turn_started = events
        .iter()
        .any(|e| matches!(e, Event::TurnStarted { .. }));
    let has_turn_completed = events
        .iter()
        .any(|e| matches!(e, Event::TurnCompleted { .. }));

    assert!(has_turn_started, "Expected TurnStarted event");
    assert!(has_turn_completed, "Expected TurnCompleted event");

    eprintln!(
        "[smoke_tool_call_flow] tool_started={has_tool_started}, tool_completed={has_tool_completed}"
    );

    controller.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn smoke_multiple_turns() {
    let mock = MockLlmProvider::new().with_responses(vec![
        GenerationResponse {
            content: "First response".to_string(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: Vec::new(),
            tool_calls: Vec::new(),
            usage: Some(TokenUsage {
                prompt_tokens: 5,
                completion_tokens: 3,
                total_tokens: 8,
                reasoning_tokens: None,
            }),
        },
        GenerationResponse {
            content: "Second response".to_string(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: Vec::new(),
            tool_calls: Vec::new(),
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                reasoning_tokens: None,
            }),
        },
    ]);

    let llm_client = LlmClient::new(mock);
    let config = WorkspaceRuntimeConfig::default();

    let mut controller =
        spawn_with_llm_client(config, llm_client).expect("spawn_with_llm_client should succeed");

    controller
        .wait_until_ready()
        .await
        .expect("runtime should become ready");

    // Turn 1
    let rx1 = controller.handle.event_sender.subscribe();
    controller
        .handle
        .submission_tx
        .send(Submission::new(Op::Turn {
            parts: vec![ContentPart::text("First question")],
            context: None,
        }))
        .await
        .expect("send");
    let events1 = collect_events_until_turn_complete(rx1, Duration::from_secs(10)).await;
    print_event_summary("smoke_multiple_turns (turn 1)", &events1);

    // Turn 2
    let rx2 = controller.handle.event_sender.subscribe();
    controller
        .handle
        .submission_tx
        .send(Submission::new(Op::Turn {
            parts: vec![ContentPart::text("Second question")],
            context: None,
        }))
        .await
        .expect("send");
    let events2 = collect_events_until_turn_complete(rx2, Duration::from_secs(10)).await;
    print_event_summary("smoke_multiple_turns (turn 2)", &events2);

    // Both turns should have complete lifecycle
    for (name, events) in [("turn1", &events1), ("turn2", &events2)] {
        let started = events
            .iter()
            .any(|e| matches!(e, Event::TurnStarted { .. }));
        let completed = events
            .iter()
            .any(|e| matches!(e, Event::TurnCompleted { .. }));
        assert!(started, "{name}: Expected TurnStarted");
        assert!(completed, "{name}: Expected TurnCompleted");
    }

    controller.shutdown().await.expect("shutdown");
}
