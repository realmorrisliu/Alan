//! WebSocket handler for real-time communication.

use axum::{
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use alan_protocol::{Event, EventEnvelope, Submission};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::state::AppState;

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    info!(%session_id, "WebSocket connection requested");
    ws.on_upgrade(move |socket| handle_socket(socket, state, session_id))
}

/// Handle an active WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: AppState, session_id: String) {
    // Check if session exists
    let session_exists = state.get_session(&session_id).await.is_some();

    if !session_exists {
        warn!(%session_id, "Session not found for WebSocket");
        let envelope = control_envelope(
            &session_id,
            Event::Error {
                message: "Session not found".to_string(),
                recoverable: false,
            },
        );
        let error_msg = serde_json::to_string(&envelope)
            .unwrap_or_else(|_| r#"{"type":"error","message":"serialize failed"}"#.to_string());

        let _ = socket.send(Message::Text(error_msg.into())).await;
        return;
    }

    // Clone necessary channels outside of any await point
    let (mut events_rx, submission_tx) = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session) => (session.events_tx.subscribe(), session.submission_tx.clone()),
            None => {
                warn!(%session_id, "Session not found for WebSocket after check");
                let envelope = control_envelope(
                    &session_id,
                    Event::Error {
                        message: "Session not found".to_string(),
                        recoverable: false,
                    },
                );
                let error_msg = serde_json::to_string(&envelope)
                    .unwrap_or_else(|_| r#"{"type":"error","message":"serialize failed"}"#.to_string());

                let _ = socket.send(Message::Text(error_msg.into())).await;
                return;
            }
        }
    };

    info!(%session_id, "WebSocket connected");

    // Main message loop
    let mut last_event_id: Option<String> = None;
    loop {
        tokio::select! {
            // Handle incoming messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!(%session_id, "Received WS message");

                        // Try to parse as a submission
                        match serde_json::from_str::<Submission>(&text) {
                            Ok(submission) => {
                                // Update session inbound activity
                                state.touch_session_inbound(&session_id).await;
                                // Use the cloned sender instead of holding the lock
                                if submission_tx.send(submission).await.is_err() {
                                    error!(%session_id, "Failed to send submission to agent");
                                }
                            }
                            Err(e) => {
                                warn!(%session_id, ?e, "Failed to parse WebSocket message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!(%session_id, "WebSocket closed by client");
                        break;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        debug!(%session_id, "Received pong");
                    }
                    Some(Err(e)) => {
                        error!(%session_id, ?e, "WebSocket error");
                        break;
                    }
                    None => {
                        info!(%session_id, "WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Forward agent events to websocket
            event = events_rx.recv() => {
                match event {
                    Ok(envelope) => {
                        // Update outbound activity tracking
                        state.touch_session_outbound(&session_id).await;
                        last_event_id = Some(envelope.event_id.clone());

                        let payload = serde_json::to_string(&envelope)
                            .unwrap_or_else(|_| "Failed to serialize event".to_string());
                        if socket.send(Message::Text(payload.into())).await.is_err() {
                            debug!(%session_id, "WebSocket closed (send failed)");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!(%session_id, missed = count, "WebSocket lagged behind events");
                        let payload = serde_json::to_string(&stream_lagged_envelope(
                            &session_id,
                            count,
                            last_event_id.clone(),
                        ))
                        .unwrap_or_else(|_| "Failed to serialize event".to_string());
                        let _ = socket.send(Message::Text(payload.into())).await;
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!(%session_id, "Event stream closed");
                        break;
                    }
                }
            }

            // Periodic ping to keep connection alive
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    debug!(%session_id, "WebSocket closed (ping failed)");
                    break;
                }
            }
        }
    }

    info!(%session_id, "WebSocket disconnected");
    // Note: We don't remove the session here because multiple clients may be
    // connected to the same session. The session is cleaned up by:
    // 1. TTL cleanup when session is idle
    // 2. Explicit DELETE request to the session endpoint
    // 3. Application shutdown
}

/// Create a control envelope for errors and other control messages.
/// Control envelopes use special IDs to distinguish them from regular events.
fn control_envelope(session_id: &str, event: Event) -> EventEnvelope {
    let event_type = match &event {
        Event::Error { .. } => "error",
        Event::StreamLagged { .. } => "lagged",
        _ => "control",
    };

    EventEnvelope {
        event_id: format!("control_{}_{}", event_type, uuid::Uuid::new_v4()),
        sequence: 0,
        session_id: session_id.to_string(),
        submission_id: None,
        turn_id: "turn_control".to_string(),
        item_id: "item_control".to_string(),
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        event,
    }
}

fn stream_lagged_envelope(
    session_id: &str,
    skipped: u64,
    replay_from_event_id: Option<String>,
) -> EventEnvelope {
    EventEnvelope {
        event_id: format!("control_lagged_{}", uuid::Uuid::new_v4()),
        sequence: 0,
        session_id: session_id.to_string(),
        submission_id: None,
        turn_id: "turn_control".to_string(),
        item_id: "item_control".to_string(),
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        event: Event::StreamLagged {
            skipped,
            replay_from_event_id,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::ws_handler;
    use crate::manager::{AgentManager, ManagerConfig};
    use crate::state::{AppState, SessionEntry, SessionEventLog};
    use alan_runtime::{Config, runtime::AgentRuntimeConfig};
    use axum::{Router, routing::get};
    use futures::{SinkExt, StreamExt};
    use alan_protocol::{Event, EventEnvelope, Op, Submission};
    use tokio::sync::{broadcast, mpsc};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    async fn spawn_ws_server(state: AppState) -> Option<(String, tokio::task::JoinHandle<()>)> {
        let app = Router::new()
            .route("/api/v1/sessions/{id}/ws", get(ws_handler))
            .with_state(state);
        let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => listener,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("skipping websocket test: cannot bind local tcp listener: {err}");
                return None;
            }
            Err(err) => panic!("failed to bind websocket test listener: {err}"),
        };
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Some((format!("ws://{}", addr), handle))
    }

    fn test_state() -> AppState {
        let base_dir =
            std::env::temp_dir().join(format!("agentd-ws-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base_dir).unwrap();
        let manager = AgentManager::with_runtime_config(
            ManagerConfig::with_base_dir(base_dir),
            AgentRuntimeConfig::from(Config::default()),
        );
        AppState::from_parts(Config::default(), std::sync::Arc::new(manager), 3600)
    }

    fn test_session_entry(agent_id: &str) -> (SessionEntry, mpsc::Receiver<Submission>) {
        let (submission_tx, submission_rx) = mpsc::channel(8);
        let (events_tx, _) = broadcast::channel(8);
        let event_log = std::sync::Arc::new(tokio::sync::RwLock::new(SessionEventLog::new(32)));
        let now = std::time::Instant::now();
        (
            SessionEntry {
                agent_id: agent_id.to_string(),
                approval_policy: alan_protocol::ApprovalPolicy::OnRequest,
                sandbox_mode: alan_protocol::SandboxMode::WorkspaceWrite,
                submission_tx,
                events_tx,
                event_log,
                event_bridge_task: None,
                rollout_path: None,
                created_at: now,
                last_inbound_activity: now,
                last_outbound_activity: now,
            },
            submission_rx,
        )
    }

    async fn next_text_message(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> String {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        match msg {
            Message::Text(text) => text.to_string(),
            other => panic!("Expected text message, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn websocket_returns_error_envelope_for_missing_session() {
        let state = test_state();
        let Some((base, server)) = spawn_ws_server(state).await else {
            return;
        };
        let (mut ws, _) = connect_async(format!("{}/api/v1/sessions/missing/ws", base))
            .await
            .unwrap();

        let text = next_text_message(&mut ws).await;
        // Should receive EventEnvelope, not bare Event
        let envelope: EventEnvelope = serde_json::from_str(&text).unwrap();
        
        // Verify envelope metadata
        assert!(envelope.event_id.starts_with("control_error_"), 
            "Expected control error event_id, got: {}", envelope.event_id);
        assert_eq!(envelope.session_id, "missing");
        assert_eq!(envelope.turn_id, "turn_control");
        assert_eq!(envelope.item_id, "item_control");
        assert_eq!(envelope.sequence, 0);
        
        // Verify the wrapped event
        match envelope.event {
            Event::Error {
                message,
                recoverable,
            } => {
                assert_eq!(message, "Session not found");
                assert!(!recoverable);
            }
            other => panic!("Unexpected event: {:?}", other),
        }

        server.abort();
    }

    #[tokio::test]
    async fn websocket_forwards_events_and_submissions() {
        let state = test_state();
        let (entry, mut submission_rx) = test_session_entry("agent-1");
        let events_tx = entry.events_tx.clone();
        state
            .sessions
            .write()
            .await
            .insert("sess-1".to_string(), entry);

        let Some((base, server)) = spawn_ws_server(state.clone()).await else {
            return;
        };
        let (mut ws, _) = connect_async(format!("{}/api/v1/sessions/sess-1/ws", base))
            .await
            .unwrap();

        events_tx
            .send(EventEnvelope {
                event_id: "evt_0000000000000001".to_string(),
                sequence: 1,
                session_id: "sess-1".to_string(),
                submission_id: Some("sub-test".to_string()),
                turn_id: "turn_000001".to_string(),
                item_id: "item_000001_0001".to_string(),
                timestamp_ms: 1,
                event: Event::Thinking {
                    message: "from-runtime".to_string(),
                },
            })
            .unwrap();

        let text = next_text_message(&mut ws).await;
        let envelope: EventEnvelope = serde_json::from_str(&text).unwrap();
        assert_eq!(envelope.event_id, "evt_0000000000000001");
        match envelope.event {
            Event::Thinking { message } => assert_eq!(message, "from-runtime"),
            other => panic!("Unexpected event: {:?}", other),
        }

        let submission = Submission::new(Op::Cancel);
        ws.send(Message::Text(
            serde_json::to_string(&submission).unwrap().into(),
        ))
        .await
        .unwrap();

        let forwarded =
            tokio::time::timeout(std::time::Duration::from_secs(2), submission_rx.recv())
                .await
                .unwrap()
                .unwrap();
        assert!(matches!(forwarded.op, Op::Cancel));

        let _ = ws.close(None).await;
        server.abort();
    }

    /// Test that all WebSocket messages follow EventEnvelope format consistently
    #[tokio::test]
    async fn websocket_consistent_envelope_format() {
        let state = test_state();
        let Some((base, server)) = spawn_ws_server(state.clone()).await else {
            return;
        };

        // Test 1: Missing session returns EventEnvelope
        let (mut ws1, _) = connect_async(format!("{}/api/v1/sessions/nonexistent/ws", base))
            .await
            .unwrap();
        let text1 = next_text_message(&mut ws1).await;
        let result1: Result<EventEnvelope, _> = serde_json::from_str(&text1);
        assert!(result1.is_ok(), "Missing session should return EventEnvelope, got: {}", text1);
        
        // Verify it's a control envelope
        let envelope1 = result1.unwrap();
        assert!(envelope1.event_id.starts_with("control_"));
        assert!(matches!(envelope1.event, Event::Error { .. }));
        let _ = ws1.close(None).await;

        // Test 2: Valid session also returns EventEnvelope
        let (entry, _) = test_session_entry("agent-2");
        let events_tx = entry.events_tx.clone();
        state.sessions.write().await.insert("sess-2".to_string(), entry);

        let (mut ws2, _) = connect_async(format!("{}/api/v1/sessions/sess-2/ws", base))
            .await
            .unwrap();

        // Send a test event
        events_tx.send(EventEnvelope {
            event_id: "evt_test_001".to_string(),
            sequence: 1,
            session_id: "sess-2".to_string(),
            submission_id: None,
            turn_id: "turn_001".to_string(),
            item_id: "item_001".to_string(),
            timestamp_ms: 12345,
            event: Event::TurnStarted {},
        }).unwrap();

        let text2 = next_text_message(&mut ws2).await;
        let result2: Result<EventEnvelope, _> = serde_json::from_str(&text2);
        assert!(result2.is_ok(), "Valid session should return EventEnvelope, got: {}", text2);
        
        let envelope2 = result2.unwrap();
        assert_eq!(envelope2.event_id, "evt_test_001");
        assert!(matches!(envelope2.event, Event::TurnStarted { .. }));
        
        let _ = ws2.close(None).await;
        server.abort();
    }
}
