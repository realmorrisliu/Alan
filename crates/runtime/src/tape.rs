//! Tape — the AI Turing Machine's conversation tape.
//!
//! The tape holds the ordered sequence of messages (user, assistant, tool),
//! optional compaction summary, and reference context items.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_payload: Option<serde_json::Value>,
    /// Tool calls made by the assistant (for OpenAI API compatibility).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Role of the message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    Context,
    User,
    Assistant,
    Tool,
}

/// A tool call in a message (for OpenAI compatibility).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

pub const SUMMARY_PREFIX: &str = "The following is a compacted summary of the earlier conversation history in this session. Use this context to continue the work seamlessly without duplicating what has already been done:";

#[derive(Debug, Clone)]
pub struct Tape {
    reference_context: ReferenceContextState,
    messages: Vec<Message>,
    messages_token_estimate: usize,
    summary: Option<String>,
    summary_token_estimate: usize,
    /// Number of times compaction has been applied in this session.
    compaction_count: usize,
}

#[derive(Debug, Clone, Default)]
struct ReferenceContextState {
    items: Vec<ContextItem>,
    rendered_messages: Vec<Message>,
    rendered_token_estimate: usize,
    revision: u64,
    last_delta: ContextItemsDelta,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContextItemsDelta {
    pub changed: bool,
    pub reordered: bool,
    pub revision: u64,
    pub added_ids: Vec<String>,
    pub updated_ids: Vec<String>,
    pub removed_ids: Vec<String>,
}

impl ContextItemsDelta {
    pub fn is_empty(&self) -> bool {
        !self.changed
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceContextSnapshot {
    pub revision: u64,
    pub item_count: usize,
    pub delta: ContextItemsDelta,
}

#[derive(Debug, Clone)]
pub struct PromptContextView {
    pub messages: Vec<Message>,
    pub estimated_tokens: usize,
    pub reference_context: ReferenceContextSnapshot,
}

#[derive(Debug, Clone)]
pub struct ContextItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub fingerprint: String,
}

impl ContextItem {
    pub fn new(
        id: impl Into<String>,
        kind: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let id = id.into();
        let kind = kind.into();
        let title = title.into();
        let content = content.into();
        let fingerprint = fingerprint_context(&kind, &title, &content);
        Self {
            id,
            kind,
            title,
            content,
            fingerprint,
        }
    }

    pub fn with_fingerprint(
        id: impl Into<String>,
        kind: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            title: title.into(),
            content: content.into(),
            fingerprint: fingerprint.into(),
        }
    }

