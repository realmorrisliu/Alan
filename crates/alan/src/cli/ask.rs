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
    tool_starts: HashMap<String, (String, Instant)>, // id -> (name, start_time)
    accumulated_text: String,
    last_event_id: Option<String>,
}

pub struct AskOptions {
    pub workspace: Option<PathBuf>,
    pub mode: OutputMode,
    pub show_thinking: bool,
    pub timeout_secs: u64,
    pub agent_name: Option<String>,
    pub streaming_mode: Option<alan_runtime::StreamingMode>,
    pub partial_stream_recovery_mode: Option<alan_runtime::PartialStreamRecoveryMode>,
}

/// Byte-safe NDJSON parser that tolerates chunk boundaries.
#[derive(Default)]
struct NdjsonLineParser {
    buffer: Vec<u8>,
}

impl NdjsonLineParser {
    fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);

        let mut lines = Vec::new();
        let mut consumed = 0usize;
        while let Some(rel_pos) = self.buffer[consumed..]
            .iter()
            .position(|byte| *byte == b'\n')
        {
            let end = consumed + rel_pos;
            let line_bytes = self.buffer[consumed..end]
                .strip_suffix(b"\r")
                .unwrap_or(&self.buffer[consumed..end])
                .to_vec();
            consumed = end + 1;
            if line_bytes.is_empty() {
                continue;
            }
            match String::from_utf8(line_bytes) {
                Ok(line) => lines.push(line),
                Err(err) => {
                    eprintln!(
                        "Warning: dropped invalid UTF-8 event line ({} bytes)",
                        err.into_bytes().len()
                    );
                }
            }
        }
        if consumed > 0 {
            self.buffer.drain(..consumed);
        }
        lines
    }

    fn finish(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            return None;
        }
        let line_bytes = std::mem::take(&mut self.buffer);
        let line_bytes = line_bytes
            .strip_suffix(b"\r")
            .unwrap_or(&line_bytes)
            .to_vec();
        if line_bytes.is_empty() {
            return None;
        }
        match String::from_utf8(line_bytes) {
            Ok(line) => Some(line),
            Err(err) => {
                eprintln!(
                    "Warning: dropped invalid UTF-8 trailing event line ({} bytes)",
                    err.into_bytes().len()
                );
                None
            }
        }
    }
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
pub async fn run_ask(question: &str, options: AskOptions) -> i32 {
    match run_ask_inner(question, options).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {:#}", e);
            1
        }
    }
}

