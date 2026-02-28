//! Response guardrails for assistant outputs.
//!
//! These checks enforce runtime-level invariants before emitting a response.
//! They are intentionally independent from task/domain skills.

use super::agent_loop::RuntimeLoopState;

const RULE_ID_CAPABILITY_CONTRADICTION: &str = "capability_contradiction";

/// Model draft before output is emitted.
pub(super) struct AssistantDraft<'a> {
    content: &'a str,
    has_tool_calls: bool,
}

impl<'a> AssistantDraft<'a> {
    pub(super) fn new(content: &'a str, has_tool_calls: bool) -> Self {
        Self {
            content,
            has_tool_calls,
        }
    }
}

/// Guardrail evaluation context derived from runtime state.
pub(super) struct ResponseGuardrailContext {
    has_any_tools: bool,
    has_network_capability: bool,
    streaming_response: bool,
}

impl ResponseGuardrailContext {
    pub(super) fn from_state(state: &RuntimeLoopState, streaming_response: bool) -> Self {
        let mut has_any_tools = false;
        let mut has_network_capability = false;
        let empty_args = serde_json::json!({});

        for tool_name in state.tools.list_tools() {
            has_any_tools = true;
            if matches!(
                state.tools.capability_for_tool(tool_name, &empty_args),
                Some(alan_protocol::ToolCapability::Network)
            ) {
                has_network_capability = true;
            }
        }

        for tool in state.session.dynamic_tools.values() {
            has_any_tools = true;
            if matches!(
                tool.capability,
                Some(alan_protocol::ToolCapability::Network)
            ) {
                has_network_capability = true;
            }
        }

        Self {
            has_any_tools,
            has_network_capability,
            streaming_response,
        }
    }

    #[cfg(test)]
    fn for_tests(
        has_any_tools: bool,
        has_network_capability: bool,
        streaming_response: bool,
    ) -> Self {
        Self {
            has_any_tools,
            has_network_capability,
            streaming_response,
        }
    }
}

pub(super) enum GuardrailDecision {
    Accept,
    Regenerate {
        rule_id: &'static str,
        reason: String,
        instruction: String,
    },
}

/// Runtime guardrails pipeline.
pub(super) struct ResponseGuardrails {
    max_regenerations: usize,
    regeneration_count: usize,
}

impl Default for ResponseGuardrails {
    fn default() -> Self {
        Self {
            max_regenerations: 1,
            regeneration_count: 0,
        }
    }
}

impl ResponseGuardrails {
    pub(super) fn evaluate(
        &mut self,
        context: &ResponseGuardrailContext,
        draft: &AssistantDraft<'_>,
    ) -> GuardrailDecision {
        if self.regeneration_count >= self.max_regenerations
            || context.streaming_response
            || draft.has_tool_calls
            || draft.content.trim().is_empty()
        {
            return GuardrailDecision::Accept;
        }

        let normalized = draft.content.to_lowercase();

        if context.has_any_tools && claims_tools_unavailable(&normalized) {
            self.regeneration_count += 1;
            return GuardrailDecision::Regenerate {
                rule_id: RULE_ID_CAPABILITY_CONTRADICTION,
                reason: "Assistant claimed tools are unavailable despite registered tools."
                    .to_string(),
                instruction: "Correction: tools are available in this session. Do not claim tools are unavailable. If a tool is needed, call a relevant tool first. If it fails, report the observed failure.".to_string(),
            };
        }

        if context.has_network_capability && claims_network_unavailable(&normalized) {
            self.regeneration_count += 1;
            return GuardrailDecision::Regenerate {
                rule_id: RULE_ID_CAPABILITY_CONTRADICTION,
                reason:
                    "Assistant claimed external/current data access is unavailable despite network-capable tools."
                        .to_string(),
                instruction: "Correction: network-capable tools are available in this session. For requests requiring external or current data, call a relevant tool before stating limitations. If the tool fails, include the actual error.".to_string(),
            };
        }

        GuardrailDecision::Accept
    }
}

fn claims_tools_unavailable(text: &str) -> bool {
    [
        "don't have access to tools",
        "do not have access to tools",
        "can't use tools",
        "cannot use tools",
        "no tools available",
        "without tools",
        "unable to use tools",
        "无法使用工具",
        "没有可用工具",
    ]
    .iter()
    .any(|pattern| text.contains(pattern))
}

fn claims_network_unavailable(text: &str) -> bool {
    [
        "don't have access to real-time",
        "do not have access to real-time",
        "can't access the internet",
        "cannot access the internet",
        "no internet access",
        "can't browse the web",
        "cannot browse the web",
        "don't have web access",
        "do not have web access",
        "can't check current",
        "cannot check current",
        "unable to access live data",
        "无法访问互联网",
        "无法联网",
        "无法获取实时",
    ]
    .iter()
    .any(|pattern| text.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_unavailable_claim_triggers_regeneration_when_tools_exist() {
        let mut guardrails = ResponseGuardrails::default();
        let context = ResponseGuardrailContext::for_tests(true, false, false);
        let draft = AssistantDraft::new("I don't have access to tools in this environment.", false);

        let decision = guardrails.evaluate(&context, &draft);
        assert!(matches!(
            decision,
            GuardrailDecision::Regenerate {
                rule_id: RULE_ID_CAPABILITY_CONTRADICTION,
                ..
            }
        ));
    }

    #[test]
    fn network_unavailable_claim_triggers_regeneration_when_network_tool_exists() {
        let mut guardrails = ResponseGuardrails::default();
        let context = ResponseGuardrailContext::for_tests(true, true, false);
        let draft = AssistantDraft::new("I can't access the internet right now.", false);

        let decision = guardrails.evaluate(&context, &draft);
        assert!(matches!(
            decision,
            GuardrailDecision::Regenerate {
                rule_id: RULE_ID_CAPABILITY_CONTRADICTION,
                ..
            }
        ));
    }

    #[test]
    fn no_regeneration_when_claim_not_present() {
        let mut guardrails = ResponseGuardrails::default();
        let context = ResponseGuardrailContext::for_tests(true, true, false);
        let draft = AssistantDraft::new("I'll check this for you.", false);

        let decision = guardrails.evaluate(&context, &draft);
        assert!(matches!(decision, GuardrailDecision::Accept));
    }

    #[test]
    fn no_regeneration_when_tool_call_exists() {
        let mut guardrails = ResponseGuardrails::default();
        let context = ResponseGuardrailContext::for_tests(true, true, false);
        let draft = AssistantDraft::new("I can't access the internet right now.", true);

        let decision = guardrails.evaluate(&context, &draft);
        assert!(matches!(decision, GuardrailDecision::Accept));
    }

    #[test]
    fn no_regeneration_for_streaming_path() {
        let mut guardrails = ResponseGuardrails::default();
        let context = ResponseGuardrailContext::for_tests(true, true, true);
        let draft = AssistantDraft::new("I can't access the internet right now.", false);

        let decision = guardrails.evaluate(&context, &draft);
        assert!(matches!(decision, GuardrailDecision::Accept));
    }

    #[test]
    fn max_regeneration_limit_is_enforced() {
        let mut guardrails = ResponseGuardrails::default();
        let context = ResponseGuardrailContext::for_tests(true, true, false);
        let draft = AssistantDraft::new("I can't access the internet right now.", false);

        let first = guardrails.evaluate(&context, &draft);
        let second = guardrails.evaluate(&context, &draft);

        assert!(matches!(first, GuardrailDecision::Regenerate { .. }));
        assert!(matches!(second, GuardrailDecision::Accept));
    }
}
