//! OpenRouter SDK-backed chat adapter.

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures::StreamExt;
use openrouter_rs::{
    OpenRouterClient as OpenRouterSdkClient,
    api::chat::{
        ChatCompletionRequest, ChatCompletionRequestBuilder, DebugOptions, Message as OrMessage,
        Modality, Plugin, StopSequence, StreamOptions, TraceOptions,
    },
    types::{
        Effort, FinishReason, FunctionCall as OrFunctionCall, ProviderPreferences, ReasoningConfig,
        ResponseFormat, ResponseUsage, Role, Tool as OrTool, ToolCall as OrToolCall,
        UnifiedStreamEvent, completion::CompletionsResponse,
    },
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;

use crate::{
    GenerationRequest, GenerationResponse, LlmProvider, Message, MessageRole, StreamChunk,
    TokenUsage, ToolCall, ToolCallDelta, ToolDefinition,
};

pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Alan LLM provider backed by the `openrouter-rs` SDK.
#[derive(Clone)]
pub struct OpenRouterClient {
    client: OpenRouterSdkClient,
    model: String,
}

#[derive(Debug, Clone, Default)]
pub struct OpenRouterClientMetadata {
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
    pub app_categories: Option<Vec<String>>,
}

impl OpenRouterClient {
    pub fn with_params(api_key: &str, base_url: &str, model: &str) -> Result<Self> {
        Self::with_metadata(
            api_key,
            base_url,
            model,
            OpenRouterClientMetadata::default(),
        )
    }

    pub fn with_metadata(
        api_key: &str,
        base_url: &str,
        model: &str,
        metadata: OpenRouterClientMetadata,
    ) -> Result<Self> {
        let mut builder = OpenRouterSdkClient::builder();
        builder.api_key(api_key).base_url(base_url);
        if let Some(http_referer) = metadata.http_referer {
            builder.http_referer(http_referer);
        }
        if let Some(x_title) = metadata.x_title {
            builder.x_title(x_title);
        }
        if let Some(app_categories) = metadata.app_categories {
            builder.app_categories(app_categories);
        }

        Ok(Self {
            client: builder
                .build()
                .context("failed to build OpenRouter client")?,
            model: model.to_string(),
        })
    }

    async fn generate_chat(&self, request: GenerationRequest) -> Result<GenerationResponse> {
        let chat_request = build_openrouter_chat_request(&self.model, request)?;
        let response = self
            .client
            .chat()
            .create(&chat_request)
            .await
            .context("OpenRouter chat completion request failed")?;
        Ok(convert_openrouter_response(response))
    }

    async fn generate_chat_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<mpsc::Receiver<StreamChunk>> {
        let chat_request = build_openrouter_chat_request(&self.model, request)?;
        let (tx, rx) = mpsc::channel(100);
        let client = self.client.clone();

        tokio::spawn(async move {
            let stream = match client.chat().stream_unified(&chat_request).await {
                Ok(stream) => stream,
                Err(error) => {
                    debug!(?error, "OpenRouter stream failed before yielding output");
                    return;
                }
            };

            consume_openrouter_stream(stream, tx).await;
        });

        Ok(rx)
    }
}

#[async_trait]
impl LlmProvider for OpenRouterClient {
    async fn generate(&mut self, request: GenerationRequest) -> Result<GenerationResponse> {
        self.generate_chat(request).await
    }

    async fn chat(&mut self, system: Option<&str>, user: &str) -> Result<String> {
        let mut request = GenerationRequest::new().with_user_message(user);
        if let Some(system) = system {
            request = request.with_system_prompt(system);
        }
        Ok(self.generate_chat(request).await?.content)
    }

    async fn generate_stream(
        &mut self,
        request: GenerationRequest,
    ) -> Result<mpsc::Receiver<StreamChunk>> {
        self.generate_chat_stream(request).await
    }

    fn provider_name(&self) -> &'static str {
        "openrouter"
    }
}

