//! OpenAI-compatible LLM client
//!
//! Supports OpenAI, Azure OpenAI, and compatible APIs (DeepSeek, etc.)

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::{
    GenerationRequest, GenerationResponse, LlmProvider, MessageRole, StreamChunk, TokenUsage,
    ToolCall as LlmToolCall, ToolCallDelta,
};
use async_trait::async_trait;

/// Client for OpenAI-compatible APIs
pub struct OpenAiClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
}

// ============================================================================
// Request Types (OpenAI Chat Completions API)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // system, user, assistant, tool
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string, needs parsing
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

// ============================================================================
// Streaming Response Types
// ============================================================================

/// Stream chunk from OpenAI streaming API
#[derive(Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

/// A choice in streaming response
#[derive(Debug, Deserialize)]
pub struct ChunkChoice {
    pub index: i32,
    pub delta: DeltaMessage,
    pub finish_reason: Option<String>,
}

/// Delta message in streaming response (incremental content)
#[derive(Debug, Deserialize, Default)]
pub struct DeltaMessage {
    pub role: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<StreamToolCall>>,
}

/// Tool call in streaming response
#[derive(Debug, Deserialize)]
pub struct StreamToolCall {
    pub index: i32,
    pub id: Option<String>,
    pub r#type: Option<String>,
    pub function: Option<StreamFunctionCall>,
}

/// Function call in streaming response
#[derive(Debug, Deserialize)]
pub struct StreamFunctionCall {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

// ============================================================================
// Client Implementation
// ============================================================================

impl OpenAiClient {
    /// Create with explicit parameters
    pub fn with_params(api_key: &str, base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }

    /// Chat completion (non-streaming)
    #[instrument(skip(self, request))]
    pub async fn chat_completion(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        // Use the model from the client if not set in the request
        if request.model.is_empty() {
            request.model = self.model.clone();
        }

        debug!(url = %url, model = %request.model, "Sending chat completion request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error ({}): {}", status, error_text);
        }

        let result: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        Ok(result)
    }

    /// Chat completion with streaming (SSE)
    #[instrument(skip(self, request, tx))]
    pub async fn stream_chat_completion(
        &self,
        mut request: ChatCompletionRequest,
        tx: tokio::sync::mpsc::Sender<ChatCompletionChunk>,
    ) -> Result<()> {
        let url = format!("{}/chat/completions", self.base_url);

        // Use the model from the client if not set in the request
        if request.model.is_empty() {
            request.model = self.model.clone();
        }

        // Ensure stream is set to true
        request.stream = Some(true);

        debug!(url = %url, model = %request.model, "Sending streaming chat completion request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to OpenAI API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI streaming API error ({}): {}", status, error_text);
        }

