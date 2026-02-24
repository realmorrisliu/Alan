//! Gemini LLM client for Vertex AI.
//!
//! This module provides a minimal client for Google's Gemini models via Vertex AI REST API.
//! Authentication is handled via `gcloud auth print-access-token`.

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, error, instrument, warn};

/// Client for Vertex AI Gemini API
pub struct GeminiClient {
    /// HTTP client
    client: reqwest::Client,
    /// GCP Project ID
    project_id: String,
    /// GCP Location (e.g., us-central1)
    location: String,
    /// Model name (e.g., gemini-2.0-flash)
    model: String,
    /// Cached access token
    access_token: Option<String>,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request body for generateContent
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    /// Conversation contents
    pub contents: Vec<Content>,
    /// System instruction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    /// Tools for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// Content represents a message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Role: "user", "model", or "function"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Parts of the content (may be missing in some responses)
    #[serde(default)]
    pub parts: Vec<Part>,
}

/// Part of content - can be text, function call, or function response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    /// Text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Function call from model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    /// Function response to model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
}

/// Function call requested by the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name
    pub name: String,
    /// Arguments as JSON object
    pub args: serde_json::Value,
}

/// Response to a function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// Function name
    pub name: String,
    /// Response data
    pub response: serde_json::Value,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Function declarations
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// Function declaration schema
#[derive(Debug, Clone, Serialize)]
pub struct FunctionDeclaration {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Parameters JSON schema
    pub parameters: serde_json::Value,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Temperature (0.0-2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Max output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    /// Top-P sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-K sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response from generateContent
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    /// Generated candidates
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    /// Usage metadata
    pub usage_metadata: Option<UsageMetadata>,
    /// Model version
    pub model_version: Option<String>,
    /// Prompt feedback (e.g., when blocked by safety filters)
    pub prompt_feedback: Option<PromptFeedback>,
}

/// A generated candidate response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Content of the response
    pub content: Option<Content>,
    /// Why generation stopped
    pub finish_reason: Option<String>,
    /// Index of this candidate
    pub index: Option<i32>,
    /// Safety ratings for the generated content
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
}

/// Prompt feedback when content is blocked
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFeedback {
    /// Why the prompt was blocked
    pub block_reason: Option<String>,
    /// Safety ratings for the prompt
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
}

/// Safety rating for content
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyRating {
    /// Safety category
    pub category: String,
    /// Probability of harm
    pub probability: String,
}

/// Token usage metadata
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: Option<i32>,
    pub candidates_token_count: Option<i32>,
    pub total_token_count: Option<i32>,
}

// ============================================================================
// Client Implementation
// ============================================================================

