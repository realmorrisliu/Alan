## Why

Adding `model_reasoning_effort` showed that provider request controls currently
do not have a single runtime owner: config parsing, model metadata, runtime
overrides, turn-scoped overrides, daemon metadata, and provider adapters each
perform part of the same resolution. That works for one control, but it makes
the next control expensive and easy to implement inconsistently.

This change creates a clearer layering for request controls so Alan can add
future provider knobs without duplicating effective-value logic across runtime,
daemon, clients, and provider adapters.

## What Changes

- Introduce a runtime-owned request-control resolution layer that produces a
  single effective control set for a runtime session and each turn.
- Move reasoning-effort/model-default/legacy-budget precedence and validation
  out of `Config`, `RuntimeConfig`, and `turn_executor` call sites into that
  resolver.
- Keep `model_reasoning_effort` as the canonical user-facing config key and
  retain `thinking_budget_tokens` as a provider-specific compatibility input.
- Make provider adapters consume normalized request controls and only perform
  provider-wire projection.
- Make daemon/session metadata report the resolver output rather than
  independently reconstructing reasoning effort.
- Add contract tests that prevent reintroducing per-layer reasoning-effort
  resolution.
- No external API breaking change is intended.

## Capabilities

### New Capabilities

- `provider-request-controls`: Runtime-owned resolution and provider projection
  contract for model/request controls such as reasoning effort and thinking
  budget.

### Modified Capabilities

- None.

## Impact

- `crates/protocol`: canonical wire/control enums remain here; may gain a
  stable request-controls DTO if needed for turn-scoped overrides.
- `crates/runtime`: add the resolver/owner module; simplify `Config`,
  `RuntimeConfig`, runtime startup, turn execution, child-agent override merge,
  rollout/session metadata plumbing.
- `crates/llm`: provider adapters project normalized controls to provider
  payloads; they should not infer Alan-level defaults or precedence.
- `crates/alan`: daemon session create/fork/read responses use runtime resolver
  metadata instead of duplicating effective reasoning-effort rules.
- `clients/tui` and `clients/apple`: remain presentation and request DTO layers;
  no independent request-control semantics.
- Tests/docs: add focused resolver tests, provider projection tests, daemon
  payload contract tests, and update provider capability documentation.
