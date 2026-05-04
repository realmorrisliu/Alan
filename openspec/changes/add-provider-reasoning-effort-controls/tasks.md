## 1. Protocol And Shared Types

- [x] 1.1 Add `ReasoningEffort` to `alan-protocol` with lowercase serde values `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`.
- [x] 1.2 Add `ReasoningControls` and, if included in this phase, `ReasoningSummaryMode` shared protocol/runtime types.
- [x] 1.3 Add serialization/deserialization tests for valid values, invalid values, and the distinction between unset and `none`.
- [x] 1.4 Re-export the new protocol types from `crates/protocol/src/lib.rs`.

## 2. Model Catalog Metadata

- [x] 2.1 Extend `ModelInfo`, TOML parsing, validation, and overlays with `supported_reasoning_efforts` and `default_reasoning_effort`.
- [x] 2.2 Add optional per-model effort-to-budget mappings for budget-native providers.
- [x] 2.3 Migrate bundled model catalog entries to declare supported/default reasoning efforts explicitly or derive a conservative compatibility set from `supports_reasoning = true`.
- [x] 2.4 Add catalog tests for valid defaults, invalid defaults, overlay replacement, and backward-compatible `supports_reasoning` handling.
- [x] 2.5 Expose supported/default reasoning effort metadata through daemon/client model metadata surfaces if such surfaces are present in the implementation path. (Audited: no daemon model-list metadata surface exists in this path; effective session/client metadata is exposed.)

## 3. Runtime Config And Override Resolution

- [x] 3.1 Add `model_reasoning_effort` to `Config`, config TOML loading, agent-root overlay merging, and tests.
- [x] 3.2 Add `model_reasoning_effort` to `RuntimeConfig`, `AgentConfig`, explicit runtime override tracking, and merge/sync tests.
- [x] 3.3 Reject explicit configs that set both `model_reasoning_effort` and `thinking_budget_tokens`.
- [x] 3.4 Resolve effective effort from turn override, session/runtime override, agent config, model default, or provider default in that order.
- [x] 3.5 Validate effective effort against the resolved model's supported effort set before dispatch.
- [x] 3.6 Preserve `thinking_budget_tokens` behavior when no explicit effort is set.

## 4. API, Turn, And Child-Agent Surfaces

- [x] 4.1 Add optional reasoning effort to daemon create-session, fork-session, and session-list/read response metadata.
- [x] 4.2 Add optional reasoning effort to `Op::Turn.context` if one-turn overrides are implemented in this phase.
- [x] 4.3 Add optional reasoning effort to `SpawnRuntimeOverrides` for delegated child runtimes.
- [x] 4.4 Update TUI and Apple protocol/client types for any newly exposed request or response fields.
- [x] 4.5 Add route/protocol tests for session overrides, turn overrides, child-agent overrides, and response metadata.

## 5. Generation Request Plumbing

- [x] 5.1 Add canonical reasoning controls to `alan-llm::GenerationRequest`.
- [x] 5.2 Update runtime turn construction to populate `GenerationRequest` with resolved effort and/or legacy budget.
- [x] 5.3 Remove OpenAI-specific reasoning effort inference from provider `extra_params` as the primary path.
- [x] 5.4 Add request-construction tests proving effective effort, legacy budget, and no-control cases.

## 6. Provider Adapter Mapping

- [x] 6.1 Map `ReasoningEffort` to `reasoning.effort` in `openai_responses` and preserve reasoning encrypted-content include behavior.
- [x] 6.2 Map `ReasoningEffort` to `reasoning_effort` in `openai_chat_completions`.
- [x] 6.3 Apply ChatGPT managed Responses reasoning controls only when current provider capabilities allow them.
- [x] 6.4 Map Anthropic effort levels to configured `thinking.budget_tokens` presets and enforce provider constraints.
- [x] 6.5 Map Gemini 3 effort levels to `thinkingConfig.thinkingLevel`.
- [x] 6.6 Map Gemini 2.5 effort levels to configured `thinkingConfig.thinkingBudget` presets and preserve explicit budget behavior.
- [x] 6.7 Map compatible-provider and OpenRouter reasoning effort only when the model/provider declares support; otherwise reject explicit effort.
- [x] 6.8 Add adapter unit tests for each provider's request payload, unsupported effort rejection, and legacy budget compatibility behavior.

## 7. Observability And Persistence

- [x] 7.1 Include effective reasoning effort in request logs and request metadata without exposing hidden reasoning content.
- [x] 7.2 Persist effective reasoning effort in rollout/session metadata when available.
- [x] 7.3 Add tests for resume/fork behavior so persisted effort does not accidentally override explicit new session settings.

## 8. Documentation And Migration

- [x] 8.1 Update `AGENTS.md` and config examples to prefer `model_reasoning_effort = "medium"`.
- [x] 8.2 Document `thinking_budget_tokens` as provider-specific compatibility control and explain when to keep using it.
- [x] 8.3 Update `docs/spec/provider_capability_contract.md` with effort-control semantics and provider degradation rules.
- [x] 8.4 Add migration notes for replacing budget-based OpenAI reasoning control with named effort.

## 9. Verification

- [x] 9.1 Run `cargo fmt --all`.
- [x] 9.2 Run focused `cargo test -p alan-protocol` tests for reasoning effort serialization.
- [x] 9.3 Run focused `cargo test -p alan-runtime` tests for config, model catalog, override resolution, and turn request construction.
- [x] 9.4 Run focused `cargo test -p alan-llm` tests for provider request mapping.
- [x] 9.5 Run focused `cargo test -p alan` tests for daemon/session API metadata.
- [x] 9.6 Run live provider harness cases for OpenAI, Anthropic, Gemini, and compatible/OpenRouter providers when credentials are available. (Harness target ran; live cases were ignored because `ALAN_LIVE_PROVIDER_TESTS=1` and credentials were not configured.)
- [x] 9.7 Run `openspec validate add-provider-reasoning-effort-controls` and confirm the change remains complete.