impl GeminiClient {
    /// Create a client with explicit parameters
    pub fn with_params(project_id: &str, location: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            project_id: project_id.to_string(),
            location: location.to_string(),
            model: model.to_string(),
            access_token: None,
        }
    }

    /// Get access token via gcloud CLI
    fn get_access_token(&mut self) -> Result<String> {
        // Return cached token if available
        if let Some(ref token) = self.access_token {
            return Ok(token.clone());
        }

        debug!("Fetching access token via gcloud");

        let output = Command::new("gcloud")
            .args(["auth", "print-access-token"])
            .output()
            .context("Failed to run gcloud command. Is gcloud CLI installed and authenticated?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("gcloud auth failed: {}", stderr);
        }

        let token = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in access token")?
            .trim()
            .to_string();

        self.access_token = Some(token.clone());
        Ok(token)
    }

    /// Build the API endpoint URL
    fn endpoint(&self) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
            self.location, self.project_id, self.location, self.model
        )
    }

    /// Build the streaming API endpoint URL
    fn stream_endpoint(&self) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:streamGenerateContent",
            self.location, self.project_id, self.location, self.model
        )
    }

    /// Generate content (non-streaming)
    #[instrument(skip(self, request))]
    pub async fn generate_content(
        &mut self,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let token = self.get_access_token()?;
        let endpoint = self.endpoint();

        debug!(%endpoint, "Sending generateContent request");

        let response = self
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // Clear token on auth error to force refresh
            if status.as_u16() == 401 {
                warn!("Auth error, clearing cached token");
                self.access_token = None;
            }
            anyhow::bail!("Gemini API error ({}): {}", status, error_text);
        }

        // Get response text first for better error diagnostics
        let response_text = response
            .text()
            .await
            .context("Failed to read Gemini response body")?;

        // Try to parse as JSON
        let result: GenerateContentResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(
                    "Failed to parse Gemini response: {}\nResponse body: {}",
                    e,
                    &response_text[..response_text.len().min(2000)]
                );
                anyhow::anyhow!("Failed to parse Gemini response: {}", e)
            })?;

        Ok(result)
    }

    /// Generate content with streaming (SSE)
    #[instrument(skip(self, request, tx))]
    pub async fn stream_generate_content(
        &mut self,
        request: GenerateContentRequest,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        let token = self.get_access_token()?;
        let endpoint = self.stream_endpoint();

        debug!(%endpoint, "Sending streamGenerateContent request");

        let response = self
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Gemini API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                warn!("Auth error, clearing cached token");
                self.access_token = None;
            }
            anyhow::bail!("Gemini streaming API error ({}): {}", status, error_text);
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

                    match serde_json::from_str::<StreamChunk>(data) {
                        Ok(stream_chunk) => {
                            if tx.send(stream_chunk).await.is_err() {
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

    /// Simple chat helper - send message and get text response
    pub async fn chat(&mut self, user_message: &str) -> Result<String> {
        let request = GenerateContentRequest {
            contents: vec![Content {
                role: Some("user".to_string()),
                parts: vec![Part {
                    text: Some(user_message.to_string()),
                    function_call: None,
                    function_response: None,
                }],
            }],
            system_instruction: None,
            tools: None,
            generation_config: None,
        };

        let response = self.generate_content(request).await?;

        // Extract text from first candidate
        let text = response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        Ok(text)
    }

    /// Chat with system instruction
    pub async fn chat_with_system(&mut self, system: &str, user_message: &str) -> Result<String> {
        let request = GenerateContentRequest {
            contents: vec![Content {
                role: Some("user".to_string()),
                parts: vec![Part {
                    text: Some(user_message.to_string()),
                    function_call: None,
                    function_response: None,
                }],
            }],
            system_instruction: Some(Content {
                role: None,
                parts: vec![Part {
                    text: Some(system.to_string()),
                    function_call: None,
                    function_response: None,
                }],
            }),
            tools: None,
            generation_config: None,
        };

        let response = self.generate_content(request).await?;

        let text = response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        Ok(text)
    }
}

// ============================================================================
// Streaming Response Types
// ============================================================================

/// Stream chunk from Gemini streaming API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    /// Generated candidates
    #[serde(default)]
    pub candidates: Vec<StreamCandidate>,
    /// Usage metadata (only present in final chunk)
    pub usage_metadata: Option<UsageMetadata>,
}

/// A candidate in streaming response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamCandidate {
    /// Content of the response
    pub content: Option<Content>,
    /// Why generation stopped (only in final chunk)
    pub finish_reason: Option<String>,
    /// Index of this candidate
    pub index: Option<i32>,
}

// ============================================================================
// Helper functions
// ============================================================================

impl Part {
    /// Create a text part
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            function_call: None,
            function_response: None,
        }
    }

    /// Create a function call part
    pub fn function_call(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            text: None,
            function_call: Some(FunctionCall {
                name: name.into(),
                args,
            }),
            function_response: None,
        }
    }

    /// Create a function response part
    pub fn function_response(name: impl Into<String>, response: serde_json::Value) -> Self {
        Self {
            text: None,
            function_call: None,
            function_response: Some(FunctionResponse {
                name: name.into(),
                response,
            }),
        }
    }
}

impl Content {
    /// Create user content
    pub fn user(parts: Vec<Part>) -> Self {
        Self {
            role: Some("user".to_string()),
            parts,
        }
    }

