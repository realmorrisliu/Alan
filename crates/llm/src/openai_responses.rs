//! OpenAI Responses API client.
//!
//! This module provides the provider surface for the OpenAI Responses API.

use anyhow::Result;
use async_trait::async_trait;

use crate::openai_chat_completions::{
    OpenAiChatCompletionsClient, OpenAiResponsesRequest, OpenAiResponsesResponse,
    convert_openai_responses_output,
};
use crate::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk};

#[cfg(test)]
use crate::openai_chat_completions::{
    OpenAiResponsesInputItem, OpenAiResponsesOutputTokensDetails, OpenAiResponsesUsage,
};

pub use crate::openai_chat_completions::{
    OpenAiResponsesFunctionCallItem as OpenAiResponsesFunctionCallEvent,
    OpenAiResponsesFunctionCallOutputItem as OpenAiResponsesFunctionCallOutputEvent,
    OpenAiResponsesInputItem as OpenAiResponsesRequestInputItem,
    OpenAiResponsesInputMessage as OpenAiResponsesRequestInputMessage,
    OpenAiResponsesOutputTokensDetails as OpenAiResponsesUsageOutputTokensDetails,
    OpenAiResponsesReasoning as OpenAiResponsesReasoningConfig,
    OpenAiResponsesRequest as OpenAiResponsesApiRequest,
    OpenAiResponsesResponse as OpenAiResponsesApiResponse,
    OpenAiResponsesUsage as OpenAiResponsesApiUsage,
};

/// Client for the OpenAI Responses API.
pub struct OpenAiResponsesClient {
    inner: OpenAiChatCompletionsClient,
}

impl OpenAiResponsesClient {
    pub fn with_params(api_key: &str, base_url: &str, model: &str) -> Self {
        Self {
            inner: OpenAiChatCompletionsClient::official_with_params(api_key, base_url, model),
        }
    }

    pub async fn openai_responses(
        &self,
        request: OpenAiResponsesRequest,
    ) -> Result<OpenAiResponsesResponse> {
        self.inner.openai_responses(request).await
    }

    pub async fn stream_openai_responses(
        &self,
        request: OpenAiResponsesRequest,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        self.inner.stream_openai_responses(request, tx).await
    }

    fn build_openai_responses_request(
        &self,
        request: GenerationRequest,
        stream: bool,
    ) -> OpenAiResponsesRequest {
        self.inner.build_openai_responses_request(request, stream)
    }
}

#[async_trait]
impl LlmProvider for OpenAiResponsesClient {
    async fn generate(&mut self, request: GenerationRequest) -> anyhow::Result<GenerationResponse> {
        let response_request = self.build_openai_responses_request(request, false);
        let response = self.openai_responses(response_request).await?;
        Ok(convert_openai_responses_output(response))
    }

    async fn chat(&mut self, system: Option<&str>, user: &str) -> anyhow::Result<String> {
        let request = match system {
            Some(system) => GenerationRequest::new()
                .with_system_prompt(system)
                .with_user_message(user),
            None => GenerationRequest::new().with_user_message(user),
        };
        let response = self.generate(request).await?;
        Ok(response.content)
    }

    async fn generate_stream(
        &mut self,
        request: GenerationRequest,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        let response_request = self.build_openai_responses_request(request, true);
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let client = Self {
            inner: self.inner.clone_with_same_config(),
        };
        tokio::spawn(async move {
            let _ = client.stream_openai_responses(response_request, tx).await;
        });
        Ok(rx)
    }

    fn provider_name(&self) -> &'static str {
        "openai_responses"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageRole;
    use serde_json::json;

    #[test]
    fn test_openai_responses_client_with_params() {
        let client =
            OpenAiResponsesClient::with_params("test-key", "https://api.openai.com/v1", "gpt-5.4");
        assert_eq!(client.provider_name(), "openai_responses");
        drop(client);
    }

    #[test]
    fn test_convert_messages_for_openai_responses_projects_tool_history() {
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

        let converted = crate::openai_chat_completions::convert_messages_for_openai_responses(
            Some("System prompt".to_string()),
            messages,
        );
        assert_eq!(converted.len(), 4);

        match &converted[0] {
            OpenAiResponsesInputItem::Message(message) => {
                assert_eq!(message.role, "system");
                assert_eq!(message.content, "System prompt");
            }
            _ => panic!("expected system message"),
        }

        match &converted[1] {
            OpenAiResponsesInputItem::Message(message) => {
                assert_eq!(message.role, "assistant");
                assert_eq!(message.content, "Let me inspect that.");
            }
            _ => panic!("expected assistant message"),
        }

        match &converted[2] {
            OpenAiResponsesInputItem::FunctionCall(tool_call) => {
                assert_eq!(tool_call.call_id, "call_1");
                assert_eq!(tool_call.name, "lookup");
                assert_eq!(tool_call.arguments, "{\"query\":\"alan\"}");
            }
            _ => panic!("expected function call"),
        }

        match &converted[3] {
            OpenAiResponsesInputItem::FunctionCallOutput(tool_output) => {
                assert_eq!(tool_output.call_id, "call_1");
                assert_eq!(tool_output.output, "{\"ok\":true}");
            }
            _ => panic!("expected function call output"),
        }
    }

    #[test]
    fn test_convert_openai_responses_output_extracts_final_tool_arguments() {
        let response = OpenAiResponsesResponse {
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
            usage: Some(OpenAiResponsesUsage {
                input_tokens: 11,
                output_tokens: 22,
                total_tokens: 33,
                output_tokens_details: Some(OpenAiResponsesOutputTokensDetails {
                    reasoning_tokens: Some(7),
                }),
            }),
        };

        let converted = convert_openai_responses_output(response);
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
}
