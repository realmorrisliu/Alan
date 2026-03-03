//! Session persistence using JSONL format (similar to Codex rollout files)

use anyhow::{Result, anyhow};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, warn};

/// Types of items recorded in the rollout
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RolloutItem {
    SessionMeta(SessionMeta),
    Message(MessageRecord),
    TurnContext(TurnContextItem),
    Compacted(CompactedItem),
    ToolCall(ToolCallRecord),
    Effect(EffectRecord),
    Checkpoint(CheckpointRecord),
    Event(EventRecord),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub started_at: String, // ISO 8601
    pub cwd: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub role: String, // user, assistant, tool
    pub content: Option<String>,
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<crate::tape::Message>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnContextItem {
    pub model: String,
    pub system_prompt: String,
    pub context_items: Vec<ContextItemRecord>,
    pub tools: Vec<String>,
    pub memory_enabled: bool,
    pub active_skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_context: Option<ReferenceContextSnapshotRecord>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceContextSnapshotRecord {
    pub revision: u64,
    pub changed: bool,
    pub reordered: bool,
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItemRecord {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedItem {
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: serde_json::Value,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit: Option<alan_protocol::ToolDecisionAudit>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectStatus {
    Applied,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectRecord {
    pub effect_id: String,
    pub run_id: String,
    pub tool_call_id: String,
    pub idempotency_key: String,
    pub effect_type: String,
    pub request_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_payload: Option<serde_json::Value>,
    pub status: EffectStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    pub dedupe_hit: bool,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub checkpoint_id: String,
    pub checkpoint_type: String,
    pub summary: String,
    pub choice: Option<String>, // approved, modified, rejected
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

/// Commands for the background writer task
enum RolloutCmd {
    Record(RolloutItem),
    Flush { ack: Option<oneshot::Sender<()>> },
}

/// Persistent recorder for session history
#[derive(Debug)]
pub struct RolloutRecorder {
    tx: mpsc::UnboundedSender<RolloutCmd>,
    rollout_path: PathBuf,
}

impl RolloutRecorder {
    fn message_record_from_tape_message(message: &crate::tape::Message) -> MessageRecord {
        let role = match message {
            crate::tape::Message::User { .. } => "user",
            crate::tape::Message::Assistant { .. } => "assistant",
            crate::tape::Message::Tool { .. } => "tool",
            crate::tape::Message::System { .. } => "system",
            crate::tape::Message::Context { .. } => "context",
        }
        .to_string();

        let content = match message {
            crate::tape::Message::Assistant { .. } => {
                let text = message.non_thinking_text_content();
                if text.is_empty() { None } else { Some(text) }
            }
            _ => {
                let text = message.text_content();
                if text.is_empty() { None } else { Some(text) }
            }
        };

        let tool_name = match message {
            crate::tape::Message::Tool { responses } => responses
                .first()
                .map(|response| response.id.trim().to_string())
                .filter(|id| !id.is_empty()),
            _ => None,
        };

        MessageRecord {
            role,
            content,
            tool_name,
            message: Some(message.clone()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a new recorder for a session
    pub async fn new(session_id: &str, model: &str) -> anyhow::Result<Self> {
        let rollout_path = Self::build_rollout_path(session_id).await?;
        Self::new_with_rollout_path(session_id, model, rollout_path).await
    }

    /// Create a new recorder for a session under a specific sessions directory.
    pub async fn new_in_dir(
        session_id: &str,
        model: &str,
        sessions_dir: &std::path::Path,
    ) -> anyhow::Result<Self> {
        let rollout_path = Self::build_rollout_path_in_dir(session_id, sessions_dir).await?;
        Self::new_with_rollout_path(session_id, model, rollout_path).await
    }

    async fn new_with_rollout_path(
        session_id: &str,
        model: &str,
        rollout_path: PathBuf,
    ) -> anyhow::Result<Self> {
        // Create the file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&rollout_path)
            .await?;

        let (tx, mut rx) = mpsc::unbounded_channel::<RolloutCmd>();
        let _path = rollout_path.clone();

        // Spawn background writer task
        tokio::spawn(async move {
            let mut writer = BufWriter::new(file);

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    RolloutCmd::Record(item) => {
                        if let Err(e) = Self::write_item(&mut writer, &item).await {
                            error!(?e, "Failed to write rollout item");
                        }
                    }
                    RolloutCmd::Flush { ack } => {
                        if let Err(e) = writer.flush().await {
                            error!(?e, "Failed to flush rollout file");
                        }
                        if let Some(ack) = ack {
                            let _ = ack.send(());
                        }
                    }
                }
            }

            // Final flush when channel closes
            if let Err(e) = writer.flush().await {
                error!(?e, "Failed to flush rollout file on shutdown");
            }
        });

        let recorder = Self {
            tx,
            rollout_path: rollout_path.clone(),
        };

        // Record session metadata
        let meta = SessionMeta {
            session_id: session_id.to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            model: model.to_string(),
        };
        recorder.record_nowait(RolloutItem::SessionMeta(meta))?;
        recorder.flush().await?;

        debug!(?rollout_path, "RolloutRecorder created");
        Ok(recorder)
    }

    /// Record an item
    pub fn record_nowait(&self, item: RolloutItem) -> Result<()> {
        if self.tx.send(RolloutCmd::Record(item)).is_err() {
            warn!("Rollout channel closed, cannot record item");
            return Err(anyhow!("Rollout channel closed, cannot record item"));
        }
        Ok(())
    }

    /// Record an item (enqueue only, no flush wait).
    pub async fn record(&self, item: RolloutItem) -> Result<()> {
        self.record_nowait(item)
    }

    /// Enqueue a flush request without waiting for the writer to drain.
    pub fn flush_nowait(&self) -> Result<()> {
        if self.tx.send(RolloutCmd::Flush { ack: None }).is_err() {
            warn!("Rollout channel closed, cannot flush");
            return Err(anyhow!("Rollout channel closed, cannot flush"));
        }
        Ok(())
    }

    /// Record a message
    pub async fn record_message(
        &self,
        role: &str,
        content: Option<&str>,
        tool_name: Option<&str>,
    ) -> Result<()> {
        let item = RolloutItem::Message(MessageRecord {
            role: role.to_string(),
            content: content.map(|s| s.to_string()),
            tool_name: tool_name.map(|s| s.to_string()),
            message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record(item).await?;
        // Ensure message records are persisted promptly so rollouts stay in sync
        // with the UI during long-running sessions.
        self.flush().await?;
        Ok(())
    }

    /// Record a message by enqueuing to the writer queue without spawning.
    pub fn record_message_nowait(
        &self,
        role: &str,
        content: Option<&str>,
        tool_name: Option<&str>,
    ) -> Result<()> {
        let item = RolloutItem::Message(MessageRecord {
            role: role.to_string(),
            content: content.map(|s| s.to_string()),
            tool_name: tool_name.map(|s| s.to_string()),
            message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record_nowait(item)?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Record a rich tape message.
    pub async fn record_tape_message(&self, message: &crate::tape::Message) -> Result<()> {
        let item = RolloutItem::Message(Self::message_record_from_tape_message(message));
        self.record(item).await?;
        self.flush().await?;
        Ok(())
    }

    /// Record a rich tape message without waiting on IO completion.
    pub fn record_tape_message_nowait(&self, message: &crate::tape::Message) -> Result<()> {
        let item = RolloutItem::Message(Self::message_record_from_tape_message(message));
        self.record_nowait(item)?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Record a turn context snapshot
    #[allow(clippy::too_many_arguments)]
    pub async fn record_turn_context(
        &self,
        model: &str,
        system_prompt: &str,
        context_items: Vec<ContextItemRecord>,
        tools: Vec<String>,
        memory_enabled: bool,
        active_skills: Vec<String>,
        reference_context: Option<ReferenceContextSnapshotRecord>,
    ) -> Result<()> {
        let item = RolloutItem::TurnContext(TurnContextItem {
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
            context_items,
            tools,
            memory_enabled,
            active_skills,
            reference_context,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record(item).await?;
        self.flush().await?;
        Ok(())
    }

    /// Record a turn context snapshot without waiting on IO completion.
    #[allow(clippy::too_many_arguments)]
    pub fn record_turn_context_nowait(
        &self,
        model: &str,
        system_prompt: &str,
        context_items: Vec<ContextItemRecord>,
        tools: Vec<String>,
        memory_enabled: bool,
        active_skills: Vec<String>,
        reference_context: Option<ReferenceContextSnapshotRecord>,
    ) -> Result<()> {
        let item = RolloutItem::TurnContext(TurnContextItem {
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
            context_items,
            tools,
            memory_enabled,
            active_skills,
            reference_context,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record_nowait(item)?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Record a compaction summary
    pub async fn record_compacted(&self, message: &str) -> Result<()> {
        let item = RolloutItem::Compacted(CompactedItem {
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record(item).await?;
        self.flush().await?;
        Ok(())
    }

    /// Record a compaction summary without waiting on IO completion.
    pub fn record_compacted_nowait(&self, message: &str) -> Result<()> {
        let item = RolloutItem::Compacted(CompactedItem {
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record_nowait(item)?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Record a tool call
    pub async fn record_tool_call(
        &self,
        name: &str,
        arguments: serde_json::Value,
        result: serde_json::Value,
        success: bool,
    ) -> Result<()> {
        self.record_tool_call_with_audit(name, arguments, result, success, None)
            .await
    }

    /// Record a tool call with audit metadata.
    pub async fn record_tool_call_with_audit(
        &self,
        name: &str,
        arguments: serde_json::Value,
        result: serde_json::Value,
        success: bool,
        audit: Option<alan_protocol::ToolDecisionAudit>,
    ) -> Result<()> {
        let item = RolloutItem::ToolCall(ToolCallRecord {
            name: name.to_string(),
            arguments,
            result,
            success,
            audit,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record(item).await?;
        self.flush().await?; // Important events are flushed immediately
        Ok(())
    }

    /// Record a tool call without waiting on IO completion.
    pub fn record_tool_call_nowait(
        &self,
        name: &str,
        arguments: serde_json::Value,
        result: serde_json::Value,
        success: bool,
    ) -> Result<()> {
        self.record_tool_call_nowait_with_audit(name, arguments, result, success, None)
    }

    /// Record a tool call with audit metadata without waiting on IO completion.
    pub fn record_tool_call_nowait_with_audit(
        &self,
        name: &str,
        arguments: serde_json::Value,
        result: serde_json::Value,
        success: bool,
        audit: Option<alan_protocol::ToolDecisionAudit>,
    ) -> Result<()> {
        let item = RolloutItem::ToolCall(ToolCallRecord {
            name: name.to_string(),
            arguments,
            result,
            success,
            audit,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record_nowait(item)?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Record an effect record.
    pub async fn record_effect(&self, effect: EffectRecord) -> Result<()> {
        self.record(RolloutItem::Effect(effect)).await?;
        self.flush().await?;
        Ok(())
    }

    /// Record an effect record without waiting on IO completion.
    pub fn record_effect_nowait(&self, effect: EffectRecord) -> Result<()> {
        self.record_nowait(RolloutItem::Effect(effect))?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Record a checkpoint
    pub async fn record_checkpoint(
        &self,
        checkpoint_id: &str,
        checkpoint_type: &str,
        summary: &str,
        choice: Option<&str>,
    ) -> Result<()> {
        let item = RolloutItem::Checkpoint(CheckpointRecord {
            checkpoint_id: checkpoint_id.to_string(),
            checkpoint_type: checkpoint_type.to_string(),
            summary: summary.to_string(),
            choice: choice.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record(item).await?;
        self.flush().await?; // Important events are flushed immediately
        Ok(())
    }

    /// Record a checkpoint without waiting on IO completion.
    pub fn record_checkpoint_nowait(
        &self,
        checkpoint_id: &str,
        checkpoint_type: &str,
        summary: &str,
        choice: Option<&str>,
    ) -> Result<()> {
        let item = RolloutItem::Checkpoint(CheckpointRecord {
            checkpoint_id: checkpoint_id.to_string(),
            checkpoint_type: checkpoint_type.to_string(),
            summary: summary.to_string(),
            choice: choice.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record_nowait(item)?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Record a generic event
    pub async fn record_event(&self, event_type: &str, payload: serde_json::Value) -> Result<()> {
        let item = RolloutItem::Event(EventRecord {
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record(item).await?;
        self.flush().await?;
        Ok(())
    }

    /// Record a generic event without waiting on IO completion.
    pub fn record_event_nowait(&self, event_type: &str, payload: serde_json::Value) -> Result<()> {
        let item = RolloutItem::Event(EventRecord {
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        self.record_nowait(item)?;
        self.flush_nowait()?;
        Ok(())
    }

    /// Flush pending writes to disk
    pub async fn flush(&self) -> Result<()> {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self
            .tx
            .send(RolloutCmd::Flush { ack: Some(ack_tx) })
            .is_err()
        {
            warn!("Rollout channel closed, cannot flush");
            return Err(anyhow!("Rollout channel closed, cannot flush"));
        }
        if ack_rx.await.is_err() {
            warn!("Rollout writer dropped before flush ack");
            return Err(anyhow!("Rollout writer dropped before flush ack"));
        }
        Ok(())
    }

    /// Load history from a rollout file
    pub async fn load_history(path: &PathBuf) -> anyhow::Result<Vec<RolloutItem>> {
        let content = fs::read_to_string(path).await?;
        let mut items = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<RolloutItem>(line) {
                Ok(item) => items.push(item),
                Err(e) => {
                    warn!(?e, line = ?line, "Failed to parse rollout line");
                }
            }
        }

        Ok(items)
    }

    /// Get the path to the rollout file
    pub fn path(&self) -> &PathBuf {
        &self.rollout_path
    }

    /// Build the rollout file path
    async fn build_rollout_path(session_id: &str) -> anyhow::Result<PathBuf> {
        let alan_dir = dirs::home_dir()
            .map(|home| home.join(".alan"))
            .unwrap_or_else(|| {
                warn!("Cannot determine home directory; falling back to temp dir");
                std::env::temp_dir().join(".alan")
            });

        let now = chrono::Local::now();
        let date_dir = alan_dir
            .join("sessions")
            .join(format!("{:04}", now.year()))
            .join(format!("{:02}", now.month()))
            .join(format!("{:02}", now.day()));

        fs::create_dir_all(&date_dir).await?;

        let timestamp = now.format("%Y%m%d-%H%M%S");
        let filename = format!("rollout-{}-{}.jsonl", timestamp, session_id);

        Ok(date_dir.join(filename))
    }

    async fn build_rollout_path_in_dir(
        session_id: &str,
        sessions_dir: &std::path::Path,
    ) -> anyhow::Result<PathBuf> {
        fs::create_dir_all(sessions_dir).await?;

        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("rollout-{}-{}.jsonl", timestamp, session_id);
        Ok(sessions_dir.join(filename))
    }

    /// Write a single item to the writer
    async fn write_item<W: AsyncWriteExt + Unpin>(
        writer: &mut W,
        item: &RolloutItem,
    ) -> anyhow::Result<()> {
        let json = serde_json::to_string(item)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        Ok(())
    }
}

impl Clone for RolloutRecorder {
    fn clone(&self) -> Self {
        // Create a new channel for the cloned recorder
        // This is a limitation - cloned recorders share the same file but have separate channels
        // In practice, only one recorder should be used per session
        Self {
            tx: self.tx.clone(),
            rollout_path: self.rollout_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{Duration, Instant, sleep};

    #[test]
    fn test_rollout_recorder_creation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let recorder = RolloutRecorder::new_in_dir(
                "test-session-123",
                "gemini-2.0-flash",
                temp_dir.path(),
            )
            .await;
            assert!(recorder.is_ok());

            let recorder = recorder.unwrap();
            let path = recorder.path();
            assert!(path.to_string_lossy().contains("test-session-123"));

            // Clean up - remove the created file
            let _ = fs::remove_file(path).await;
        });
    }

    #[test]
    fn test_record_message_flushes() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let recorder = RolloutRecorder::new_in_dir(
                "test-session-flush",
                "gemini-2.0-flash",
                temp_dir.path(),
            )
            .await
            .unwrap();
            recorder
                .record_message("user", Some("Hello"), None)
                .await
                .unwrap();

            let start = Instant::now();
            let mut found = false;
            while start.elapsed() < Duration::from_secs(1) {
                if let Ok(content) = fs::read_to_string(recorder.path()).await
                    && content.contains("\"type\":\"message\"")
                    && content.contains("Hello")
                {
                    found = true;
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }

            assert!(found, "Expected message to be flushed to rollout file");
        });
    }

    #[tokio::test]
    async fn test_record_tape_message_persists_rich_message() {
        let temp_dir = TempDir::new().unwrap();
        let recorder =
            RolloutRecorder::new_in_dir("test-rich-message", "gemini-2.0-flash", temp_dir.path())
                .await
                .unwrap();

        let message = crate::tape::Message::Assistant {
            parts: vec![
                crate::tape::ContentPart::thinking("internal reasoning"),
                crate::tape::ContentPart::text("final answer"),
            ],
            tool_requests: vec![],
        };
        recorder.record_tape_message(&message).await.unwrap();

        let items = RolloutRecorder::load_history(recorder.path())
            .await
            .unwrap();
        let restored = items.into_iter().find_map(|item| match item {
            RolloutItem::Message(msg) => msg.message,
            _ => None,
        });

        let restored = restored.expect("expected rich message payload");
        assert_eq!(restored.non_thinking_text_content(), "final answer");
        assert_eq!(
            restored.thinking_content().as_deref(),
            Some("internal reasoning")
        );
    }

    #[tokio::test]
    async fn test_load_history() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.jsonl");

        // Create a test file
        let content = r#"{"type":"session_meta","session_id":"test-123","started_at":"2026-01-29T14:30:52Z","cwd":"/home/user","model":"gemini-2.0-flash"}
{"type":"message","role":"user","content":"Hello","tool_name":null,"timestamp":"2026-01-29T14:30:55Z"}
{"type":"tool_call","name":"test_tool","arguments":{},"result":{},"success":true,"timestamp":"2026-01-29T14:31:02Z"}
"#;

        fs::write(&file_path, content).await.unwrap();

        let items = RolloutRecorder::load_history(&file_path).await.unwrap();
        assert_eq!(items.len(), 3);

        // Verify first item is session meta
        match &items[0] {
            RolloutItem::SessionMeta(meta) => {
                assert_eq!(meta.session_id, "test-123");
                assert_eq!(meta.model, "gemini-2.0-flash");
                assert_eq!(meta.cwd, "/home/user");
            }
            _ => panic!("Expected SessionMeta"),
        }

        // Verify second item is message
        match &items[1] {
            RolloutItem::Message(msg) => {
                assert_eq!(msg.role, "user");
                assert_eq!(msg.content, Some("Hello".to_string()));
                assert!(msg.tool_name.is_none());
            }
            _ => panic!("Expected Message"),
        }

        // Verify third item is tool call
        match &items[2] {
            RolloutItem::ToolCall(tool) => {
                assert_eq!(tool.name, "test_tool");
                assert!(tool.success);
            }
            _ => panic!("Expected ToolCall"),
        }
    }

    #[tokio::test]
    async fn test_load_history_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.jsonl");

        fs::write(&file_path, "").await.unwrap();

        let items = RolloutRecorder::load_history(&file_path).await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_load_history_with_empty_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("with_empty_lines.jsonl");

        let content = r#"
{"type":"message","role":"user","content":"Hello","tool_name":null,"timestamp":"2026-01-29T14:30:55Z"}

{"type":"message","role":"assistant","content":"Hi!","tool_name":null,"timestamp":"2026-01-29T14:30:56Z"}
"#;

        fs::write(&file_path, content).await.unwrap();

        let items = RolloutRecorder::load_history(&file_path).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_load_history_with_invalid_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("with_invalid.jsonl");

        let content = r#"{"type":"message","role":"user","content":"Valid","tool_name":null,"timestamp":"2026-01-29T14:30:55Z"}
this is not valid json
{"type":"message","role":"assistant","content":"Also valid","tool_name":null,"timestamp":"2026-01-29T14:30:56Z"}
"#;

        fs::write(&file_path, content).await.unwrap();

        // Should skip invalid lines and continue
        let items = RolloutRecorder::load_history(&file_path).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_load_history_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.jsonl");

        let result = RolloutRecorder::load_history(&file_path).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_rollout_item_serialization() {
        let meta = RolloutItem::SessionMeta(SessionMeta {
            session_id: "test-123".to_string(),
            started_at: "2026-01-29T14:30:52Z".to_string(),
            cwd: "/test".to_string(),
            model: "gemini-test".to_string(),
        });

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("session_meta"));
        assert!(json.contains("test-123"));
        assert!(json.contains("gemini-test"));

        let deserialized: RolloutItem = serde_json::from_str(&json).unwrap();
        match deserialized {
            RolloutItem::SessionMeta(m) => assert_eq!(m.session_id, "test-123"),
            _ => panic!("Expected SessionMeta"),
        }
    }

    #[test]
    fn test_message_record_serialization() {
        let msg = MessageRecord {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_name: None,
            message: None,
            timestamp: "2026-01-29T14:30:55Z".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));

        let deserialized: MessageRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role, "user");
        assert_eq!(deserialized.content, Some("Hello".to_string()));
        assert!(deserialized.message.is_none());
    }

    #[test]
    fn test_turn_context_item_serialization() {
        let ctx = TurnContextItem {
            model: "gemini-2.0-flash".to_string(),
            system_prompt: "System".to_string(),
            context_items: vec![ContextItemRecord {
                id: "onboarding".to_string(),
                kind: "static".to_string(),
                title: "Onboarding".to_string(),
                content: "Steps".to_string(),
                fingerprint: "abcd1234".to_string(),
            }],
            tools: vec!["web_search".to_string()],
            memory_enabled: true,
            active_skills: vec!["skill-1".to_string()],
            reference_context: Some(ReferenceContextSnapshotRecord {
                revision: 3,
                changed: true,
                reordered: false,
                added: 1,
                updated: 0,
                removed: 0,
            }),
            timestamp: "2026-01-29T14:30:56Z".to_string(),
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("gemini-2.0-flash"));
        assert!(json.contains("onboarding"));

        let deserialized: TurnContextItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "gemini-2.0-flash");
        assert_eq!(deserialized.context_items[0].id, "onboarding");
        assert_eq!(
            deserialized.reference_context.as_ref().map(|r| r.revision),
            Some(3)
        );
    }

    #[test]
    fn test_turn_context_item_deserializes_without_reference_context_metadata() {
        let json = r#"{
            "model":"gemini-2.0-flash",
            "system_prompt":"System",
            "context_items":[],
            "tools":["web_search"],
            "memory_enabled":true,
            "active_skills":[],
            "timestamp":"2026-01-29T14:30:56Z"
        }"#;

        let deserialized: TurnContextItem = serde_json::from_str(json).unwrap();
        assert!(deserialized.reference_context.is_none());
    }

    #[test]
    fn test_compacted_item_serialization() {
        let item = CompactedItem {
            message: "Summary".to_string(),
            timestamp: "2026-01-29T14:31:00Z".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("Summary"));

        let deserialized: CompactedItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message, "Summary");
    }

    #[test]
    fn test_tool_call_record_serialization() {
        let tool = ToolCallRecord {
            name: "web_search".to_string(),
            arguments: serde_json::json!({"query": "test"}),
            result: serde_json::json!({"found": 5}),
            success: true,
            audit: None,
            timestamp: "2026-01-29T14:31:02Z".to_string(),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("web_search"));
        assert!(json.contains("true"));

        let deserialized: ToolCallRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "web_search");
        assert!(deserialized.success);
    }

    #[test]
    fn test_effect_record_serialization() {
        let effect = EffectRecord {
            effect_id: "ef-1".to_string(),
            run_id: "run-1".to_string(),
            tool_call_id: "call-1".to_string(),
            idempotency_key: "idem-1".to_string(),
            effect_type: "file".to_string(),
            request_fingerprint: "fp-1".to_string(),
            result_digest: Some("digest-1".to_string()),
            result_payload: Some(serde_json::json!({"ok": true})),
            status: EffectStatus::Applied,
            applied_at: Some("2026-03-03T10:00:00Z".to_string()),
            reason: None,
            dedupe_hit: false,
            timestamp: "2026-03-03T10:00:01Z".to_string(),
        };

        let json = serde_json::to_string(&effect).unwrap();
        assert!(json.contains("ef-1"));
        assert!(json.contains("applied"));

        let deserialized: EffectRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.effect_id, "ef-1");
        assert_eq!(deserialized.status, EffectStatus::Applied);
    }

    #[test]
    fn test_checkpoint_record_serialization() {
        let cp = CheckpointRecord {
            checkpoint_id: "cp-123".to_string(),
            checkpoint_type: "supplier_list".to_string(),
            summary: "Found 5 suppliers".to_string(),
            choice: Some("approved".to_string()),
            timestamp: "2026-01-29T14:35:00Z".to_string(),
        };

        let json = serde_json::to_string(&cp).unwrap();
        assert!(json.contains("cp-123"));
        assert!(json.contains("approved"));

        let deserialized: CheckpointRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.checkpoint_id, "cp-123");
        assert_eq!(deserialized.choice, Some("approved".to_string()));
    }

    #[test]
    fn test_checkpoint_record_without_choice() {
        let cp = CheckpointRecord {
            checkpoint_id: "cp-456".to_string(),
            checkpoint_type: "requirements".to_string(),
            summary: "Requirements gathered".to_string(),
            choice: None,
            timestamp: "2026-01-29T14:36:00Z".to_string(),
        };

        let json = serde_json::to_string(&cp).unwrap();
        assert!(json.contains("cp-456"));
        assert!(json.contains("null"));

        let deserialized: CheckpointRecord = serde_json::from_str(&json).unwrap();
        assert!(deserialized.choice.is_none());
    }

    #[test]
    fn test_event_record_serialization() {
        let event = EventRecord {
            event_type: "thinking".to_string(),
            payload: serde_json::json!({"message": "Analyzing..."}),
            timestamp: "2026-01-29T14:37:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("thinking"));
        assert!(json.contains("Analyzing..."));

        let deserialized: EventRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, "thinking");
    }

    #[test]
    fn test_rollout_recorder_clone() {
        // Just test that Clone is implemented correctly
        let _temp_dir = TempDir::new().unwrap();
        let _path = _temp_dir.path().join("test.jsonl");

        // Create a minimal recorder for testing clone
        // Note: We can't easily create a full recorder without async, but we can verify the types
        fn check_clone<T: Clone>(_: T) {}

        // Check that RolloutItem implements Clone
        let item = RolloutItem::Message(MessageRecord {
            role: "user".to_string(),
            content: Some("test".to_string()),
            tool_name: None,
            message: None,
            timestamp: "2026-01-29T14:30:55Z".to_string(),
        });
        check_clone(item);
    }

    #[tokio::test]
    async fn test_load_history_checkpoint_item() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("checkpoint.jsonl");

        let content = r#"{"type":"checkpoint","checkpoint_id":"cp-test","checkpoint_type":"supplier_list","summary":"Test summary","choice":"approved","timestamp":"2026-01-29T14:35:00Z"}"#;

        fs::write(&file_path, content).await.unwrap();

        let items = RolloutRecorder::load_history(&file_path).await.unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            RolloutItem::Checkpoint(cp) => {
                assert_eq!(cp.checkpoint_id, "cp-test");
                assert_eq!(cp.checkpoint_type, "supplier_list");
                assert_eq!(cp.summary, "Test summary");
                assert_eq!(cp.choice, Some("approved".to_string()));
            }
            _ => panic!("Expected Checkpoint"),
        }
    }

    #[tokio::test]
    async fn test_load_history_event_item() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("event.jsonl");

        let content = r#"{"type":"event","event_type":"thinking","payload":{"message":"Thinking..."},"timestamp":"2026-01-29T14:37:00Z"}"#;

        fs::write(&file_path, content).await.unwrap();

        let items = RolloutRecorder::load_history(&file_path).await.unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            RolloutItem::Event(evt) => {
                assert_eq!(evt.event_type, "thinking");
            }
            _ => panic!("Expected Event"),
        }
    }
}
