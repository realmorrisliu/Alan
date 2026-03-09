# Durable Run Contract (Checkpoint / Idempotency / Side-Effect Recovery)

> Status: VNext contract (defines run-level continuity and safe side-effect recovery).

## Goals

Durable Run solves:

1. Run continuation after process/system restart.
2. No duplicate irreversible side effects during recovery.
3. Coherent semantics across replay / rollback / fork.

## Scope and Boundaries

### Durable Run MUST

1. Persist critical run execution state (`checkpoint`).
2. Bind side effects to idempotency keys and result records.
3. Provide restore flow (`bootstrap -> reconcile -> resume`).

### Durable Run MUST NOT

1. Depend on the model to "remember" recovery semantics.
2. Backfill history via unaudited paths.
3. Skip governance boundaries during recovery.

## Core Objects

### RunCheckpoint

- `checkpoint_id`
- `task_id`
- `run_id`
- `session_id`
- `turn_id` (optional)
- `run_state` (`running/sleeping/yielded/...`)
- `pending_yield` (optional)
- `next_action_hint`
- `created_at`

### EffectRecord

- `effect_id`
- `run_id`
- `tool_call_id`
- `idempotency_key`
- `effect_type` (`file/network/process/...`)
- `request_fingerprint`
- `result_digest`
- `status` (`applied` / `failed` / `unknown`)
- `applied_at`

## Checkpoint Write Timing

Write checkpoints at least on:

1. Turn start (restore entry established).
2. Before entering `yielded/sleeping`.
3. After each critical side effect is confirmed (with synchronized effect record).
4. Turn terminal boundaries (`completed/failed/interrupted`).

Requirements:

1. Checkpoint/effect ordering must preserve causal reconstruction.
2. Persistence failure must be visible (warning/error), never silent.

## Recovery Flow Contract

Suggested `restore_run(run_id)` flow:

1. Read latest `RunCheckpoint`.
2. Validate and normalize run state.
3. Rebuild pending yield / scheduler links as needed.
4. Resume runtime from explicit restore entry.

Normalization examples:

1. Crash during side-effect call: mark effect `unknown` and dedupe-check.
2. Crash during dispatching: move to retriable state instead of dropping run.

## Idempotency Semantics

1. Same logical side effect must reuse same `idempotency_key`.
2. If recovery detects previously applied successful key:
   - skip physical re-execution,
   - emit dedupe-hit audit entry.
3. For `unknown` side effects:
   - default to governance-protected path (`escalate`) or safe retry policy.

## Side-Effect Recovery Strategy

Suggested by effect type:

1. **File writes**: dedupe by content hash/mtime/fingerprint.
2. **Network calls**: external idempotency API + local effect records.
3. **Process execution**: conservative default for non-idempotent commands.

## Relationship with Replay / Rollback / Fork

### Replay

1. Default replay re-emits events only, no side-effect replay.
2. Explicit re-execute must create a new run and re-enter idempotency protections.

### Rollback

1. Rollback updates rollback-safe context only; effect audit chain is immutable.
2. Repeated post-rollback actions still go through idempotency checks.
3. Rollback itself is non-durable: it changes in-memory context and audit markers, but does not survive runtime restart.

### Fork

1. Fork inherits required context/summary but not applied-side-effect authority.
2. Fork run must isolate idempotency namespace.

## Governance Alignment

1. High-risk recovery actions still go through policy + boundary checks.
2. "In recovery" does not imply "approval bypass."
3. Auto-recovery decisions must include auditable reasons.

## Observability and Audit

Minimum searchable fields:

1. `run_id/checkpoint_id/effect_id`
2. `recovery_attempt`
3. `dedupe_hit` (bool)
4. `decision` (`resume/retry/escalate/fail`)
5. `reason`

## Failure Degradation

1. Unreadable checkpoint: mark `failed_recovery` and require human intervention.
2. Undecidable effect status: enter safe mode, do not continue high-risk path.
3. Repeated recovery failure over threshold: stop auto-retry and alert.

## Acceptance Criteria

1. Runs continue from checkpoint after restart.
2. Successfully applied side effects are not re-executed.
3. rollback/fork/replay do not break side-effect audit chain.
4. Recovery decisions are fully traceable.
