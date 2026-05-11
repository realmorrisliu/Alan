## Context

Alan already has connection profiles, model catalog metadata, and runtime-owned
reasoning-effort resolution for a selected model. That lets an operator choose a
model and effort, but it does not let the agent behave like a two-speed system:
normally quick and inexpensive, while able to recognize when a task deserves
deeper reasoning.

The desired model is inspired by System 1/System 2 from "Thinking, Fast and
Slow": System 1 is fast, automatic, and pattern-driven; System 2 is slower,
more deliberate, and used for complex reasoning or high-cost errors. Alan should
borrow the useful structure without reproducing the human failure mode where the
fast system is overconfident and the slow system fails to engage.

## Goals / Non-Goals

**Goals:**

- Let `agent.toml` configure System 1 and System 2 using existing connection
  profile IDs and optional reasoning-effort intent.
- Default to System 1 when safe, with explicit override and deterministic
  runtime gates.
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

### Decision: CognitiveRouter manages routing, not provider adapters

`CognitiveRouter` runs in runtime before provider dispatch. It selects a
cognitive system and connection profile, then delegates reasoning-effort
resolution to the existing request-control resolver for the selected profile.
Provider adapters receive a normal `GenerationRequest` and do not know why the
runtime selected it.

Alternatives considered:

- Make each provider adapter decide whether to use a faster or deeper model.
  This duplicates policy and breaks provider projection isolation.
- Put routing entirely in daemon/client code. This prevents runtime from
  auditing decisions consistently and makes CLI/TUI/macOS behavior diverge.

### Decision: Use deterministic gates plus System 1 self-escalation

Routing has three layers:

1. explicit turn/session/config overrides,
2. deterministic gates for known high-risk or high-complexity cases,
3. a System 1 attempt that can call internal `escalate_to_system2`.

This avoids a separate router LLM call on every turn while still letting the
fast model recognize when a task should not be answered quickly.

Alternatives considered:

- Always call a small classifier model first. This is simple but adds latency to
  every turn and makes the classifier another hidden model path.
- Hard-code all routing rules. This is predictable but will feel brittle and
  miss semantic complexity.

### Decision: Escalation is an internal virtual action

System 1 receives an internal-only action such as
`escalate_to_system2(reason, needed_context)`. When runtime captures it, the
fast draft is not accepted or streamed to the user. Runtime reruns System 2 with
the original task and bounded triage notes.

Alternatives considered:

- Ask System 1 to emit a JSON preamble. This is easier to parse but less aligned
  with Alan's action-oriented machine model.
- Let System 1 answer and append a note that deeper reasoning is needed. This
  leaks low-confidence output and weakens the user experience.

### Decision: Routing metadata is visible but bounded

Turn/session metadata includes selected cognitive system, profile id, model,
reasoning effort, routing source, and a short routing reason. It does not expose
private reasoning traces.

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

## Risks / Trade-offs

- System 1 fails to escalate -> Mitigate with deterministic gates and response
  guardrails that can force System 2 on contradiction or retry.
- Routing adds latency -> Mitigate by using System 1 by default and avoiding a
  separate classifier call.
- Profile switching complicates provider state -> Mitigate by resolving profile
  selection before constructing the request and by keeping provider continuation
  scoped to compatible profile/model boundaries.
- Metadata becomes noisy -> Mitigate with compact routing reasons and stable
  enum fields.
- Config ambiguity -> Mitigate by keeping `connection_profile` as the fallback
  default and making cognition config explicit.

## Migration Plan

1. Add cognition config parsing while preserving existing `connection_profile`
   behavior as the default single-system path.
2. Add routing metadata types and persistence with no behavior change.
3. Add deterministic routing gates and explicit overrides.
4. Add System 1 profile dispatch.
5. Add internal System 1 escalation and System 2 rerun.
6. Expose metadata in daemon/client DTOs.

Existing agents without a cognition block continue to use their current single
profile and request-control behavior.

## Open Questions

- Which deterministic gates belong in V1 versus later policy configuration.
- Whether `system2` should default to the same profile with higher effort when
  no explicit System 2 profile is configured.
- How much routing state can safely coexist with provider-managed continuation
  for stateful Responses profiles.