async fn run_ask_inner(question: &str, options: AskOptions) -> Result<i32, anyhow::Error> {
    use anyhow::Context;

    let AskOptions {
        workspace,
        mode,
        show_thinking,
        timeout_secs,
        agent_name,
        streaming_mode,
        partial_stream_recovery_mode,
    } = options;

    // Track whether `ask` spawned the daemon so we can tear it down on exit.
    let daemon_started_by_ask = super::daemon::ensure_daemon_running_with_state().await?;

    let result: Result<i32, anyhow::Error> = async {
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
        if let Some(agent_name) = agent_name {
            create_body.insert(
                "agent_name".to_string(),
                serde_json::Value::String(agent_name),
            );
        }
        // `ask` is a one-shot non-interactive mode: favor autonomous execution.
        create_body.insert(
            "governance".to_string(),
            serde_json::json!({
                "profile": "autonomous"
            }),
        );
        if let Some(mode) = streaming_mode {
            let mode_value = match mode {
                alan_runtime::StreamingMode::Auto => "auto",
                alan_runtime::StreamingMode::On => "on",
                alan_runtime::StreamingMode::Off => "off",
            };
            create_body.insert(
                "streaming_mode".to_string(),
                serde_json::Value::String(mode_value.to_string()),
            );
        }
        if let Some(mode) = partial_stream_recovery_mode {
            let mode_value = match mode {
                alan_runtime::PartialStreamRecoveryMode::ContinueOnce => "continue_once",
                alan_runtime::PartialStreamRecoveryMode::Off => "off",
            };
            create_body.insert(
                "partial_stream_recovery_mode".to_string(),
                serde_json::Value::String(mode_value.to_string()),
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
                let canonical_config_path = alan_runtime::AlanHomePaths::detect()
                    .map(|paths| paths.global_agent_config_path)
                    .unwrap_or_else(|| std::path::PathBuf::from("~/.alan/agent/agent.toml"));
                eprintln!("Error: {}", error_detail);
                eprintln!();
                eprintln!("Hint: Make sure your LLM config is set.");
                eprintln!("  {}", canonical_config_path.display());
                eprintln!("  (or set ALAN_CONFIG_PATH to a custom file)");
                return Ok(3);
            } else if status.as_u16() == 500 && error_detail.is_empty() {
                eprintln!("Failed to create session (internal error)");
                eprintln!();
                eprintln!("Possible causes:");
                eprintln!("  • LLM config is missing or invalid");
                eprintln!("    (~/.alan/agent/agent.toml or ALAN_CONFIG_PATH)");
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
                "type": "turn",
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
        let code = process_events(
            events_resp,
            &client,
            &base_url,
            &session_id,
            mode,
            show_thinking,
            timeout_secs,
        )
        .await;

        // Flush accumulated text for quiet mode.
        if !code.accumulated_text.is_empty() {
            print!("{}", code.accumulated_text);
            let _ = std::io::stdout().flush();
        }

        delete_session(&client, &base_url, &session_id).await;
        Ok(code.code)
    }
    .await;

    if daemon_started_by_ask && let Err(err) = super::daemon::stop_daemon().await {
        eprintln!("Warning: failed to stop daemon started by `alan ask`: {err}");
    }

    result
}

struct EventResult {
    code: i32,
    accumulated_text: String,
}

#[derive(serde::Deserialize)]
struct ReadEventsResponse {
    #[serde(default)]
    gap: bool,
    #[serde(default)]
    oldest_event_id: Option<String>,
    #[serde(default)]
    latest_event_id: Option<String>,
    #[serde(default)]
    events: Vec<serde_json::Value>,
}

async fn process_events(
    events_resp: reqwest::Response,
    client: &reqwest::Client,
    base_url: &str,
    session_id: &str,
    mode: OutputMode,
    show_thinking: bool,
    timeout_secs: u64,
) -> EventResult {
    let mut stream = events_resp.bytes_stream();
    let mut parser = NdjsonLineParser::default();
    let mut state = AskState {
        start: Instant::now(),
        tool_count: 0,
        tool_starts: HashMap::new(),
        accumulated_text: String::new(),
        last_event_id: None,
    };
    let timeout_deadline = Instant::now() + Duration::from_secs(timeout_secs);

    let make_result = |code: i32, state: &mut AskState| EventResult {
        code,
        accumulated_text: if matches!(mode, OutputMode::Quiet) {
            std::mem::take(&mut state.accumulated_text)
        } else {
            String::new()
        },
    };

    loop {
        let now = Instant::now();
        if now >= timeout_deadline {
            eprintln!("Timeout after {}s", timeout_secs);
            return make_result(2, &mut state);
        }

        let remaining = timeout_deadline - now;
        let next_chunk = match tokio::time::timeout(remaining, stream.next()).await {
            Ok(item) => item,
            Err(_) => {
                eprintln!("Timeout after {}s", timeout_secs);
                return make_result(2, &mut state);
            }
        };

        let Some(chunk) = next_chunk else {
            break;
        };

        match chunk {
            Ok(bytes) => {
                for line in parser.push(&bytes) {
                    if let Some(code) = process_line_by_mode(&line, mode, show_thinking, &mut state)
                    {
                        return make_result(code, &mut state);
                    }
                }
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                break;
            }
        }
    }

    if let Some(line) = parser.finish()
        && let Some(code) = process_line_by_mode(&line, mode, show_thinking, &mut state)
    {
        return make_result(code, &mut state);
    }

    if let Some(code) = replay_after_stream_end(
        client,
        base_url,
        session_id,
        mode,
        show_thinking,
        timeout_deadline,
        &mut state,
    )
    .await
    {
        return make_result(code, &mut state);
    }

    // Stream ended without turn_completed and replay did not complete the turn.
    make_result(1, &mut state)
}

fn line_type(line: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|env| env.get("type").and_then(|t| t.as_str()).map(str::to_string))
}

fn process_line_by_mode(
    line: &str,
    mode: OutputMode,
    show_thinking: bool,
    state: &mut AskState,
) -> Option<i32> {
    if let Ok(env) = serde_json::from_str::<serde_json::Value>(line)
        && let Some(event_id) = env.get("event_id").and_then(|v| v.as_str())
    {
        state.last_event_id = Some(event_id.to_string());
    }

    match mode {
        OutputMode::Json => {
            println!("{}", line);
            if line_type(line).as_deref() == Some("turn_completed") {
                return Some(0);
            }
            None
        }
        OutputMode::Text => handle_text_event(line, state, show_thinking),
        OutputMode::Quiet => handle_quiet_event(line, state),
    }
}

async fn replay_after_stream_end(
    client: &reqwest::Client,
    base_url: &str,
    session_id: &str,
    mode: OutputMode,
    show_thinking: bool,
    timeout_deadline: Instant,
    state: &mut AskState,
) -> Option<i32> {
    const REPLAY_PAGE_LIMIT: usize = 200;
    const MAX_REPLAY_EVENTS: usize = 20_000;

    eprintln!(
        "Warning: event stream ended before turn completion; attempting replay from buffered events."
    );

    let mut replayed = 0usize;
    let mut after_event_id = state.last_event_id.clone();
    loop {
        let now = Instant::now();
        if now >= timeout_deadline {
            eprintln!("Timeout while replaying buffered events");
            return Some(2);
        }
        let remaining = timeout_deadline - now;

        let mut replay_url = format!(
            "{}/api/v1/sessions/{}/events/read?limit={}",
            base_url, session_id, REPLAY_PAGE_LIMIT
        );
        if let Some(after) = &after_event_id {
            replay_url.push_str("&after_event_id=");
            replay_url.push_str(after);
        }
        let request = client.get(replay_url);

        let response = match tokio::time::timeout(remaining, request.send()).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(err)) => {
                eprintln!("Warning: replay request failed: {err}");
                return None;
            }
            Err(_) => {
                eprintln!("Timeout while replaying buffered events");
                return Some(2);
            }
        };
        if !response.status().is_success() {
            eprintln!(
                "Warning: replay request failed with status {}",
                response.status()
            );
            return None;
        }

        let page = match response.json::<ReadEventsResponse>().await {
            Ok(page) => page,
            Err(err) => {
                eprintln!("Warning: failed to decode replay page: {err}");
                return None;
            }
        };

        if page.gap {
            let oldest = page.oldest_event_id.as_deref().unwrap_or("unknown");
            let latest = page.latest_event_id.as_deref().unwrap_or("unknown");
            eprintln!(
                "Warning: replay gap detected (oldest={oldest}, latest={latest}); output may be incomplete."
            );
        }

        if page.events.is_empty() {
            return None;
        }

        for envelope in &page.events {
            let line = match serde_json::to_string(envelope) {
                Ok(line) => line,
                Err(err) => {
                    eprintln!("Warning: failed to serialize replay event: {err}");
                    continue;
                }
            };
            if let Some(code) = process_line_by_mode(&line, mode, show_thinking, state) {
                return Some(code);
            }
        }

        replayed += page.events.len();
        if replayed > MAX_REPLAY_EVENTS {
            eprintln!(
                "Warning: replay exceeded safety limit ({} events); stopping replay.",
                MAX_REPLAY_EVENTS
            );
            return None;
        }

        let page_last_event_id = page
            .events
            .last()
            .and_then(|event| event.get("event_id"))
            .and_then(|event_id| event_id.as_str())
            .map(str::to_string);
        let Some(page_last_event_id) = page_last_event_id else {
            eprintln!("Warning: replay page contained event without event_id.");
            return None;
        };
        state.last_event_id = Some(page_last_event_id.clone());
        after_event_id = Some(page_last_event_id.clone());

        if page.events.len() < REPLAY_PAGE_LIMIT
            || page.latest_event_id.as_deref() == Some(page_last_event_id.as_str())
        {
            return None;
        }
    }
}

