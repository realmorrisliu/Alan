## Context

The `model_reasoning_effort` change is cross-cutting because the control is
visible at multiple layers:

- `alan_protocol` defines `ReasoningEffort` and turn-scoped override fields.
- `alan_runtime::Config` parses user config, resolves model metadata, validates
  config conflicts, and currently derives effective model defaults.
- `RuntimeConfig` copies effective reasoning state from `Config`, while
  `AgentConfig` has explicit override bookkeeping for both effort and legacy
  thinking budget.
- `turn_executor` chooses between turn override, runtime effort, and legacy
  budget immediately before building `GenerationRequest`.
- `alan_llm::GenerationRequest` carries both `thinking_budget_tokens` and
  `ReasoningControls`, and provider adapters contain additional precedence and
  budget-to-effort mapping logic.
- The daemon/session stores and clients carry `reasoning_effort` as metadata,
  but they should not own the rules that decide what is effective.

That shape is understandable for an incremental feature, but it mixes intent,
effective value, provider capability validation, and provider-wire projection.
In Rust terms, several crates now know too much about the same state machine.
The owner should be the crate that has all required inputs and controls runtime
semantics: `alan-runtime`.

## Goals / Non-Goals

**Goals:**

- Establish `alan-runtime` as the single owner for effective request-control
  resolution.
- Separate four concepts in types: user intent, override precedence, resolved
  runtime controls, and provider-wire projection.
- Preserve the current external config and daemon API for reasoning effort.
- Keep provider adapters responsible for provider-specific payload shape, not
  Alan-level defaults or precedence.
- Make the next request control require changes in predictable locations:
  protocol DTO if it is externally visible, runtime resolver, provider
  projection, tests/docs.

**Non-Goals:**

- Do not remove `model_reasoning_effort`.
- Do not remove `thinking_budget_tokens` in this change; it remains a
  compatibility input.
- Do not redesign the model catalog wholesale.
- Do not generate OpenAPI/client bindings in this change, although the design
  should leave room for that later.
- Do not change provider behavior except where the current behavior relies on
  duplicated or inconsistent resolution.

## Decisions

### Decision: Add a Runtime Request-Control Resolver

Add a new runtime module, tentatively `crates/runtime/src/request_controls.rs`,
with types similar to:

```rust
pub struct RequestControlIntent {
    pub reasoning_effort: Option<ReasoningEffort>,
    pub thinking_budget_tokens: Option<u32>,
}

pub struct RequestControlOverrides {
    pub session: RequestControlIntent,
    pub turn: RequestControlIntent,
}

pub struct ResolvedRequestControls {
    pub reasoning: ReasoningControls,
    pub reasoning_source: Option<RequestControlSource>,
    pub diagnostics: Vec<RequestControlDiagnostic>,
}

pub enum RequestControlSource {
    TurnOverride,
    SessionOverride,
    AgentConfig,
    ModelDefault,
    LegacyBudget,
    ProviderDefault,
}
```

The resolver takes `Config`, resolved provider type/capabilities, model catalog
metadata, session override, and turn override. It returns a single normalized
`ResolvedRequestControls` value used by runtime startup metadata, per-turn
`GenerationRequest`, logging, rollout/session metadata, and daemon responses.

Alternative considered: keep the logic on `Config`.
That keeps code local to config parsing, but `Config` does not own turn-scoped
overrides, provider capabilities, or dispatch-time validation. It would keep
request semantics in a type whose job should be loading configuration.

### Decision: Treat `Config` as Input, Not Resolver

`Config` should keep user-facing fields and local config validation, including
rejecting explicit `model_reasoning_effort` plus `thinking_budget_tokens` in
the same config file. It should not expose methods that look like final runtime
truth for request controls, such as `effective_model_reasoning_effort`.

The replacement shape should be:

- `Config` exposes raw intent fields.
- Model catalog exposes model capability/default metadata.
- The resolver combines raw config, model metadata, provider capabilities, and
  overrides into effective controls.

Alternative considered: rename the existing `effective_*` methods and keep
them as helper methods.
That still invites daemon/runtime code to recompute final values from partial
inputs. The resolver should be the only path for effective request controls.

### Decision: RuntimeConfig Stores Intent or Snapshot, Not Independent Truth