    fn ensure_fingerprint_matches(&mut self) {
        let expected = fingerprint_context(&self.kind, &self.title, &self.content);
        if self.fingerprint != expected {
            self.fingerprint = expected;
        }
    }
}

impl Default for Tape {
    fn default() -> Self {
        Self::new()
    }
}

impl Tape {
    pub fn new() -> Self {
        Self {
            reference_context: ReferenceContextState::default(),
            messages: Vec::new(),
            messages_token_estimate: 0,
            summary: None,
            summary_token_estimate: 0,
            compaction_count: 0,
        }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get the current compaction summary, if any.
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    /// Get the number of compactions applied in this session.
    pub fn compaction_count(&self) -> usize {
        self.compaction_count
    }

    pub fn prompt_view(&self) -> PromptContextView {
        let mut out = Vec::with_capacity(
            self.reference_context.rendered_messages.len()
                + usize::from(self.summary.is_some())
                + self.messages.len(),
        );
        out.extend(self.reference_context.rendered_messages.clone());
        if let Some(summary) = &self.summary {
            out.push(summary_prompt_message(summary));
        }
        out.extend(self.messages.clone());

        PromptContextView {
            messages: out,
            estimated_tokens: self.estimated_prompt_tokens(),
            reference_context: ReferenceContextSnapshot {
                revision: self.reference_context.revision,
                item_count: self.reference_context.items.len(),
                delta: self.reference_context.last_delta.clone(),
            },
        }
    }

    /// Backward-compatible wrapper for callers/tests that only need the prompt messages.
    pub fn messages_for_prompt(&self) -> Vec<Message> {
        self.prompt_view().messages
    }

    /// Lightweight token estimate for prompt budgeting and compaction heuristics.
    /// We intentionally avoid provider-specific tokenizers here to keep runtime cost low.
    pub fn estimated_prompt_tokens(&self) -> usize {
        self.reference_context.rendered_token_estimate
            + self.summary_token_estimate
            + self.messages_token_estimate
    }

    pub fn push(&mut self, message: Message) {
        self.messages_token_estimate += estimate_message_tokens(&message);
        self.messages.push(message);
    }

    pub fn clear(&mut self) {
        self.reference_context = ReferenceContextState::default();
        self.messages.clear();
        self.messages_token_estimate = 0;
        self.summary = None;
        self.summary_token_estimate = 0;
        self.compaction_count = 0;
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn replace(&mut self, messages: Vec<Message>) {
        self.messages = messages;
        self.messages_token_estimate = self
            .messages
            .iter()
            .map(estimate_message_tokens)
            .sum::<usize>();
    }

    pub fn context_items(&self) -> &[ContextItem] {
        &self.reference_context.items
    }

    pub fn last_context_delta(&self) -> &ContextItemsDelta {
        &self.reference_context.last_delta
    }

    pub fn context_revision(&self) -> u64 {
        self.reference_context.revision
    }

    /// Apply a new reference context snapshot and return a diff against the previous snapshot.
    pub fn apply_context_items(&mut self, items: Vec<ContextItem>) -> ContextItemsDelta {
        let items = normalize_context_items(items);
        let delta = diff_context_items(
            &self.reference_context.items,
            &items,
            self.reference_context.revision,
        );

        if delta.changed {
            let (rendered_messages, rendered_token_estimate) = render_context_items(&items);
            self.reference_context.items = items;
            self.reference_context.rendered_messages = rendered_messages;
            self.reference_context.rendered_token_estimate = rendered_token_estimate;
            self.reference_context.revision = delta.revision;
        }

        self.reference_context.last_delta = delta.clone();
        delta
    }

    pub fn set_summary(&mut self, summary: String) {
        self.summary_token_estimate = estimate_message_tokens(&summary_prompt_message(&summary));
        self.summary = Some(summary);
    }

    pub fn clear_summary(&mut self) {
        self.summary = None;
        self.summary_token_estimate = 0;
    }

    pub fn compact(&mut self, summary: String, keep_last: usize) {
        let keep = keep_last.min(self.messages.len());
        let tail = self.messages[self.messages.len().saturating_sub(keep)..].to_vec();
        self.messages = tail;
        self.messages_token_estimate = self
            .messages
            .iter()
            .map(estimate_message_tokens)
            .sum::<usize>();
        self.set_summary(summary);
        self.compaction_count += 1;
    }
}

fn summary_prompt_message(summary: &str) -> Message {
    Message {
        role: MessageRole::Context,
        content: format!("{SUMMARY_PREFIX}\n{}", summary),
        tool_name: None,
        tool_payload: None,
        tool_calls: None,
    }
}

fn normalize_context_items(mut items: Vec<ContextItem>) -> Vec<ContextItem> {
    for item in &mut items {
        item.ensure_fingerprint_matches();
    }
    items
}

fn diff_context_items(
    old_items: &[ContextItem],
    new_items: &[ContextItem],
    current_revision: u64,
) -> ContextItemsDelta {
    let old_map: HashMap<&str, &str> = old_items
        .iter()
        .map(|item| (item.id.as_str(), item.fingerprint.as_str()))
        .collect();
    let new_map: HashMap<&str, &str> = new_items
        .iter()
        .map(|item| (item.id.as_str(), item.fingerprint.as_str()))
        .collect();

    let mut added_ids = Vec::new();
    let mut updated_ids = Vec::new();
    let mut removed_ids = Vec::new();

    for item in new_items {
        match old_map.get(item.id.as_str()) {
            None => added_ids.push(item.id.clone()),
            Some(old_fp) if *old_fp != item.fingerprint => updated_ids.push(item.id.clone()),
            _ => {}
        }
    }

    for item in old_items {
        if !new_map.contains_key(item.id.as_str()) {
            removed_ids.push(item.id.clone());
        }
    }

    let old_order: Vec<&str> = old_items.iter().map(|i| i.id.as_str()).collect();
    let new_order: Vec<&str> = new_items.iter().map(|i| i.id.as_str()).collect();
    let reordered = old_order != new_order
        && added_ids.is_empty()
        && updated_ids.is_empty()
        && removed_ids.is_empty();

    let changed =
        reordered || !added_ids.is_empty() || !updated_ids.is_empty() || !removed_ids.is_empty();

    ContextItemsDelta {
        changed,
        reordered,
        revision: if changed {
            current_revision.saturating_add(1)
        } else {
            current_revision
        },
        added_ids,
        updated_ids,
        removed_ids,
    }
}

fn render_context_items(items: &[ContextItem]) -> (Vec<Message>, usize) {
    let mut rendered_messages = Vec::new();
    let mut token_estimate = 0;

    for item in items {
        let content = format_context_item(item);
        if content.is_empty() {
            continue;
        }
        let message = Message {
            role: MessageRole::Context,
            content,
            tool_name: None,
            tool_payload: None,
            tool_calls: None,
        };
        token_estimate += estimate_message_tokens(&message);
        rendered_messages.push(message);
    }

    (rendered_messages, token_estimate)
}

fn estimate_message_tokens(message: &Message) -> usize {
    // Rough heuristic: ~4 chars/token plus a small per-message framing overhead.
    let tool_payload_tokens = message
        .tool_payload
        .as_ref()
        .map(estimate_json_tokens)
        .unwrap_or(0);
    let tool_calls_tokens = message
        .tool_calls
        .as_ref()
        .map(|calls| calls.iter().map(estimate_tool_call_tokens).sum::<usize>())
        .unwrap_or(0);

    estimate_text_tokens(&message.content)
        + message
            .tool_name
            .as_deref()
            .map(estimate_text_tokens)
            .unwrap_or(0)
        + tool_payload_tokens
        + tool_calls_tokens
        + 6
}

fn estimate_tool_call_tokens(call: &ToolCall) -> usize {
    estimate_text_tokens(&call.id)
        + estimate_text_tokens(&call.name)
        + estimate_json_tokens(&call.arguments)
        + 4
}

fn estimate_json_tokens(value: &serde_json::Value) -> usize {
    estimate_text_tokens(&value.to_string())
}

fn estimate_text_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    chars.div_ceil(4)
}

pub fn format_context_item(item: &ContextItem) -> String {
    let content = item.content.trim();
    if content.is_empty() {
        return String::new();
    }
    format!("{}:\n{}", item.title.trim(), content)
}

pub fn fingerprint_context(kind: &str, title: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\n");
    hasher.update(title.as_bytes());
    hasher.update(b"\n");
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: MessageRole, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            tool_name: None,
            tool_payload: None,
            tool_calls: None,
        }
    }

