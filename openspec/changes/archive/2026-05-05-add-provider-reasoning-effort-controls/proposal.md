## Why

Alan currently exposes `thinking_budget_tokens`, but modern reasoning models
increasingly use named reasoning effort levels rather than raw token budgets.
This makes Alan's cross-provider control surface hard to reason about and forces
adapters such as OpenAI Chat/Responses to infer effort from arbitrary budget
thresholds.

## What Changes

- Introduce a typed `ReasoningEffort` control with the canonical levels
  `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`.
- Add model-catalog metadata for supported reasoning efforts and default
  reasoning effort, following the shape used in `~/Developer/codex`.
- Add agent/runtime configuration for explicit reasoning effort, while keeping
  `thinking_budget_tokens` as a provider-specific compatibility field.
- Update turn construction so every `GenerationRequest` carries explicit
  reasoning controls instead of relying on `extra_params` or budget heuristics.
- Adapt provider adapters to map Alan's canonical effort to provider-native
  controls:
  - OpenAI Responses and OpenAI Chat Completions: `reasoning.effort` /
    `reasoning_effort`.
  - ChatGPT managed Responses: same Responses-shaped field set, constrained by
    its live-validated capability matrix.
  - Anthropic Messages: named effort maps to provider budget presets unless an
    explicit `thinking_budget_tokens` override is present.
  - Google Gemini GenerateContent: Gemini 3 uses `thinkingLevel`, Gemini 2.5
    uses `thinkingBudget`.
  - OpenAI-compatible and OpenRouter-style providers use explicit supported
    extension fields only when the provider/model declares support.
- Add validation so unsupported model/provider effort combinations fail clearly
  or degrade with a warning only where the provider capability contract permits
  it.
- Update docs and examples from token-budget-first reasoning control to
  effort-first reasoning control.

## Capabilities

### New Capabilities

- `provider-reasoning-effort-controls`: Defines Alan's canonical reasoning
  effort model, configuration resolution, model-catalog support metadata,
  provider-native mapping rules, validation behavior, and verification
  requirements.

### Modified Capabilities

- None. No archived OpenSpec capability currently owns provider reasoning
  controls or model-catalog reasoning metadata.

## Impact

- `crates/llm`: shared `ReasoningEffort` types, `GenerationRequest` fields,
  provider adapter request mapping, tests, and live harness coverage.
- `crates/runtime`: config loading/overlay behavior, runtime config,
  model-catalog metadata, turn request construction, child-agent inheritance,
  and provider capability handling.
- `crates/alan`: daemon session create/response metadata, connection/runtime
  profile visibility where applicable, CLI overrides, and docs examples.
- `clients/tui` and `clients/apple`: any model picker or session configuration
  UI that should expose supported/default reasoning efforts.
- `docs/spec/provider_capability_contract.md`, testing docs, and AGENTS config
  examples.
