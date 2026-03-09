# Execution Model (Task / Run / Session / Turn)

> Status: VNext target contract (compatible with current Session/Turn model and extended for autonomy).

## Goals

Alan must support both:

1. Short interactive tasks (instant Q&A).
2. Long-running autonomous execution (across context windows and time slices).

Execution hierarchy is therefore: `Task -> Run -> Session -> Turn`.

## Object Hierarchy

### Task (Business-Level Goal)

- Full delegated objective (goal + constraints + owner).
- Lifecycle typically spans multiple Runs/Sessions.
- Typical fields:
  - `task_id`
  - `goal`
  - `constraints` (budget, policy, timeline)
  - `owner`
  - `success_criteria`

### Run (Single Execution Attempt)

- One retryable attempt under a Task.
- Can end from interruption, sleep, timeout, or escalation; next attempt creates a new Run.
- Typical fields:
  - `run_id`
  - `task_id`
  - `attempt`
  - `started_at` / `ended_at`
  - `status` (`pending/running/sleeping/yielded/succeeded/failed/cancelled`)

### Session (Bounded Context Container)

- Run execution window within a specific time slice.
- Constrained by model context window; can be compacted/archived/rotated.
- Typical fields:
  - `session_id`
  - `run_id`
  - `workspace_id`
  - `tape`
  - `rollout`

### Turn (Smallest State-Advancement Unit)

- One execution cycle triggered by `Op::Turn`.
- Includes input, LLM generation, tool batch, yield/resume, terminal events.

## Current-to-Target Mapping

Current Alan centers on Session/Turn. Mapping:

- Current `Session` ~= target `Session`
- Ongoing interaction inside one session ~= single `Run`
- `Task` is not yet first-class protocol object (owned by higher-level orchestration)

Migration principle: introduce Task/Run metadata without breaking existing Op/Event semantics.

Related contracts:

1. Scheduling: [`scheduler_contract.md`](./scheduler_contract.md)
2. Input routing: [`interaction_inbox_contract.md`](./interaction_inbox_contract.md)
3. Recovery/idempotency: [`durable_run_contract.md`](./durable_run_contract.md)

## Turn State Machine

### States

1. `Idle`
2. `Running`
3. `Yielded`
4. `Completed`
5. `Interrupted`
6. `Failed`

### Transitions

1. `Idle --(Op::Turn)--> Running`
2. `Running --(policy escalate / virtual input request)--> Yielded`
3. `Yielded --(Op::Resume)--> Running`
4. `Running --(done)--> Completed`
5. `Running --(Op::Interrupt)--> Interrupted`
6. `Running|Yielded --(fatal error)--> Failed`

## Operation Semantic Contract

### `turn`

- Starts a new turn and establishes boundary.
- Must emit `turn_started` and terminal boundary (`turn_completed` or error boundary).

### `input` (First-Class Input Modes)

`input` must support:

1. `steer`: injected guidance into active running turn.
2. `follow_up`: process after current execution completes.
3. `next_turn`: context for future turn only.

Constraints:

1. During tool batches, check for `steer` at least after each tool completion.
2. `follow_up/next_turn` must not break current turn causality.
3. In `yielded`, `resume` remains sole advancement entry.

### `resume`

- Valid only while turn is `Yielded`.
- Must include `request_id` matching exactly one pending request.

### `interrupt`

- Terminates current execution as soon as possible.
- Turn transitions to `Interrupted`, and future turns remain possible.

### `compact`

- Compresses session context to release window pressure.
- Must be explicit or explainable via automatic strategy.

### `rollback`

- Rolls back rollback-safe state for recent N turns.
- Rollback is in-memory only and non-durable across runtime restart.
- Must write auditable markers (no silent history rewrite).

## Concurrency and Queueing

1. One active turn per session.
2. One active runtime per workspace (host-layer constraint).
3. Recommended input priority: `steer > follow_up > next_turn`.
4. Queue overflow must be visible; no silent drop.

## Recovery and Replay

1. After Session/Run recovery, latest turn terminal state must be decidable.
2. `Yielded` turns may continue waiting for `resume` after recovery.
3. `Sleeping` runs must be awakened by scheduler (no implicit continuation).
4. Replay semantics must distinguish:
  - event replay only (no side-effect re-execution)
  - explicit re-execution (idempotency protection required)

## Migration Plan (Recommended)

1. **Phase 1**: add optional `run_id/task_id` to Session metadata.
2. **Phase 2**: add `mode` to `input` (default `steer` for compatibility).
3. **Phase 3**: add scheduler + durable-run checkpoints.
4. **Phase 4**: gate autonomy scenarios in harness releases.

## Acceptance Criteria

1. Turn state machine is unambiguous and testable.
2. `steer/follow_up/next_turn` semantics are stable and non-conflicting.
3. steering/resume/interrupt control paths do not conflict.
4. Cross-session/run recovery does not duplicate side effects.