    /// Create model content
    pub fn model(parts: Vec<Part>) -> Self {
        Self {
            role: Some("model".to_string()),
            parts,
        }
    }

    /// Create function response content
    pub fn function(parts: Vec<Part>) -> Self {
        Self {
            role: Some("function".to_string()),
            parts,
        }
    }
}

// ============================================================================
// LlmProvider Trait Implementation
// ============================================================================

use crate::{
    GenerationRequest, GenerationResponse, LlmProvider, MessageRole,
    StreamChunk as UnifiedStreamChunk, TokenUsage, ToolCall as LlmToolCall,
};

#[async_trait::async_trait]
impl LlmProvider for GeminiClient {
    async fn generate(&mut self, request: GenerationRequest) -> anyhow::Result<GenerationResponse> {
        // Convert messages to Gemini format
        let contents: Vec<Content> = request
            .messages
            .iter()
            .filter_map(|msg| match msg.role {
                MessageRole::User | MessageRole::Context => {
                    Some(Content::user(vec![Part::text(msg.content.clone())]))
                }
                MessageRole::Assistant => {
                    Some(Content::model(vec![Part::text(msg.content.clone())]))
                }
                MessageRole::Tool => {
                    let name = msg.tool_call_id.clone()?;
                    let payload = serde_json::from_str(&msg.content)
                        .unwrap_or_else(|_| serde_json::json!({"result": msg.content}));
                    Some(Content::function(vec![Part::function_response(
                        name, payload,
                    )]))
                }
                MessageRole::System => None, // System prompt handled separately
            })
            .collect();

        // Build tools
        let tools_payload = if request.tools.is_empty() {
            None
        } else {
            let declarations: Vec<FunctionDeclaration> = request
                .tools
                .into_iter()
                .map(|tool| FunctionDeclaration {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                })
                .collect();
            Some(vec![Tool {
                function_declarations: declarations,
            }])
        };

        // Build system instruction
        let system_instruction = request.system_prompt.map(|sys| Content {
            role: None,
            parts: vec![Part::text(sys)],
        });

        let gemini_request = GenerateContentRequest {
            contents,
            system_instruction,
            tools: tools_payload,
            generation_config: Some(GenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                top_p: None,
                top_k: None,
            }),
        };

        let response = self.generate_content(gemini_request).await?;

        // Check if prompt was blocked
        if let Some(feedback) = response.prompt_feedback
            && let Some(block_reason) = feedback.block_reason
        {
            anyhow::bail!("Content blocked by safety filter: {}", block_reason);
        }

        // Get first candidate
        let candidate = match response.candidates.first() {
            Some(c) => c,
            None => {
                return Ok(GenerationResponse {
                    content: String::new(),
                    tool_calls: vec![],
                    usage: None,
                });
            }
        };

        // Check finish reason
        if let Some(finish_reason) = &candidate.finish_reason {
            match finish_reason.as_str() {
                "SAFETY" => anyhow::bail!("Response blocked by safety filter"),
                "RECITATION" => anyhow::bail!("Response blocked due to recitation"),
                "OTHER" => anyhow::bail!("Response blocked for unknown reason"),
                _ => {}
            }
        }

        // Extract content
        let content = candidate
            .content
            .clone()
            .unwrap_or_else(|| Content::model(vec![]));

        let text = content
            .parts
            .iter()
            .filter_map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("");

        let tool_calls: Vec<LlmToolCall> = content
            .parts
            .iter()
            .filter_map(|p| {
                p.function_call.as_ref().map(|fc| LlmToolCall {
                    id: None,
                    name: fc.name.clone(),
                    arguments: fc.args.clone(),
                })
            })
            .collect();

        let usage = response.usage_metadata.map(|u| TokenUsage {
            prompt_tokens: u.prompt_token_count.unwrap_or(0),
            completion_tokens: u.candidates_token_count.unwrap_or(0),
            total_tokens: u.total_token_count.unwrap_or(0),
        });

