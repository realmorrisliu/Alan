# Capability Router Contract

> Status: VNext contract (defines unified routing across builtin / extension / bridge providers).

## Goals

Capability Router decouples "which capability is called" from "who implements it":

1. Runtime depends on capability names, not provider location.
2. Local and bridge providers share one invocation semantic.
3. Governance, idempotency, timeout, and auditing share one call pipeline.

## Non-Goals

1. Does not define business workflows (owned by skills).
2. Does not replace governance or execution-backend source of truth.
3. Does not require all discovery mechanisms at once.

## Roles and Responsibilities

### Router MUST

1. Select provider and route calls per capability.
2. Inject `idempotency_key`, `deadline`, `trace_context` consistently.
3. Emit unified call events and audit fields.
4. Perform bounded fallback only where safe.

### Router MUST NOT

1. Bypass `PolicyEngine` for side-effect capabilities.
2. Do silent retry + provider switch after side effects may have happened.
3. Modify turn state-machine semantics.

## Core Objects

### CapabilityCall

1. `call_id`
2. `task_id/run_id/session_id/turn_id`
3. `name` (capability)
4. `input`
5. `side_effect_mode`: `none | reversible | irreversible`
6. `idempotency_key`
7. `deadline_ms`
8. `route_mode`: `strict | best_effort | shadow`

### ProviderRef

1. `provider_id`
2. `source`: `builtin | extension_local | extension_bridge`
3. `priority`
4. `health_status`
5. `supports` (capability list + version)
6. `cost_class` (optional)

### RouteDecision

1. `selected_provider`
2. `fallback_chain`
3. `policy_action`: `allow | deny | escalate`
4. `reason`

## Registration and Discovery

Router maintains a unified registry from:

1. Runtime builtin providers.
2. Extension Host local providers.
3. Harness Bridge remote providers.

Constraints:

1. Providers must pass manifest/compatibility validation before registration.
2. Multiple providers can serve same capability with deterministic priority rules.

## Routing Algorithm (Recommended)

1. Normalize capability name/version constraints.
2. Find candidate providers.
3. Evaluate request through `PolicyEngine` (risk + context).
4. If `deny/escalate`, return governance result without execution.
5. If `allow`, score and choose provider:
   - prefer healthy low-latency local providers;
   - then order by `priority` and `cost_class`.
6. Dispatch and await result under `deadline_ms`.
7. Fallback only when safety conditions allow.
8. Record events and effect audit entries.

## Fallback Rules

1. `side_effect_mode=none`: fallback allowed under `best_effort`.
2. `reversible`: no automatic fallback unless capability declares transaction rollback support.
3. `irreversible`: automatic fallback forbidden; require governance path.
4. `shadow`: evaluation-only mode, no real side effects.

## Idempotency and Side Effects

1. Router must propagate `idempotency_key` to provider.
2. Dedupe hits must return `dedup_hit` and emit audit records.
3. Successful side-effect calls must persist `effect_refs` linked to `call_id`.

## Alignment with Turn / Run Semantics

1. Router is internal to a turn and must not create implicit turns.
2. Timeout/failure maps to recoverable error or yield path in current turn.
3. Restored runs must reuse original idempotency keys for retried side effects.

## Alignment with Input Modes

1. `steer` replanning may cancel unexecuted capability calls.
2. `follow_up/next_turn` affect planning only, not in-flight calls.
3. Router does not auto-reenter while yielded; waits for explicit `resume`.

## Events and Observability

Recommended events (or rollout-equivalent fields):

1. `capability_route_selected`
2. `capability_call_started`
3. `capability_call_completed`
4. `capability_call_failed`
5. `capability_call_deduped`
6. `capability_route_fallback`

Recommended fields per event:

1. `call_id/provider_id/capability`
2. `run_id/session_id/turn_id`
3. `policy_action`
4. `latency_ms/status`

## Performance and Backpressure

1. Router should enforce concurrency limits and queue protection.
2. Provider overload should return retryable errors, not block turn indefinitely.
3. Tail-latency providers may use circuit breaking and temporary degradation.

## Error Semantics

1. `provider_unavailable`: retryable or fallbackable.
2. `capability_not_found`: request-level failure.
3. `policy_denied` / `policy_escalated`: governance outcomes, no provider execution.
4. `deadline_exceeded`: execution-level error, may map to retry/backoff.

## Relationship with Harness

Harness should cover at least:

1. Multi-provider selection determinism.
2. No illegal fallback for side-effect calls.
3. Correct dedupe and recovery retry semantics.
4. Consistent event/audit fields across bridge and local providers.

## Acceptance Criteria

1. Runtime capability calls are provider-location agnostic.
2. High-risk calls cannot bypass governance at routing layer.
3. Side-effect calls remain idempotent under failure/recovery.
4. Router decisions are replayable, auditable, and testable.
