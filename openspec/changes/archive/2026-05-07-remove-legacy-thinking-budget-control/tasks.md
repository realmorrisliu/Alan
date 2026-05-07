## 1. Public Contract Removal

- [x] 1.1 Remove `thinking_budget_tokens` from user-facing agent config examples and parsing, or convert remaining parser support into an explicit rejection path.
- [x] 1.2 Remove `thinking_budget_tokens` from daemon session/fork/turn DTOs and client DTOs where exposed.
- [x] 1.3 Remove public `GenerationRequest.thinking_budget_tokens` compatibility fields/builders and update call sites to use normalized reasoning controls.
- [x] 1.4 Add clear errors that direct users from `thinking_budget_tokens` to `model_reasoning_effort`.
- [x] 1.5 Remove raw `reasoning.budget_tokens` from canonical request controls so budget-only inputs cannot be accepted and ignored.

## 2. Resolver And Runtime

- [x] 2.1 Remove legacy budget intent from request-control resolver inputs and precedence tests.
- [x] 2.2 Keep resolver output centered on canonical reasoning effort plus provider-default/no-control states.
- [x] 2.3 Confirm child-agent runtime overrides accept reasoning effort only.
- [x] 2.4 Update rollout/session metadata so it never reports legacy budget as an effective public control.

## 3. Provider Projection

- [x] 3.1 Keep provider-native budget projection where required, derived only from canonical reasoning effort and model/provider metadata.
- [x] 3.2 Remove OpenAI legacy budget-to-effort mapping.
- [x] 3.3 Remove Anthropic, Gemini, OpenRouter, and compatible-provider public budget fallback projection paths.
- [x] 3.4 Add provider projection tests for effort-derived budgets and rejected legacy budget inputs.

## 4. Documentation And Migration

- [x] 4.1 Update provider reasoning documentation to make `model_reasoning_effort` the only public reasoning control.
- [x] 4.2 Add a breaking migration note for replacing `thinking_budget_tokens`.
- [x] 4.3 Remove examples that show `thinking_budget_tokens` as valid configuration.

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`.
- [x] 5.2 Run focused runtime, LLM provider, daemon payload, and client DTO tests.
- [x] 5.3 Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] 5.4 Run `openspec validate remove-legacy-thinking-budget-control --strict`.
