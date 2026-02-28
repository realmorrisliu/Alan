use alan_protocol::Event;
use anyhow::Result;
use serde_json::json;

use crate::approval::PendingConfirmation;
use crate::llm::ToolDefinition;

use super::agent_loop::{NormalizedToolCall, RuntimeLoopState};
use super::turn_support::tool_result_preview;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VirtualToolOutcome {
    NotVirtual,
    Continue { refresh_context: bool },
    PauseTurn,
    EndTurn,
}

pub(super) fn virtual_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        request_confirmation_tool_definition(),
        request_user_input_tool_definition(),
        update_plan_tool_definition(),
    ]
}

pub(super) async fn try_handle_virtual_tool_call<E, F>(
    state: &mut RuntimeLoopState,
    tool_call: &NormalizedToolCall,
    tool_arguments: &serde_json::Value,
    emit: &mut E,
) -> Result<VirtualToolOutcome>
where
    E: FnMut(Event) -> F,
    F: std::future::Future<Output = ()>,
{
    match tool_call.name.as_str() {
        "request_confirmation" => {
            emit(Event::ToolCallStarted {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
            })
            .await;

            if let Some(pending) = parse_confirmation_request(&tool_call.id, tool_arguments) {
                let pending_payload = json!({
                    "status": "pending_confirmation",
                    "request_id": pending.checkpoint_id
                });
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&pending_payload),
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    pending_payload,
                    true,
                );
                state.turn_state.set_confirmation(pending.clone());
                emit(Event::Yield {
                    request_id: pending.checkpoint_id,
                    kind: alan_protocol::YieldKind::Confirmation,
                    payload: json!({
                        "checkpoint_type": pending.checkpoint_type,
                        "summary": pending.summary,
                        "details": pending.details,
                        "options": pending.options,
                    }),
                })
                .await;
            } else {
                let error_payload = json!({
                    "status": "invalid_request",
                    "error": "Invalid confirmation request."
                });
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&error_payload),
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    error_payload,
                    false,
                );
                emit(Event::Error {
                    message: "Invalid confirmation request.".to_string(),
                    recoverable: true,
                })
                .await;
                return Ok(VirtualToolOutcome::EndTurn);
            }
            Ok(VirtualToolOutcome::PauseTurn)
        }
        "request_user_input" => {
            emit(Event::ToolCallStarted {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
            })
            .await;

            if let Some(request) =
                parse_structured_user_input_request(&tool_call.id, tool_arguments)
            {
                let request_id = request.request_id.clone();
                let pending_payload =
                    json!({"status": "pending_structured_input", "request_id": request_id});
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&pending_payload),
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    pending_payload,
                    true,
                );
                state.turn_state.set_structured_input(request.clone());
                emit(Event::Yield {
                    request_id: request.request_id,
                    kind: alan_protocol::YieldKind::StructuredInput,
                    payload: json!({
                        "title": request.title,
                        "prompt": request.prompt,
                        "questions": request.questions,
                    }),
                })
                .await;
            } else {
                let error_payload = json!({
                    "status": "invalid_request",
                    "error": "Invalid structured user input request."
                });
                emit(Event::ToolCallCompleted {
                    id: tool_call.id.clone(),
                    result_preview: tool_result_preview(&error_payload),
                })
                .await;
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    error_payload,
                    false,
                );
                emit(Event::Error {
                    message: "Invalid structured user input request.".to_string(),
                    recoverable: true,
                })
                .await;
                return Ok(VirtualToolOutcome::EndTurn);
            }
            Ok(VirtualToolOutcome::PauseTurn)
        }
        "update_plan" => {
            emit(Event::ToolCallStarted {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
            })
            .await;
            match parse_plan_update(tool_arguments) {
                Some((explanation, items)) => {
                    emit(Event::PlanUpdated {
                        explanation: explanation.clone(),
                        items: items.clone(),
                    })
                    .await;
                    let payload = json!({
                        "status": "plan_updated",
                        "items_count": items.len()
                    });
                    emit(Event::ToolCallCompleted {
                        id: tool_call.id.clone(),
                        result_preview: tool_result_preview(&payload),
                    })
                    .await;
                    state.session.record_tool_call(
                        &tool_call.name,
                        tool_arguments.clone(),
                        payload.clone(),
                        true,
                    );
                    state
                        .session
                        .add_tool_message(&tool_call.id, &tool_call.name, payload);
                    Ok(VirtualToolOutcome::Continue {
                        refresh_context: true,
                    })
                }
                None => {
                    let error_payload = json!({
                        "status": "invalid_request",
                        "error": "Invalid plan update payload."
                    });
                    emit(Event::ToolCallCompleted {
                        id: tool_call.id.clone(),
                        result_preview: tool_result_preview(&error_payload),
                    })
                    .await;
                    state.session.record_tool_call(
                        &tool_call.name,
                        tool_arguments.clone(),
                        error_payload,
                        false,
                    );
                    emit(Event::Error {
                        message: "Invalid plan update payload.".to_string(),
                        recoverable: true,
                    })
                    .await;
                    Ok(VirtualToolOutcome::Continue {
                        refresh_context: false,
                    })
                }
            }
        }
        _ => Ok(VirtualToolOutcome::NotVirtual),
    }
}

