//! OpenAI-compatible LLM client
//!
//! Supports OpenAI, Azure OpenAI, and compatible APIs (DeepSeek, etc.)

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument, warn};

use crate::{
    GenerationRequest, GenerationResponse, LlmProvider, Message as LlmMessage, MessageRole,
    SseEventParser, StreamChunk, TokenUsage, ToolCall as LlmToolCall, ToolCallDelta,
    ToolDefinition as LlmToolDefinition,
};
use async_trait::async_trait;

/// Client for OpenAI-compatible APIs
pub struct OpenAiClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    api_flavor: OpenAiApiFlavor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenAiApiFlavor {
    Official,
    Compatible,
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
    pub max_completion_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extra_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // system, user, assistant, tool
    pub content: Option<String>,
    /// Provider-specific reasoning/thinking content (e.g. DeepSeek `reasoning_content`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Provider-specific reasoning metadata payload (e.g. encrypted reasoning state).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<serde_json::Value>,
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
// Responses API Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ResponsesRequest {
    pub model: String,
    pub input: Vec<ResponseInputItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponsesReasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extra_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ResponsesReasoning {
    pub effort: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ResponseInputItem {
    Message(ResponseInputMessage),
    FunctionCall(ResponseFunctionCallItem),
    FunctionCallOutput(ResponseFunctionCallOutputItem),
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseInputMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseFunctionCallItem {
    #[serde(rename = "type")]
    pub kind: String,
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseFunctionCallOutputItem {
    #[serde(rename = "type")]
    pub kind: String,
    pub call_id: String,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponsesApiResponse {
    #[serde(default)]
    pub output: Vec<serde_json::Value>,
    pub usage: Option<ResponsesUsage>,
}

#[derive(Debug, Deserialize)]
pub struct ResponsesUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
    pub output_tokens_details: Option<ResponsesOutputTokensDetails>,
}

#[derive(Debug, Deserialize)]
pub struct ResponsesOutputTokensDetails {
    pub reasoning_tokens: Option<i32>,
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
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

#[derive(Debug, Deserialize)]
pub struct CompletionTokensDetails {
    pub reasoning_tokens: Option<i32>,
    pub audio_tokens: Option<i32>,
    pub accepted_prediction_tokens: Option<i32>,
    pub rejected_prediction_tokens: Option<i32>,
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
    pub usage: Option<Usage>,
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
    pub reasoning_content: Option<String>,
    pub reasoning: Option<serde_json::Value>,
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
    fn new(api_key: &str, base_url: &str, model: &str, api_flavor: OpenAiApiFlavor) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
            api_flavor,
        }
    }

    /// Create a client for official OpenAI endpoints.
    pub fn official_with_params(api_key: &str, base_url: &str, model: &str) -> Self {
        Self::new(api_key, base_url, model, OpenAiApiFlavor::Official)
    }

    /// Create a client for OpenAI-compatible chat/completions endpoints.
    pub fn compatible_with_params(api_key: &str, base_url: &str, model: &str) -> Self {
        Self::new(api_key, base_url, model, OpenAiApiFlavor::Compatible)
    }

    fn clone_with_same_config(&self) -> Self {
        Self {
            client: self.client.clone(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            api_flavor: self.api_flavor,
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

        // Process SSE stream with event-boundary parsing.
        let mut stream = response.bytes_stream();
        let mut parser = SseEventParser::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            for data in parser.push(&chunk) {
                if data == "[DONE]" {
                    debug!("Stream completed");
                    return Ok(());
                }

                match serde_json::from_str::<ChatCompletionChunk>(&data) {
                    Ok(chunk) => {
                        if tx.send(chunk).await.is_err() {
                            debug!("Receiver dropped, stopping stream");
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        debug!(?e, data, "Failed to parse stream chunk");
                    }
                }
            }
        }

        for data in parser.finish() {
            if data == "[DONE]" {
                debug!("Stream completed");
                return Ok(());
            }

            match serde_json::from_str::<ChatCompletionChunk>(&data) {
                Ok(chunk) => {
                    if tx.send(chunk).await.is_err() {
                        debug!("Receiver dropped, stopping stream");
                        return Ok(());
                    }
                }
                Err(e) => {
                    debug!(?e, data, "Failed to parse stream chunk");
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self, request))]
    pub async fn responses_completion(
        &self,
        mut request: ResponsesRequest,
    ) -> Result<ResponsesApiResponse> {
        let url = format!("{}/responses", self.base_url.trim_end_matches('/'));

        if request.model.is_empty() {
            request.model = self.model.clone();
        }

        debug!(url = %url, model = %request.model, "Sending responses request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI Responses API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI Responses API error ({}): {}", status, error_text);
        }

        response
            .json()
            .await
            .context("Failed to parse OpenAI Responses API response")
    }

    #[instrument(skip(self, request, tx))]
    pub async fn stream_responses_completion(
        &self,
        mut request: ResponsesRequest,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        let url = format!("{}/responses", self.base_url.trim_end_matches('/'));

        if request.model.is_empty() {
            request.model = self.model.clone();
        }
        request.stream = Some(true);

        debug!(url = %url, model = %request.model, "Sending streaming responses request");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to OpenAI Responses API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "OpenAI Responses streaming API error ({}): {}",
                status,
                error_text
            );
        }

        let mut stream = response.bytes_stream();
        let mut parser = SseEventParser::new();
        let mut latest_usage: Option<TokenUsage> = None;
        let mut emitted_payload = false;
        let mut saw_tool_calls = false;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read Responses stream chunk")?;
            for data in parser.push(&chunk) {
                if data == "[DONE]" {
                    if emitted_payload {
                        let _ = tx
                            .send(StreamChunk {
                                text: None,
                                thinking: None,
                                thinking_signature: None,
                                redacted_thinking: None,
                                usage: latest_usage,
                                tool_call_delta: None,
                                is_finished: true,
                                finish_reason: Some(
                                    responses_finish_reason(saw_tool_calls).to_string(),
                                ),
                            })
                            .await;
                    }
                    return Ok(());
                }

                let Ok(event) = serde_json::from_str::<serde_json::Value>(&data) else {
                    debug!(data, "Failed to parse Responses stream event");
                    continue;
                };

                let Some(event_type) = event.get("type").and_then(serde_json::Value::as_str) else {
                    continue;
                };

                match event_type {
                    "response.output_text.delta" | "response.refusal.delta" => {
                        if let Some(text) = event
                            .get("delta")
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| is_non_empty(value))
                        {
                            emitted_payload = true;
                            if tx
                                .send(StreamChunk {
                                    text: Some(text.to_string()),
                                    thinking: None,
                                    thinking_signature: None,
                                    redacted_thinking: None,
                                    usage: None,
                                    tool_call_delta: None,
                                    is_finished: false,
                                    finish_reason: None,
                                })
                                .await
                                .is_err()
                            {
                                debug!("Receiver dropped, stopping Responses stream");
                                return Ok(());
                            }
                        }
                    }
                    "response.reasoning_text.delta" | "response.reasoning_summary_text.delta" => {
                        if let Some(thinking) = event
                            .get("delta")
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| is_non_empty(value))
                        {
                            emitted_payload = true;
                            if tx
                                .send(StreamChunk {
                                    text: None,
                                    thinking: Some(thinking.to_string()),
                                    thinking_signature: None,
                                    redacted_thinking: None,
                                    usage: None,
                                    tool_call_delta: None,
                                    is_finished: false,
                                    finish_reason: None,
                                })
                                .await
                                .is_err()
                            {
                                debug!("Receiver dropped, stopping Responses stream");
                                return Ok(());
                            }
                        }
                    }
                    "response.function_call_arguments.delta" => {
                        let delta = event
                            .get("delta")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        if !delta.is_empty() {
                            emitted_payload = true;
                            if tx
                                .send(StreamChunk {
                                    text: None,
                                    thinking: None,
                                    thinking_signature: None,
                                    redacted_thinking: None,
                                    usage: None,
                                    tool_call_delta: Some(ToolCallDelta {
                                        index: responses_stream_index(&event),
                                        id: responses_stream_tool_id(event.get("item"), &event),
                                        name: responses_stream_tool_name(event.get("item"), &event),
                                        arguments_delta: Some(delta.to_string()),
                                        arguments: None,
                                    }),
                                    is_finished: false,
                                    finish_reason: None,
                                })
                                .await
                                .is_err()
                            {
                                debug!("Receiver dropped, stopping Responses stream");
                                return Ok(());
                            }
                        }
                    }
                    "response.output_item.done" => {
                        let Some(item) = event.get("item") else {
                            continue;
                        };
                        if item.get("type").and_then(serde_json::Value::as_str)
                            != Some("function_call")
                        {
                            continue;
                        }

                        let arguments = item
                            .get("arguments")
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| is_non_empty(value));
                        let name = responses_stream_tool_name(Some(item), &event);

                        if let (Some(arguments), Some(name)) = (arguments, name) {
                            emitted_payload = true;
                            saw_tool_calls = true;
                            if tx
                                .send(StreamChunk {
                                    text: None,
                                    thinking: None,
                                    thinking_signature: None,
                                    redacted_thinking: None,
                                    usage: None,
                                    tool_call_delta: Some(ToolCallDelta {
                                        index: responses_stream_index(&event),
                                        id: responses_stream_tool_id(Some(item), &event),
                                        name: Some(name),
                                        arguments_delta: None,
                                        arguments: Some(arguments.to_string()),
                                    }),
                                    is_finished: false,
                                    finish_reason: None,
                                })
                                .await
                                .is_err()
                            {
                                debug!("Receiver dropped, stopping Responses stream");
                                return Ok(());
                            }
                        }
                    }
                    "response.completed" => {
                        if let Some(response) = event.get("response").cloned() {
                            match serde_json::from_value::<ResponsesApiResponse>(response) {
                                Ok(parsed) => {
                                    latest_usage = parsed.usage.map(convert_responses_usage);
                                    if !saw_tool_calls {
                                        saw_tool_calls =
                                            responses_output_contains_tool_call(&parsed.output);
                                    }
                                }
                                Err(error) => {
                                    debug!(?error, "Failed to parse response.completed payload");
                                }
                            }
                        }

                        let _ = tx
                            .send(StreamChunk {
                                text: None,
                                thinking: None,
                                thinking_signature: None,
                                redacted_thinking: None,
                                usage: latest_usage,
                                tool_call_delta: None,
                                is_finished: true,
                                finish_reason: Some(
                                    responses_finish_reason(saw_tool_calls).to_string(),
                                ),
                            })
                            .await;
                        return Ok(());
                    }
                    "response.failed" | "error" => {
                        if emitted_payload {
                            let _ = tx
                                .send(StreamChunk {
                                    text: None,
                                    thinking: None,
                                    thinking_signature: None,
                                    redacted_thinking: None,
                                    usage: latest_usage,
                                    tool_call_delta: None,
                                    is_finished: true,
                                    finish_reason: Some("stream_error".to_string()),
                                })
                                .await;
                        }
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        for data in parser.finish() {
            if data == "[DONE]" {
                if emitted_payload {
                    let _ = tx
                        .send(StreamChunk {
                            text: None,
                            thinking: None,
                            thinking_signature: None,
                            redacted_thinking: None,
                            usage: latest_usage,
                            tool_call_delta: None,
                            is_finished: true,
                            finish_reason: Some(
                                responses_finish_reason(saw_tool_calls).to_string(),
                            ),
                        })
                        .await;
                }
                return Ok(());
            }
        }

        if emitted_payload {
            let _ = tx
                .send(StreamChunk {
                    text: None,
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: latest_usage,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some(responses_finish_reason(saw_tool_calls).to_string()),
                })
                .await;
        }

        Ok(())
    }

    fn uses_responses_api(&self) -> bool {
        matches!(self.api_flavor, OpenAiApiFlavor::Official)
    }

    fn build_responses_request(
        &self,
        request: GenerationRequest,
        stream: bool,
    ) -> ResponsesRequest {
        let GenerationRequest {
            system_prompt,
            messages,
            tools,
            temperature,
            max_tokens,
            thinking_budget_tokens,
            mut extra_params,
        } = request;

        let (response_tools, tool_choice) = convert_tools_for_openai(tools);
        ResponsesRequest {
            model: self.model.clone(),
            input: convert_messages_for_responses(system_prompt, messages),
            tools: response_tools,
            tool_choice,
            temperature,
            max_output_tokens: build_max_completion_tokens(max_tokens, &mut extra_params),
            reasoning: build_responses_reasoning(thinking_budget_tokens, &mut extra_params),
            stream: Some(stream),
            extra_params,
        }
    }

    /// Simple chat helper
    pub async fn chat(&self, system: Option<&str>, user_message: &str) -> Result<String> {
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(sys.to_string()),
                reasoning_content: None,
                reasoning: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
            reasoning_content: None,
            reasoning: None,
            tool_calls: None,
            tool_call_id: None,
        });

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools: None,
            tool_choice: None,
            temperature: Some(0.7),
            max_completion_tokens: Some(2048),
            reasoning_effort: None,
            stream: Some(false),
            stream_options: None,
            extra_params: HashMap::new(),
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
            reasoning_content: None,
            reasoning: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            reasoning_content: None,
            reasoning: None,
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
            reasoning_content: None,
            reasoning: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a tool message (response to a tool call)
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            reasoning_content: None,
            reasoning: None,
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

fn convert_messages_for_openai(messages: Vec<LlmMessage>) -> Vec<ChatMessage> {
    messages
        .into_iter()
        .map(|msg| {
            let LlmMessage {
                role,
                content,
                thinking,
                thinking_signature,
                redacted_thinking: _,
                tool_calls,
                tool_call_id,
            } = msg;

            let role = match role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
                MessageRole::Context => "system", // Context treated as system
            };

            let tool_calls: Option<Vec<ToolCall>> = tool_calls.map(|calls| {
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

            let reasoning_content = if role == "assistant" {
                thinking.filter(|value| is_non_empty(value))
            } else {
                None
            };
            let reasoning = if role == "assistant" {
                thinking_signature
                    .filter(|value| is_non_empty(value))
                    .map(|signature| serde_json::json!({ "encrypted_content": signature }))
            } else {
                None
            };

            ChatMessage {
                role: role.to_string(),
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                reasoning_content,
                reasoning,
                tool_calls,
                tool_call_id,
            }
        })
        .collect()
}

fn convert_tools_for_openai(
    tools: Vec<LlmToolDefinition>,
) -> (Option<Vec<ToolDefinition>>, Option<String>) {
    if tools.is_empty() {
        (None, None)
    } else {
        (
            Some(
                tools
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
            ),
            Some("auto".to_string()),
        )
    }
}

fn convert_messages_for_responses(
    system_prompt: Option<String>,
    messages: Vec<LlmMessage>,
) -> Vec<ResponseInputItem> {
    let mut input = Vec::new();

    if let Some(system) = system_prompt.filter(|value| is_non_empty(value)) {
        input.push(ResponseInputItem::Message(ResponseInputMessage {
            role: "system".to_string(),
            content: system,
        }));
    }

    for message in messages {
        match message.role {
            MessageRole::System | MessageRole::Context | MessageRole::User => {
                if !message.content.is_empty() {
                    let role = match message.role {
                        MessageRole::User => "user",
                        _ => "system",
                    };
                    input.push(ResponseInputItem::Message(ResponseInputMessage {
                        role: role.to_string(),
                        content: message.content,
                    }));
                }
            }
            MessageRole::Assistant => {
                if !message.content.is_empty() {
                    input.push(ResponseInputItem::Message(ResponseInputMessage {
                        role: "assistant".to_string(),
                        content: message.content,
                    }));
                }

                if let Some(tool_calls) = message.tool_calls {
                    for tool_call in tool_calls {
                        let call_id = tool_call.id.unwrap_or_default();
                        if call_id.is_empty() {
                            warn!(
                                tool_name = %tool_call.name,
                                "Skipping assistant tool call without id in Responses API projection"
                            );
                            continue;
                        }

                        input.push(ResponseInputItem::FunctionCall(ResponseFunctionCallItem {
                            kind: "function_call".to_string(),
                            call_id,
                            name: tool_call.name,
                            arguments: tool_call.arguments.to_string(),
                        }));
                    }
                }
            }
            MessageRole::Tool => {
                let Some(call_id) = message.tool_call_id.filter(|value| is_non_empty(value)) else {
                    warn!("Skipping tool message without tool_call_id in Responses API projection");
                    continue;
                };

                input.push(ResponseInputItem::FunctionCallOutput(
                    ResponseFunctionCallOutputItem {
                        kind: "function_call_output".to_string(),
                        call_id,
                        output: message.content,
                    },
                ));
            }
        }
    }

    input
}

fn build_responses_reasoning(
    thinking_budget_tokens: Option<u32>,
    extra_params: &mut HashMap<String, serde_json::Value>,
) -> Option<ResponsesReasoning> {
    build_reasoning_effort(thinking_budget_tokens, extra_params)
        .map(|effort| ResponsesReasoning { effort })
}

fn convert_responses_usage(usage: ResponsesUsage) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage.input_tokens,
        completion_tokens: usage.output_tokens,
        total_tokens: usage.total_tokens,
        reasoning_tokens: usage
            .output_tokens_details
            .and_then(|details| details.reasoning_tokens),
    }
}

fn convert_responses_output(response: ResponsesApiResponse) -> GenerationResponse {
    let mut content = String::new();
    let mut thinking_parts = Vec::new();
    let mut tool_calls = Vec::new();
    let mut warnings = Vec::new();

    for item in response.output {
        match item.get("type").and_then(serde_json::Value::as_str) {
            Some("message") => {
                if let Some(parts) = item.get("content").and_then(serde_json::Value::as_array) {
                    for part in parts {
                        match part.get("type").and_then(serde_json::Value::as_str) {
                            Some("output_text") => {
                                if let Some(text) =
                                    part.get("text").and_then(serde_json::Value::as_str)
                                {
                                    content.push_str(text);
                                }
                            }
                            Some("refusal") => {
                                if let Some(text) = part
                                    .get("refusal")
                                    .and_then(serde_json::Value::as_str)
                                    .or_else(|| {
                                        part.get("text").and_then(serde_json::Value::as_str)
                                    })
                                {
                                    content.push_str(text);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Some("reasoning") => {
                if let Some(reasoning) = extract_reasoning_text_from_value(&item)
                    && !reasoning.is_empty()
                {
                    thinking_parts.push(reasoning);
                }
            }
            Some("function_call") => {
                let Some(name) = item
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| is_non_empty(value))
                else {
                    continue;
                };
                let arguments_raw = item
                    .get("arguments")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("{}");

                match serde_json::from_str::<serde_json::Value>(arguments_raw) {
                    Ok(arguments) => tool_calls.push(LlmToolCall {
                        id: item
                            .get("call_id")
                            .or_else(|| item.get("id"))
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned),
                        name: name.to_string(),
                        arguments,
                    }),
                    Err(err) => {
                        warn!(
                            tool_name = %name,
                            error = %err,
                            "Dropping malformed Responses API tool call arguments"
                        );
                        warnings.push(format!(
                            "Dropped malformed Responses API tool call `{name}` arguments."
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    GenerationResponse {
        content,
        thinking: if thinking_parts.is_empty() {
            None
        } else {
            Some(thinking_parts.join("\n"))
        },
        thinking_signature: None,
        redacted_thinking: Vec::new(),
        tool_calls,
        usage: response.usage.map(convert_responses_usage),
        warnings,
    }
}

fn responses_finish_reason(saw_tool_calls: bool) -> &'static str {
    if saw_tool_calls { "tool_calls" } else { "stop" }
}

fn responses_stream_index(event: &serde_json::Value) -> usize {
    event
        .get("output_index")
        .or_else(|| event.get("item_index"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default() as usize
}

fn responses_stream_tool_id(
    item: Option<&serde_json::Value>,
    event: &serde_json::Value,
) -> Option<String> {
    item.and_then(|value| {
        value
            .get("call_id")
            .or_else(|| value.get("id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    })
    .or_else(|| {
        event
            .get("call_id")
            .or_else(|| event.get("item_id"))
            .or_else(|| event.get("id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    })
}

fn responses_stream_tool_name(
    item: Option<&serde_json::Value>,
    event: &serde_json::Value,
) -> Option<String> {
    item.and_then(|value| {
        value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .filter(|value| is_non_empty(value))
            .map(str::to_owned)
    })
    .or_else(|| {
        event
            .get("name")
            .and_then(serde_json::Value::as_str)
            .filter(|value| is_non_empty(value))
            .map(str::to_owned)
    })
}

fn responses_output_contains_tool_call(output: &[serde_json::Value]) -> bool {
    output
        .iter()
        .any(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("function_call"))
}

fn map_thinking_budget_to_effort(thinking_budget_tokens: u32) -> &'static str {
    if thinking_budget_tokens <= 256 {
        "minimal"
    } else if thinking_budget_tokens <= 1_024 {
        "low"
    } else if thinking_budget_tokens <= 4_096 {
        "medium"
    } else if thinking_budget_tokens <= 8_192 {
        "high"
    } else {
        "xhigh"
    }
}

fn is_valid_reasoning_effort(effort: &str) -> bool {
    matches!(
        effort,
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh"
    )
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn extract_reasoning_text_from_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) if is_non_empty(text) => Some(text.clone()),
        serde_json::Value::Object(map) => {
            for key in ["content", "text"] {
                if let Some(text) = map
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .filter(|text| is_non_empty(text))
                {
                    return Some(text.to_string());
                }
            }

            for key in ["content", "summary"] {
                if let Some(serde_json::Value::Array(items)) = map.get(key) {
                    let mut joined = String::new();
                    for item in items {
                        if let Some(text) = item.as_str().filter(|text| is_non_empty(text)) {
                            joined.push_str(text);
                        } else if let Some(text) = item
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .filter(|text| is_non_empty(text))
                        {
                            joined.push_str(text);
                        } else if let Some(text) = item
                            .get("content")
                            .and_then(serde_json::Value::as_str)
                            .filter(|text| is_non_empty(text))
                        {
                            joined.push_str(text);
                        }
                    }
                    if !joined.is_empty() {
                        return Some(joined);
                    }
                }
            }

            None
        }
        _ => None,
    }
}

fn extract_reasoning_signature(reasoning: Option<&serde_json::Value>) -> Option<String> {
    reasoning.and_then(|value| match value {
        serde_json::Value::Object(map) => map
            .get("encrypted_content")
            .and_then(serde_json::Value::as_str)
            .filter(|value| is_non_empty(value))
            .map(ToString::to_string),
        _ => None,
    })
}

fn extract_reasoning_fields(
    reasoning_content: Option<&str>,
    reasoning: Option<&serde_json::Value>,
) -> (Option<String>, Option<String>) {
    let thinking = reasoning_content
        .filter(|value| is_non_empty(value))
        .map(ToString::to_string)
        .or_else(|| reasoning.and_then(extract_reasoning_text_from_value));

    let thinking_signature = extract_reasoning_signature(reasoning);

    (thinking, thinking_signature)
}

fn build_reasoning_effort(
    thinking_budget_tokens: Option<u32>,
    extra_params: &mut HashMap<String, serde_json::Value>,
) -> Option<String> {
    if let Some(value) = extra_params.remove("reasoning_effort") {
        if let Some(effort) = value.as_str() {
            if is_valid_reasoning_effort(effort) {
                return Some(effort.to_string());
            }
            debug!(
                effort,
                "Ignoring invalid `reasoning_effort`; expected one of: none, minimal, low, medium, high, xhigh"
            );
        } else {
            debug!(
                value = %value,
                "Ignoring non-string `reasoning_effort` in extra_params"
            );
        }
    }

    thinking_budget_tokens
        .map(map_thinking_budget_to_effort)
        .map(str::to_string)
}

fn build_max_completion_tokens(
    max_tokens: Option<i32>,
    extra_params: &mut HashMap<String, serde_json::Value>,
) -> Option<i32> {
    if let Some(value) = extra_params.remove("max_completion_tokens") {
        if let Some(tokens) = value.as_i64() {
            return i32::try_from(tokens).ok();
        }
        debug!(
            value = %value,
            "Ignoring non-integer `max_completion_tokens` in extra_params"
        );
    }
    max_tokens
}

fn convert_usage(usage: Usage) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
        reasoning_tokens: usage
            .completion_tokens_details
            .and_then(|details| details.reasoning_tokens),
    }
}

fn allocate_stream_tool_index(
    tool_index_map: &mut HashMap<(i32, i32), usize>,
    next_tool_index: &mut usize,
    choice_index: i32,
    tool_call_index: i32,
) -> usize {
    *tool_index_map
        .entry((choice_index, tool_call_index))
        .or_insert_with(|| {
            let assigned = *next_tool_index;
            *next_tool_index = next_tool_index.saturating_add(1);
            assigned
        })
}

fn select_stream_choice_index(
    selected_choice_index: Option<i32>,
    emitted_payload: bool,
    choices: &[ChunkChoice],
) -> Option<i32> {
    if choices.is_empty() {
        return selected_choice_index;
    }

    let has_index_zero = choices.iter().any(|choice| choice.index == 0);
    match selected_choice_index {
        Some(0) => Some(0),
        Some(_current) if has_index_zero && !emitted_payload => Some(0),
        Some(current) => Some(current),
        None if has_index_zero => Some(0),
        None => Some(choices[0].index),
    }
}

fn select_primary_choice(choices: &[Choice]) -> Option<&Choice> {
    choices
        .iter()
        .find(|choice| choice.index == 0)
        .or_else(|| choices.first())
}

// ============================================================================
// LlmProvider Implementation
// ============================================================================

#[async_trait]
impl LlmProvider for OpenAiClient {
    async fn generate(&mut self, request: GenerationRequest) -> anyhow::Result<GenerationResponse> {
        let chat_request = request.clone();

        if self.uses_responses_api() {
            let response_request = self.build_responses_request(request, false);
            match self.responses_completion(response_request).await {
                Ok(response) => return Ok(convert_responses_output(response)),
                Err(err) => {
                    warn!(
                        error = %err,
                        "Responses API request failed; falling back to chat/completions"
                    );
                    let mut fallback = self.generate_via_chat_completion(chat_request).await?;
                    fallback.warnings.insert(
                        0,
                        format!(
                            "Responses API request failed; fell back to chat/completions: {err}"
                        ),
                    );
                    return Ok(fallback);
                }
            }
        }

        self.generate_via_chat_completion(chat_request).await
    }

    async fn chat(&mut self, system: Option<&str>, user: &str) -> anyhow::Result<String> {
        // Directly use the existing chat method
        self.chat(system, user).await
    }

    async fn generate_stream(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        if self.uses_responses_api() {
            match self.generate_stream_via_responses(request.clone()).await {
                Ok(rx) => return Ok(rx),
                Err(err) => {
                    warn!(
                        error = %err,
                        "Responses API streaming failed; falling back to chat/completions streaming"
                    );
                }
            }
        }

        self.generate_stream_via_chat_completion(request).await
    }

    fn provider_name(&self) -> &'static str {
        match self.api_flavor {
            OpenAiApiFlavor::Official => "openai",
            OpenAiApiFlavor::Compatible => "openai_compatible",
        }
    }
}

impl OpenAiClient {
    async fn generate_via_chat_completion(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<GenerationResponse> {
        let GenerationRequest {
            system_prompt,
            messages: request_messages,
            tools: request_tools,
            temperature,
            max_tokens,
            thinking_budget_tokens,
            mut extra_params,
        } = request;

        let mut messages: Vec<ChatMessage> = Vec::new();
        if let Some(system) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system),
                reasoning_content: None,
                reasoning: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
        messages.extend(convert_messages_for_openai(request_messages));

        let (tools, tool_choice) = convert_tools_for_openai(request_tools);
        let reasoning_effort = build_reasoning_effort(thinking_budget_tokens, &mut extra_params);
        let max_completion_tokens = build_max_completion_tokens(max_tokens, &mut extra_params);

        let chat_request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools,
            tool_choice,
            temperature,
            max_completion_tokens,
            reasoning_effort,
            stream: Some(false),
            stream_options: None,
            extra_params,
        };

        let response = self.chat_completion(chat_request).await?;
        let choice = select_primary_choice(&response.choices).context("No choices in response")?;
        let message = &choice.message;

        let mut response_warnings: Vec<String> = Vec::new();
        let tool_calls: Vec<LlmToolCall> = message
            .tool_calls
            .as_ref()
            .map(|calls| {
                let mut parsed_calls = Vec::new();
                for call in calls {
                    match serde_json::from_str::<serde_json::Value>(&call.function.arguments) {
                        Ok(args) => parsed_calls.push(LlmToolCall {
                            id: Some(call.id.clone()),
                            name: call.function.name.clone(),
                            arguments: args,
                        }),
                        Err(err) => {
                            warn!(
                                tool_name = %call.function.name,
                                error = %err,
                                "Dropping malformed non-streaming tool call arguments"
                            );
                            response_warnings.push(format!(
                                "Dropped malformed non-streaming tool call `{}` arguments.",
                                call.function.name
                            ));
                        }
                    }
                }
                parsed_calls
            })
            .unwrap_or_default();

        let usage = response.usage.map(convert_usage);
        let (thinking, thinking_signature) = extract_reasoning_fields(
            message.reasoning_content.as_deref(),
            message.reasoning.as_ref(),
        );

        Ok(GenerationResponse {
            content: message.content.clone().unwrap_or_default(),
            thinking,
            thinking_signature,
            redacted_thinking: Vec::new(),
            tool_calls,
            usage,
            warnings: response_warnings,
        })
    }

    async fn generate_stream_via_responses(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        let response_request = self.build_responses_request(request, true);
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let client = self.clone_with_same_config();
        tokio::spawn(async move {
            if let Err(error) = client
                .stream_responses_completion(response_request, tx)
                .await
            {
                warn!(
                    error = %error,
                    "Responses API stream ended before emitting a terminal chunk"
                );
            }
        });
        Ok(rx)
    }

    async fn generate_stream_via_chat_completion(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        let GenerationRequest {
            system_prompt,
            messages: request_messages,
            tools: request_tools,
            temperature,
            max_tokens,
            thinking_budget_tokens,
            mut extra_params,
        } = request;

        let mut messages: Vec<ChatMessage> = Vec::new();
        if let Some(system) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system),
                reasoning_content: None,
                reasoning: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
        messages.extend(convert_messages_for_openai(request_messages));

        let (tools, tool_choice) = convert_tools_for_openai(request_tools);
        let reasoning_effort = build_reasoning_effort(thinking_budget_tokens, &mut extra_params);
        let max_completion_tokens = build_max_completion_tokens(max_tokens, &mut extra_params);

        let chat_request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools,
            tool_choice,
            temperature,
            max_completion_tokens,
            reasoning_effort,
            stream: Some(true),
            stream_options: Some(StreamOptions {
                include_usage: true,
            }),
            extra_params,
        };

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel(100);
        let (stream_status_tx, stream_status_rx) =
            tokio::sync::oneshot::channel::<Option<String>>();

        let client = self.clone_with_same_config();
        tokio::spawn(async move {
            let outcome = match client.stream_chat_completion(chat_request, chunk_tx).await {
                Ok(()) => None,
                Err(e) => {
                    debug!(error = ?e, "Stream chat completion failed");
                    Some(e.to_string())
                }
            };
            let _ = stream_status_tx.send(outcome);
        });

        tokio::spawn(async move {
            let mut latest_finish_reason: Option<String> = None;
            let mut latest_usage: Option<TokenUsage> = None;
            let mut emitted_payload = false;
            let mut selected_choice_index: Option<i32> = None;
            let mut tool_index_map: HashMap<(i32, i32), usize> = HashMap::new();
            let mut next_tool_index: usize = 0;
            while let Some(chunk) = chunk_rx.recv().await {
                if let Some(usage) = chunk.usage {
                    latest_usage = Some(convert_usage(usage));
                }

                selected_choice_index = select_stream_choice_index(
                    selected_choice_index,
                    emitted_payload,
                    &chunk.choices,
                );
                let Some(active_choice_index) = selected_choice_index else {
                    continue;
                };

                for choice in &chunk.choices {
                    if choice.index != active_choice_index {
                        continue;
                    }
                    let delta = &choice.delta;

                    if let Some(ref reason) = choice.finish_reason {
                        latest_finish_reason = Some(reason.clone());
                    }

                    let (thinking, thinking_signature) = extract_reasoning_fields(
                        delta.reasoning_content.as_deref(),
                        delta.reasoning.as_ref(),
                    );

                    if let Some(reasoning_content) = thinking {
                        emitted_payload = true;
                        let _ = tx
                            .send(StreamChunk {
                                text: None,
                                thinking: Some(reasoning_content),
                                thinking_signature: None,
                                redacted_thinking: None,
                                usage: None,
                                tool_call_delta: None,
                                is_finished: false,
                                finish_reason: None,
                            })
                            .await;
                    }
                    if let Some(signature) = thinking_signature {
                        emitted_payload = true;
                        let _ = tx
                            .send(StreamChunk {
                                text: None,
                                thinking: None,
                                thinking_signature: Some(signature),
                                redacted_thinking: None,
                                usage: None,
                                tool_call_delta: None,
                                is_finished: false,
                                finish_reason: None,
                            })
                            .await;
                    }

                    if let Some(content) = &delta.content {
                        emitted_payload = true;
                        let _ = tx
                            .send(StreamChunk {
                                text: Some(content.clone()),
                                thinking: None,
                                thinking_signature: None,
                                redacted_thinking: None,
                                usage: None,
                                tool_call_delta: None,
                                is_finished: false,
                                finish_reason: None,
                            })
                            .await;
                    }

                    if let Some(tool_calls) = &delta.tool_calls {
                        for tool_call in tool_calls {
                            emitted_payload = true;
                            let stream_tool_index = allocate_stream_tool_index(
                                &mut tool_index_map,
                                &mut next_tool_index,
                                choice.index,
                                tool_call.index,
                            );
                            let tool_delta = ToolCallDelta {
                                index: stream_tool_index,
                                id: tool_call.id.clone(),
                                name: tool_call.function.as_ref().and_then(|f| f.name.clone()),
                                arguments_delta: tool_call
                                    .function
                                    .as_ref()
                                    .and_then(|f| f.arguments.clone()),
                                arguments: None,
                            };

                            let _ = tx
                                .send(StreamChunk {
                                    text: None,
                                    thinking: None,
                                    thinking_signature: None,
                                    redacted_thinking: None,
                                    usage: None,
                                    tool_call_delta: Some(tool_delta),
                                    is_finished: false,
                                    finish_reason: None,
                                })
                                .await;
                        }
                    }
                }
            }

            let upstream_error = stream_status_rx.await.ok().flatten();
            if upstream_error.is_some() && !emitted_payload {
                return;
            }

            let _ = tx
                .send(StreamChunk {
                    text: None,
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: latest_usage,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: latest_finish_reason
                        .or_else(|| upstream_error.map(|_| "stream_error".to_string())),
                })
                .await;
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_openai_client_with_params() {
        let client =
            OpenAiClient::official_with_params("test-key", "https://api.openai.com/v1", "gpt-4");
        assert_eq!(client.provider_name(), "openai");
        drop(client);
    }

    #[test]
    fn test_openai_compatible_client_with_params() {
        let client = OpenAiClient::compatible_with_params(
            "test-key",
            "https://proxy.example/v1",
            "gpt-4o-mini",
        );
        assert_eq!(client.provider_name(), "openai_compatible");
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
            max_completion_tokens: Some(100),
            reasoning_effort: None,
            stream: Some(false),
            stream_options: None,
            extra_params: HashMap::new(),
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
            max_completion_tokens: None,
            reasoning_effort: None,
            stream: None,
            stream_options: None,
            extra_params: HashMap::new(),
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
    fn test_chat_completion_response_with_reasoning_tokens() {
        let json = r#"{
            "id": "chatcmpl-rsn",
            "object": "chat.completion",
            "created": 1677652289,
            "model": "deepseek-reasoner",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Final answer"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 22,
                "total_tokens": 33,
                "completion_tokens_details": {
                    "reasoning_tokens": 7
                }
            }
        }"#;

        let response: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        let usage = response.usage.unwrap();
        assert_eq!(
            usage.completion_tokens_details.unwrap().reasoning_tokens,
            Some(7)
        );
    }

    #[test]
    fn test_chat_completion_response_deserialization_with_reasoning_content() {
        let json = r#"{
            "id": "chatcmpl-rsn-content",
            "object": "chat.completion",
            "created": 1677652290,
            "model": "deepseek-reasoner",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Final answer",
                    "reasoning_content": "Internal reasoning trail"
                },
                "finish_reason": "stop"
            }],
            "usage": null
        }"#;

        let response: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        let message = &response.choices[0].message;
        assert_eq!(
            message.reasoning_content.as_deref(),
            Some("Internal reasoning trail")
        );
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
    fn test_chat_completion_chunk_deserialization_with_reasoning_content() {
        let json = r#"{
            "id": "chatcmpl-791",
            "object": "chat.completion.chunk",
            "created": 1677652292,
            "model": "deepseek-reasoner",
            "choices": [{
                "index": 0,
                "delta": {
                    "reasoning_content": "Thinking..."
                },
                "finish_reason": null
            }]
        }"#;

        let chunk: ChatCompletionChunk = serde_json::from_str(json).unwrap();
        assert_eq!(
            chunk.choices[0].delta.reasoning_content.as_deref(),
            Some("Thinking...")
        );
    }

    #[test]
    fn test_chat_completion_chunk_deserialization_with_usage() {
        let json = r#"{
            "id": "chatcmpl-790",
            "object": "chat.completion.chunk",
            "created": 1677652291,
            "model": "deepseek-reasoner",
            "choices": [],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 2,
                "total_tokens": 3,
                "completion_tokens_details": {
                    "reasoning_tokens": 1
                }
            }
        }"#;

        let chunk: ChatCompletionChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices.len(), 0);
        assert_eq!(
            chunk
                .usage
                .and_then(|u| u.completion_tokens_details)
                .and_then(|d| d.reasoning_tokens),
            Some(1)
        );
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
        assert!(delta.reasoning_content.is_none());
        assert!(delta.reasoning.is_none());
        assert!(delta.tool_calls.is_none());
    }

    #[test]
    fn test_build_reasoning_effort_prefers_extra_params() {
        let mut extra_params = HashMap::from([(
            "reasoning_effort".to_string(),
            serde_json::Value::String("high".to_string()),
        )]);

        let effort = build_reasoning_effort(Some(256), &mut extra_params);
        assert_eq!(effort.as_deref(), Some("high"));
        assert!(!extra_params.contains_key("reasoning_effort"));
    }

    #[test]
    fn test_build_reasoning_effort_maps_budget() {
        let mut extra_params = HashMap::new();

        assert_eq!(
            build_reasoning_effort(Some(64), &mut extra_params).as_deref(),
            Some("minimal")
        );
        assert_eq!(
            build_reasoning_effort(Some(512), &mut extra_params).as_deref(),
            Some("low")
        );
        assert_eq!(
            build_reasoning_effort(Some(2048), &mut extra_params).as_deref(),
            Some("medium")
        );
        assert_eq!(
            build_reasoning_effort(Some(7000), &mut extra_params).as_deref(),
            Some("high")
        );
        assert_eq!(
            build_reasoning_effort(Some(10000), &mut extra_params).as_deref(),
            Some("xhigh")
        );
    }

    #[test]
    fn test_build_reasoning_effort_accepts_extended_values() {
        let mut extra_params = HashMap::from([(
            "reasoning_effort".to_string(),
            serde_json::Value::String("xhigh".to_string()),
        )]);
        let effort = build_reasoning_effort(Some(256), &mut extra_params);
        assert_eq!(effort.as_deref(), Some("xhigh"));
        assert!(!extra_params.contains_key("reasoning_effort"));
    }

    #[test]
    fn test_convert_messages_for_openai_preserves_assistant_reasoning_content() {
        let messages = vec![crate::Message {
            role: MessageRole::Assistant,
            content: "Done".to_string(),
            thinking: Some("step by step".to_string()),
            thinking_signature: Some("encrypted_state".to_string()),
            redacted_thinking: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        let converted = convert_messages_for_openai(messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(
            converted[0].reasoning_content.as_deref(),
            Some("step by step")
        );
        assert_eq!(
            converted[0]
                .reasoning
                .as_ref()
                .and_then(|value| value.get("encrypted_content"))
                .and_then(serde_json::Value::as_str),
            Some("encrypted_state")
        );
    }

    #[test]
    fn test_convert_messages_for_responses_projects_tool_history() {
        let messages = vec![
            crate::Message {
                role: MessageRole::Assistant,
                content: "Let me inspect that.".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: None,
                tool_calls: Some(vec![crate::ToolCall {
                    id: Some("call_1".to_string()),
                    name: "lookup".to_string(),
                    arguments: json!({"query": "alan"}),
                }]),
                tool_call_id: None,
            },
            crate::Message {
                role: MessageRole::Tool,
                content: "{\"ok\":true}".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: None,
                tool_calls: None,
                tool_call_id: Some("call_1".to_string()),
            },
        ];

        let converted = convert_messages_for_responses(Some("System prompt".to_string()), messages);
        assert_eq!(converted.len(), 4);

        match &converted[0] {
            ResponseInputItem::Message(message) => {
                assert_eq!(message.role, "system");
                assert_eq!(message.content, "System prompt");
            }
            _ => panic!("expected system message"),
        }

        match &converted[1] {
            ResponseInputItem::Message(message) => {
                assert_eq!(message.role, "assistant");
                assert_eq!(message.content, "Let me inspect that.");
            }
            _ => panic!("expected assistant message"),
        }

        match &converted[2] {
            ResponseInputItem::FunctionCall(tool_call) => {
                assert_eq!(tool_call.call_id, "call_1");
                assert_eq!(tool_call.name, "lookup");
                assert_eq!(tool_call.arguments, "{\"query\":\"alan\"}");
            }
            _ => panic!("expected function call"),
        }

        match &converted[3] {
            ResponseInputItem::FunctionCallOutput(tool_output) => {
                assert_eq!(tool_output.call_id, "call_1");
                assert_eq!(tool_output.output, "{\"ok\":true}");
            }
            _ => panic!("expected function call output"),
        }
    }

    #[test]
    fn test_convert_responses_output_extracts_final_tool_arguments() {
        let response = ResponsesApiResponse {
            output: vec![
                json!({
                    "type": "reasoning",
                    "summary": [{"text": "Inspecting tool input"}]
                }),
                json!({
                    "type": "message",
                    "content": [
                        {"type": "output_text", "text": "I'll look that up."}
                    ]
                }),
                json!({
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "lookup",
                    "arguments": "{\"query\":\"alan\"}"
                }),
            ],
            usage: Some(ResponsesUsage {
                input_tokens: 11,
                output_tokens: 22,
                total_tokens: 33,
                output_tokens_details: Some(ResponsesOutputTokensDetails {
                    reasoning_tokens: Some(7),
                }),
            }),
        };

        let converted = convert_responses_output(response);
        assert_eq!(converted.content, "I'll look that up.");
        assert_eq!(converted.thinking.as_deref(), Some("Inspecting tool input"));
        assert_eq!(converted.tool_calls.len(), 1);
        assert_eq!(converted.tool_calls[0].id.as_deref(), Some("call_1"));
        assert_eq!(converted.tool_calls[0].name, "lookup");
        assert_eq!(converted.tool_calls[0].arguments, json!({"query": "alan"}));
        assert_eq!(
            converted.usage.map(|usage| usage.reasoning_tokens),
            Some(Some(7))
        );
    }

    #[test]
    fn test_build_max_completion_tokens_prefers_extra_params() {
        let mut extra_params = HashMap::from([(
            "max_completion_tokens".to_string(),
            serde_json::Value::Number(serde_json::Number::from(1234)),
        )]);

        let max_tokens = build_max_completion_tokens(Some(100), &mut extra_params);
        assert_eq!(max_tokens, Some(1234));
        assert!(!extra_params.contains_key("max_completion_tokens"));
    }

    #[test]
    fn test_convert_usage_extracts_reasoning_tokens() {
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
            completion_tokens_details: Some(CompletionTokensDetails {
                reasoning_tokens: Some(7),
                audio_tokens: None,
                accepted_prediction_tokens: None,
                rejected_prediction_tokens: None,
            }),
        };

        let token_usage = convert_usage(usage);
        assert_eq!(token_usage.prompt_tokens, 10);
        assert_eq!(token_usage.completion_tokens, 20);
        assert_eq!(token_usage.total_tokens, 30);
        assert_eq!(token_usage.reasoning_tokens, Some(7));
    }

    #[test]
    fn test_allocate_stream_tool_index_is_stable_per_choice_and_tool_index() {
        let mut tool_index_map = HashMap::new();
        let mut next_tool_index = 0usize;

        let first = allocate_stream_tool_index(&mut tool_index_map, &mut next_tool_index, 0, 0);
        let first_repeat =
            allocate_stream_tool_index(&mut tool_index_map, &mut next_tool_index, 0, 0);
        let second = allocate_stream_tool_index(&mut tool_index_map, &mut next_tool_index, 1, 0);
        let third = allocate_stream_tool_index(&mut tool_index_map, &mut next_tool_index, 1, 1);

        assert_eq!(first, first_repeat);
        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_eq!(next_tool_index, 3);
    }

    #[test]
    fn test_select_stream_choice_index_prefers_zero_then_falls_back() {
        let choices_non_zero = vec![
            ChunkChoice {
                index: 2,
                delta: DeltaMessage::default(),
                finish_reason: None,
            },
            ChunkChoice {
                index: 3,
                delta: DeltaMessage::default(),
                finish_reason: None,
            },
        ];
        assert_eq!(
            select_stream_choice_index(None, false, &choices_non_zero),
            Some(2)
        );

        let choices_zero = vec![
            ChunkChoice {
                index: 5,
                delta: DeltaMessage::default(),
                finish_reason: None,
            },
            ChunkChoice {
                index: 0,
                delta: DeltaMessage::default(),
                finish_reason: None,
            },
        ];
        assert_eq!(
            select_stream_choice_index(None, false, &choices_zero),
            Some(0)
        );

        // If no payload has been emitted yet, switch to index=0 when it appears.
        assert_eq!(
            select_stream_choice_index(Some(2), false, &choices_zero),
            Some(0)
        );
        // Once payload has been emitted, keep stable selection to avoid mixed output.
        assert_eq!(
            select_stream_choice_index(Some(2), true, &choices_zero),
            Some(2)
        );
    }

    #[test]
    fn test_select_primary_choice_prefers_index_zero() {
        let choices = vec![
            Choice {
                index: 1,
                message: ChatMessage::assistant("secondary"),
                finish_reason: Some("stop".to_string()),
            },
            Choice {
                index: 0,
                message: ChatMessage::assistant("primary"),
                finish_reason: Some("stop".to_string()),
            },
        ];
        let selected = select_primary_choice(&choices).expect("expected choice");
        assert_eq!(selected.index, 0);
        assert_eq!(selected.message.content.as_deref(), Some("primary"));
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
