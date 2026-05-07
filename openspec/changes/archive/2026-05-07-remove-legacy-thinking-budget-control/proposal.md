## Why

Alan now has a canonical reasoning-effort control and a runtime-owned
request-control resolver. Keeping `thinking_budget_tokens` as a legacy public
fallback leaves two user-facing ways to request reasoning behavior, which weakens
the single-owner boundary and keeps compatibility semantics that Alan no longer
needs.

## What Changes

- **BREAKING** Remove `thinking_budget_tokens` as a supported user-facing
  request-control field in agent config, session/turn API payloads, client DTOs,
  and public `GenerationRequest` construction.
- **BREAKING** Reject old `thinking_budget_tokens` configuration or request
  payloads with a clear error instead of mapping them to effort or provider
  budgets.
- Keep `model_reasoning_effort` as the only public reasoning control.
- Keep provider-internal budget projection where a provider requires budgets,
  but derive those budgets only from canonical reasoning effort and model/provider
  metadata.
- Remove OpenRouter-specific legacy `thinking_budget_tokens` projection behavior.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `provider-request-controls`: Remove legacy thinking-budget compatibility from
  the canonical request-control contract.
- `openrouter-provider-adapter`: Remove OpenRouter request projection for public
  `thinking_budget_tokens` fallback.

## Impact

- `crates/runtime`: config parsing, request-control intent, resolver inputs,
  child-agent overrides, rollout/session metadata, and validation errors.
- `crates/protocol`: session or turn request DTOs if they expose
  `thinking_budget_tokens`.
- `crates/llm`: `GenerationRequest` public fields/builders and provider adapter
  tests that currently accept legacy budget input.
- `crates/alan`: daemon create/fork/read payloads and error behavior for old
  request fields.
- `clients/tui` and `clients/apple`: remove any DTO field or UI affordance for
  legacy thinking budgets.
- Documentation and tests: update examples to use `model_reasoning_effort` only
  and add negative coverage for rejected legacy fields.
