## Context

Alan already carries provider-returned thinking/reasoning through tape,
streaming events, and provider capability metadata. It also has a
`thinking_budget_tokens: Option<u32>` runtime setting that is copied into every
`GenerationRequest`. That field is useful for budget-native providers such as
Anthropic, but it is not the control shape exposed by newer flagship models.

OpenAI's current Responses API exposes `reasoning.effort` with named values
including `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`. Gemini uses
`thinkingLevel` for Gemini 3 and `thinkingBudget` for Gemini 2.5. Anthropic
extended thinking remains budget-based and has provider-specific constraints
such as minimum budget and `max_tokens` interactions.

Codex models this as a typed `ReasoningEffort` enum plus model-catalog metadata:
each model declares supported/default reasoning efforts, config can set
`model_reasoning_effort`, turn start can override `effort`, and request builders
map the selected effort into provider-native request fields. Alan should adopt
the same broad shape, but keep its existing provider capability and
connection-profile architecture.

References used while designing this change:

- `~/Developer/codex/codex-rs/protocol/src/openai_models.rs`
- `~/Developer/codex/codex-rs/core/src/client.rs`
- `~/Developer/codex/codex-rs/core/src/session/turn_context.rs`
- `~/Developer/codex/codex-rs/model-provider/src/amazon_bedrock/catalog.rs`
- OpenAI Responses API reasoning documentation
- Anthropic extended thinking documentation
- Gemini API thinking documentation
- OpenRouter Responses reasoning documentation

## Goals / Non-Goals

**Goals:**

- Introduce a typed, serializable `ReasoningEffort` with values
  `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`.
- Make explicit reasoning effort the main cross-provider control surface.
- Add model-catalog metadata for supported reasoning efforts and default
  reasoning effort.
- Validate selected effort against the resolved model before dispatch.
- Preserve existing `thinking_budget_tokens` as a provider-specific compatibility
  field, not as the primary model-control abstraction.
- Map the canonical effort to OpenAI, ChatGPT, Anthropic, Gemini,
  OpenAI-compatible, and OpenRouter-style providers according to explicit
  provider capabilities.
- Expose effective reasoning effort in session metadata so clients can display
  and restore it.

**Non-Goals:**

- Do not expose raw chain-of-thought by default. This change controls provider
  reasoning effort, not UI visibility policy for reasoning output.
- Do not require every provider to support every effort level.
- Do not make `thinking_budget_tokens` a stable cross-provider API.
- Do not add automatic online model discovery as part of this change.
- Do not add a separate plan-mode reasoning effort until Alan has a stable
  runtime plan-mode boundary that needs it.

## Decisions

### Decision: Define canonical reasoning controls in `alan-protocol`

Add a shared `ReasoningEffort` enum in `alan-protocol`, serialized as lowercase
strings:

- `none`
- `minimal`
- `low`
- `medium`
- `high`
- `xhigh`

Add a `ReasoningControls` transport/runtime type with:

- `effort: Option<ReasoningEffort>`
- `summary: Option<ReasoningSummaryMode>` if the implementation also exposes
  provider reasoning summaries in this change
- `budget_tokens: Option<u32>` as a provider-specific escape hatch carried from
  the existing `thinking_budget_tokens`

Rationale: `alan-protocol` is the shared boundary for daemon clients,
operations, child-agent launch specs, and future Apple/TUI surfaces. Keeping
the enum there prevents each crate from inventing slightly different strings.

Alternative considered: define the enum only in `alan-llm`. That would work for
adapter internals but would force runtime/API/client layers to use untyped
strings.

### Decision: Keep `thinking_budget_tokens`, but make effort primary

Add `model_reasoning_effort: Option<ReasoningEffort>` to agent/runtime config.
Keep `thinking_budget_tokens` for existing configs and budget-native providers.

Resolution rules:

1. An explicit turn/session/agent `model_reasoning_effort` wins over model
   defaults.
2. If no explicit effort is set and `thinking_budget_tokens` is set, adapters
   may translate the budget into provider-native controls as they do today.
3. If a user explicitly sets both `model_reasoning_effort` and
   `thinking_budget_tokens`, Alan rejects the configuration as ambiguous.
4. If neither is set, Alan uses the resolved model's default reasoning effort
   when the model catalog declares one.

Rationale: Existing users with `thinking_budget_tokens` should not lose
functionality, but new configuration should be readable across providers.

Alternative considered: delete `thinking_budget_tokens`. That would simplify
the API but would regress Anthropic and Gemini 2.5 control where raw budgets are
the native shape.

### Decision: Extend the model catalog with supported/default effort metadata

Replace the current single `supports_reasoning: bool` decision point with:

- `supports_reasoning: bool` retained for compatibility and broad capability
  checks
- `supported_reasoning_efforts: Vec<ReasoningEffortPreset>`
- `default_reasoning_effort: Option<ReasoningEffort>`
- optional provider mapping metadata for budget-native models, such as
  `effort_budget_tokens` per supported effort

For existing catalog entries that only set `supports_reasoning = true`, the
implementation should initially derive a conservative default set such as
`low`, `medium`, `high` with `medium` as default, then explicitly override
models whose supported set differs.

Rationale: Reasoning support is no longer binary. Codex's model catalog shows
that clients and runtime both need supported/default effort metadata to avoid
offering invalid choices.

Alternative considered: hard-code effort support by provider family. That would
miss model-level differences such as `none` support or fixed-effort models.

