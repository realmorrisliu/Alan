use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Anthropic-compatible client (Messages API).
pub struct AnthropicCompatibleClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    custom_headers: HeaderMap,
}

#[derive(Debug, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<ContentBlockInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockInput {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    pub usage: Option<Usage>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: Option<i32>,
    pub content_block: Option<ContentBlock>,
    pub delta: Option<StreamDelta>,
    pub message: Option<StreamMessage>,
    pub error: Option<StreamError>,
}

#[derive(Debug, Deserialize)]
pub struct StreamDelta {
    #[serde(rename = "type")]
    pub delta_type: Option<String>,
    pub text: Option<String>,
    pub partial_json: Option<String>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamMessage {
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamError {
    pub error: Option<serde_json::Value>,
    pub message: Option<String>,
    pub r#type: Option<String>,
}

impl AnthropicCompatibleClient {
    pub fn with_params(api_key: &str, base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            custom_headers: HeaderMap::new(),
        }
    }

    /// Set custom headers to be included in all requests
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        for (key, value) in headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                self.custom_headers.insert(name, val);
            }
        }
        self
    }

    /// Set a client name header (for usage tracking)
    pub fn with_client_name(mut self, name: &str) -> Self {
        if let Ok(val) = HeaderValue::from_str(name) {
            self.custom_headers.insert("X-Client-Name", val);
        }
        self
    }

    /// Set User-Agent header
    pub fn with_user_agent(mut self, user_agent: &str) -> Self {
        if let Ok(val) = HeaderValue::from_str(user_agent) {
            self.custom_headers.insert("User-Agent", val);
        }
        self
    }

    pub async fn messages(&self, mut request: MessageRequest) -> Result<MessageResponse> {
        let url = self.messages_url();
        if request.model.is_empty() {
            request.model = self.model.clone();
        }

        let mut req_builder = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01");

        // Apply custom headers
        for (name, value) in &self.custom_headers {
            req_builder = req_builder.header(name, value);
        }

        let response = req_builder
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Anthropic-compatible API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Anthropic-compatible API error ({}): {}",
                status,
                error_text
            );
        }

        let result: MessageResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic-compatible response")?;

        Ok(result)
    }

    pub async fn stream_messages(
        &self,
        mut request: MessageRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<()> {
        let url = self.messages_url();
        if request.model.is_empty() {
            request.model = self.model.clone();
        }
        request.stream = Some(true);

        let mut req_builder = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01");

        // Apply custom headers
        for (name, value) in &self.custom_headers {
            req_builder = req_builder.header(name, value);
        }

        let response = req_builder
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Anthropic-compatible API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Anthropic-compatible streaming API error ({}): {}",
                status,
                error_text
            );
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        return Ok(());
                    }

                    if let Ok(event) = serde_json::from_str::<StreamEvent>(data)
                        && tx.send(event).await.is_err()
                    {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn chat(&self, system: Option<&str>, user_message: &str) -> Result<String> {
        let request = MessageRequest {
            model: self.model.clone(),
            system: system.map(ToString::to_string),
            messages: vec![Message::user_text(user_message)],
            max_tokens: 2048,
            temperature: Some(0.7),
            tools: None,
            stream: None,
        };

        let response = self.messages(request).await?;
        let text = response
            .content
            .into_iter()
            .filter(|block| block.block_type == "text")
            .filter_map(|block| block.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }

    fn messages_url(&self) -> String {
        if self.base_url.ends_with("/v1") {
            format!("{}/messages", self.base_url)
        } else {
            format!("{}/v1/messages", self.base_url)
        }
    }
}

impl Message {
    pub fn user_text(text: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![ContentBlockInput::Text {
                text: text.to_string(),
            }],
        }
    }

    pub fn assistant_text(text: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: vec![ContentBlockInput::Text {
                text: text.to_string(),
            }],
        }
    }

    pub fn user_tool_result(tool_use_id: &str, content: String) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![ContentBlockInput::ToolResult {
                tool_use_id: tool_use_id.to_string(),
                content,
                is_error: None,
            }],
        }
    }
}

impl ToolDefinition {
    pub fn new(name: &str, description: &str, input_schema: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        }
    }
}

