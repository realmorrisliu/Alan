## Context

Alan already has connection profiles, model catalog metadata, and runtime-owned
reasoning-effort resolution for a selected model. That lets an operator choose a
provider-backed model and effort, but it does not let the agent behave like a two-speed system:
normally quick and inexpensive, while able to recognize when a task deserves
deeper reasoning.

The desired model is inspired by System 1/System 2 from "Thinking, Fast and
Slow": System 1 is fast, automatic, and pattern-driven; System 2 is slower,
more deliberate, and used for complex reasoning or high-cost errors. Alan should
borrow the useful structure without reproducing the human failure mode where the
fast system is overconfident and the slow system fails to engage.

## Goals / Non-Goals

**Goals:**

- Let `agent.toml` configure System 1 and System 2 as model bindings layered
  above provider/credential configuration, with optional reasoning-effort intent.
- Honor the configured default route when no override or deterministic gate
  applies, falling back to System 1 when no default is configured.
- Let the System 1 model self-escalate through an internal-only runtime action.
- Suppress fast drafts when escalation occurs and rerun the original task on
  System 2 with bounded triage notes.
- Record every routing decision in audit-friendly metadata.

**Non-Goals:**

- Run two models in parallel by default.
- Turn System 1/System 2 into separate child agents.
- Move routing decisions into provider adapters.
- Replace existing request-control resolution.
- Expose hidden reasoning content as routing metadata.

## Decisions

### Decision: Split provider availability from cognitive model binding

Configuration should have two conceptual layers:

1. provider/credential availability, which describes authenticated AI providers
   and the models Alan can use through them,
2. cognitive model binding, which selects which available model acts as System 1
   and which acts as System 2.

This keeps System 1/System 2 out of provider auth and avoids copying provider
credentials into the cognition block. Existing connection profiles can remain a
compatibility path and can feed the provider/model availability layer, but
cognitive routing should resolve to a concrete provider, credential scope,
model, and request-control intent before dispatch.

Alternatives considered:

- Make System 1/System 2 reference full connection profiles directly. This is
  simple but keeps model selection tangled with credential/profile ownership.
- Create separate provider config for each system. This duplicates credentials
  and makes model swaps harder to reason about.

### Decision: CognitiveRouter manages routing, not provider adapters

`CognitiveRouter` runs in runtime before provider dispatch. It selects a
cognitive system and concrete model binding, then delegates reasoning-effort
resolution to the existing request-control resolver for the selected binding.
Provider adapters receive a normal `GenerationRequest` and do not know why the
runtime selected it.

Alternatives considered:

- Make each provider adapter decide whether to use a faster or deeper model.
  This duplicates policy and breaks provider projection isolation.
- Put routing entirely in daemon/client code. This prevents runtime from
  auditing decisions consistently and makes CLI/TUI/macOS behavior diverge.

### Decision: Use safety-first routing gates plus System 1 self-escalation

Routing has five layers:

1. explicit System 2 override, which can always choose the deeper route,
2. deterministic gates for known high-risk or high-complexity cases,
3. explicit System 1 override, which is honored only when no deterministic gate
   requires System 2,
4. configured default route, which may select System 1 or System 2,
5. System 1 fallback attempt that can call internal `escalate_to_system2`.

This avoids a separate router LLM call on every turn while still letting the
fast model recognize when a task should not be answered quickly.
It also prevents clients from forcing the fast route through conditions the
runtime already knows require System 2.

Alternatives considered:

- Always call a small classifier model first. This is simple but adds latency to
  every turn and makes the classifier another hidden model path.
- Let System 1 overrides outrank deterministic gates. This makes manual control
  simple but undercuts the safety boundary.
- Hard-code all routing rules. This is predictable but will feel brittle and
  miss semantic complexity.

### Decision: Escalation is an internal virtual action with a side-effect boundary

System 1 receives an internal-only action such as
`escalate_to_system2(reason, needed_context)`. When runtime captures it, the
fast draft is not accepted or streamed to the user. Runtime reruns System 2 with
the original task and bounded triage notes.

For V1, System 1's auto route should behave like human fast cognition in a
controlled environment: it can form an impression, gather read-only context, or
ask to escalate, but it should not perform irreversible external side effects
before System 2 has taken over or the runtime has accepted the System 1 plan.
Runtime must therefore withhold side-effecting tools from the unaccepted System
1 phase. If a read-only tool was used before escalation, System 2 receives
those results as part of the rerun context. If a side effect already happened
after runtime accepted a System 1 execution phase, or due to an external client
state change, System 2 must continue from the observed post-side-effect state
rather than replaying the original task as if nothing changed.