    fn item(id: &str, content: &str) -> ContextItem {
        ContextItem {
            id: id.to_string(),
            kind: "test".to_string(),
            title: format!("Title {}", id),
            content: content.to_string(),
            fingerprint: fingerprint_context("test", &format!("Title {}", id), content),
        }
    }

    #[test]
    fn test_messages_for_prompt_includes_summary() {
        let mut ctx = Tape::new();
        ctx.push(msg(MessageRole::User, "hello"));
        ctx.set_summary("short summary".to_string());

        let messages = ctx.messages_for_prompt();
        assert_eq!(messages.len(), 2);
        assert!(matches!(messages[0].role, MessageRole::Context));
        assert!(messages[0].content.contains(SUMMARY_PREFIX));
        assert!(messages[0].content.contains("short summary"));
        assert!(matches!(messages[1].role, MessageRole::User));
    }

    #[test]
    fn test_apply_context_items_computes_baseline_and_delta() {
        let mut ctx = Tape::new();

        let delta = ctx.apply_context_items(vec![item("a", "alpha"), item("b", "beta")]);
        assert!(delta.changed);
        assert_eq!(delta.revision, 1);
        assert_eq!(delta.added_ids, vec!["a", "b"]);
        assert!(delta.updated_ids.is_empty());
        assert!(delta.removed_ids.is_empty());
        assert_eq!(ctx.context_revision(), 1);

        let unchanged = ctx.apply_context_items(vec![item("a", "alpha"), item("b", "beta")]);
        assert!(!unchanged.changed);
        assert_eq!(unchanged.revision, 1);
        assert_eq!(ctx.context_revision(), 1);
    }