`RuntimeConfig` currently carries `model_reasoning_effort` and
`thinking_budget_tokens` as mutable effective values. That creates a second
truth beside `Config` and requires explicit sync helpers.

The target shape is:

- `RuntimeConfig` may store launch/session override intent.
- Runtime startup computes a `ResolvedRequestControls` snapshot for metadata.
- Turn execution recomputes or derives a turn snapshot through the resolver
  when turn overrides are present.
- Any persisted metadata stores the resolver output with source information,
  not another independent effective field.

Alternative considered: store only the resolved session value and mutate it for
turns.
That makes turn overrides hard to reason about and loses the reason why a value
was selected. A small immutable snapshot per request is easier to test.

### Decision: Provider Adapters Project, They Do Not Decide Precedence

`alan_llm::GenerationRequest` should carry normalized controls. Provider
adapters may map those controls to provider payloads:

- OpenAI Responses: `reasoning.effort`
- OpenAI Chat Completions: `reasoning_effort`
- Anthropic: effort-to-budget presets or explicit budget when budget is the
  normalized control
- Gemini: effort-to-`thinkingLevel`/`thinkingBudget` based on model family
- OpenRouter/compatible: extension fields when enabled by runtime metadata

Adapters should not infer Alan-level defaults from legacy budgets or consume
`extra_params.reasoning_effort` as a competing source when canonical controls
are present. Compatibility parsing can remain temporarily, but it should feed
the same normalized control path or be removed in a follow-up.

Alternative considered: move all provider projection into runtime.
That would make runtime depend on provider payload details and weaken the LLM
crate boundary. The adapter owns wire shape; runtime owns Alan semantics.

### Decision: Keep Protocol Narrow

`alan_protocol` should continue to own portable wire enums and client-visible
DTOs such as `ReasoningEffort`. It should not own model catalog logic or
provider capability resolution. If a broader request-control DTO becomes
necessary for turns, it should represent user intent only, not an effective
runtime value.

### Decision: Make Contract Tests Guard the Layering

Add focused tests that fail when effective request-control rules leak back into
the wrong layer:

- resolver unit tests for precedence, config conflict, model default,
  unsupported explicit effort, provider unsupported effort, and legacy budget.
- turn-executor tests proving it consumes `ResolvedRequestControls`.
- provider adapter tests proving canonical controls override provider-specific
  extra params and that adapters do not synthesize Alan defaults.
- daemon contract tests proving responses mirror resolver metadata.

## Risks / Trade-offs

- Resolver becomes a central module with several inputs -> Keep the API small,
  immutable, and heavily unit-tested.
- Temporary duplication may exist during migration -> Do the refactor in
  phases, first introducing the resolver and then deleting old helpers.
- Provider-specific behavior can be over-centralized accidentally -> Keep only
  precedence and validation in runtime; wire payload mapping stays in
  `alan-llm`.
- Existing tests may need broad updates -> Prefer helper constructors for
  runtime/session test fixtures so future request controls do not require
  editing dozens of `reasoning_effort: None` literals.
- Model catalog does not cover every provider/model -> Resolver must support
  "unknown model metadata" explicitly and distinguish provider default from
  model-catalog default.

## Migration Plan

1. Add `request_controls` module and resolver tests without changing behavior.
2. Route runtime startup metadata and turn execution through the resolver.
3. Replace `Config::effective_model_reasoning_effort` and
   `validate_reasoning_effort_for_resolved_model` call sites with resolver
   calls, then make old helpers private or remove them.
4. Collapse `RuntimeConfig` request-control fields into intent/snapshot fields
   and remove sync helper duplication.
5. Normalize `GenerationRequest` so adapters read `ReasoningControls` as the
   canonical input; retain compatibility builder methods while they populate
   the normalized field.
6. Update daemon/session metadata to use resolver output.
7. Update docs and add contract tests.

Rollback is straightforward because the external config/API shape is preserved:
revert the internal resolver changes and restore direct field plumbing.

## Open Questions

- Should `ResolvedRequestControls` live in `alan-runtime` only, or be exported
  for daemon metadata construction?
- Should provider capability metadata evolve from booleans to typed projection
  modes in this change, or remain a follow-up?
- Should `thinking_budget_tokens` be removed from `GenerationRequest` now or
  deprecated in place for one release?