This acceptance is a runtime-owned commit point, not a user-confirmation prompt.
Before that commit point, System 1 may perform model-internal reasoning,
calculation, planning, draft generation that is not streamed as accepted output,
and read-only tool use. After runtime accepts the fast route, System 1 can
execute permitted side-effecting tools under the same governance and policy
rules as any other routed turn. Human confirmation is still required only when
the active governance or tool policy would have required it anyway.

Alternatives considered:

- Ask System 1 to emit a JSON preamble. This is easier to parse but less aligned
  with Alan's action-oriented machine model.
- Let System 1 answer and append a note that deeper reasoning is needed. This
  leaks low-confidence output and weakens the user experience.
- Treat runtime acceptance as human approval. This would preserve safety but
  would add unnecessary interruptions and undercut autonomous operation.

### Decision: Routing metadata is visible but bounded

Turn/session metadata includes selected cognitive system, model binding id,
provider, model, reasoning effort, routing source, and a short routing reason.
It does not expose private reasoning traces.

Alternatives considered:

- Hide routing completely. This undermines trust and makes debugging difficult.
- Store full chain-of-thought. This violates provider and product boundaries.

### Decision: Keep first version single-runtime and single-active-turn

The first implementation changes provider selection within the existing turn
loop. It does not spawn a second agent, merge transcripts, or run competing
models.

Alternatives considered:

- Use child agents for System 2. This is useful later for delegated work but too
  heavy for normal routing.
- Run both systems and compare. This is expensive and creates answer selection
  complexity.

### Decision: Treat provider-native continuation as an optimization partitioned by model binding

Alan's tape-level continuation remains compatible across System 1 and System 2
because runtime can project the accepted tape into whichever model binding is
selected. Provider-native continuation state, such as a Responses
`previous_response_id`, is not assumed to be portable across different cognitive
model bindings.

Runtime must partition, clear, or replay provider-native continuation when the
selected cognitive binding changes provider family, credential scope, model, or
other continuation-affecting settings. Reuse is allowed only inside a proven
compatible binding boundary.

Alternatives considered:

- Assume all configured System 1/System 2 models can share provider continuation.
  This is too provider-specific and risks invalid request chains.
- Disable provider-native continuation whenever cognition is enabled. This is
  safe but gives up useful stateful-provider efficiency.

## Implementation Ownership

This change is primarily a runtime-routing change, not a prompt-only or
skill-only feature.

- Runtime owns `CognitiveRouter`, route precedence, the System 1 acceptance
  commit point, escalation handling, routing metadata, and provider-native
  continuation partitioning.
- Tool governance owns read-only versus side-effecting capability classification
  and enforces the unaccepted System 1 side-effect gate before tool execution.
- Prompts describe the selected cognitive role, speculative boundary, and
  internal escalation contract to the model, but prompts are not the security or
  side-effect boundary.
- Skills may adapt their guidance to the routed context, but they do not decide
  the cognitive route, bypass tool governance, or expose `escalate_to_system2`
  as a normal user tool.
- Daemon and client surfaces carry override intent and display bounded routing
  metadata; they do not independently decide provider/model routing.

## Risks / Trade-offs

- System 1 fails to escalate -> Mitigate with safety-first deterministic gates
  and response guardrails that can force System 2 on contradiction or retry.
- Routing adds latency -> Mitigate by using System 1 by default and avoiding a
  separate classifier call.
- Model binding switching complicates provider state -> Mitigate by resolving
  the binding before constructing the request and by partitioning provider
  continuation by compatible provider/model/credential boundaries.
- System 1 mutates workspace before escalation -> Mitigate by withholding
  side-effecting tools until runtime accepts the fast route or routes to System
  2.
- Metadata becomes noisy -> Mitigate with compact routing reasons and stable
  enum fields.
- Config ambiguity -> Mitigate by keeping `connection_profile` as the fallback
  default and making cognition config explicit.

## Migration Plan

1. Add cognition config parsing while preserving existing `connection_profile`
   behavior as the default single-system path.
2. Add routing metadata types and persistence with no behavior change.
3. Add deterministic routing gates and explicit overrides, with System 2 gates
   superseding forced System 1 intent.
4. Add System 1 model binding dispatch.
5. Add internal System 1 escalation and System 2 rerun.
6. Expose metadata in daemon/client DTOs.

Existing agents without a cognition block continue to use their current single
profile and request-control behavior. When cognition is enabled, both System 1
and System 2 model bindings must resolve successfully.

## Open Questions

- Which deterministic gates belong in V1 versus later policy configuration.
