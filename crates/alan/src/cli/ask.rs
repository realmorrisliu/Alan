//! `alan ask` — one-shot question mode.
//!
//! Creates a session, sends the question, streams the response, then exits.

use crate::OutputMode;
use futures::StreamExt;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tokio::time::{Duration, Instant};

/// Internal state tracked during the event stream.
struct AskState {
    start: Instant,
    tool_count: u32,
    tool_starts: HashMap<String, (String, Instant)>, // call_id -> (tool_name, start_time)
    accumulated_text: String,
}

/// Extract a short key argument from tool call arguments for display.
/// Prefers `path`, then first string value, then "...".
fn key_arg(arguments: &serde_json::Value) -> String {
    if let Some(obj) = arguments.as_object() {
        if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
            return path.to_string();
        }
        for val in obj.values() {
            if let Some(s) = val.as_str() {
                return s.to_string();
            }
        }
    }
    "...".to_string()
}

/// Format a duration as a human-readable string.
fn fmt_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else {
        format!("{:.1}s", secs)
    }
}

/// Delete a session, ignoring errors.
async fn delete_session(client: &reqwest::Client, base_url: &str, session_id: &str) {
    let _ = client
        .delete(format!("{}/api/v1/sessions/{}", base_url, session_id))
        .send()
        .await;
}

/// Ask a one-shot question.
///
/// Returns an exit code: 0 = success, 1 = runtime error, 2 = timeout, 3 = LLM config missing.
pub async fn run_ask(
    question: &str,
    workspace: Option<PathBuf>,
    mode: OutputMode,
    show_thinking: bool,
    timeout_secs: u64,
) -> i32 {
    match run_ask_inner(question, workspace, mode, show_thinking, timeout_secs).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {:#}", e);
            1
        }
    }
}

async fn run_ask_inner(
    question: &str,
    workspace: Option<PathBuf>,
    mode: OutputMode,
    show_thinking: bool,
    timeout_secs: u64,
) -> Result<i32, anyhow::Error> {
    use anyhow::Context;

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

        let error_detail = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or_else(|| body.clone());

        let lower = error_detail.to_lowercase();
        if lower.contains("api key")
            || lower.contains("api_key")
            || lower.contains("unauthorized")
            || lower.contains("authentication")
            || lower.contains("anthropic_api_key")
            || lower.contains("not set")
            || lower.contains("llm client")
        {
            eprintln!("Error: {}", error_detail);
            eprintln!();
            eprintln!("Hint: Make sure your LLM API key is configured.");
            eprintln!("  export ANTHROPIC_API_KEY=sk-...");
            eprintln!("  Or set it in ~/.alan/.env");
            return Ok(3);
        } else if status.as_u16() == 500 && error_detail.is_empty() {
            eprintln!("Failed to create session (internal error)");
            eprintln!();
            eprintln!("Possible causes:");
            eprintln!("  • LLM API key not configured (export ANTHROPIC_API_KEY=...)");
            eprintln!("  • Daemon encountered an unexpected error");
            eprintln!();
            eprintln!("Check daemon logs for details: alan daemon start --foreground");
            return Ok(3);
        }

        anyhow::bail!("Failed to create session ({}): {}", status, error_detail);
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
            "type": "input",
            "parts": [{"type": "text", "text": question}]
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
        delete_session(&client, &base_url, &session_id).await;
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
        delete_session(&client, &base_url, &session_id).await;
        anyhow::bail!("Failed to stream events: {}", events_resp.status());
    }

    // Process event stream with timeout
    let timeout_dur = Duration::from_secs(timeout_secs);
    let code = match tokio::time::timeout(
        timeout_dur,
        process_events(events_resp, mode, show_thinking),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            eprintln!("Timeout after {}s", timeout_secs);
            EventResult {
                code: 2,
                accumulated_text: String::new(),
            }
        }
    };

    // In quiet mode or on timeout, flush accumulated text
    if !code.accumulated_text.is_empty() {
        print!("{}", code.accumulated_text);
        let _ = std::io::stdout().flush();
    }

    delete_session(&client, &base_url, &session_id).await;
    Ok(code.code)
}

struct EventResult {
    code: i32,
    accumulated_text: String,
}