/// Handle a single NDJSON line in text mode.
/// Returns Some(exit_code) if the stream should end.
fn handle_text_event(line: &str, state: &mut AskState, show_thinking: bool) -> Option<i32> {
    let envelope: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Warning: failed to parse event JSON: {err}");
            return None;
        }
    };
    let Some(event_type) = envelope.get("type").and_then(|t| t.as_str()) else {
        eprintln!("Warning: dropped event without type field");
        return None;
    };

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
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let tool_name = envelope
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            state.tool_count += 1;
            state
                .tool_starts
                .insert(call_id, (tool_name.to_string(), Instant::now()));

            eprint!("\x1b[2m🔧 {}\x1b[0m", tool_name);
            let _ = std::io::stderr().flush();
        }
        "tool_call_completed" => {
            let call_id = envelope.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if let Some((_name, start)) = state.tool_starts.remove(call_id) {
                let dur = fmt_duration(start.elapsed());
                let preview = envelope
                    .get("result_preview")
                    .and_then(|v| v.as_str())
                    .filter(|value| !value.trim().is_empty());
                if let Some(preview) = preview {
                    eprintln!("\x1b[2m [{}] {}\x1b[0m", dur, preview);
                } else {
                    eprintln!("\x1b[2m [{}]\x1b[0m", dur);
                }
            } else {
                eprintln!();
            }
        }
        "error" => {
            if let Some(msg) = envelope.get("message").and_then(|m| m.as_str()) {
                let recoverable = envelope
                    .get("recoverable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if recoverable {
                    eprintln!("⚠ {}", msg);
                } else {
                    eprintln!("❌ {}", msg);
                }
            }
        }
        "warning" => {
            if let Some(msg) = envelope.get("message").and_then(|m| m.as_str()) {
                eprintln!("⚠ {}", msg);
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
    let envelope: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Warning: failed to parse event JSON: {err}");
            return None;
        }
    };
    let Some(event_type) = envelope.get("type").and_then(|t| t.as_str()) else {
        eprintln!("Warning: dropped event without type field");
        return None;
    };

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
                let recoverable = envelope
                    .get("recoverable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if recoverable {
                    eprintln!("⚠ {}", msg);
                } else {
                    eprintln!("❌ {}", msg);
                }
            }
        }
        "warning" => {
            if let Some(msg) = envelope.get("message").and_then(|m| m.as_str()) {
                eprintln!("⚠ {}", msg);
            }
        }
        "turn_completed" => {
            return Some(0);
        }
        _ => {}
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{AskState, NdjsonLineParser, process_line_by_mode};
    use crate::OutputMode;
    use std::collections::HashMap;
    use tokio::time::Instant;

    #[test]
    fn test_ndjson_parser_handles_utf8_split_across_chunks() {
        let mut parser = NdjsonLineParser::default();
        let line = "{\"type\":\"text_delta\",\"chunk\":\"😀\"}\n".as_bytes();
        let split = line.len().saturating_sub(3);

        let first = parser.push(&line[..split]);
        assert!(first.is_empty());

        let second = parser.push(&line[split..]);
        assert_eq!(second, vec!["{\"type\":\"text_delta\",\"chunk\":\"😀\"}"]);
    }

    #[test]
    fn test_ndjson_parser_flushes_trailing_line_without_newline() {
        let mut parser = NdjsonLineParser::default();
        assert!(parser.push(b"{\"type\":\"turn_completed\"}").is_empty());
        assert_eq!(
            parser.finish(),
            Some("{\"type\":\"turn_completed\"}".to_string())
        );
    }

    #[test]
    fn test_process_line_by_mode_tracks_event_id_and_turn_completion() {
        let mut state = AskState {
            start: Instant::now(),
            tool_count: 0,
            tool_starts: HashMap::new(),
            accumulated_text: String::new(),
            last_event_id: None,
        };

        let line =
            r#"{"event_id":"evt_0000000000000002","type":"turn_completed","summary":"done"}"#;
        let code = process_line_by_mode(line, OutputMode::Quiet, false, &mut state);
        assert_eq!(code, Some(0));
        assert_eq!(state.last_event_id.as_deref(), Some("evt_0000000000000002"));
    }

    #[test]
    fn test_process_line_by_mode_accumulates_quiet_text() {
        let mut state = AskState {
            start: Instant::now(),
            tool_count: 0,
            tool_starts: HashMap::new(),
            accumulated_text: String::new(),
            last_event_id: None,
        };

        let line = r#"{"event_id":"evt_0000000000000009","type":"text_delta","chunk":"hello"}"#;
        let code = process_line_by_mode(line, OutputMode::Quiet, false, &mut state);
        assert_eq!(code, None);
        assert_eq!(state.accumulated_text, "hello");
        assert_eq!(state.last_event_id.as_deref(), Some("evt_0000000000000009"));
    }
}
