//! ChatGPT/Codex managed-auth Responses client.

use crate::openai_chat_completions::{
    OpenAiResponsesReasoning, OpenAiResponsesRequest, OpenAiResponsesResponse,
    OpenAiResponsesUsage, build_max_completion_tokens, build_reasoning_effort,
    convert_messages_for_openai_responses, convert_openai_responses_output,
    convert_tools_for_openai_chat_completions,
};
use crate::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk, ToolCallDelta};
use alan_auth::{ChatgptAuthError, ChatgptAuthManager};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::HashMap;
use tracing::{debug, instrument};

use crate::{SseEventParser, TokenUsage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamEventAction {
    Continue,
    Finish,
}

/// Client for the ChatGPT/Codex managed-auth Responses-compatible surface.
pub struct ChatgptResponsesClient {
    client: reqwest::Client,
    auth_manager: ChatgptAuthManager,
    base_url: String,
    model: String,
    custom_headers: HashMap<String, String>,
}

impl ChatgptResponsesClient {
    pub fn with_params(
        base_url: &str,
        model: &str,
        custom_headers: HashMap<String, String>,
    ) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            auth_manager: ChatgptAuthManager::detect()
                .context("Failed to initialize ChatGPT auth manager")?,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            custom_headers,
        })
    }

    fn clone_with_same_config(&self) -> Self {
        Self {
            client: self.client.clone(),
            auth_manager: self.auth_manager.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            custom_headers: self.custom_headers.clone(),
        }
    }

    fn build_openai_responses_request(
        &self,
        request: GenerationRequest,
        stream: bool,
    ) -> OpenAiResponsesRequest {
        let GenerationRequest {
            system_prompt,
            messages,
            tools,
            temperature,
            max_tokens,
            thinking_budget_tokens,
            mut extra_params,
        } = request;

        let (response_tools, tool_choice) = convert_tools_for_openai_chat_completions(tools);
        OpenAiResponsesRequest {
            model: self.model.clone(),
            input: convert_messages_for_openai_responses(system_prompt, messages),
            tools: response_tools,
            tool_choice,
            temperature,
            max_output_tokens: build_max_completion_tokens(max_tokens, &mut extra_params),
            reasoning: build_reasoning_effort(thinking_budget_tokens, &mut extra_params)
                .map(|effort| OpenAiResponsesReasoning { effort }),
            stream: Some(stream),
            extra_params,
        }
    }

    #[instrument(skip(self, request))]
    pub async fn openai_responses(
        &self,
        request: OpenAiResponsesRequest,
    ) -> Result<OpenAiResponsesResponse> {
        let response = self.execute_with_auth_retry(request, false).await?;
        response
            .json()
            .await
            .context("Failed to parse ChatGPT Responses API response")
    }

    #[instrument(skip(self, request, tx))]
    pub async fn stream_openai_responses(
        &self,
        request: OpenAiResponsesRequest,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        let response = self.execute_with_auth_retry(request, true).await?;

        let mut stream = response.bytes_stream();
        let mut parser = SseEventParser::new();
        let mut latest_usage: Option<TokenUsage> = None;
        let mut emitted_payload = false;
        let mut saw_tool_calls = false;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read ChatGPT Responses stream chunk")?;
            for data in parser.push(&chunk) {
                if self
                    .handle_stream_event(
                        &tx,
                        &data,
                        &mut latest_usage,
                        &mut emitted_payload,
                        &mut saw_tool_calls,
                    )
                    .await?
                    == StreamEventAction::Finish
                {
                    return Ok(());
                }
            }
        }

        for data in parser.finish() {
            if self
                .handle_stream_event(
                    &tx,
                    &data,
                    &mut latest_usage,
                    &mut emitted_payload,
                    &mut saw_tool_calls,
                )
                .await?
                == StreamEventAction::Finish
            {
                return Ok(());
            }
        }

        Ok(())
    }

    async fn handle_stream_event(
        &self,
        tx: &tokio::sync::mpsc::Sender<StreamChunk>,
        data: &str,
        latest_usage: &mut Option<TokenUsage>,
        emitted_payload: &mut bool,
        saw_tool_calls: &mut bool,
    ) -> Result<StreamEventAction> {
        if data == "[DONE]" {
            if *emitted_payload {
                let _ = tx
                    .send(StreamChunk {
                        text: None,
                        thinking: None,
                        thinking_signature: None,
                        redacted_thinking: None,
                        usage: *latest_usage,
                        tool_call_delta: None,
                        is_finished: true,
                        finish_reason: Some(responses_finish_reason(*saw_tool_calls).to_string()),
                    })
                    .await;
            }
            return Ok(StreamEventAction::Finish);
        }

        let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
            debug!(data, "Failed to parse ChatGPT Responses stream event");
            return Ok(StreamEventAction::Continue);
        };

        let Some(event_type) = event.get("type").and_then(serde_json::Value::as_str) else {
            return Ok(StreamEventAction::Continue);
        };

        match event_type {
            "response.output_text.delta" | "response.refusal.delta" => {
                if let Some(text) = event
                    .get("delta")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| is_non_empty(value))
                {
                    *emitted_payload = true;
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
                        return Ok(StreamEventAction::Finish);
                    }
                }
            }
            "response.reasoning_text.delta" | "response.reasoning_summary_text.delta" => {
                if let Some(thinking) = event
                    .get("delta")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| is_non_empty(value))
                {
                    *emitted_payload = true;
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
                        return Ok(StreamEventAction::Finish);
                    }
                }
            }
            "response.function_call_arguments.delta" => {
                let delta = event
                    .get("delta")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                if !delta.is_empty() {
                    *emitted_payload = true;
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
                        return Ok(StreamEventAction::Finish);
                    }
                }
            }
            "response.output_item.done" => {
                let Some(item) = event.get("item") else {
                    return Ok(StreamEventAction::Continue);
                };
                if item.get("type").and_then(serde_json::Value::as_str) != Some("function_call") {
                    return Ok(StreamEventAction::Continue);
                }

                let arguments = item
                    .get("arguments")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| is_non_empty(value));
                let name = responses_stream_tool_name(Some(item), &event);

                if let (Some(arguments), Some(name)) = (arguments, name) {
                    *emitted_payload = true;
                    *saw_tool_calls = true;
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
                        return Ok(StreamEventAction::Finish);
                    }
                }
            }
            "response.completed" => {
                if let Some(response) = event.get("response").cloned() {
                    match serde_json::from_value::<OpenAiResponsesResponse>(response) {
                        Ok(parsed) => {
                            *latest_usage = parsed.usage.map(convert_openai_responses_usage);
                            if !*saw_tool_calls {
                                *saw_tool_calls =
                                    responses_output_contains_tool_call(&parsed.output);
                            }
                        }
                        Err(error) => {
                            debug!(?error, "Failed to parse ChatGPT response.completed payload");
                        }
                    }
                }

                let _ = tx
                    .send(StreamChunk {
                        text: None,
                        thinking: None,
                        thinking_signature: None,
                        redacted_thinking: None,
                        usage: *latest_usage,
                        tool_call_delta: None,
                        is_finished: true,
                        finish_reason: Some(responses_finish_reason(*saw_tool_calls).to_string()),
                    })
                    .await;
                return Ok(StreamEventAction::Finish);
            }
            "response.failed" | "error" => {
                if *emitted_payload {
                    let _ = tx
                        .send(StreamChunk {
                            text: None,
                            thinking: None,
                            thinking_signature: None,
                            redacted_thinking: None,
                            usage: *latest_usage,
                            tool_call_delta: None,
                            is_finished: true,
                            finish_reason: Some("stream_error".to_string()),
                        })
                        .await;
                }
                return Ok(StreamEventAction::Finish);
            }
            _ => {}
        }

        Ok(StreamEventAction::Continue)
    }

    async fn execute_with_auth_retry(
        &self,
        request: OpenAiResponsesRequest,
        stream: bool,
    ) -> Result<reqwest::Response> {
        let response = self.send_request(&request, stream, false).await?;
        if response.status() != reqwest::StatusCode::UNAUTHORIZED {
            return check_chatgpt_response_status(response).await;
        }

        debug!("ChatGPT Responses request returned 401; attempting managed refresh");
        let retry = self.send_request(&request, stream, true).await?;
        check_chatgpt_response_status(retry).await
    }

    async fn send_request(
        &self,
        request: &OpenAiResponsesRequest,
        stream: bool,
        force_refresh: bool,
    ) -> Result<reqwest::Response> {
        let auth = if force_refresh {
            self.auth_manager.force_refresh_auth().await?
        } else {
            self.auth_manager.request_auth().await?
        };
        let mut builder = self
            .client
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", auth.access_token))
            .header("ChatGPT-Account-ID", auth.account_id)
            .json(request);

        for (name, value) in &self.custom_headers {
            builder = builder.header(name, value);
        }

        let response = builder
            .send()
            .await
            .context("Failed to send request to ChatGPT Responses API")?;

        if stream {
            debug!("Started ChatGPT streaming Responses request");
        }

        Ok(response)
    }
}