// ============================================================================
// LlmProvider Trait Implementation
// ============================================================================

use crate::{
    GenerationRequest, GenerationResponse, LlmProvider, Message as LlmMessage, MessageRole,
    StreamChunk, TokenUsage, ToolCall as LlmToolCall, ToolCallDelta,
    ToolDefinition as LlmToolDefinition,
};
use async_trait::async_trait;

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn convert_messages_for_anthropic(messages: Vec<LlmMessage>) -> Vec<Message> {
    messages
        .into_iter()
        .filter_map(|msg| {
            let LlmMessage {
                role,
                content,
                tool_calls,
                tool_call_id,
            } = msg;

            let content_blocks = match role {
                MessageRole::User => {
                    if content.is_empty() {
                        Vec::new()
                    } else {
                        vec![ContentBlockInput::Text { text: content }]
                    }
                }
                MessageRole::Assistant => {
                    let mut blocks = Vec::new();

                    if !content.is_empty() {
                        blocks.push(ContentBlockInput::Text { text: content });
                    }

                    if let Some(calls) = tool_calls {
                        for call in calls {
                            if let Some(id) = call.id.filter(|id| is_non_empty(id)) {
                                blocks.push(ContentBlockInput::ToolUse {
                                    id,
                                    name: call.name,
                                    input: call.arguments,
                                });
                            }
                        }
                    }

                    blocks
                }
                MessageRole::Tool => {
                    if let Some(tool_use_id) = tool_call_id.filter(|id| is_non_empty(id)) {
                        vec![ContentBlockInput::ToolResult {
                            tool_use_id,
                            content,
                            is_error: None,
                        }]
                    } else if !content.is_empty() {
                        vec![ContentBlockInput::Text { text: content }]
                    } else {
                        Vec::new()
                    }
                }
                MessageRole::System | MessageRole::Context => Vec::new(),
            };

            if content_blocks.is_empty() {
                return None;
            }

            Some(Message {
                role: match role {
                    MessageRole::User | MessageRole::Tool => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System | MessageRole::Context => return None,
                },
                content: content_blocks,
            })
        })
        .collect()
}

fn convert_tools_for_anthropic(tools: Vec<LlmToolDefinition>) -> Option<Vec<ToolDefinition>> {
    if tools.is_empty() {
        None
    } else {
        Some(
            tools
                .into_iter()
                .map(|tool| ToolDefinition {
                    name: tool.name,
                    description: tool.description,
                    input_schema: tool.parameters,
                })
                .collect(),
        )
    }
}

#[async_trait]
impl LlmProvider for AnthropicCompatibleClient {
    async fn generate(&mut self, request: GenerationRequest) -> anyhow::Result<GenerationResponse> {
        let messages = convert_messages_for_anthropic(request.messages);
        let tools = convert_tools_for_anthropic(request.tools);

        let anthropic_request = MessageRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            system: request.system_prompt,
            temperature: request.temperature,
            tools,
            stream: Some(false),
        };

        let response = self.messages(anthropic_request).await?;

        // Extract text and tool calls
        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for block in response.content {
            match block.block_type.as_str() {
                "text" => {
                    if let Some(t) = block.text {
                        text_parts.push(t);
                    }
                }
                "tool_use" => {
                    if let (Some(name), Some(input)) = (block.name, block.input) {
                        tool_calls.push(LlmToolCall {
                            id: block.id,
                            name,
                            arguments: input,
                        });
                    }
                }
                _ => {}
            }
        }

