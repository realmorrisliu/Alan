//! Unified LLM client abstraction
//!
//! This module provides a unified, trait-based interface for different LLM providers.
//! The design uses the `LlmProvider` trait from `alan_llm` crate, allowing for
//! easy mocking in tests.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │     LlmClient (wrapper with convenience)    │
//! └─────────────┬───────────────────────────────┘
//!               │ holds
//!               ▼
//! ┌─────────────────────────────────────────────┐
//! │      Box<dyn LlmProvider> (trait object)    │
//! └─────────────┬───────────────────────────────┘
//!               │ implements
//!     ┌─────────┼─────────┬──────────────┐
//!     ▼         ▼         ▼              ▼
//! ┌───────┐ ┌───────┐ ┌──────────┐ ┌─────────┐
//! │Gemini │ │OpenAI │ │Anthropic │ │  Mock   │
//! │Client │ │Client │ │  Client  │ │Provider │
//! └───────┘ └───────┘ └──────────┘ └─────────┘
//! ```

use anyhow::Result;

pub use alan_llm::{
    GenerationRequest, GenerationResponse, LlmProvider, Message, MessageRole, StreamChunk,
    TokenUsage, ToolCall, ToolDefinition,
};

pub use alan_llm::factory::{self, ProviderConfig, ProviderType};

/// Unified LLM client that wraps any provider implementing `LlmProvider`.
pub struct LlmClient {
    provider: Box<dyn LlmProvider>,
    provider_type: ProviderType,
}

impl LlmClient {
    /// Create a new LLM client from any provider implementing `LlmProvider`.
    pub fn new<P>(provider: P) -> Self
    where
        P: LlmProvider + 'static,
    {
        let provider_type = match provider.provider_name() {
            "gemini" => ProviderType::Gemini,
            "openai" => ProviderType::OpenAi,
            "anthropic" => ProviderType::Anthropic,
            _ => ProviderType::OpenAi, // Default fallback
        };

        Self {
            provider: Box::new(provider),
            provider_type,
        }
    }

    /// Create an LLM client from a provider configuration.
    pub fn from_config(config: ProviderConfig) -> Result<Self> {
        let provider_type = config.provider_type;
        let provider = factory::create_provider(config)?;
        Ok(Self {
            provider,
            provider_type,
        })
    }

    /// Create a client from core Config
    pub fn from_core_config(config: &crate::config::Config) -> Result<Self> {
        let provider_config = config.to_provider_config()?;
        Self::from_config(provider_config)
    }

    /// Generate a response using the underlying provider.
    pub async fn generate(&mut self, request: GenerationRequest) -> Result<GenerationResponse> {
        self.provider.generate(request).await
    }

    /// Simple chat interface.
    pub async fn chat(&mut self, system: Option<&str>, user: &str) -> Result<String> {
        self.provider.chat(system, user).await
    }

    /// Simple text generation without system prompt.
    /// Used for semantic matching and other internal tasks.
    pub async fn generate_simple(&mut self, prompt: &str) -> Result<String> {
        self.provider.chat(None, prompt).await
    }

    /// Generate with streaming support.
    pub async fn generate_stream(
        &mut self,
        request: GenerationRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        self.provider.generate_stream(request).await
    }

    /// Get the provider name.
    pub fn provider_name(&self) -> &'static str {
        self.provider.provider_name()
    }

    /// Check if this is a Gemini client.
    pub fn is_gemini(&self) -> bool {
        matches!(self.provider_type, ProviderType::Gemini)
    }

    /// Check if this is an OpenAI-compatible client.
    pub fn is_openai(&self) -> bool {
        matches!(self.provider_type, ProviderType::OpenAi)
    }

    /// Check if this is an Anthropic-compatible client.
    pub fn is_anthropic(&self) -> bool {
        matches!(self.provider_type, ProviderType::Anthropic)
    }
}

impl std::fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmClient")
            .field("provider_name", &self.provider_name())
            .field("provider_type", &self.provider_type)
            .finish()
    }
}

// ============================================================================
// Conversion Helpers
// ============================================================================