#[async_trait]
impl LlmProvider for ChatgptResponsesClient {
    async fn generate(&mut self, request: GenerationRequest) -> Result<GenerationResponse> {
        let response_request = self.build_openai_responses_request(request, false);
        let response = self.openai_responses(response_request).await?;
        Ok(convert_openai_responses_output(response))
    }

    async fn chat(&mut self, system: Option<&str>, user: &str) -> Result<String> {
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
    ) -> Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        let response_request = self.build_openai_responses_request(request, true);
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let client = self.clone_with_same_config();
        tokio::spawn(async move {
            let _ = client.stream_openai_responses(response_request, tx).await;
        });
        Ok(rx)
    }

    fn provider_name(&self) -> &'static str {
        "chatgpt"
    }
}

async fn check_chatgpt_response_status(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = response.text().await.unwrap_or_default();
        return Err(ChatgptAuthError::Unauthorized(body).into());
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("ChatGPT Responses API error ({}): {}", status, body);
    }
    Ok(response)
}

fn convert_openai_responses_usage(usage: OpenAiResponsesUsage) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage.input_tokens,
        completion_tokens: usage.output_tokens,
        total_tokens: usage.total_tokens,
        reasoning_tokens: usage
            .output_tokens_details
            .and_then(|details| details.reasoning_tokens),
    }
}