### Decision: Validate before provider dispatch

Effective effort selection should happen before building the provider request:

1. select effort from explicit runtime/turn override, legacy budget, or model
   default;
2. validate that the resolved model supports the selected effort;
3. if valid, put the selected effort on `GenerationRequest`;
4. if invalid, fail before the provider call with a clear error.

Tier C providers may use "drop with warning" only for non-critical metadata, but
explicit user-selected reasoning effort is not non-critical; unsupported effort
must be rejected.

Rationale: Failing after an upstream provider error gives poor diagnostics and
risks inconsistent behavior across adapters.

Alternative considered: always send the effort and let providers reject it. That
would be simpler but makes Alan's model catalog and capability matrix less
trustworthy.

### Decision: Map effort to provider-native controls in adapters

Provider adapters own the final projection:

- `openai_responses`: send `reasoning.effort`; send `reasoning.summary` only if
  summary support is implemented and the model supports it.
- `chatgpt`: use the Responses-shaped reasoning field only when current
  live-validated capability flags allow it.
- `openai_chat_completions`: send `reasoning_effort` for models/endpoints that
  support it.
- `anthropic_messages`: map effort to budget presets when no explicit
  `thinking_budget_tokens` is set; enforce Anthropic minimum budget,
  `max_tokens > budget_tokens`, and temperature rules.
- `google_gemini_generate_content`: for Gemini 3 model families, map effort to
  `thinkingLevel`; for Gemini 2.5 families, map effort to `thinkingBudget` via
  catalog presets; map `none` only where the model supports disabling thinking.
- `openai_chat_completions_compatible`: send `reasoning_effort` only for models
  whose catalog entry declares support for that extension; otherwise reject.
- `openrouter`: after the OpenRouter SDK-backed provider lands, map effort to
  the SDK/provider field supported by the selected OpenRouter endpoint and model.

Rationale: The kernel should expose one typed control, but each provider has a
different native shape.

Alternative considered: convert all efforts into token budgets. That repeats
Alan's current issue in reverse and loses fidelity for OpenAI/Gemini 3.

### Decision: Add session and turn override surfaces, but keep precedence simple

Add optional reasoning effort to:

- `agent.toml` / resolved agent config as `model_reasoning_effort`
- daemon create-session and fork-session requests as a session-scoped override
- `Op::Turn.context` as a per-turn override if clients need one-turn control
- child-agent spawn runtime overrides so delegated agents can select a supported
  effort explicitly

Precedence:

1. turn context override
2. session/create/fork or child-agent runtime override
3. resolved agent config
4. model-catalog default
5. provider default when no Alan default is known

Rationale: This mirrors Codex's effective model/effort flow while fitting Alan's
existing session and child-agent launch surfaces.

Alternative considered: only support `agent.toml`. That is enough for a default
but blocks UI model pickers and per-session experimentation.

## Risks / Trade-offs

- Provider support changes quickly -> Keep support metadata in Alan's model
  catalog/overlays and validate through live provider harnesses.
- `none` is not universally supported -> Treat `none` as an explicit effort,
  not as "unset", and reject it when the model does not declare support.
- Existing `thinking_budget_tokens` configs conflict with new effort settings ->
  Reject when both are explicitly set and document the migration path.
- Budget mappings are approximate for Anthropic/Gemini 2.5 -> Store mappings in
  model/provider metadata rather than hard-coding universal thresholds.
- Client UIs may offer stale options -> Expose supported/default effort metadata
  from daemon APIs and generated/checked client types.

## Migration Plan

1. Add protocol types for `ReasoningEffort`, optional
   `ReasoningSummaryMode`, and `ReasoningControls`.
2. Extend model catalog TOML parsing and bundled entries with
   `supported_reasoning_efforts`, `default_reasoning_effort`, and optional
   provider budget mappings.
3. Add `model_reasoning_effort` to config loading, profile/agent overlays,
   runtime config, and explicit runtime override tracking.
4. Add session/turn/child-agent override fields and include effective effort in
   session metadata responses.
5. Extend `GenerationRequest` with canonical reasoning controls and update turn
   construction to resolve and validate the effective effort before dispatch.
6. Update each provider adapter's request mapping and remove the current
   OpenAI-specific heuristic that infers `reasoning_effort` solely from
   `thinking_budget_tokens`.
7. Add unit tests for config precedence, conflict handling, catalog validation,
   provider request mapping, and unsupported-effort rejection.
8. Add live harness cases for OpenAI Responses/Chat, Anthropic, Gemini, and
   compatible providers where credentials are available.
9. Update docs and examples to prefer `model_reasoning_effort = "medium"` and
   document `thinking_budget_tokens` as provider-specific.

Rollback before release is straightforward: remove the new config/API fields and
restore adapter-local budget mapping. After release, keep deserialization for
the new fields and ignore them only with warnings if a provider regression
requires temporarily disabling mappings.

## Open Questions

- Should Alan implement `ReasoningSummaryMode` in the same change, or keep this
  first pass effort-only and rely on existing thinking output handling?
- Which budget presets should Alan assign to `minimal`, `low`, `medium`, and
  `high` for Anthropic and Gemini 2.5 models?
- Should model-catalog overlays be allowed to remove supported effort levels, or
  only replace the full model entry?
- Should turn-scoped reasoning effort persist for subsequent turns, as Codex's
  app-server `effort` does, or be strictly one-turn-only in Alan?