    #[test]
    fn test_apply_context_items_detects_updates_removals_and_reorder() {
        let mut ctx = Tape::new();
        ctx.apply_context_items(vec![
            item("a", "alpha"),
            item("b", "beta"),
            item("c", "gamma"),
        ]);

        let delta = ctx.apply_context_items(vec![item("b", "beta"), item("a", "alpha2")]);
        assert!(delta.changed);
        assert_eq!(delta.revision, 2);
        assert_eq!(delta.updated_ids, vec!["a"]);
        assert_eq!(delta.removed_ids, vec!["c"]);
        assert!(delta.added_ids.is_empty());

        let reorder_only = ctx.apply_context_items(vec![item("a", "alpha2"), item("b", "beta")]);
        assert!(reorder_only.changed);
        assert!(reorder_only.reordered);
        assert!(reorder_only.added_ids.is_empty());
        assert!(reorder_only.updated_ids.is_empty());
        assert!(reorder_only.removed_ids.is_empty());
    }

    #[test]
    fn test_apply_context_items_detects_content_change_with_stale_fingerprint() {
        let mut ctx = Tape::new();
        let original = item("a", "alpha");
        let stale_fingerprint = original.fingerprint.clone();
        ctx.apply_context_items(vec![original]);

        let delta = ctx.apply_context_items(vec![ContextItem {
            id: "a".to_string(),
            kind: "test".to_string(),
            title: "Title a".to_string(),
            content: "beta".to_string(),
            fingerprint: stale_fingerprint,
        }]);

        assert!(delta.changed);
        assert_eq!(delta.updated_ids, vec!["a"]);
        assert_eq!(
            ctx.context_items()[0].fingerprint,
            fingerprint_context("test", "Title a", "beta")
        );
    }

    #[test]
    fn test_prompt_view_exposes_reference_context_snapshot_metadata() {
        let mut ctx = Tape::new();
        let delta = ctx.apply_context_items(vec![item("ctx_1", "important background")]);
        assert!(delta.changed);
        ctx.push(msg(MessageRole::User, "hello"));

        let view = ctx.prompt_view();
        assert_eq!(view.reference_context.item_count, 1);
        assert_eq!(view.reference_context.revision, 1);
        assert!(view.reference_context.delta.changed);
        assert_eq!(view.messages.len(), 2);
        assert!(matches!(view.messages[0].role, MessageRole::Context));
        assert!(matches!(view.messages[1].role, MessageRole::User));
    }