pub(crate) fn build_openrouter_chat_request(
    model: &str,
    request: GenerationRequest,
) -> Result<ChatCompletionRequest> {
    let GenerationRequest {
        system_prompt,
        messages,
        tools,
        temperature,
        max_tokens,
        thinking_budget_tokens,
        extra_params,
    } = request;

    let mut projected_messages = Vec::new();
    if let Some(system_prompt) = system_prompt.filter(|value| !value.is_empty()) {
        projected_messages.push(OrMessage::new(Role::System, system_prompt));
    }
    projected_messages.extend(convert_messages_for_openrouter(messages));

    let mut builder = ChatCompletionRequest::builder();
    builder.model(model).messages(projected_messages);

    if let Some(temperature) = temperature {
        builder.temperature(f64::from(temperature));
    }
    if let Some(max_tokens) = max_tokens {
        builder.max_tokens(non_negative_u32("max_tokens", max_tokens)?);
    }
    if let Some(thinking_budget_tokens) = thinking_budget_tokens {
        builder.reasoning(ReasoningConfig::with_max_tokens(thinking_budget_tokens));
    }

    let tools = convert_tools_for_openrouter(tools);
    if !tools.is_empty() {
        builder.tools(tools);
        builder.tool_choice_auto();
    }

    apply_openrouter_extra_params(&mut builder, extra_params)?;
    builder
        .build()
        .context("failed to build OpenRouter chat completion request")
}

pub(crate) fn convert_messages_for_openrouter(messages: Vec<Message>) -> Vec<OrMessage> {
    messages
        .into_iter()
        .map(|message| {
            let Message {
                role,
                content,
                thinking: _,
                thinking_signature: _,
                redacted_thinking: _,
                tool_calls,
                tool_call_id,
            } = message;

            match role {
                MessageRole::System => OrMessage::new(Role::System, content),
                MessageRole::User => OrMessage::new(Role::User, content),
                MessageRole::Assistant => {
                    let tool_calls = tool_calls.map(convert_tool_calls_for_openrouter);
                    match tool_calls {
                        Some(tool_calls) if !tool_calls.is_empty() => {
                            OrMessage::assistant_with_tool_calls(content, tool_calls)
                        }
                        _ => OrMessage::new(Role::Assistant, content),
                    }
                }
                MessageRole::Tool => {
                    let tool_call_id = tool_call_id.unwrap_or_default();
                    OrMessage::tool_response(&tool_call_id, content)
                }
                MessageRole::Context => OrMessage::new(Role::System, content),
            }
        })
        .collect()
}

pub(crate) fn convert_tools_for_openrouter(tools: Vec<ToolDefinition>) -> Vec<OrTool> {
    tools
        .into_iter()
        .map(|tool| OrTool::new(&tool.name, &tool.description, tool.parameters))
        .collect()
}

pub(crate) fn convert_openrouter_response(response: CompletionsResponse) -> GenerationResponse {
    let mut content = String::new();
    let mut thinking = String::new();
    let mut tool_calls = Vec::new();
    let mut warnings = Vec::new();
    let mut finish_reason = None;

    for choice in &response.choices {
        if let Some(choice_content) = choice.content() {
            content.push_str(choice_content);
        }
        if let Some(choice_reasoning) = choice.reasoning() {
            thinking.push_str(choice_reasoning);
        } else if let Some(details) = choice.reasoning_details() {
            for detail in details {
                if let Some(part) = detail.content() {
                    thinking.push_str(part);
                }
            }
        }
        if let Some(reason) = choice.finish_reason() {
            finish_reason = Some(finish_reason_to_string(reason).to_string());
        }
        if let Some(calls) = choice.tool_calls() {
            for call in calls {
                match convert_openrouter_tool_call(call) {
                    Ok(call) => tool_calls.push(call),
                    Err(warning) => warnings.push(warning),
                }
            }
        }
    }

    GenerationResponse {
        content,
        thinking: (!thinking.is_empty()).then_some(thinking),
        thinking_signature: first_reasoning_signature(&response),
        redacted_thinking: Vec::new(),
        tool_calls,
        usage: response.usage.map(convert_usage),
        finish_reason,
        provider_response_id: Some(response.id),
        provider_response_status: None,
        warnings,
    }
}

