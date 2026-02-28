# Scheduler Contract (Schedule / Sleep / Wake / Boot Recovery)

> Status: VNext contract (defines scheduling source-of-truth semantics for long-running execution).

## Goals

Scheduler is a Host/Daemon system capability responsible for:

1. Time-based run triggers (reminder / cron-like / delay).
2. Moving runs into recoverable `sleeping` state and waking them on time.
3. Recovering non-terminal schedule items after daemon/system restart.

This contract defines mechanism semantics, not business workflow content (owned by skills).

## Scope and Boundaries

### Scheduler MUST

1. Persist schedule and run-wake state.
2. Provide at-least-once due-time dispatch.
3. Prevent duplicate irreversible effects under redelivery through idempotency keys.
4. Record auditable scheduling event chains.

### Scheduler MUST NOT

1. Define business goals or steps.
2. Execute tools directly outside runtime state machine.
3. Override policy/sandbox decisions.

## Core Objects

### ScheduleItem

- `schedule_id`
- `task_id`
- `run_id`
- `trigger_type` (`at` / `interval` / `retry_backoff`)
- `next_wake_at`
- `status` (`waiting` / `due` / `dispatching` / `cancelled` / `completed` / `failed`)
- `attempt`
- `idempotency_key`

### SchedulerState (Minimal Persisted Fields)

- `last_dispatched_at`
- `last_completed_at`
- `last_error`
- `updated_at`

## ScheduleItem State Machine

1. `waiting -> due`: time reached or condition satisfied.
2. `due -> dispatching`: scheduler begins dispatch.
3. `dispatching -> waiting`: needs re-trigger (interval/backoff).
4. `dispatching -> completed`: one-shot task finished.
5. `dispatching -> failed`: non-recoverable failure.
6. `* -> cancelled`: explicit cancellation.

Constraints:

1. If process crashes in `dispatching`, redelivery after restart is allowed but must reuse same `idempotency_key`.
2. `completed/cancelled` are terminal states and must not auto-return to `waiting`.

## Scheduling Action Contracts

### `schedule_at(run_id, wake_at, payload)`

1. Create one-shot `ScheduleItem`.
2. If `wake_at <= now`, item may immediately become `due`.

### `sleep_until(run_id, wake_at)`

1. Set run state to `sleeping`.
2. Create or link the corresponding `ScheduleItem`.
3. On wake, transition run back to `running` or enqueue it for execution.

### `retry_with_backoff(run_id, policy)`

1. Compute next `next_wake_at` from `attempt`.
2. Persist backoff inputs (`attempt`, `base`, `factor`, `max`).

### `on_boot_resume()`

1. Scan all `waiting/due/dispatching` items after daemon starts.
2. Mark expired items as `due` and requeue.
3. Do not miss interrupted `dispatching` items.

## Alignment with Run State Semantics

1. `sleeping` runs must have explicit wake conditions.
2. `yielded` runs are not auto-advanced by scheduler (require external `resume`).
3. `running` runs must not be re-activated for the same execution fragment.

## Idempotency and Side-Effect Boundaries

1. Every dispatch attempt must carry stable `idempotency_key`.
2. Runtime/tool layer uses that key to dedupe side effects.
3. Under redelivery, repeated computation is acceptable; repeated irreversible side effects are not.

## Recovery Strategy

Boot recovery steps:

1. Load persisted `ScheduleItem` snapshot.
2. Normalize timed-out `dispatching` items back to retriable `due`.
3. Bulk-advance items where `next_wake_at <= now` to `due`.
4. Resume concurrent dispatch under configured limits.

## Observability and Audit

Per schedule cycle, record at least:

1. `schedule_id/run_id/task_id`
2. `trigger_type`
3. `wake_at/dispatched_at/completed_at`
4. `attempt/idempotency_key`
5. `result` (`success/retry/cancel/fail`)
6. `error` (if failed)

## Failure Degradation

1. Scheduler store temporarily unwritable: reject new schedules with recoverable errors.
2. Scheduler worker failure: auto-restart without losing persisted state.
3. Clock skew: emit `clock_skew_detected` warning; do not silently skip tasks.

## Acceptance Criteria

1. Due tasks fire before and after restarts without loss.
2. Redelivery does not duplicate irreversible side effects.
3. `sleep_until` run transitions are consistent and auditable.
4. `on_boot_resume` restores interrupted `dispatching` tasks.
