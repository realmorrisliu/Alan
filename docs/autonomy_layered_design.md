# Alan Autonomy Layered Architecture

> Status: VNext design document (aligns abstraction boundaries and rollout order).
> Scope: explains which capabilities belong in runtime / daemon / skills / harness and aligns them with Alan philosophy and protocol.

## 1. Background and Problem

Alan already provides:

1. Core turn state machine, yield/resume, and tool orchestration.
2. Session persistence and daemon restart session recovery.
3. Policy-over-sandbox governance path.
4. Steering interruption during tool batches.

When Alan evolves into a general long-running agent foundation, new system-level requirements appear:

1. Agents must schedule future execution points (reminders, timed jobs, sleep/wake).
2. Work must continue after restart, not just recover session shells.
3. Human input and agent execution must coexist without strict serialization bottlenecks.
4. Prompt/policy optimization must be gated by verifiable harness loops, not production drift.

These are reliability and boundary-design problems, not issues solvable by prompt or skill alone.

## 2. Design Goals

1. Clarify layered responsibilities while keeping kernel small and stable.
2. Push recoverability/auditability/idempotency into system layers.
3. Keep replaceable and evolvable workflows in skill/tool layers.
4. Put effectiveness judgment into offline harness evaluation and regression gates.

Non-goals:

1. Turning runtime into a business workflow engine.
2. Making skills responsible for system reliability (scheduling/restart/idempotency).
3. Enabling direct self-modifying prompt updates in production paths.

## 3. Alignment with Alan Philosophy

This design follows Alan's established principles:

1. **AI Turing Machine**: runtime advances state, not business semantics (`docs/architecture.md`).
2. **Small Stable Kernel**: prioritize invariants and state-machine integrity (`docs/spec/kernel_contract.md`).
3. **Skills-first + Extensions-ready**: skills orchestrate workflows; extensions implement pluggable capabilities; tools remain atomic side effects.
4. **Human-in-the-End**: humans own outcomes and intervene at boundaries/exceptions (`docs/human_in_the_end.md`).
5. **Policy over Sandbox**: policy decides "should"; sandbox enforces "can" (`docs/policy_over_sandbox.md`).
6. **UNIX philosophy**: generic kernel mechanisms + composable tools/skills.

## 4. Layer Placement Rules

For each new capability:

1. If it needs determinism + persistence + crash recovery, place it in `daemon/runtime`.
2. If workflow logic must be replaceable, place it in `skills`.
3. If it performs external side effects, place it in `tools`.
4. If it evaluates quality/effectiveness, place it in `harness`.

## 5. Layered Architecture Overview

### L0: Protocol

Responsibilities:

1. Define input/output semantics (`Op` / `Event`).
2. Provide stable contracts for multi-client systems.
3. Formalize control-plane semantics (turn/yield/resume/steer).

References: `crates/protocol/src/op.rs`, `crates/protocol/src/event.rs`, `docs/spec/app_server_protocol.md`.

### L1: Runtime Kernel

Responsibilities:

1. Turn state machine and tool loop.
2. Tape/rollout source-of-truth consistency.
3. Policy integration and yield/resume symmetry.
4. Checkpoint primitives and idempotency semantics for run recovery.

Not responsible for:

1. Cross-day scheduling.
2. Product-level reminder/queue strategy.
3. Domain-specific workflow DSL.

### L2: Daemon/Host

Responsibilities:

1. Runtime lifecycle orchestration (start/recover/reconnect).
2. Persistent task queue and scheduler (timing/retry/sleep-wake).
3. Task-level object management (Task/Run metadata).
4. Input inbox routing (`steer/follow_up/next_turn`).

### L3: Skills + Tools

Responsibilities:

1. Skills define workflow decomposition and tool usage.
2. Tools execute external actions (file/command/network/API).
3. Provide replaceable capabilities within kernel constraints.

Boundaries:

1. Skills do not own system recovery/idempotency guarantees.
2. Tools cannot bypass policy/sandbox.

### L4: Harness

Responsibilities:

1. System-level regression scenarios (long-run/recovery/boundary/safety).
2. Prompt/profile evaluation and promotion gating.
3. Metric-based comparison (success/cost/violation/recovery rates).

## 6. Capability-to-Layer Mapping

| Capability | Primary Layer | Notes |
| --- | --- | --- |
| Timed reminders / scheduled execution | L2 Daemon | needs durable scheduler queue + reboot recovery |
| Idle sleep / wake | L2 Daemon + L1 Runtime | daemon controls wake timing, runtime ensures resume semantics |
| Continue after system reboot | L2 + L1 | daemon rebuilds run plane, runtime restores from checkpoint |
| Parallel human message input | L0 + L2 + L1 | protocol defines semantics, daemon routes inbox, runtime executes |
| Follow-up intent influences current planning | L2 Inbox + L1 planning hook | queued follow-up preview injected into planning context |
| Self-bootstrap (self-edit/restart/continue) | L1/L2 mechanisms + L3 skill flow | mechanism built-in, flow orchestrated by skills |
| Prompt-level self-eval optimization | L4 Harness | offline promotion only, no direct online self-modification |

## 7. Core Object Model (Task / Run / Session / Turn)