async fn consume_openrouter_stream(
    mut stream: openrouter_rs::types::UnifiedStream,
    tx: mpsc::Sender<StreamChunk>,
) {
    let mut emitted_payload = false;
    let mut latest_usage = None;
    let mut latest_response_id = None;
    let mut latest_finish_reason = None;

    while let Some(event) = stream.next().await {
        match event {
            UnifiedStreamEvent::ContentDelta(text) => {
                if !text.is_empty() {
                    emitted_payload = true;
                    let _ = tx.send(stream_text_chunk(text)).await;
                }
            }
            UnifiedStreamEvent::ReasoningDelta(thinking) => {
                if !thinking.is_empty() {
                    emitted_payload = true;
                    let _ = tx.send(stream_thinking_chunk(thinking)).await;
                }
            }
            UnifiedStreamEvent::ReasoningDetailsDelta(details) => {
                for detail in details {
                    if let Some(thinking) = detail.content().filter(|value| !value.is_empty()) {
                        emitted_payload = true;
                        let _ = tx.send(stream_thinking_chunk(thinking.to_string())).await;
                    }
                    if let Some(signature) = detail.signature.filter(|value| !value.is_empty()) {
                        emitted_payload = true;
                        let _ = tx.send(stream_thinking_signature_chunk(signature)).await;
                    }
                }
            }
            UnifiedStreamEvent::ToolDelta(value) => {
                if let Some(delta) = tool_delta_from_value(value) {
                    emitted_payload = true;
                    let _ = tx.send(stream_tool_chunk(delta)).await;
                }
            }
            UnifiedStreamEvent::Done {
                id,
                finish_reason,
                usage,
                ..
            } => {
                latest_response_id = id;
                latest_finish_reason = finish_reason;
                latest_usage = usage.and_then(usage_from_value);
                break;
            }
            UnifiedStreamEvent::Error(error) => {
                debug!(?error, "OpenRouter stream failed");
                if emitted_payload {
                    latest_finish_reason = Some("stream_error".to_string());
                    break;
                }
                return;
            }
            UnifiedStreamEvent::Raw { .. } => {}
        }
    }

    let _ = tx
        .send(StreamChunk {
            text: None,
            thinking: None,
            thinking_signature: None,
            redacted_thinking: None,
            usage: latest_usage,
            provider_response_id: latest_response_id,
            provider_response_status: None,
            sequence_number: None,
            tool_call_delta: None,
            is_finished: true,
            finish_reason: latest_finish_reason,
        })
        .await;
}

fn apply_openrouter_extra_params(
    builder: &mut ChatCompletionRequestBuilder,
    extra_params: HashMap<String, Value>,
) -> Result<()> {
    for (key, value) in extra_params {
        match key.as_str() {
            "max_completion_tokens" => builder.max_completion_tokens(value_as_u32(&key, value)?),
            "seed" => builder.seed(value_as_u32(&key, value)?),
            "top_p" => builder.top_p(value_as_f64(&key, value)?),
            "top_k" => builder.top_k(value_as_u32(&key, value)?),
            "frequency_penalty" => builder.frequency_penalty(value_as_f64(&key, value)?),
            "presence_penalty" => builder.presence_penalty(value_as_f64(&key, value)?),
            "repetition_penalty" => builder.repetition_penalty(value_as_f64(&key, value)?),
            "logit_bias" => builder.logit_bias(value_as::<HashMap<String, f64>>(&key, value)?),
            "logprobs" => builder.logprobs(value_as_bool(&key, value)?),
            "top_logprobs" => builder.top_logprobs(value_as_u32(&key, value)?),
            "min_p" => builder.min_p(value_as_f64(&key, value)?),
            "top_a" => builder.top_a(value_as_f64(&key, value)?),
            "transforms" => builder.transforms(value_as::<Vec<String>>(&key, value)?),
            "models" => builder.models(value_as::<Vec<String>>(&key, value)?),
            "route" => builder.route(value_as_string(&key, value)?),
            "user" => builder.user(value_as_string(&key, value)?),
            "session_id" => builder.session_id(value_as_string(&key, value)?),
            "trace" => builder.trace(value_as::<TraceOptions>(&key, value)?),
            "provider" => builder.provider(value_as::<ProviderPreferences>(&key, value)?),
            "metadata" => builder.metadata(value_as::<HashMap<String, String>>(&key, value)?),
            "plugins" => builder.plugins(value_as::<Vec<Plugin>>(&key, value)?),
            "modalities" => builder.modalities(value_as::<Vec<Modality>>(&key, value)?),
            "image_config" => {
                builder.image_config(value_as::<HashMap<String, Value>>(&key, value)?)
            }
            "response_format" => builder.response_format(value_as::<ResponseFormat>(&key, value)?),
            "reasoning" => builder.reasoning(value_as::<ReasoningConfig>(&key, value)?),
            "reasoning_effort" => builder.reasoning_effort(effort_from_value(&key, value)?),
            "reasoning_max_tokens" => builder.reasoning_max_tokens(value_as_u32(&key, value)?),
            "include_reasoning" => builder.include_reasoning(value_as_bool(&key, value)?),
            "stop" => builder.stop(value_as::<StopSequence>(&key, value)?),
            "stream_options" => builder.stream_options(value_as::<StreamOptions>(&key, value)?),
            "debug" => builder.debug(value_as::<DebugOptions>(&key, value)?),
            "parallel_tool_calls" => builder.parallel_tool_calls(value_as_bool(&key, value)?),
            unsupported => {
                bail!("OpenRouter provider does not support extra parameter `{unsupported}`");
            }
        };
    }
    Ok(())
}