async fn process_events(
    events_resp: reqwest::Response,
    mode: OutputMode,
    show_thinking: bool,
) -> EventResult {
    let mut stream = events_resp.bytes_stream();
    let mut buffer = String::new();
    let mut state = AskState {
        start: Instant::now(),
        tool_count: 0,
        tool_starts: HashMap::new(),
        accumulated_text: String::new(),
    };

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    match mode {
                        OutputMode::Json => {
                            // Pass through raw NDJSON
                            println!("{}", line);
                            // Still need to detect turn_completed to exit cleanly
                            if let Ok(env) = serde_json::from_str::<serde_json::Value>(&line)
                                && env.get("type").and_then(|t| t.as_str())
                                    == Some("turn_completed")
                            {
                                return EventResult {
                                    code: 0,
                                    accumulated_text: String::new(),
                                };
                            }
                        }
                        OutputMode::Text => {
                            if let Some(code) = handle_text_event(&line, &mut state, show_thinking)
                            {
                                return EventResult {
                                    code,
                                    accumulated_text: String::new(),
                                };
                            }
                        }
                        OutputMode::Quiet => {
                            if let Some(code) = handle_quiet_event(&line, &mut state) {
                                return EventResult {
                                    code,
                                    accumulated_text: std::mem::take(&mut state.accumulated_text),
                                };
                            }
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

    // Stream ended without turn_completed
    let text = std::mem::take(&mut state.accumulated_text);
    EventResult {
        code: 1,
        accumulated_text: text,
    }
}

/// Handle a single NDJSON line in text mode.
/// Returns Some(exit_code) if the stream should end.
fn handle_text_event(line: &str, state: &mut AskState, show_thinking: bool) -> Option<i32> {
    let envelope: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = envelope.get("type").and_then(|t| t.as_str())?;

    match event_type {
        "thinking_delta" => {
            if show_thinking && let Some(chunk) = envelope.get("chunk").and_then(|c| c.as_str()) {
                // Gray italic on stderr
                eprint!("\x1b[2;3m{}\x1b[0m", chunk);
                let _ = std::io::stderr().flush();
            }
        }
        "text_delta" => {
            if let Some(chunk) = envelope.get("chunk").and_then(|c| c.as_str()) {
                print!("{}", chunk);
                let _ = std::io::stdout().flush();
                state.accumulated_text.push_str(chunk);
            }
            // Legacy: also check "content" field
            if let Some(content) = envelope.get("content").and_then(|c| c.as_str()) {
                print!("{}", content);
                let _ = std::io::stdout().flush();
                state.accumulated_text.push_str(content);
            }
        }
        "message_delta" | "message_delta_chunk" => {
            // Legacy compat
            if let Some(content) = envelope.get("content").and_then(|c| c.as_str()) {
                print!("{}", content);
                let _ = std::io::stdout().flush();
                state.accumulated_text.push_str(content);
            }
        }
        "tool_call_started" => {
            let call_id = envelope
                .get("call_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let tool_name = envelope
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let arguments = envelope
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let arg = key_arg(&arguments);

            state.tool_count += 1;
            state
                .tool_starts
                .insert(call_id, (tool_name.to_string(), Instant::now()));

            eprint!("\x1b[2m🔧 {}({})\x1b[0m", tool_name, arg);
            let _ = std::io::stderr().flush();
        }
        "tool_call_completed" => {
            let call_id = envelope
                .get("call_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some((_name, start)) = state.tool_starts.remove(call_id) {
                let dur = fmt_duration(start.elapsed());
                eprintln!("\x1b[2m [{}]\x1b[0m", dur);
            } else {
                eprintln!();
            }
        }
        "error" => {
            if let Some(msg) = envelope.get("message").and_then(|m| m.as_str()) {
                eprintln!("❌ {}", msg);
            }
        }
        "turn_completed" => {
            // Final newline for streamed text
            println!();
            if state.tool_count > 0 {
                let elapsed = fmt_duration(state.start.elapsed());
                eprintln!(
                    "\x1b[2m── {} tool calls · {} ──\x1b[0m",
                    state.tool_count, elapsed
                );
            }
            return Some(0);
        }
        _ => {}
    }

    None
}

/// Handle a single NDJSON line in quiet mode.
/// Returns Some(exit_code) if the stream should end.
fn handle_quiet_event(line: &str, state: &mut AskState) -> Option<i32> {
    let envelope: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = envelope.get("type").and_then(|t| t.as_str())?;

    match event_type {
        "text_delta" => {
            if let Some(chunk) = envelope.get("chunk").and_then(|c| c.as_str()) {
                state.accumulated_text.push_str(chunk);
            }
            if let Some(content) = envelope.get("content").and_then(|c| c.as_str()) {
                state.accumulated_text.push_str(content);
            }
        }
        "message_delta" | "message_delta_chunk" => {
            if let Some(content) = envelope.get("content").and_then(|c| c.as_str()) {
                state.accumulated_text.push_str(content);
            }
        }
        "error" => {
            if let Some(msg) = envelope.get("message").and_then(|m| m.as_str()) {
                eprintln!("❌ {}", msg);
            }
        }
        "turn_completed" => {
            return Some(0);
        }
        _ => {}
    }

    None
}