Extend current Session/Turn with Task/Run:

1. **Task**: business goal, constraints, owner, SLA.
2. **Run**: one execution attempt, retryable and recoverable.
3. **Session**: current bounded context window for a run.
4. **Turn**: minimal state-advancement unit.

Recommended Run states:

1. `pending`
2. `running`
3. `sleeping` (waiting for time/event)
4. `yielded` (waiting for human/structured input)
5. `completed`
6. `failed`
7. `cancelled`

Principles:

1. `sleeping` and `yielded` recover across process restart.
2. Sessions can rotate while Run remains continuous.
3. Task is owner-facing; Run is system-execution-facing.

## 8. Three Critical Execution Chains

### 8.1 Durable Scheduling Chain (Reminder / Sleep / Wake)

Minimum capabilities:

1. `schedule_at(run_id, wake_at, payload)`
2. `sleep_until(run_id, wake_at)`
3. `retry_with_backoff(run_id, policy)`
4. `on_boot_resume()` (scan and recover due/incomplete runs on daemon boot)

Storage requirements:

1. Persist task/run records (JSON/SQLite first, pluggable later).
2. Persist `last_checkpoint_id` and `next_wake_at`.
3. Enforce idempotent wake behavior per run.

### 8.2 Parallel Input Chain (Human IO / Agent IO)

Split input into:

1. `steer`: high-priority in-flight guidance.
2. `follow_up`: process immediately after current execution.
3. `next_turn`: context for the next user turn only.

Key behavior:

1. During tool batches, `steer` may skip remaining skippable tools and trigger replanning.
2. `follow_up` does not block current execution but may preview future intent.
3. Runtime preserves turn consistency; daemon owns queue priority and delivery semantics.

### 8.3 Reboot Continuation Chain (Durable Run)

Checkpoint should include at least:

1. `run_id/task_id/session_id`
2. Current turn state and pending yield
3. Recently confirmed side effects (with idempotency key)
4. Next-step intent and restore entry

Recovery flow:

1. Daemon loads all non-terminal runs at startup.
2. Normalize `running/sleeping/yielded` run states.
3. Runtime restores from checkpoint and dedupes side effects by idempotency key.

## 9. Abstraction Boundary for Self-Bootstrap Capabilities

Example: "auto-update + reboot + verify" and "self-edit + restart + continue":

1. Whether such actions are allowed is a governance decision (L1 + policy).
2. How they are executed (commands/checks/retries) is skill orchestration (L3).
3. Cross-restart continuity is guaranteed by durable run + scheduler (L2/L1).

Therefore:

1. Safe continuation is a system capability.
2. What to continue is a skill strategy.

## 10. Put Self-Eval and Prompt Evolution into Harness

Goal is verifiable evolution, not online drift:

1. Maintain `prompt_profile` sets (`baseline`, `candidate`, etc.).
2. Run fixed scenario suites in harness (protocol/recovery/boundaries/long-run).
3. Produce metrics: success rate, cost, boundary violations, recovery success, duplicate side-effect rate.
4. Promote candidate only if thresholds pass.

Suggested harness suites:

1. `autonomy/scheduler_recovery`
2. `autonomy/parallel_input_semantics`
3. `autonomy/reboot_continuation`
4. `autonomy/prompt_profile_regression`

## 11. Integration with Existing Alan Code Baseline

Reusable current foundations:

1. `turn_driver` / `turn_executor` / `tool_orchestrator`: turn execution, steering, yield/resume.
2. `session_store` + `AppState::ensure_sessions_recovered`: session-shell recovery after daemon restart.
3. `policy` + `sandbox` + `approval`: boundary decisions and human takeover mechanisms.

Recommended new modules:

1. `daemon/task_store`: Task/Run persistence.
2. `daemon/scheduler`: timing and wake executor.
3. `runtime/checkpoint`: run-level checkpoint and restore interface.
4. `protocol` extensions: explicit input mode + run/task metadata.

## 12. Phased Rollout Plan

### Phase 1: Protocol + Object Introduction (Compatibility First)

1. Add optional `task_id/run_id` in metadata.
2. Define input modes (`steer/follow_up/next_turn`) with server compatibility mapping first.
3. Keep existing `/sessions` API working.

### Phase 2: Minimal Durable Scheduling Loop

1. Implement `task_store + scheduler`.
2. Support `schedule_at/sleep_until/on_boot_resume`.
3. Deliver minimal e2e: timed wake -> execution -> emitted events.

### Phase 3: Durable Run + Reboot Continuation

1. Add checkpoints and idempotency keys.
2. Auto-recover runs after restart.
3. Add tests for "no duplicate external side effects."

### Phase 4: Harness Evaluation Loop

1. Add autonomy suites.
2. Add prompt-profile evaluation and promotion rules.
3. Integrate key metrics into release gates.

## 13. Acceptance Criteria

1. After restart, non-terminal runs recover into executable state.
2. High-risk actions are not duplicated across recovery/retry.
3. Human input can be injected during execution with stable semantics.
4. Scheduled tasks trigger within bounded timing error and are auditable.
5. Prompt/profile evolution passes harness gates before promotion.