fn convert_tool_calls_for_openrouter(tool_calls: Vec<ToolCall>) -> Vec<OrToolCall> {
    tool_calls
        .into_iter()
        .map(|call| OrToolCall {
            id: call.id.unwrap_or_default(),
            type_: "function".to_string(),
            function: OrFunctionCall {
                name: call.name,
                arguments: call.arguments.to_string(),
            },
            index: None,
        })
        .collect()
}

fn convert_openrouter_tool_call(call: &OrToolCall) -> std::result::Result<ToolCall, String> {
    let arguments = match serde_json::from_str::<Value>(call.arguments_json()) {
        Ok(arguments) => arguments,
        Err(_) => {
            return Err(format!(
                "Dropped malformed OpenRouter tool call `{}` arguments.",
                call.name()
            ));
        }
    };

    Ok(ToolCall {
        id: Some(call.id().to_string()),
        name: call.name().to_string(),
        arguments,
    })
}

fn tool_delta_from_value(value: Value) -> Option<ToolCallDelta> {
    let partial: openrouter_rs::types::PartialToolCall = serde_json::from_value(value).ok()?;
    let function = partial.function;
    Some(ToolCallDelta {
        index: partial.index.unwrap_or(0) as usize,
        id: partial.id,
        name: function.as_ref().and_then(|function| function.name.clone()),
        arguments_delta: function.and_then(|function| function.arguments),
        arguments: None,
    })
}

fn first_reasoning_signature(response: &CompletionsResponse) -> Option<String> {
    response
        .choices
        .iter()
        .filter_map(|choice| choice.reasoning_details())
        .flat_map(|details| details.iter())
        .filter_map(|detail| detail.signature.clone())
        .find(|value| !value.is_empty())
}

fn finish_reason_to_string(reason: &FinishReason) -> &'static str {
    match reason {
        FinishReason::ToolCalls => "tool_calls",
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::ContentFilter => "content_filter",
        FinishReason::Error => "error",
    }
}

fn convert_usage(usage: ResponseUsage) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage.prompt_tokens.min(i32::MAX as u32) as i32,
        cached_prompt_tokens: None,
        completion_tokens: usage.completion_tokens.min(i32::MAX as u32) as i32,
        total_tokens: usage.total_tokens.min(i32::MAX as u32) as i32,
        reasoning_tokens: None,
    }
}

fn usage_from_value(value: Value) -> Option<TokenUsage> {
    serde_json::from_value::<ResponseUsage>(value)
        .ok()
        .map(convert_usage)
}

fn stream_text_chunk(text: String) -> StreamChunk {
    StreamChunk {
        text: Some(text),
        thinking: None,
        thinking_signature: None,
        redacted_thinking: None,
        usage: None,
        provider_response_id: None,
        provider_response_status: None,
        sequence_number: None,
        tool_call_delta: None,
        is_finished: false,
        finish_reason: None,
    }
}