/// Convert session messages to LLM messages.
/// This is the `project_for_llm()` boundary — a lossy projection from the tape's
/// rich representation to the LLM provider's format.
pub fn convert_session_messages(messages: &[crate::session::Message]) -> Vec<Message> {
    use crate::tape;

    messages
        .iter()
        .map(|m| {
            let role = match m.role() {
                tape::MessageRole::System => MessageRole::System,
                tape::MessageRole::Context => MessageRole::Context,
                tape::MessageRole::User => MessageRole::User,
                tape::MessageRole::Assistant => MessageRole::Assistant,
                tape::MessageRole::Tool => MessageRole::Tool,
            };

            // Extract text content from parts
            let content = match m {
                tape::Message::Tool { responses } => {
                    // For tool messages, serialize the first response's content
                    responses
                        .first()
                        .map(|r| {
                            // If the response has structured data, serialize it
                            r.content
                                .iter()
                                .map(|part| match part {
                                    tape::ContentPart::Structured { data } => {
                                        serde_json::to_string(data)
                                            .unwrap_or_else(|_| "{}".to_string())
                                    }
                                    _ => part.as_text().unwrap_or("").to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join("")
                        })
                        .unwrap_or_default()
                }
                _ => m.non_thinking_text_content(),
            };

            // Extract thinking from assistant messages
            let thinking = m.thinking_content();

            // Extract tool calls from assistant messages
            let tool_calls = if !m.tool_requests().is_empty() {
                Some(
                    m.tool_requests()
                        .iter()
                        .map(|tc| ToolCall {
                            id: {
                                let trimmed = tc.id.trim();
                                if trimmed.is_empty() {
                                    None
                                } else {
                                    Some(trimmed.to_string())
                                }
                            },
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        })
                        .collect(),
                )
            } else {
                None
            };

            // Extract tool_call_id from tool responses
            let tool_call_id = match m {
                tape::Message::Tool { responses } => responses.first().and_then(|r| {
                    let trimmed = r.id.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }),
                _ => None,
            };

            Message {
                role,
                content,
                thinking,
                tool_calls,
                tool_call_id,
            }
        })
        .collect()
}

/// Build a generation request from session context.
pub fn build_generation_request(
    system_prompt: Option<String>,
    messages: Vec<Message>,
    tools: Vec<ToolDefinition>,
    temperature: Option<f32>,
    max_tokens: Option<i32>,
) -> GenerationRequest {
    GenerationRequest {
        system_prompt,
        messages,
        tools,
        temperature,
        max_tokens,
        thinking_budget_tokens: None,
        extra_params: std::collections::HashMap::new(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alan_llm::MockLlmProvider;

    #[tokio::test]
    async fn test_llm_client_with_mock() {
        let mock = MockLlmProvider::new().with_response(GenerationResponse {
            content: "Hello from mock".to_string(),
            thinking: None,
            tool_calls: vec![],
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        });

        let mut client = LlmClient::new(mock);

        assert_eq!(client.provider_name(), "mock");
        // Note: Mock provider is treated as OpenAi type by default since it doesn't match known providers
        assert!(!client.is_gemini());
        assert!(!client.is_anthropic());

        let request = GenerationRequest::new().with_user_message("Hi");

        let response = client.generate(request).await.unwrap();
        assert_eq!(response.content, "Hello from mock");
    }

    #[tokio::test]
    async fn test_llm_client_chat() {
        let mock = MockLlmProvider::new();
        let mut client = LlmClient::new(mock);

        let response = client.chat(Some("System"), "Hello").await.unwrap();
        assert!(response.contains("Mock response to:"));
    }

    #[tokio::test]
    async fn test_llm_client_stream() {
        let mock = MockLlmProvider::new().with_response(GenerationResponse {
            content: "Streamed content".to_string(),
            thinking: None,
            tool_calls: vec![],
            usage: None,
        });

        let mut client = LlmClient::new(mock);
        let mut rx = client
            .generate_stream(GenerationRequest::new())
            .await
            .unwrap();

        let chunk = rx.recv().await.unwrap();
        assert_eq!(chunk.text, Some("Streamed content".to_string()));
        assert!(chunk.is_finished);
    }

    #[test]
    fn test_convert_session_messages() {
        use crate::session::Message as SessionMessage;

        let session_messages = vec![
            SessionMessage::user("Hello"),
            SessionMessage::assistant("Hi there"),
        ];

        let llm_messages = convert_session_messages(&session_messages);

        assert_eq!(llm_messages.len(), 2);
        assert_eq!(llm_messages[0].role, MessageRole::User);
        assert_eq!(llm_messages[1].role, MessageRole::Assistant);
        assert_eq!(llm_messages[0].content, "Hello");
    }

    #[test]
    fn test_convert_session_messages_ignores_blank_tool_ids() {
        use crate::session::Message as SessionMessage;
        use crate::tape::ToolRequest;

        let session_messages = vec![
            SessionMessage::assistant_with_tools(
                "",
                vec![ToolRequest {
                    id: "   ".to_string(),
                    name: "web_search".to_string(),
                    arguments: serde_json::json!({"query": "test"}),
                }],
            ),
            SessionMessage::tool_text("   ", "{}"),
        ];

        let llm_messages = convert_session_messages(&session_messages);
        assert_eq!(llm_messages.len(), 2);
        assert_eq!(llm_messages[0].tool_calls.as_ref().unwrap()[0].id, None);
        assert_eq!(llm_messages[1].tool_call_id, None);
    }

    #[test]
    fn test_convert_session_messages_uses_tool_payload_for_tool_content() {
        use crate::session::Message as SessionMessage;

        let payload = serde_json::json!({
            "success": true,
            "company": "y-warm.com"
        });
        let session_messages = vec![SessionMessage::tool_structured(
            "tool_call_123",
            payload.clone(),
        )];

        let llm_messages = convert_session_messages(&session_messages);
        assert_eq!(llm_messages.len(), 1);
        assert_eq!(llm_messages[0].role, MessageRole::Tool);
        assert_eq!(
            llm_messages[0].tool_call_id.as_deref(),
            Some("tool_call_123")
        );
        assert_eq!(llm_messages[0].content, payload.to_string());
    }

    #[test]
    fn test_convert_session_messages_tool_without_payload_uses_content() {
        use crate::session::Message as SessionMessage;

        let session_messages = vec![SessionMessage::tool_text("tool_call_123", "{\"ok\":true}")];

        let llm_messages = convert_session_messages(&session_messages);
        assert_eq!(llm_messages.len(), 1);
        assert_eq!(llm_messages[0].content, "{\"ok\":true}");
    }

    #[test]
    fn test_build_generation_request() {
        let messages = vec![Message::user("Hello"), Message::assistant("Hi")];

        let request = build_generation_request(
            Some("System".to_string()),
            messages,
            vec![],
            Some(0.7),
            Some(1000),
        );

        assert_eq!(request.system_prompt, Some("System".to_string()));
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
    }
}