pub(super) fn parse_confirmation_request(
    tool_call_id: &str,
    args: &serde_json::Value,
) -> Option<PendingConfirmation> {
    let checkpoint_type = args
        .get("checkpoint_type")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("confirmation")
        .to_string();
    let summary = args.get("summary")?.as_str()?.trim().to_string();
    if summary.is_empty() {
        return None;
    }
    let details = args.get("details").cloned().unwrap_or(json!({}));
    let options = args
        .get("options")
        .and_then(|o| o.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                })
                .collect()
        })
        .filter(|opts: &Vec<String>| !opts.is_empty())
        .unwrap_or_else(|| {
            vec![
                "approve".to_string(),
                "modify".to_string(),
                "reject".to_string(),
            ]
        });

    Some(PendingConfirmation {
        checkpoint_id: tool_call_id.to_string(),
        checkpoint_type,
        summary,
        details,
        options,
    })
}

fn parse_structured_user_input_request(
    tool_call_id: &str,
    arguments: &serde_json::Value,
) -> Option<crate::approval::PendingStructuredInputRequest> {
    let title = arguments.get("title")?.as_str()?.trim().to_string();
    let prompt = arguments.get("prompt")?.as_str()?.trim().to_string();
    if title.is_empty() || prompt.is_empty() {
        return None;
    }
    let request_id = tool_call_id.to_string();

    let questions = arguments
        .get("questions")?
        .as_array()?
        .iter()
        .filter_map(|raw| {
            let id = raw.get("id")?.as_str()?.trim().to_string();
            let label = raw.get("label")?.as_str()?.trim().to_string();
            let prompt = raw.get("prompt")?.as_str()?.trim().to_string();
            if id.is_empty() || label.is_empty() || prompt.is_empty() {
                return None;
            }
            let required = raw
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let options = raw
                .get("options")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|opt| {
                            Some(alan_protocol::StructuredInputOption {
                                value: opt.get("value")?.as_str()?.to_string(),
                                label: opt.get("label")?.as_str()?.to_string(),
                                description: opt
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            Some(alan_protocol::StructuredInputQuestion {
                id,
                label,
                prompt,
                required,
                options,
            })
        })
        .collect::<Vec<_>>();

    if questions.is_empty() {
        return None;
    }

    Some(crate::approval::PendingStructuredInputRequest {
        request_id,
        title,
        prompt,
        questions,
    })
}

fn parse_plan_status(raw: &str) -> Option<alan_protocol::PlanItemStatus> {
    match raw {
        "pending" | "blocked" => Some(alan_protocol::PlanItemStatus::Pending),
        "in_progress" => Some(alan_protocol::PlanItemStatus::InProgress),
        "completed" | "skipped" => Some(alan_protocol::PlanItemStatus::Completed),
        _ => None,
    }
}

fn parse_plan_items(value: &serde_json::Value) -> Option<Vec<alan_protocol::PlanItem>> {
    let items = value.as_array()?;
    let parsed = items
        .iter()
        .filter_map(|raw| {
            let id = raw.get("id")?.as_str()?.to_string();
            let content = raw
                .get("content")
                .or_else(|| raw.get("description"))?
                .as_str()?
                .to_string();
            let status_raw = raw.get("status")?.as_str()?;
            let status = parse_plan_status(status_raw)?;
            Some(alan_protocol::PlanItem {
                id,
                content,
                status,
            })
        })
        .collect::<Vec<_>>();
    (!parsed.is_empty()).then_some(parsed)
}

fn parse_plan_update(
    arguments: &serde_json::Value,
) -> Option<(Option<String>, Vec<alan_protocol::PlanItem>)> {
    let explanation = arguments
        .get("explanation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let items = parse_plan_items(arguments.get("items")?)?;
    Some((explanation, items))
}

fn request_confirmation_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "request_confirmation".to_string(),
        description: "Request user confirmation or approval before proceeding with a significant action. Use this when you need explicit user approval before making changes or proceeding with a recommendation.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "checkpoint_id": {
                    "type": "string",
                    "description": "Optional legacy field. Runtime uses the tool call id as request_id."
                },
                "checkpoint_type": {
                    "type": "string",
                    "description": "The type of checkpoint (e.g., 'business_understanding', 'supplier_recommendation', 'final_confirmation'). Defaults to 'confirmation'."
                },
                "summary": {
                    "type": "string",
                    "description": "A clear summary of what is being proposed or what the user should confirm"
                },
                "details": {
                    "type": "object",
                    "description": "Additional structured details relevant to the confirmation"
                }
            },
            "required": ["summary"]
        }),
    }
}