fn stream_thinking_chunk(thinking: String) -> StreamChunk {
    StreamChunk {
        text: None,
        thinking: Some(thinking),
        thinking_signature: None,
        redacted_thinking: None,
        usage: None,
        provider_response_id: None,
        provider_response_status: None,
        sequence_number: None,
        tool_call_delta: None,
        is_finished: false,
        finish_reason: None,
    }
}

fn stream_thinking_signature_chunk(thinking_signature: String) -> StreamChunk {
    StreamChunk {
        text: None,
        thinking: None,
        thinking_signature: Some(thinking_signature),
        redacted_thinking: None,
        usage: None,
        provider_response_id: None,
        provider_response_status: None,
        sequence_number: None,
        tool_call_delta: None,
        is_finished: false,
        finish_reason: None,
    }
}

fn stream_tool_chunk(tool_call_delta: ToolCallDelta) -> StreamChunk {
    StreamChunk {
        text: None,
        thinking: None,
        thinking_signature: None,
        redacted_thinking: None,
        usage: None,
        provider_response_id: None,
        provider_response_status: None,
        sequence_number: None,
        tool_call_delta: Some(tool_call_delta),
        is_finished: false,
        finish_reason: None,
    }
}

fn non_negative_u32(label: &str, value: i32) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("`{label}` must be a non-negative integer"))
}

fn value_as<T>(key: &str, value: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).with_context(|| {
        format!("OpenRouter extra parameter `{key}` has an unsupported value shape")
    })
}

fn value_as_string(key: &str, value: Value) -> Result<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("OpenRouter extra parameter `{key}` must be a string"))
}

fn value_as_bool(key: &str, value: Value) -> Result<bool> {
    value
        .as_bool()
        .ok_or_else(|| anyhow::anyhow!("OpenRouter extra parameter `{key}` must be a boolean"))
}

fn value_as_u32(key: &str, value: Value) -> Result<u32> {
    match value.as_u64().and_then(|value| u32::try_from(value).ok()) {
        Some(value) => Ok(value),
        None => bail!("OpenRouter extra parameter `{key}` must be an unsigned 32-bit integer"),
    }
}

fn value_as_f64(key: &str, value: Value) -> Result<f64> {
    value
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("OpenRouter extra parameter `{key}` must be a number"))
}

