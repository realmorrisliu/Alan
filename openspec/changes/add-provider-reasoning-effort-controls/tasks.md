## 1. Protocol And Shared Types

- [ ] 1.1 Add `ReasoningEffort` to `alan-protocol` with lowercase serde values `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`.
- [ ] 1.2 Add `ReasoningControls` and, if included in this phase, `ReasoningSummaryMode` shared protocol/runtime types.
- [ ] 1.3 Add serialization/deserialization tests for valid values, invalid values, and the distinction between unset and `none`.
- [ ] 1.4 Re-export the new protocol types from `crates/protocol/src/lib.rs`.

## 2. Model Catalog Metadata

- [ ] 2.1 Extend `ModelInfo`, TOML parsing, validation, and overlays with `supported_reasoning_efforts` and `default_reasoning_effort`.
- [ ] 2.2 Add optional per-model effort-to-budget mappings for budget-native providers.
- [ ] 2.3 Migrate bundled model catalog entries to declare supported/default reasoning efforts explicitly or derive a conservative compatibility set from `supports_reasoning = true`.
- [ ] 2.4 Add catalog tests for valid defaults, invalid defaults, overlay replacement, and backward-compatible `supports_reasoning` handling.
- [ ] 2.5 Expose supported/default reasoning effort metadata through daemon/client model metadata surfaces if such surfaces are present in the implementation path.

## 3. Runtime Config And Override Resolution

- [ ] 3.1 Add `model_reasoning_effort` to `Config`, config TOML loading, agent-root overlay merging, and tests.
- [ ] 3.2 Add `model_reasoning_effort` to `RuntimeConfig`, `AgentConfig`, explicit runtime override tracking, and merge/sync tests.
- [ ] 3.3 Reject explicit configs that set both `model_reasoning_effort` and `thinking_budget_tokens`.
- [ ] 3.4 Resolve effective effort from turn override, session/runtime override, agent config, model default, or provider default in that order.
- [ ] 3.5 Validate effective effort against the resolved model's supported effort set before dispatch.
- [ ] 3.6 Preserve `thinking_budget_tokens` behavior when no explicit effort is set.

## 4. API, Turn, And Child-Agent Surfaces

- [ ] 4.1 Add optional reasoning effort to daemon create-session, fork-session, and session-list/read response metadata.
- [ ] 4.2 Add optional reasoning effort to `Op::Turn.context` if one-turn overrides are implemented in this phase.
- [ ] 4.3 Add optional reasoning effort to `SpawnRuntimeOverrides` for delegated child runtimes.
- [ ] 4.4 Update TUI and Apple protocol/client types for any newly exposed request or response fields.
- [ ] 4.5 Add route/protocol tests for session overrides, turn overrides, child-agent overrides, and response metadata.

## 5. Generation Request Plumbing

- [ ] 5.1 Add canonical reasoning controls to `alan-llm::GenerationRequest`.
- [ ] 5.2 Update runtime turn construction to populate `GenerationRequest` with resolved effort and/or legacy budget.
- [ ] 5.3 Remove OpenAI-specific reasoning effort inference from provider `extra_params` as the primary path.
- [ ] 5.4 Add request-construction tests proving effective effort, legacy budget, and no-control cases.

## 6. Provider Adapter Mapping

- [ ] 6.1 Map `ReasoningEffort` to `reasoning.effort` in `openai_responses` and preserve reasoning encrypted-content include behavior.
- [ ] 6.2 Map `ReasoningEffort` to `reasoning_effort` in `openai_chat_completions`.
- [ ] 6.3 Apply ChatGPT managed Responses reasoning controls only when current provider capabilities allow them.
- [ ] 6.4 Map Anthropic effort levels to configured `thinking.budget_tokens` presets and enforce provider constraints.
- [ ] 6.5 Map Gemini 3 effort levels to `thinkingConfig.thinkingLevel`.
- [ ] 6.6 Map Gemini 2.5 effort levels to configured `thinkingConfig.thinkingBudget` presets and preserve explicit budget behavior.
- [ ] 6.7 Map compatible-provider and OpenRouter reasoning effort only when the model/provider declares support; otherwise reject explicit effort.
- [ ] 6.8 Add adapter unit tests for each provider's request payload, unsupported effort rejection, and legacy budget compatibility behavior.

## 7. Observability And Persistence

- [ ] 7.1 Include effective reasoning effort in request logs and request metadata without exposing hidden reasoning content.
- [ ] 7.2 Persist effective reasoning effort in rollout/session metadata when available.
- [ ] 7.3 Add tests for resume/fork behavior so persisted effort does not accidentally override explicit new session settings.

## 8. Documentation And Migration

- [ ] 8.1 Update `AGENTS.md` and config examples to prefer `model_reasoning_effort = "medium"`.
- [ ] 8.2 Document `thinking_budget_tokens` as provider-specific compatibility control and explain when to keep using it.
- [ ] 8.3 Update `docs/spec/provider_capability_contract.md` with effort-control semantics and provider degradation rules.
- [ ] 8.4 Add migration notes for replacing budget-based OpenAI reasoning control with named effort.

## 9. Verification

- [ ] 9.1 Run `cargo fmt --all`.
- [ ] 9.2 Run focused `cargo test -p alan-protocol` tests for reasoning effort serialization.
- [ ] 9.3 Run focused `cargo test -p alan-runtime` tests for config, model catalog, override resolution, and turn request construction.
- [ ] 9.4 Run focused `cargo test -p alan-llm` tests for provider request mapping.
- [ ] 9.5 Run focused `cargo test -p alan` tests for daemon/session API metadata.
- [ ] 9.6 Run live provider harness cases for OpenAI, Anthropic, Gemini, and compatible/OpenRouter providers when credentials are available.
- [ ] 9.7 Run `openspec validate add-provider-reasoning-effort-controls` and confirm the change remains complete.
