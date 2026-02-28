# Interaction Inbox Contract (steer / follow_up / next_turn)

> Status: VNext contract (upgrades human input from a single `input` type to three first-class semantics).

## Goals

Enable concurrent human I/O and agent I/O without breaking turn consistency:

1. High-priority in-flight guidance (`steer`).
2. Deferred additions processed immediately after current execution (`follow_up`).
3. Intent queued for future turns only (`next_turn`).

## Input Categories

### `steer`

- Purpose: interrupt current path and trigger replanning.
- Timing: accepted during active turn.
- Semantics: inject at the next safe interruption point.

### `follow_up`

- Purpose: process immediately after current execution completes.
- Timing: can be queued during active turn.
- Semantics: does not interrupt current execution path.

### `next_turn`

- Purpose: context for the next user turn.
- Timing: can be queued at any time.
- Semantics: does not trigger immediate execution and does not interrupt current turn.

## Transport Representation (Protocol Recommendation)

Add mode to `turn/input` (or `Op::Input`):

1. `mode = steer | follow_up | next_turn`
2. Default should remain `steer` for backward compatibility.

Compatibility mapping:

1. Legacy `turn/steer` maps to `turn/input{mode=steer}`.
2. Legacy mode-less `Op::Input` maps to `mode=steer`.

## Queueing and Priority

Maintain three logical queues:

1. `Q_steer` (highest)
2. `Q_follow_up`
3. `Q_next_turn`

Priority rules:

1. Check `Q_steer` at tool-batch boundaries.
2. After turn terminal state, check `Q_steer` then `Q_follow_up`.
3. Inject `Q_next_turn` only when creating a new user turn.

## Execution Behavior Matrix

### During active turn + tool batch

1. `steer`: may skip remaining skippable tools and continue same turn with injected steer.
2. `follow_up`: queue only, no interruption.
3. `next_turn`: queue only, no interruption.

### During active turn + yielded

1. `resume` remains the only operation that advances yielded execution.
2. `steer/follow_up/next_turn` may queue but do not replace `resume`.

### Idle (no active turn)

1. `steer/follow_up` may optionally trigger a new turn (`trigger_turn=true`).
2. `next_turn` defaults to queue-only until explicit `turn/start`.

## Consistency Constraints

1. Only one active turn is allowed per session.
2. Queued inputs must not reorder committed `resume` causality.
3. In-turn injections must be auditable (`source/mode/enqueued_at/applied_at`).

## Backpressure and Capacity

1. Each queue should have caps (for example `steer <= 16`).
2. On overflow, reject latest input with recoverable error.
3. Rejections must emit observable warning/error (never silent drop).

## Event Recommendations

Optional events (or equivalent rollout fields):

1. `input_queued`: `{mode, queue_size}`
2. `input_applied`: `{mode, turn_id}`
3. `input_dropped`: `{mode, reason}`

If new events are deferred, equivalent audit records must still exist.

## Planning-Quality Impact

To avoid "feature 2 discovered only after feature 1 is done":

1. `follow_up` entries can be previewed in future-intent context.
2. Preview may influence planning but must not force immediate execution.
3. Mark preview source as queued intent to avoid confusion with active instruction.

## Migration Recommendations

1. Phase 1: add `mode` field with default compatibility behavior.
2. Phase 2: mark `turn/steer` as compatibility alias.
3. Phase 3: enforce regression gating of three-mode semantics in harness.

## Acceptance Criteria

1. `steer` reliably interrupts and replans during tool batches.
2. `follow_up` does not block current execution and is consumed after completion.
3. `next_turn` does not trigger immediate execution and appears in next turn context.
4. Queue overflow behavior is predictable and auditable.