fn effort_from_value(key: &str, value: Value) -> Result<Effort> {
    let raw = value_as_string(key, value)?;
    match raw.as_str() {
        "xhigh" => Ok(Effort::Xhigh),
        "high" => Ok(Effort::High),
        "medium" => Ok(Effort::Medium),
        "low" => Ok(Effort::Low),
        "minimal" => Ok(Effort::Minimal),
        "none" => Ok(Effort::None),
        _ => bail!("OpenRouter extra parameter `{key}` has unsupported reasoning effort `{raw}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openrouter_rs::types::{
        Choice, NonStreamingChoice, ObjectType, ReasoningDetail,
        completion::Message as OrResponseMessage,
    };
    use serde_json::json;

    #[test]
    fn projects_messages_tools_and_reasoning_budget() {
        let mut request = GenerationRequest::new()
            .with_system_prompt("system")
            .with_user_message("hello")
            .with_assistant_message("thinking done")
            .with_tool(ToolDefinition::new("lookup", "Lookup data"))
            .with_thinking_budget_tokens(512);
        request.messages.push(Message {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
            thinking_signature: None,
            redacted_thinking: None,
            tool_calls: Some(vec![
                ToolCall::new("lookup", json!({"q":"rust"})).with_id("call-1"),
            ]),
            tool_call_id: None,
        });
        request
            .messages
            .push(Message::tool("call-1", "tool result"));

        let projected = build_openrouter_chat_request("openrouter/model", request).unwrap();
        let value = serde_json::to_value(projected).unwrap();

        assert_eq!(value["model"], "openrouter/model");
        assert_eq!(value["messages"][0]["role"], "system");
        assert_eq!(value["messages"][1]["role"], "user");
        assert_eq!(value["messages"][3]["tool_calls"][0]["id"], "call-1");
        assert_eq!(value["messages"][4]["role"], "tool");
        assert_eq!(value["messages"][4]["tool_call_id"], "call-1");
        assert_eq!(value["tools"][0]["function"]["name"], "lookup");
        assert_eq!(value["tool_choice"], "auto");
        assert_eq!(value["reasoning"]["max_tokens"], 512);
    }

    #[test]
    fn unsupported_extra_parameter_fails_projection() {
        let mut request = GenerationRequest::new().with_user_message("hello");
        request
            .extra_params
            .insert("unsupported".to_string(), json!(true));

        let error = build_openrouter_chat_request("openrouter/model", request).unwrap_err();
        assert!(error.to_string().contains("unsupported"));
    }

    #[test]
    fn supported_extra_parameters_are_projected() {
        let mut request = GenerationRequest::new().with_user_message("hello");
        request
            .extra_params
            .insert("route".to_string(), json!("fallback"));
        request.extra_params.insert(
            "provider".to_string(),
            json!({ "allow_fallbacks": false, "require_parameters": true }),
        );
        request
            .extra_params
            .insert("transforms".to_string(), json!(["middle-out"]));
        request
            .extra_params
            .insert("reasoning_effort".to_string(), json!("high"));

        let projected = build_openrouter_chat_request("openrouter/model", request).unwrap();
        let value = serde_json::to_value(projected).unwrap();

        assert_eq!(value["route"], "fallback");
        assert_eq!(value["provider"]["allow_fallbacks"], false);
        assert_eq!(value["transforms"][0], "middle-out");
        assert_eq!(value["reasoning"]["effort"], "high");
    }

    #[test]
    fn maps_content_reasoning_usage_finish_and_response_id() {
        let response = response_with_choice(Choice::NonStreaming(NonStreamingChoice {
            finish_reason: Some(FinishReason::Stop),
            native_finish_reason: None,
            message: OrResponseMessage {
                content: Some("answer".to_string()),
                role: Some("assistant".to_string()),
                name: None,
                tool_calls: None,
                reasoning: Some("because".to_string()),
                reasoning_details: None,
                images: None,
                audio: None,
                refusal: None,
                annotations: None,
            },
            error: None,
            index: Some(0),
            logprobs: None,
        }));

        let converted = convert_openrouter_response(response);
        assert_eq!(converted.content, "answer");
        assert_eq!(converted.thinking.as_deref(), Some("because"));
        assert_eq!(converted.finish_reason.as_deref(), Some("stop"));
        assert_eq!(converted.provider_response_id.as_deref(), Some("resp-1"));
        assert_eq!(converted.usage.unwrap().total_tokens, 7);
    }

    #[test]
    fn maps_tool_call_and_drops_malformed_arguments() {
        let response = response_with_choice(Choice::NonStreaming(NonStreamingChoice {
            finish_reason: Some(FinishReason::ToolCalls),
            native_finish_reason: None,
            message: OrResponseMessage {
                content: Some(String::new()),
                role: Some("assistant".to_string()),
                name: None,
                tool_calls: Some(vec![
                    OrToolCall {
                        id: "call-ok".to_string(),
                        type_: "function".to_string(),
                        function: OrFunctionCall {
                            name: "lookup".to_string(),
                            arguments: "{\"q\":\"rust\"}".to_string(),
                        },
                        index: Some(0),
                    },
                    OrToolCall {
                        id: "call-bad".to_string(),
                        type_: "function".to_string(),
                        function: OrFunctionCall {
                            name: "broken".to_string(),
                            arguments: "{bad".to_string(),
                        },
                        index: Some(1),
                    },
                ]),
                reasoning: None,
                reasoning_details: None,
                images: None,
                audio: None,
                refusal: None,
                annotations: None,
            },
            error: None,
            index: Some(0),
            logprobs: None,
        }));

        let converted = convert_openrouter_response(response);
        assert_eq!(converted.tool_calls.len(), 1);
        assert_eq!(converted.tool_calls[0].id.as_deref(), Some("call-ok"));
        assert_eq!(converted.tool_calls[0].arguments["q"], "rust");
        assert_eq!(converted.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(converted.warnings.len(), 1);
        assert!(converted.warnings[0].contains("broken"));
    }

    #[test]
    fn maps_reasoning_detail_signature() {
        let response = response_with_choice(Choice::NonStreaming(NonStreamingChoice {
            finish_reason: Some(FinishReason::Stop),
            native_finish_reason: None,
            message: OrResponseMessage {
                content: Some("answer".to_string()),
                role: Some("assistant".to_string()),
                name: None,
                tool_calls: None,
                reasoning: None,
                reasoning_details: Some(vec![ReasoningDetail {
                    block_type: "reasoning.text".to_string(),
                    text: Some("detail".to_string()),
                    data: None,
                    signature: Some("sig".to_string()),
                    format: None,
                    id: None,
                    index: None,
                }]),
                images: None,
                audio: None,
                refusal: None,
                annotations: None,
            },
            error: None,
            index: Some(0),
            logprobs: None,
        }));

        let converted = convert_openrouter_response(response);
        assert_eq!(converted.thinking.as_deref(), Some("detail"));
        assert_eq!(converted.thinking_signature.as_deref(), Some("sig"));
    }

    #[test]
    fn maps_stream_tool_delta() {
        let delta = tool_delta_from_value(json!({
            "index": 2,
            "id": "call-1",
            "type": "function",
            "function": {
                "name": "lookup",
                "arguments": "{\"q\""
            }
        }))
        .unwrap();

        assert_eq!(delta.index, 2);
        assert_eq!(delta.id.as_deref(), Some("call-1"));
        assert_eq!(delta.name.as_deref(), Some("lookup"));
        assert_eq!(delta.arguments_delta.as_deref(), Some("{\"q\""));
    }

    #[tokio::test]
    async fn maps_stream_text_reasoning_completion_and_errors() {
        let events = futures::stream::iter(vec![
            UnifiedStreamEvent::ContentDelta("hel".to_string()),
            UnifiedStreamEvent::ReasoningDelta("why".to_string()),
            UnifiedStreamEvent::ToolDelta(json!({
                "index": 0,
                "id": "call-1",
                "type": "function",
                "function": { "name": "lookup", "arguments": "{}" }
            })),
            UnifiedStreamEvent::Done {
                source: openrouter_rs::types::UnifiedStreamSource::Chat,
                id: Some("resp-stream".to_string()),
                model: Some("model".to_string()),
                finish_reason: Some("tool_calls".to_string()),
                usage: Some(json!({
                    "prompt_tokens": 2,
                    "completion_tokens": 3,
                    "total_tokens": 5
                })),
            },
        ])
        .boxed();
        let (tx, mut rx) = mpsc::channel(10);
        consume_openrouter_stream(events, tx).await;

        assert_eq!(rx.recv().await.unwrap().text.as_deref(), Some("hel"));
        assert_eq!(rx.recv().await.unwrap().thinking.as_deref(), Some("why"));
        assert_eq!(
            rx.recv()
                .await
                .unwrap()
                .tool_call_delta
                .unwrap()
                .name
                .as_deref(),
            Some("lookup")
        );
        let done = rx.recv().await.unwrap();
        assert!(done.is_finished);
        assert_eq!(done.provider_response_id.as_deref(), Some("resp-stream"));
        assert_eq!(done.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(done.usage.unwrap().total_tokens, 5);
    }

    #[tokio::test]
    async fn stream_error_after_partial_output_emits_terminal_error_chunk() {
        let events = futures::stream::iter(vec![
            UnifiedStreamEvent::ContentDelta("partial".to_string()),
            UnifiedStreamEvent::Error(openrouter_rs::error::OpenRouterError::Unknown(
                "boom".to_string(),
            )),
        ])
        .boxed();
        let (tx, mut rx) = mpsc::channel(10);
        consume_openrouter_stream(events, tx).await;

        assert_eq!(rx.recv().await.unwrap().text.as_deref(), Some("partial"));
        let done = rx.recv().await.unwrap();
        assert!(done.is_finished);
        assert_eq!(done.finish_reason.as_deref(), Some("stream_error"));
    }

    fn response_with_choice(choice: Choice) -> CompletionsResponse {
        CompletionsResponse {
            id: "resp-1".to_string(),
            choices: vec![choice],
            created: 1,
            model: "openrouter/model".to_string(),
            object_type: ObjectType::ChatCompletion,
            provider: Some("openrouter".to_string()),
            system_fingerprint: None,
            usage: Some(ResponseUsage {
                prompt_tokens: 3,
                completion_tokens: 4,
                total_tokens: 7,
            }),
        }
    }
}