fn request_user_input_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "request_user_input".to_string(),
        description: "Request structured user input (questions/options) from the client UI and wait for a structured response before continuing.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "request_id": {
                    "type": "string",
                    "description": "Optional legacy field. Runtime uses the tool call id as request_id."
                },
                "title": { "type": "string" },
                "prompt": { "type": "string" },
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "label": { "type": "string" },
                            "prompt": { "type": "string" },
                            "required": { "type": "boolean" },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "value": { "type": "string" },
                                        "label": { "type": "string" },
                                        "description": { "type": "string" }
                                    },
                                    "required": ["value", "label"]
                                }
                            }
                        },
                        "required": ["id", "label", "prompt"]
                    }
                }
            },
            "required": ["title", "prompt", "questions"]
        }),
    }
}

fn update_plan_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "update_plan".to_string(),
        description: "Publish a normalized plan/progress update to the client UI. Use this when the task plan changes or step status changes.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "explanation": { "type": "string" },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "content": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                        },
                        "required": ["id", "content", "status"]
                    }
                }
            },
            "required": ["items"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        llm::LlmClient,
        runtime::{RuntimeConfig, TurnState},
        session::Session,
        tools::ToolRegistry,
    };
    use alan_llm::{GenerationRequest, GenerationResponse, LlmProvider, StreamChunk};
    use async_trait::async_trait;

    // Simple mock provider for testing
    struct SimpleMockProvider;

    #[async_trait]
    impl LlmProvider for SimpleMockProvider {
        async fn generate(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<GenerationResponse> {
            Ok(GenerationResponse {
                content: "test".to_string(),
                thinking: None,
                thinking_signature: None,
                redacted_thinking: Vec::new(),
                tool_calls: vec![],
                usage: None,
            })
        }

        async fn chat(&mut self, _system: Option<&str>, _user: &str) -> anyhow::Result<String> {
            Ok("mock".to_string())
        }

        async fn generate_stream(
            &mut self,
            _request: GenerationRequest,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(StreamChunk {
                    text: Some("test".to_string()),
                    thinking: None,
                    thinking_signature: None,
                    redacted_thinking: None,
                    usage: None,
                    tool_call_delta: None,
                    is_finished: true,
                    finish_reason: Some("stop".to_string()),
                })
                .await;
            Ok(rx)
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }
    }

    fn create_test_agent_loop_state() -> super::super::agent_loop::RuntimeLoopState {
        let config = Config::default();
        let session = Session::new();
        let tools = ToolRegistry::new();
        let runtime_config = RuntimeConfig::default();

        super::super::agent_loop::RuntimeLoopState {
            workspace_id: "test-workspace".to_string(),
            session,
            llm_client: LlmClient::new(SimpleMockProvider),
            tools,
            core_config: config,
            runtime_config,
            turn_state: TurnState::default(),
        }
    }

    #[test]
    fn test_virtual_tool_definitions_include_all_runtime_virtual_tools() {
        let defs = virtual_tool_definitions();
        assert_eq!(defs.len(), 3);
        assert!(defs.iter().any(|d| d.name == "request_confirmation"));
        assert!(defs.iter().any(|d| d.name == "request_user_input"));
        assert!(defs.iter().any(|d| d.name == "update_plan"));
    }

    #[test]
    fn test_request_confirmation_tool_definition_schema_shape() {
        let def = request_confirmation_tool_definition();
        assert_eq!(def.name, "request_confirmation");
        assert!(def.description.contains("confirmation"));
        assert_eq!(def.parameters["type"], "object");
        assert_eq!(
            def.parameters["properties"]["checkpoint_id"]["type"],
            "string"
        );
        assert_eq!(
            def.parameters["properties"]["checkpoint_type"]["type"],
            "string"
        );
        assert_eq!(def.parameters["properties"]["summary"]["type"], "string");
        assert_eq!(def.parameters["properties"]["details"]["type"], "object");
    }

    #[test]
    fn test_request_user_input_tool_definition() {
        let def = request_user_input_tool_definition();
        assert_eq!(def.name, "request_user_input");
        assert!(def.description.contains("structured"));
        assert_eq!(def.parameters["type"], "object");
        assert!(def.parameters["properties"].get("title").is_some());
        assert!(def.parameters["properties"].get("prompt").is_some());
        assert!(def.parameters["properties"].get("questions").is_some());
    }

    #[test]
    fn test_update_plan_tool_definition() {
        let def = update_plan_tool_definition();
        assert_eq!(def.name, "update_plan");
        assert!(def.description.contains("plan"));
        assert_eq!(def.parameters["type"], "object");
        assert!(def.parameters["properties"].get("explanation").is_some());
        assert!(def.parameters["properties"].get("items").is_some());
    }

    // Tests for parse_confirmation_request
    #[test]
    fn test_parse_confirmation_request_valid() {
        let args = json!({
            "checkpoint_type": "test_type",
            "summary": "Test summary",
            "details": {"key": "value"},
            "options": ["approve", "reject"]
        });

        let result = parse_confirmation_request("call_1", &args);
        assert!(result.is_some());

        let pending = result.unwrap();
        assert_eq!(pending.checkpoint_id, "call_1");
        assert_eq!(pending.checkpoint_type, "test_type");
        assert_eq!(pending.summary, "Test summary");
        assert_eq!(pending.options, vec!["approve", "reject"]);
    }

    #[test]
    fn test_parse_confirmation_request_default_options() {
        let args = json!({
            "checkpoint_type": "test_type",
            "summary": "Test summary"
        });

        let result = parse_confirmation_request("call_1", &args);
        assert!(result.is_some());

        let pending = result.unwrap();
        assert_eq!(pending.checkpoint_id, "call_1");
        assert_eq!(pending.options, vec!["approve", "modify", "reject"]);
    }

    #[test]
    fn test_parse_confirmation_request_missing_required() {
        // Missing summary
        let args = json!({
            "checkpoint_type": "test_type",
            "details": {"k": "v"}
        });
        assert!(parse_confirmation_request("call_1", &args).is_none());

        // Missing checkpoint_type falls back to default
        let args = json!({
            "summary": "Test summary"
        });
        let parsed = parse_confirmation_request("call_1", &args).unwrap();
        assert_eq!(parsed.checkpoint_type, "confirmation");
    }

    #[test]
    fn test_parse_confirmation_request_non_string_fields() {
        // summary must be a non-empty string
        let args = json!({
            "checkpoint_type": "test_type",
            "summary": 123
        });
        assert!(parse_confirmation_request("call_1", &args).is_none());
    }

    // Tests for parse_structured_user_input_request
    #[test]
    fn test_parse_structured_user_input_request_valid() {
        let args = json!({
            "title": "Test Title",
            "prompt": "Test Prompt",
            "questions": [
                {
                    "id": "q1",
                    "label": "Question 1",
                    "prompt": "What is your name?",
                    "required": true
                }
            ]
        });

        let result = parse_structured_user_input_request("call_1", &args);
        assert!(result.is_some());

        let request = result.unwrap();
        assert_eq!(request.title, "Test Title");
        assert_eq!(request.prompt, "Test Prompt");
        assert_eq!(request.questions.len(), 1);
        assert_eq!(request.questions[0].id, "q1");
        assert!(request.questions[0].required);
    }

    #[test]
    fn test_parse_structured_user_input_request_with_options() {
        let args = json!({
            "title": "Test",
            "prompt": "Prompt",
            "questions": [
                {
                    "id": "q1",
                    "label": "Label",
                    "prompt": "Prompt?",
                    "required": false,
                    "options": [
                        {"value": "yes", "label": "Yes", "description": "Yes option"}
                    ]
                }
            ]
        });

        let result = parse_structured_user_input_request("call_1", &args);
        assert!(result.is_some());

        let request = result.unwrap();
        assert_eq!(request.questions[0].options.len(), 1);
        assert_eq!(request.questions[0].options[0].value, "yes");
        assert_eq!(request.questions[0].options[0].label, "Yes");
    }

    #[test]
    fn test_parse_structured_user_input_request_missing_required() {
        // Missing title
        let args = json!({
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing prompt
        let args = json!({
            "title": "Title",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing questions
        let args = json!({
            "title": "Title",
            "prompt": "Prompt"
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_empty_fields() {
        // Empty title
        let args = json!({
            "title": "",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Empty prompt
        let args = json!({
            "title": "Title",
            "prompt": "  ",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_empty_questions() {
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": []
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_invalid_question() {
        // Missing question id
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"label": "Label", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing question label
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "prompt": "Prompt?"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());

        // Missing question prompt
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label"}]
        });
        assert!(parse_structured_user_input_request("call_1", &args).is_none());
    }

    #[test]
    fn test_parse_structured_user_input_request_custom_request_id() {
        let args = json!({
            "request_id": "custom_id",
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });

        let result = parse_structured_user_input_request("call_1", &args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().request_id, "call_1");
    }

    #[test]
    fn test_parse_structured_user_input_request_fallback_request_id() {
        let args = json!({
            "title": "Title",
            "prompt": "Prompt",
            "questions": [{"id": "q1", "label": "Label", "prompt": "Prompt?"}]
        });

        let result = parse_structured_user_input_request("fallback_id", &args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().request_id, "fallback_id");
    }

    // Tests for parse_plan_update
    #[test]
    fn test_parse_plan_update_valid() {
        let args = json!({
            "explanation": "Test explanation",
            "items": [
                {"id": "1", "content": "Step 1", "status": "pending"},
                {"id": "2", "content": "Step 2", "status": "in_progress"}
            ]
        });

        let result = parse_plan_update(&args);
        assert!(result.is_some());

        let (explanation, items) = result.unwrap();
        assert_eq!(explanation, Some("Test explanation".to_string()));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "1");
        assert_eq!(items[1].content, "Step 2");
    }

    #[test]
    fn test_parse_plan_update_without_explanation() {
        let args = json!({
            "items": [{"id": "1", "content": "Step 1", "status": "completed"}]
        });

        let result = parse_plan_update(&args);
        assert!(result.is_some());

        let (explanation, items) = result.unwrap();
        assert_eq!(explanation, None);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_plan_update_missing_items() {
        let args = json!({
            "explanation": "Test"
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_empty_items() {
        let args = json!({
            "items": []
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_missing_id() {
        let args = json!({
            "items": [{"content": "Step 1", "status": "pending"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_missing_content() {
        let args = json!({
            "items": [{"id": "1", "status": "pending"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_missing_status() {
        let args = json!({
            "items": [{"id": "1", "content": "Step 1"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_invalid_status() {
        let args = json!({
            "items": [{"id": "1", "content": "Step 1", "status": "invalid_status"}]
        });
        assert!(parse_plan_update(&args).is_none());
    }

    #[test]
    fn test_parse_plan_update_using_description() {
        // Tests that "description" field can be used as fallback for "content"
        let args = json!({
            "items": [{"id": "1", "description": "Step description", "status": "pending"}]
        });

        let result = parse_plan_update(&args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().1[0].content, "Step description");
    }

    // Tests for parse_plan_status
    #[test]
    fn test_parse_plan_status_valid_values() {
        assert!(parse_plan_status("pending").is_some());
        assert!(parse_plan_status("blocked").is_some());
        assert!(parse_plan_status("in_progress").is_some());
        assert!(parse_plan_status("completed").is_some());
        assert!(parse_plan_status("skipped").is_some());
    }

    #[test]
    fn test_parse_plan_status_invalid_values() {
        assert!(parse_plan_status("unknown").is_none());
        assert!(parse_plan_status("").is_none());
        assert!(parse_plan_status("PENDING").is_none()); // Case sensitive
    }

    // Tests for try_handle_virtual_tool_call
    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_request_confirmation() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                "checkpoint_id": "chk_123",
                "checkpoint_type": "test",
                "summary": "Test confirmation"
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::PauseTurn));

        // Verify confirmation was set
        assert!(state.turn_state.pending_confirmation().is_some());
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invalid_confirmation() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_confirmation".to_string(),
            arguments: json!({
                // Invalid summary type
                "summary": 42
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::EndTurn));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_request_user_input() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_user_input".to_string(),
            arguments: json!({
                "title": "Test Input",
                "prompt": "Enter value",
                "questions": [{"id": "q1", "label": "Q1", "prompt": "What?"}]
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::PauseTurn));

        // Verify structured input was set
        assert!(state.turn_state.has_pending_interaction());
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invalid_user_input() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "request_user_input".to_string(),
            arguments: json!({
                // Missing required fields
                "title": "Test"
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::EndTurn));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_update_plan() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                "explanation": "Test plan",
                "items": [{"id": "1", "content": "Step 1", "status": "in_progress"}]
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: true
            }
        ));
    }

    #[tokio::test]
    async fn test_try_handle_virtual_tool_call_invalid_update_plan() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "update_plan".to_string(),
            arguments: json!({
                // Missing items
                "explanation": "Test"
            }),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            VirtualToolOutcome::Continue {
                refresh_context: false
            }
        ));
    }

    #[tokio::test]
    async fn test_try_handle_non_virtual_tool() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::NotVirtual));
    }

    #[tokio::test]
    async fn test_try_handle_unknown_tool() {
        let mut state = create_test_agent_loop_state();

        let tool_call = NormalizedToolCall {
            id: "call_1".to_string(),
            name: "unknown_tool".to_string(),
            arguments: json!({}),
        };

        let mut events = vec![];
        let mut emit = |event: Event| {
            events.push(event);
            async {}
        };

        let result =
            try_handle_virtual_tool_call(&mut state, &tool_call, &tool_call.arguments, &mut emit)
                .await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), VirtualToolOutcome::NotVirtual));
    }
}
