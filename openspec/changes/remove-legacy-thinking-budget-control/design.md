## Context

`provider-request-controls` centralized request-control ownership around
canonical reasoning effort, but it intentionally kept `thinking_budget_tokens` as
a public compatibility input. That compatibility now works against the desired
architecture: clients, configs, runtime launch paths, and provider adapters must
continue to preserve a second reasoning-control vocabulary that can drift from
`model_reasoning_effort`.

Some providers still require budget-shaped wire fields. This change removes the
public compatibility control, not provider-internal budget projection.

## Goals / Non-Goals

**Goals:**

- Make `model_reasoning_effort` the only user-facing reasoning control.
- Reject legacy `thinking_budget_tokens` config and request payloads with clear
  migration guidance.
- Remove public `GenerationRequest.thinking_budget_tokens` compatibility paths.
- Keep provider adapters able to derive provider-native budgets from canonical
  effort where the provider API requires budgets.
- Keep request-control precedence owned by the runtime resolver.

**Non-Goals:**

- Do not remove canonical reasoning effort.
- Do not remove provider-native budget wire fields where Anthropic, Gemini, or
  another provider requires them.
- Do not add automatic migration or alias behavior for old configs.
- Do not change unrelated provider settings or OpenRouter provider identity.

## Decisions

1. Public legacy budget inputs are rejected, not ignored.

   Rejecting old `thinking_budget_tokens` values makes stale configuration
   visible and avoids silent reasoning behavior changes. Ignoring the field would
   be easier to deploy but would make debugging model behavior harder.

2. Provider-internal budget projection remains behind normalized controls.

   Anthropic extended thinking and Gemini 2.5 thinking may require budget-shaped
   payloads. Those budgets should be derived from `ReasoningEffort`, model
   metadata, and provider rules inside adapter projection, not accepted as a
   separate public control.

3. The resolver remains the single request-control owner.

   Config parsing can reject obsolete fields, and provider adapters can validate
   provider-specific wire constraints, but no daemon, client, or adapter should
   recompute Alan-level effort precedence.

4. Migration is explicit and breaking.

   Existing users must replace `thinking_budget_tokens` with
   `model_reasoning_effort`. Alan should return an actionable error instead of
   rewriting old configuration automatically.

## Risks / Trade-offs

- Existing local configs may fail to load. -> Error messages and migration docs
  must identify `model_reasoning_effort` as the replacement.
- Provider-specific behavior may change for users who tuned exact budgets. -> The
  model catalog and provider mapping tests must document effort-to-budget presets.
- Removing public fields can create broad fixture churn. -> Use helper builders
  and DTO constructors to keep tests focused on behavior.

## Migration Plan

1. Update specs and docs to define `model_reasoning_effort` as the only public
   reasoning control.
2. Remove or reject `thinking_budget_tokens` from config, protocol DTOs, daemon
   payloads, client DTOs, and public LLM request builders.
3. Preserve provider-native budget projection derived from canonical effort.
4. Add negative tests for legacy field rejection and positive tests for
   effort-derived provider budgets.
5. Release with a breaking migration note: replace `thinking_budget_tokens` with
   the closest supported `model_reasoning_effort` value.

## Open Questions

- Which effort preset should replace common legacy budget examples in docs?
- Should the error include provider-specific advice when the old budget was close
  to a known effort preset?