    #[test]
    fn test_compact_keeps_tail_and_sets_summary() {
        let mut ctx = Tape::new();
        ctx.push(msg(MessageRole::User, "m1"));
        ctx.push(msg(MessageRole::Assistant, "m2"));
        ctx.push(msg(MessageRole::User, "m3"));

        ctx.compact("summary".to_string(), 1);
        let messages = ctx.messages_for_prompt();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].content.contains("summary"));
        assert_eq!(messages[1].content, "m3");
    }

    #[test]
    fn test_clear_resets_messages_summary_and_reference_context() {
        let mut ctx = Tape::new();
        ctx.apply_context_items(vec![item("x", "ctx")]);
        ctx.push(msg(MessageRole::User, "hello"));
        ctx.set_summary("summary".to_string());
        ctx.clear();

        let messages = ctx.messages_for_prompt();
        assert!(messages.is_empty());
        assert_eq!(ctx.context_revision(), 0);
        assert!(ctx.context_items().is_empty());
    }

    #[test]
    fn test_clear_summary_preserves_messages() {
        let mut ctx = Tape::new();
        ctx.push(msg(MessageRole::User, "hello"));
        ctx.set_summary("summary".to_string());
        ctx.clear_summary();

        let messages = ctx.messages_for_prompt();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "hello");
    }

    #[test]
    fn test_replace_messages() {
        let mut ctx = Tape::new();
        ctx.push(msg(MessageRole::User, "old"));
        ctx.replace(vec![msg(MessageRole::Assistant, "new")]);
        let messages = ctx.messages();
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0].role, MessageRole::Assistant));
    }

    #[test]
    fn test_context_items_render_before_messages() {
        let mut ctx = Tape::new();
        ctx.apply_context_items(vec![ContextItem {
            id: "onboarding".to_string(),
            kind: "static".to_string(),
            title: "Onboarding".to_string(),
            content: "Follow the steps".to_string(),
            fingerprint: fingerprint_context("static", "Onboarding", "Follow the steps"),
        }]);
        ctx.push(msg(MessageRole::User, "hello"));

        let messages = ctx.messages_for_prompt();
        assert_eq!(messages.len(), 2);
        assert!(matches!(messages[0].role, MessageRole::Context));
        assert!(messages[0].content.contains("Onboarding"));
        assert!(matches!(messages[1].role, MessageRole::User));
    }

    #[test]
    fn test_estimated_prompt_tokens_includes_summary_and_context_items() {
        let mut ctx = Tape::new();
        ctx.apply_context_items(vec![ContextItem {
            id: "ctx_1".to_string(),
            kind: "domain".to_string(),
            title: "Domain".to_string(),
            content: "Important background".to_string(),
            fingerprint: fingerprint_context("domain", "Domain", "Important background"),
        }]);
        ctx.push(msg(MessageRole::User, "hello world"));
        ctx.set_summary("previous summary".to_string());

        let estimated = ctx.estimated_prompt_tokens();
        assert!(estimated > 0);

        let without_summary = {
            let mut clone = ctx.clone();
            clone.clear_summary();
            clone.estimated_prompt_tokens()
        };
        assert!(
            estimated > without_summary,
            "summary content should contribute to token estimate"
        );
    }
}
#[cfg(test)]
mod message_tests {
    use super::*;

    #[test]
    fn test_message_role_serialization() {
        let roles = vec![
            (MessageRole::System, "\"system\""),
            (MessageRole::Context, "\"context\""),
            (MessageRole::User, "\"user\""),
            (MessageRole::Assistant, "\"assistant\""),
            (MessageRole::Tool, "\"tool\""),
        ];

        for (role, expected) in roles {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, expected);

            let deserialized: MessageRole = serde_json::from_str(expected).unwrap();
            assert!(std::mem::discriminant(&deserialized) == std::mem::discriminant(&role));
        }
    }

    #[test]
    fn test_message_serialization() {
        let message = Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_name: None,
            tool_payload: None,
            tool_calls: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("user"));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Hello");
    }

    #[test]
    fn test_message_serialization_with_tool() {
        let message = Message {
            role: MessageRole::Tool,
            content: String::new(),
            tool_name: Some("web_search".to_string()),
            tool_payload: Some(serde_json::json!({"result": "found"})),
            tool_calls: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("web_search"));
        assert!(json.contains("found"));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tool_name, Some("web_search".to_string()));
    }
}