        let usage = response.usage.map(|u| TokenUsage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });

        Ok(GenerationResponse {
            content: text_parts.join(""),
            tool_calls,
            usage,
        })
    }

    async fn chat(&mut self, system: Option<&str>, user: &str) -> anyhow::Result<String> {
        self.chat(system, user).await
    }

    async fn generate_stream(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        let messages = convert_messages_for_anthropic(request.messages);
        let tools = convert_tools_for_anthropic(request.tools);

        let anthropic_request = MessageRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            system: request.system_prompt,
            temperature: request.temperature,
            tools,
            stream: Some(true),
        };

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

        // Spawn streaming task
        let client =
            AnthropicCompatibleClient::with_params(&self.api_key, &self.base_url, &self.model);
        tokio::spawn(async move {
            if let Err(e) = client.stream_messages(anthropic_request, event_tx).await {
                tracing::debug!(error = ?e, "Anthropic stream failed");
            }
        });

        // Transform events to StreamChunk
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event.event_type.as_str() {
                    "content_block_delta" => {
                        if let Some(delta) = event.delta {
                            if let Some(text) = delta.text {
                                let _ = tx
                                    .send(StreamChunk {
                                        text: Some(text),
                                        tool_call_delta: None,
                                        is_finished: false,
                                        finish_reason: None,
                                    })
                                    .await;
                            }
                            if let (Some(partial_json), Some(index)) =
                                (delta.partial_json, event.index)
                            {
                                let _ = tx
                                    .send(StreamChunk {
                                        text: None,
                                        tool_call_delta: Some(ToolCallDelta {
                                            index: index as usize,
                                            id: None,
                                            name: None,
                                            arguments_delta: Some(partial_json),
                                        }),
                                        is_finished: false,
                                        finish_reason: None,
                                    })
                                    .await;
                            }
                        }
                    }
                    "message_stop" => {
                        let _ = tx
                            .send(StreamChunk {
                                text: None,
                                tool_call_delta: None,
                                is_finished: true,
                                finish_reason: event.message.and_then(|m| m.stop_reason),
                            })
                            .await;
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }

    fn provider_name(&self) -> &'static str {
        "anthropic"
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_url_appends_v1_when_missing() {
        let client =
            AnthropicCompatibleClient::with_params("k", "https://api.kimi.com/coding", "k2p5");
        assert_eq!(
            client.messages_url(),
            "https://api.kimi.com/coding/v1/messages"
        );
    }

    #[test]
    fn messages_url_preserves_existing_v1() {
        let client =
            AnthropicCompatibleClient::with_params("k", "https://api.anthropic.com/v1", "claude");
        assert_eq!(
            client.messages_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_anthropic_client_with_params() {
        let client = AnthropicCompatibleClient::with_params(
            "test-key",
            "https://api.anthropic.com/v1",
            "claude-3-opus",
        );
        // Just verify client creation works
        drop(client);
    }

    #[test]
    fn test_message_request_serialization() {
        let request = MessageRequest {
            model: "claude-3-opus".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![ContentBlockInput::Text {
                    text: "Hello".to_string(),
                }],
            }],
            max_tokens: 1024,
            system: None,
            temperature: Some(0.7),
            tools: None,
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-3-opus"));
        assert!(json.contains("messages"));
        assert!(json.contains("max_tokens"));
    }

    #[test]
    fn test_message_response_deserialization() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "content": [
                {"type": "text", "text": "Hello!"}
            ],
            "model": "claude-3-opus",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        }"#;

        let response: MessageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "msg_123");
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 10);
    }

    #[test]
    fn test_content_block() {
        let block = ContentBlock {
            block_type: "text".to_string(),
            text: Some("Hello".to_string()),
            id: None,
            name: None,
            input: None,
        };

        assert_eq!(block.block_type, "text");
        assert_eq!(block.text, Some("Hello".to_string()));
    }

    #[test]
    fn test_message_user_text() {
        let msg = Message::user_text("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            ContentBlockInput::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_message_user_tool_result() {
        let msg = Message::user_tool_result("tool-call-123", "Tool output".to_string());
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            ContentBlockInput::ToolResult {
                tool_use_id,
                content,
                ..
            } => {
                assert_eq!(tool_use_id, "tool-call-123");
                assert_eq!(content, "Tool output");
            }
            _ => panic!("Expected ToolResult variant"),
        }
    }

    #[test]
    fn test_tool_definition_new() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            }
        });

        let tool = ToolDefinition::new("search", "Search tool", schema.clone());
        assert_eq!(tool.name, "search");
        assert_eq!(tool.description, "Search tool");
        assert_eq!(tool.input_schema, schema);
    }

    #[test]
    fn test_content_block_input_text() {
        let block = ContentBlockInput::Text {
            text: "Hello".to_string(),
        };

        match block {
            ContentBlockInput::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_content_block_input_tool_use() {
        let block = ContentBlockInput::ToolUse {
            id: "call-123".to_string(),
            name: "my_tool".to_string(),
            input: serde_json::json!({"arg": "value"}),
        };

        match block {
            ContentBlockInput::ToolUse { id, name, input } => {
                assert_eq!(id, "call-123");
                assert_eq!(name, "my_tool");
                assert_eq!(input["arg"], "value");
            }
            _ => panic!("Expected ToolUse variant"),
        }
    }

    #[test]
    fn test_content_block_input_tool_result() {
        let block = ContentBlockInput::ToolResult {
            tool_use_id: "call-456".to_string(),
            content: "Result".to_string(),
            is_error: Some(false),
        };

        match block {
            ContentBlockInput::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_use_id, "call-456");
                assert_eq!(content, "Result");
                assert_eq!(is_error, Some(false));
            }
            _ => panic!("Expected ToolResult variant"),
        }
    }

    #[test]
    fn test_stream_event_deserialization() {
        let json = r#"{
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        }"#;

        let event: StreamEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "content_block_delta");
        assert_eq!(event.index, Some(0));
        assert!(event.delta.is_some());
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: "assistant".to_string(),
            content: vec![ContentBlockInput::Text {
                text: "Hi!".to_string(),
            }],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("assistant"));
        assert!(json.contains("Hi!"));
    }

    #[test]
    fn test_usage() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
        };

        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_stream_delta() {
        let delta = StreamDelta {
            delta_type: Some("text_delta".to_string()),
            text: Some("Hello".to_string()),
            partial_json: None,
            stop_reason: None,
        };

        assert_eq!(delta.delta_type, Some("text_delta".to_string()));
        assert_eq!(delta.text, Some("Hello".to_string()));
    }

    #[test]
    fn test_stream_message() {
        let msg = StreamMessage {
            stop_reason: Some("end_turn".to_string()),
        };
        assert_eq!(msg.stop_reason, Some("end_turn".to_string()));
    }

    #[test]
    fn test_stream_error() {
        let err = StreamError {
            error: Some(serde_json::json!({"type": "error"})),
            message: Some("Something went wrong".to_string()),
            r#type: Some("api_error".to_string()),
        };
        assert_eq!(err.message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant_text("Hello from assistant");
        assert_eq!(msg.role, "assistant");
        match &msg.content[0] {
            ContentBlockInput::Text { text } => assert_eq!(text, "Hello from assistant"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_convert_messages_for_anthropic_keeps_assistant_tool_use() {
        let assistant = crate::Message::assistant_with_tools(
            "",
            vec![
                crate::ToolCall::new("web_search", serde_json::json!({"query": "laptop"}))
                    .with_id("toolu_123"),
            ],
        );
        let tool = crate::Message::tool("toolu_123", "{\"ok\":true}");

        let converted = convert_messages_for_anthropic(vec![assistant, tool]);
        assert_eq!(converted.len(), 2);

        assert_eq!(converted[0].role, "assistant");
        assert_eq!(converted[0].content.len(), 1);
        match &converted[0].content[0] {
            ContentBlockInput::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_123");
                assert_eq!(name, "web_search");
                assert_eq!(input["query"], "laptop");
            }
            _ => panic!("Expected ToolUse variant"),
        }

        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[1].content.len(), 1);
        match &converted[1].content[0] {
            ContentBlockInput::ToolResult { tool_use_id, .. } => {
                assert_eq!(tool_use_id, "toolu_123");
            }
            _ => panic!("Expected ToolResult variant"),
        }
    }

    #[test]
    fn test_convert_messages_for_anthropic_empty_tool_call_id_falls_back_to_text() {
        let tool_msg = crate::Message {
            role: MessageRole::Tool,
            content: "tool output".to_string(),
            tool_calls: None,
            tool_call_id: Some("   ".to_string()),
        };

        let converted = convert_messages_for_anthropic(vec![tool_msg]);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[0].content.len(), 1);
        match &converted[0].content[0] {
            ContentBlockInput::Text { text } => assert_eq!(text, "tool output"),
            _ => panic!("Expected Text variant"),
        }
    }
}