        Ok(GenerationResponse {
            content: text,
            tool_calls,
            usage,
        })
    }

    async fn chat(&mut self, system: Option<&str>, user: &str) -> anyhow::Result<String> {
        if let Some(sys) = system {
            self.chat_with_system(sys, user).await
        } else {
            self.chat(user).await
        }
    }

    async fn generate_stream(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<UnifiedStreamChunk>> {
        // Convert messages to Gemini format
        let contents: Vec<Content> = request
            .messages
            .iter()
            .filter_map(|msg| match msg.role {
                MessageRole::User | MessageRole::Context => {
                    Some(Content::user(vec![Part::text(msg.content.clone())]))
                }
                MessageRole::Assistant => {
                    Some(Content::model(vec![Part::text(msg.content.clone())]))
                }
                MessageRole::Tool => {
                    let name = msg.tool_call_id.clone()?;
                    let payload = serde_json::from_str(&msg.content)
                        .unwrap_or_else(|_| serde_json::json!({"result": msg.content}));
                    Some(Content::function(vec![Part::function_response(
                        name, payload,
                    )]))
                }
                MessageRole::System => None,
            })
            .collect();

        // Build tools
        let tools_payload = if request.tools.is_empty() {
            None
        } else {
            let declarations: Vec<FunctionDeclaration> = request
                .tools
                .into_iter()
                .map(|tool| FunctionDeclaration {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                })
                .collect();
            Some(vec![Tool {
                function_declarations: declarations,
            }])
        };

        let system_instruction = request.system_prompt.map(|sys| Content {
            role: None,
            parts: vec![Part::text(sys)],
        });

        let gemini_request = GenerateContentRequest {
            contents,
            system_instruction,
            tools: tools_payload,
            generation_config: Some(GenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                top_p: None,
                top_k: None,
            }),
        };

        let (gemini_tx, mut gemini_rx) = tokio::sync::mpsc::channel(128);
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        // Start streaming
        let stream_result = self
            .stream_generate_content(gemini_request, gemini_tx)
            .await;

        if let Err(e) = stream_result {
            let _ = tx
                .send(UnifiedStreamChunk {
                    text: Some(format!("Error: {}", e)),
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("error".to_string()),
                })
                .await;
            return Ok(rx);
        }

        // Convert Gemini chunks to unified chunks
        tokio::spawn(async move {
            while let Some(gemini_chunk) = gemini_rx.recv().await {
                for candidate in gemini_chunk.candidates {
                    if let Some(content) = candidate.content {
                        for part in content.parts {
                            if let Some(text) = part.text {
                                let stream_chunk = UnifiedStreamChunk {
                                    text: Some(text),
                                    tool_call_delta: None,
                                    is_finished: false,
                                    finish_reason: None,
                                };
                                if tx.send(stream_chunk).await.is_err() {
                                    return;
                                }
                            }

                            if let Some(fc) = part.function_call {
                                let stream_chunk = UnifiedStreamChunk {
                                    text: None,
                                    tool_call_delta: Some(crate::ToolCallDelta {
                                        index: 0,
                                        id: None,
                                        name: Some(fc.name),
                                        arguments_delta: Some(fc.args.to_string()),
                                    }),
                                    is_finished: false,
                                    finish_reason: None,
                                };
                                if tx.send(stream_chunk).await.is_err() {
                                    return;
                                }
                            }
                        }

                        if candidate.finish_reason.is_some() {
                            let final_chunk = UnifiedStreamChunk {
                                text: None,
                                tool_call_delta: None,
                                is_finished: true,
                                finish_reason: candidate.finish_reason,
                            };
                            let _ = tx.send(final_chunk).await;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    fn provider_name(&self) -> &'static str {
        "gemini"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_client_with_params() {
        let client = GeminiClient::with_params("test-project", "us-central1", "gemini-2.0-flash");
        // Just verify it compiles and creates
        drop(client);
    }

    #[test]
    fn test_part_text() {
        let part = Part::text("Hello, world!");
        assert_eq!(part.text, Some("Hello, world!".to_string()));
        assert!(part.function_call.is_none());
        assert!(part.function_response.is_none());
    }

    #[test]
    fn test_part_function_call() {
        let args = serde_json::json!({"query": "test"});
        let part = Part::function_call("web_search", args.clone());
        assert!(part.text.is_none());
        assert!(part.function_call.is_some());
        let fc = part.function_call.unwrap();
        assert_eq!(fc.name, "web_search");
        assert_eq!(fc.args, args);
    }

    #[test]
    fn test_part_function_response() {
        let response = serde_json::json!({"result": "success"});
        let part = Part::function_response("my_tool", response.clone());
        assert!(part.text.is_none());
        assert!(part.function_response.is_some());
        let fr = part.function_response.unwrap();
        assert_eq!(fr.name, "my_tool");
        assert_eq!(fr.response, response);
    }

    #[test]
    fn test_content_user() {
        let parts = vec![Part::text("Hello")];
        let content = Content::user(parts);
        assert_eq!(content.role, Some("user".to_string()));
        assert_eq!(content.parts.len(), 1);
    }

    #[test]
    fn test_content_model() {
        let parts = vec![Part::text("Response")];
        let content = Content::model(parts);
        assert_eq!(content.role, Some("model".to_string()));
        assert_eq!(content.parts.len(), 1);
    }

    #[test]
    fn test_content_function() {
        let parts = vec![Part::function_response("tool", serde_json::json!({}))];
        let content = Content::function(parts);
        assert_eq!(content.role, Some("function".to_string()));
        assert_eq!(content.parts.len(), 1);
    }

    #[test]
    fn test_generation_config_default() {
        let config = GenerationConfig::default();
        assert!(config.temperature.is_none());
        assert!(config.max_output_tokens.is_none());
        assert!(config.top_p.is_none());
        assert!(config.top_k.is_none());
    }

    #[test]
    fn test_generate_content_request_serialization() {
        let request = GenerateContentRequest {
            contents: vec![Content::user(vec![Part::text("Hello")])],
            system_instruction: None,
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: Some(100),
                top_p: None,
                top_k: None,
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("contents"));
        assert!(json.contains("generationConfig"));
        assert!(json.contains("0.7"));
    }

    #[test]
    fn test_generate_content_response_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello!"}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }"#;

        let response: GenerateContentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert!(response.usage_metadata.is_some());
        let usage = response.usage_metadata.unwrap();
        assert_eq!(usage.prompt_token_count, Some(10));
        assert_eq!(usage.total_token_count, Some(15));
    }

    #[test]
    fn test_safety_rating() {
        let json = r#"{
            "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
            "probability": "NEGLIGIBLE"
        }"#;

        let rating: SafetyRating = serde_json::from_str(json).unwrap();
        assert_eq!(rating.category, "HARM_CATEGORY_SEXUALLY_EXPLICIT");
        assert_eq!(rating.probability, "NEGLIGIBLE");
    }

    #[test]
    fn test_prompt_feedback_deserialization() {
        let json = r#"{
            "blockReason": "SAFETY",
            "safetyRatings": [
                {"category": "HARM_CATEGORY_HARASSMENT", "probability": "HIGH"}
            ]
        }"#;

        let feedback: PromptFeedback = serde_json::from_str(json).unwrap();
        assert_eq!(feedback.block_reason, Some("SAFETY".to_string()));
        assert_eq!(feedback.safety_ratings.len(), 1);
    }

    #[test]
    fn test_function_declaration() {
        let decl = FunctionDeclaration {
            name: "test_function".to_string(),
            description: "A test function".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "arg1": {"type": "string"}
                }
            }),
        };

        assert_eq!(decl.name, "test_function");
        assert_eq!(decl.description, "A test function");
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            function_declarations: vec![FunctionDeclaration {
                name: "func1".to_string(),
                description: "Function 1".to_string(),
                parameters: serde_json::json!({}),
            }],
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("functionDeclarations"));
    }

    #[test]
    fn test_stream_chunk_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                },
                "finishReason": null,
                "index": 0
            }],
            "usageMetadata": null
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.candidates.len(), 1);
        assert!(chunk.usage_metadata.is_none());
    }
}