        // Process SSE stream
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete lines
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        debug!("Stream completed");
                        return Ok(());
                    }

                    match serde_json::from_str::<ChatCompletionChunk>(data) {
                        Ok(chunk) => {
                            if tx.send(chunk).await.is_err() {
                                debug!("Receiver dropped, stopping stream");
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            debug!(?e, data, "Failed to parse stream chunk");
                            // Continue processing other chunks
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Simple chat helper
    pub async fn chat(&self, system: Option<&str>, user_message: &str) -> Result<String> {
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(sys.to_string()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools: None,
            tool_choice: None,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let response = self.chat_completion(request).await?;

        Ok(response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default())
    }
}

// ============================================================================
// Helper functions
// ============================================================================

impl ChatMessage {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            role: "assistant".to_string(),
            content: if content.is_empty() {
                None
            } else {
                Some(content)
            },
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a tool message (response to a tool call)
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

impl ToolDefinition {
    /// Create a tool definition from name, description, and parameters
    pub fn new(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
            },
        }
    }
}

// ============================================================================
// LlmProvider Implementation
// ============================================================================

#[async_trait]
impl LlmProvider for OpenAiClient {
    async fn generate(&mut self, request: GenerationRequest) -> anyhow::Result<GenerationResponse> {
        // Convert messages
        let mut messages: Vec<ChatMessage> = Vec::new();

        // Add system prompt if provided
        if let Some(system) = request.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Convert request messages
        for msg in request.messages {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
                MessageRole::Context => "system", // Context treated as system
            };

            // Convert tool calls if present
            let tool_calls: Option<Vec<ToolCall>> = msg.tool_calls.map(|calls| {
                calls
                    .into_iter()
                    .map(|call| ToolCall {
                        id: call.id.unwrap_or_default(),
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name: call.name,
                            arguments: call.arguments.to_string(),
                        },
                    })
                    .collect()
            });

            messages.push(ChatMessage {
                role: role.to_string(),
                content: if msg.content.is_empty() {
                    None
                } else {
                    Some(msg.content)
                },
                tool_calls,
                tool_call_id: msg.tool_call_id,
            });
        }

        // Convert tools
        let has_tools = !request.tools.is_empty();
        let tools: Option<Vec<ToolDefinition>> = if has_tools {
            Some(
                request
                    .tools
                    .into_iter()
                    .map(|tool| ToolDefinition {
                        r#type: "function".to_string(),
                        function: FunctionDefinition {
                            name: tool.name,
                            description: tool.description,
                            parameters: tool.parameters,
                        },
                    })
                    .collect(),
            )
        } else {
            None
        };

        let tool_choice = if has_tools {
            Some("auto".to_string())
        } else {
            None
        };

        let chat_request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools,
            tool_choice,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(false),
        };

        // Call the API
        let response = self.chat_completion(chat_request).await?;

        // Convert response
        let choice = response.choices.first().context("No choices in response")?;
        let message = &choice.message;

        // Convert tool calls
        let tool_calls: Vec<LlmToolCall> = message
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|call| {
                        let args = serde_json::from_str(&call.function.arguments).ok()?;
                        Some(LlmToolCall {
                            id: Some(call.id.clone()),
                            name: call.function.name.clone(),
                            arguments: args,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(GenerationResponse {
            content: message.content.clone().unwrap_or_default(),
            thinking: None,
            tool_calls,
            usage,
        })
    }

    async fn chat(&mut self, system: Option<&str>, user: &str) -> anyhow::Result<String> {
        // Directly use the existing chat method
        self.chat(system, user).await
    }

    async fn generate_stream(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        // Convert messages
        let mut messages: Vec<ChatMessage> = Vec::new();

        // Add system prompt if provided
        if let Some(system) = request.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Convert request messages
        for msg in request.messages {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
                MessageRole::Context => "system",
            };

            let tool_calls: Option<Vec<ToolCall>> = msg.tool_calls.map(|calls| {
                calls
                    .into_iter()
                    .map(|call| ToolCall {
                        id: call.id.unwrap_or_default(),
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name: call.name,
                            arguments: call.arguments.to_string(),
                        },
                    })
                    .collect()
            });

            messages.push(ChatMessage {
                role: role.to_string(),
                content: if msg.content.is_empty() {
                    None
                } else {
                    Some(msg.content)
                },
                tool_calls,
                tool_call_id: msg.tool_call_id,
            });
        }

        // Convert tools
        let has_tools = !request.tools.is_empty();
        let tools: Option<Vec<ToolDefinition>> = if has_tools {
            Some(
                request
                    .tools
                    .into_iter()
                    .map(|tool| ToolDefinition {
                        r#type: "function".to_string(),
                        function: FunctionDefinition {
                            name: tool.name,
                            description: tool.description,
                            parameters: tool.parameters,
                        },
                    })
                    .collect(),
            )
        } else {
            None
        };

        let tool_choice = if has_tools {
            Some("auto".to_string())
        } else {
            None
        };

        let chat_request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools,
            tool_choice,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(true),
        };

        // Create channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel(100);

        // Spawn streaming task
        let client = OpenAiClient::with_params(&self.api_key, &self.base_url, &self.model);
        tokio::spawn(async move {
            if let Err(e) = client.stream_chat_completion(chat_request, chunk_tx).await {
                debug!(error = ?e, "Stream chat completion failed");
            }
        });

        // Transform OpenAI chunks to StreamChunk
        tokio::spawn(async move {
            while let Some(chunk) = chunk_rx.recv().await {
                if let Some(choice) = chunk.choices.first() {
                    let delta = &choice.delta;

                    // Check for finish reason
                    let is_finished = choice.finish_reason.is_some();
                    let finish_reason = choice.finish_reason.clone();

                    // Handle text content
                    if let Some(content) = &delta.content {
                        let _ = tx
                            .send(StreamChunk {
                                text: Some(content.clone()),
                                thinking: None,
                                tool_call_delta: None,
                                is_finished,
                                finish_reason: finish_reason.clone(),
                            })
                            .await;
                    }

                    // Handle tool call deltas
                    if let Some(tool_calls) = &delta.tool_calls {
                        for tool_call in tool_calls {
                            let tool_delta = ToolCallDelta {
                                index: tool_call.index as usize,
                                id: tool_call.id.clone(),
                                name: tool_call.function.as_ref().and_then(|f| f.name.clone()),
                                arguments_delta: tool_call
                                    .function
                                    .as_ref()
                                    .and_then(|f| f.arguments.clone()),
                            };

                            let _ = tx
                                .send(StreamChunk {
                                    text: None,
                                    thinking: None,
                                    tool_call_delta: Some(tool_delta),
                                    is_finished,
                                    finish_reason: finish_reason.clone(),
                                })
                                .await;
                        }
                    }

                    // Send final chunk if finished
                    if is_finished && delta.content.is_none() && delta.tool_calls.is_none() {
                        let _ = tx
                            .send(StreamChunk {
                                text: None,
                                thinking: None,
                                tool_call_delta: None,
                                is_finished: true,
                                finish_reason,
                            })
                            .await;
                    }
                }
            }
        });

        Ok(rx)
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_client_with_params() {
        let client = OpenAiClient::with_params("test-key", "https://api.openai.com/v1", "gpt-4");
        // Verify client creation doesn't panic
        drop(client);
    }

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("You are a helpful assistant");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, Some("You are a helpful assistant".to_string()));
    }

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("Hello!");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, Some("Hello!".to_string()));
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("Hi there!");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, Some("Hi there!".to_string()));
    }

    #[test]
    fn test_chat_message_tool() {
        let msg = ChatMessage::tool("call-123", "Tool result");
        assert_eq!(msg.role, "tool");
        assert_eq!(msg.content, Some("Tool result".to_string()));
        assert_eq!(msg.tool_call_id, Some("call-123".to_string()));
    }

    #[test]
    fn test_tool_definition_new() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            },
            "required": ["query"]
        });

        let tool = ToolDefinition::new("web_search", "Search the web", params.clone());
        assert_eq!(tool.r#type, "function");
        assert_eq!(tool.function.name, "web_search");
        assert_eq!(tool.function.description, "Search the web");
        assert_eq!(tool.function.parameters, params);
    }

    #[test]
    fn test_chat_completion_request_serialization() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                ChatMessage::system("Be helpful"),
                ChatMessage::user("Hello"),
            ],
            tools: None,
            tool_choice: None,
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: Some(false),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("model"));
        assert!(json.contains("gpt-4"));
        assert!(json.contains("messages"));
        assert!(json.contains("temperature"));
    }

    #[test]
    fn test_chat_completion_request_with_tools() {
        let tool = ToolDefinition::new("search", "Search", serde_json::json!({"type": "object"}));

        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage::user("Search for something")],
            tools: Some(vec![tool]),
            tool_choice: Some("auto".to_string()),
            temperature: None,
            max_tokens: None,
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tools"));
        assert!(json.contains("tool_choice"));
        assert!(json.contains("auto"));
    }

    #[test]
    fn test_chat_completion_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        }"#;

        let response: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 30);
    }

    #[test]
    fn test_chat_completion_response_with_tool_calls() {
        let json = r#"{
            "id": "chatcmpl-456",
            "object": "chat.completion",
            "created": 1677652289,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call-123",
                        "type": "function",
                        "function": {
                            "name": "web_search",
                            "arguments": "{\"query\": \"test\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": null
        }"#;

        let response: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        let message = &response.choices[0].message;
        assert!(message.content.is_none());
        assert!(message.tool_calls.is_some());
        let tool_calls = message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls[0].id, "call-123");
        assert_eq!(tool_calls[0].function.name, "web_search");
    }

    #[test]
    fn test_chat_completion_chunk_deserialization() {
        let json = r#"{
            "id": "chatcmpl-789",
            "object": "chat.completion.chunk",
            "created": 1677652290,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "Hello"
                },
                "finish_reason": null
            }]
        }"#;

        let chunk: ChatCompletionChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.id, "chatcmpl-789");
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_usage_deserialization() {
        let json = r#"{
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150
        }"#;

        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_function_definition() {
        let func = FunctionDefinition {
            name: "test_func".to_string(),
            description: "Test function".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        };

        assert_eq!(func.name, "test_func");
        assert_eq!(func.description, "Test function");
    }

    #[test]
    fn test_function_call() {
        let fc = FunctionCall {
            name: "my_func".to_string(),
            arguments: "{\"arg\": 123}".to_string(),
        };

        assert_eq!(fc.name, "my_func");
        assert_eq!(fc.arguments, "{\"arg\": 123}");
    }

    #[test]
    fn test_delta_message_default() {
        let delta: DeltaMessage = Default::default();
        assert!(delta.role.is_none());
        assert!(delta.content.is_none());
        assert!(delta.tool_calls.is_none());
    }

    #[test]
    fn test_stream_tool_call_deserialization() {
        let json = r#"{
            "index": 0,
            "id": "call-123",
            "type": "function",
            "function": {
                "name": "web_search",
                "arguments": "{\"query\": \"test\"}"
            }
        }"#;

        let call: StreamToolCall = serde_json::from_str(json).unwrap();
        assert_eq!(call.index, 0);
        assert_eq!(call.id, Some("call-123".to_string()));
        assert_eq!(call.r#type, Some("function".to_string()));
        assert!(call.function.is_some());
    }

    #[test]
    fn test_stream_function_call_deserialization() {
        let json = r#"{
            "name": "my_func",
            "arguments": "{\"key\": \"value\"}"
        }"#;

        let func: StreamFunctionCall = serde_json::from_str(json).unwrap();
        assert_eq!(func.name, Some("my_func".to_string()));
        assert_eq!(func.arguments, Some("{\"key\": \"value\"}".to_string()));
    }
}
