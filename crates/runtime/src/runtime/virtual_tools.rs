use anyhow::Result;
use alan_protocol::Event;
use serde_json::json;

use crate::llm::ToolDefinition;
use crate::approval::PendingConfirmation;

use super::agent_loop::{AgentLoopState, NormalizedToolCall};

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
    state: &mut AgentLoopState,
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
            if let Some(pending) = parse_confirmation_request(tool_arguments) {
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    json!({"status": "pending"}),
                    true,
                );
                state.turn_state.set_confirmation(pending.clone());
                emit(Event::ConfirmationRequired {
                    checkpoint_id: pending.checkpoint_id,
                    checkpoint_type: pending.checkpoint_type,
                    summary: pending.summary,
                    details: pending.details,
                    options: pending.options,
                })
                .await;
            } else {
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
            if let Some(request) =
                parse_structured_user_input_request(&tool_call.id, tool_arguments)
            {
                let request_id = request.request_id.clone();
                state.session.record_tool_call(
                    &tool_call.name,
                    tool_arguments.clone(),
                    json!({"status": "pending_structured_input", "request_id": request_id}),
                    true,
                );
                state.turn_state.set_structured_input(request.clone());
                emit(Event::StructuredUserInputRequested {
                    request_id: request.request_id,
                    title: request.title,
                    prompt: request.prompt,
                    questions: request.questions,
                })
                .await;
            } else {
                emit(Event::Error {
                    message: "Invalid structured user input request.".to_string(),
                    recoverable: true,
                })
                .await;
                return Ok(VirtualToolOutcome::EndTurn);
            }
            Ok(VirtualToolOutcome::PauseTurn)
        }
        "update_plan" => match parse_plan_update(tool_arguments) {
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
                    call_id: tool_call.id.clone(),
                    tool_name: tool_call.name.clone(),
                    result: payload.clone(),
                    success: true,
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
                emit(Event::Error {
                    message: "Invalid plan update payload.".to_string(),
                    recoverable: true,
                })
                .await;
                Ok(VirtualToolOutcome::Continue {
                    refresh_context: false,
                })
            }
        },
        _ => Ok(VirtualToolOutcome::NotVirtual),
    }
}

pub(super) fn parse_confirmation_request(args: &serde_json::Value) -> Option<PendingConfirmation> {
    let checkpoint_id = args.get("checkpoint_id")?.as_str()?.to_string();
    let checkpoint_type_str = args.get("checkpoint_type")?.as_str()?;
    let summary = args.get("summary")?.as_str()?.to_string();
    let details = args.get("details").cloned().unwrap_or(json!({}));
    let options = args
        .get("options")
        .and_then(|o| o.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                "approve".to_string(),
                "modify".to_string(),
                "reject".to_string(),
            ]
        });

    Some(PendingConfirmation {
        checkpoint_id,
        checkpoint_type: checkpoint_type_str.to_string(),
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
    let request_id = arguments
        .get("request_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| tool_call_id.to_string());

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
                    "description": "A unique identifier for this checkpoint"
                },
                "checkpoint_type": {
                    "type": "string",
                    "description": "The type of checkpoint (e.g., 'business_understanding', 'supplier_recommendation', 'final_confirmation')"
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
            "required": ["checkpoint_id", "checkpoint_type", "summary"]
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
                "request_id": { "type": "string" },
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
}