fn responses_finish_reason(saw_tool_calls: bool) -> &'static str {
    if saw_tool_calls { "tool_calls" } else { "stop" }
}

fn responses_output_contains_tool_call(output: &[serde_json::Value]) -> bool {
    output
        .iter()
        .any(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("function_call"))
}

fn responses_stream_index(event: &serde_json::Value) -> usize {
    event
        .get("output_index")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default() as usize
}

fn responses_stream_tool_id(
    item: Option<&serde_json::Value>,
    event: &serde_json::Value,
) -> Option<String> {
    item.and_then(|item| item.get("call_id"))
        .or_else(|| event.get("call_id"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn responses_stream_tool_name(
    item: Option<&serde_json::Value>,
    event: &serde_json::Value,
) -> Option<String> {
    item.and_then(|item| item.get("name"))
        .or_else(|| event.get("name"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::{ChatgptResponsesClient, StreamEventAction};
    use crate::factory::{ProviderConfig, ProviderType};
    use crate::{SseEventParser, StreamChunk};
    use std::collections::HashMap;

    #[test]
    fn provider_config_builds_chatgpt_client() {
        let config = ProviderConfig::chatgpt("gpt-5-codex")
            .with_base_url("https://chatgpt.com/backend-api/codex");
        assert_eq!(config.provider_type, ProviderType::ChatgptResponses);
    }

    #[test]
    fn client_requires_auth_manager_paths() {
        let client = ChatgptResponsesClient::with_params(
            "https://chatgpt.com/backend-api/codex",
            "gpt-5-codex",
            HashMap::new(),
        );
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn stream_finish_flushes_trailing_completed_event() {
        let client = ChatgptResponsesClient::with_params(
            "https://chatgpt.com/backend-api/codex",
            "gpt-5-codex",
            HashMap::new(),
        )
        .expect("client");
        let mut parser = SseEventParser::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamChunk>(4);
        let mut latest_usage = None;
        let mut emitted_payload = true;
        let mut saw_tool_calls = false;

        let completed_event = r#"data: {"type":"response.completed","response":{"id":"resp_123","output":[],"usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3}}}"#;
        assert!(parser.push(completed_event.as_bytes()).is_empty());

        for data in parser.finish() {
            let action = client
                .handle_stream_event(
                    &tx,
                    &data,
                    &mut latest_usage,
                    &mut emitted_payload,
                    &mut saw_tool_calls,
                )
                .await
                .expect("event");
            assert_eq!(action, StreamEventAction::Finish);
        }

        let final_chunk = rx.recv().await.expect("final chunk");
        assert!(final_chunk.is_finished);
        assert_eq!(final_chunk.finish_reason.as_deref(), Some("stop"));
        assert_eq!(final_chunk.usage.map(|usage| usage.total_tokens), Some(3));
    }
}
