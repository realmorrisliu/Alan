//! `alan ask` — one-shot question mode.
//!
//! Creates a session, sends the question, streams the response, then exits.

use anyhow::{Context, Result};
use futures::StreamExt;
use std::path::PathBuf;

/// Ask a one-shot question.
///
/// Ensures the daemon is running, creates a temporary session, submits the
/// question, streams the response to stdout, and cleans up the session.
pub async fn run_ask(question: &str, workspace: Option<PathBuf>) -> Result<()> {
    // Ensure daemon is running
    super::daemon::ensure_daemon_running().await?;

    let base_url = super::daemon::daemon_url();
    let client = reqwest::Client::new();

    // Create a session
    let mut create_body = serde_json::Map::new();
    if let Some(ws_path) = &workspace {
        let canonical = std::fs::canonicalize(ws_path)
            .with_context(|| format!("Cannot resolve workspace path: {}", ws_path.display()))?;
        create_body.insert(
            "workspace_dir".to_string(),
            serde_json::Value::String(canonical.to_string_lossy().to_string()),
        );
    }

    let create_resp = client
        .post(format!("{}/api/v1/sessions", base_url))
        .json(&create_body)
        .send()
        .await
        .context("Failed to create session")?;

    if !create_resp.status().is_success() {
        let status = create_resp.status();
        let body = create_resp.text().await.unwrap_or_default();

        // Try to extract error message from JSON response
        let error_detail = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or_else(|| body.clone());

        let mut msg = format!("Failed to create session: {}", error_detail);

        // Detect common causes and add hints
        let lower = error_detail.to_lowercase();
        if lower.contains("api key")
            || lower.contains("api_key")
            || lower.contains("unauthorized")
            || lower.contains("authentication")
            || lower.contains("anthropic_api_key")
            || lower.contains("not set")
            || lower.contains("llm client")
        {
            msg.push_str("\n\nHint: Make sure your LLM API key is configured.");
            msg.push_str("\n  export ANTHROPIC_API_KEY=sk-...");
            msg.push_str("\n  Or set it in ~/.alan/.env");
        } else if status.as_u16() == 500 && error_detail.is_empty() {
            msg = "Failed to create session (internal error)".to_string();
            msg.push_str("\n\nPossible causes:");
            msg.push_str("\n  • LLM API key not configured (export ANTHROPIC_API_KEY=...)");
            msg.push_str("\n  • Daemon encountered an unexpected error");
            msg.push_str("\n\nCheck daemon logs for details: alan daemon start --foreground");
        }

        anyhow::bail!("{}", msg);
    }

    let create_data: serde_json::Value = create_resp
        .json()
        .await
        .context("Failed to parse session response")?;
    let session_id = create_data["session_id"]
        .as_str()
        .context("No session_id in response")?
        .to_string();

    // Submit the question
    let submit_body = serde_json::json!({
        "op": {
            "type": "user_input",
            "content": question
        }
    });

    let submit_resp = client
        .post(format!(
            "{}/api/v1/sessions/{}/submit",
            base_url, session_id
        ))
        .json(&submit_body)
        .send()
        .await
        .context("Failed to submit question")?;

    if !submit_resp.status().is_success() {
        let status = submit_resp.status();
        let body = submit_resp.text().await.unwrap_or_default();
        // Clean up session before bailing
        let _ = client
            .delete(format!("{}/api/v1/sessions/{}", base_url, session_id))
            .send()
            .await;
        anyhow::bail!("Failed to submit question ({}): {}", status, body);
    }

    // Stream events via NDJSON
    let events_resp = client
        .get(format!(
            "{}/api/v1/sessions/{}/events",
            base_url, session_id
        ))
        .send()
        .await
        .context("Failed to connect to event stream")?;

    if !events_resp.status().is_success() {
        let _ = client
            .delete(format!("{}/api/v1/sessions/{}", base_url, session_id))
            .send()
            .await;
        anyhow::bail!("Failed to stream events: {}", events_resp.status());
    }

    // Stream NDJSON lines and print message deltas
    // Note: EventEnvelope uses #[serde(flatten)] so event fields are at root level
    let mut stream = events_resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                // Process complete lines from buffer
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&line) {
                        match envelope.get("type").and_then(|t| t.as_str()) {
                            Some("message_delta") => {
                                if let Some(content) = envelope.get("content").and_then(|c| c.as_str()) {
                                    print!("{}", content);
                                    std::io::Write::flush(&mut std::io::stdout())?;
                                }
                            }
                            Some("turn_completed") => {
                                println!();
                                // Clean up and exit
                                let _ = client
                                    .delete(format!("{}/api/v1/sessions/{}", base_url, session_id))
                                    .send()
                                    .await;
                                return Ok(());
                            }
                            Some("error") => {
                                if let Some(msg) = envelope.get("message").and_then(|m| m.as_str()) {
                                    eprintln!("\nError: {}", msg);
                                }
                                let _ = client
                                    .delete(format!("{}/api/v1/sessions/{}", base_url, session_id))
                                    .send()
                                    .await;
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                break;
            }
        }
    }

    // Stream ended or broke out of loop - clean up
    let _ = client
        .delete(format!("{}/api/v1/sessions/{}", base_url, session_id))
        .send()
        .await;

    Ok(())
}
