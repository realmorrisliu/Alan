# Alan Harness

> Status: VNext validation framework blueprint.

## Goals

Harness is Alan's system-level validation framework, not a collection of unit tests for one crate.

It focuses on:

1. Long-running stability.
2. Control under complex toolchains and governance boundaries.
3. Behavioral continuity after compaction / rollback / recovery.
4. Protocol consistency across multi-client integrations.

## Why Harness Is Needed

Unit and integration tests alone do not fully cover:

1. State drift in multi-loop tool-call executions.
2. Compaction degradation under context growth.
3. Event compensation gaps after disconnect/reconnect.
4. Human handoff paths when policy boundaries are hit.

Harness turns these runtime risks into reproducible regression scenarios.

## Scenario Layers

### 1) Protocol Conformance

- Op sequence and Event sequence consistency.
- Focus: turn boundaries, yield/resume, interrupt, and `events/read` gap behavior.

### 2) Loop Stability

- Long tool-chain turns (10+ tool loops).
- Steering insertion, interruption recovery, timeout retries.
- Goal: no dead loops, no duplicate side effects, no hanging states.

### 3) Governance Boundaries

- Validation of allow/deny/escalate decisions.
- Critical commit boundaries must trigger human handoff.

### 4) Compaction Robustness

- Continuous execution after auto/manual compaction.
- Summary fidelity and critical todo retention.

### 5) Memory Durability

- Memory write/read and cross-session recovery.
- Pre-compaction memory flush validation (when enabled).

### 6) Replay & Rollback

- Replay must not duplicate side effects.
- Post-rollback event/state consistency.

### 7) Autonomy (Scheduler & Recovery)

- Scheduled triggers, sleep/wake, reboot recovery.
- Focus: no task loss, no duplicate irreversible effects under redelivery, bounded timing error.

### 8) Self-Eval (Prompt/Profile Governance)

- Offline comparison of candidate prompt/profile sets.
- Focus: verify gains do not regress cost, risk, or boundary violations.

### 9) Coding Reference (Product-Layer Validation)

- Validate reference coding-agent behavior without runtime forks.
- Focus: minimum coding loop, input-mode stability, and recovery/dedupe continuity.

## Unified Artifacts

Each harness scenario should produce:

1. Input script (Op sequence).
2. Event trace (Event JSONL).
3. Decision trace (policy/sandbox/tool trace).
4. Assertion report (pass/fail + diff).

## Key Metrics (KPI)

1. Turn success rate and interruption recovery rate.
2. Mean tool-loop count and failure distribution.
3. Compaction trigger rate and post-compaction failure rate.
4. Escalation hit rate and human resolution latency.
5. Event-gap detection rate and recovery success rate.

## Suggested Rollout Order

1. Protocol and lifecycle baseline (Protocol + Loop).
2. Governance boundary and compaction regressions.
3. Memory durability and replay/rollback suites.
4. Autonomy and self-eval release gating.

## Relationship with Existing Tests

- `docs/testing_strategy.md`: protocol source of truth and base contract tests.
- Harness: adds long-running, failure-path, and system-level validation.

Relationship summary:

1. Contract tests guarantee interface stability.
2. Harness guarantees runtime behavior under realistic stress.

## Suggested Directory Layout

```text
docs/harness/
  README.md
  scenarios/
    protocol/
    loop/
    governance/
    compaction/
    memory/
    replay/
    autonomy/
    coding/
    self_eval/
  metrics/
    kpi.md
```

## Current Executable Runner

Run all autonomy/governance recovery scenarios:

```bash
bash scripts/harness/run_autonomy_suite.sh
```

Run CI-reliable blocking subset only:

```bash
bash scripts/harness/run_autonomy_suite.sh --ci-blocking
```

Run self-eval profile regression (baseline vs candidate):

```bash
bash scripts/harness/run_self_eval_suite.sh --mode local
```

Run coding reference harness scenarios:

```bash
bash scripts/harness/run_coding_reference_suite.sh
```

Run coding reference CI-blocking subset:

```bash
bash scripts/harness/run_coding_reference_suite.sh --ci-blocking
```

Artifacts are written to:

```text
target/harness/autonomy/latest/
```

Self-eval artifacts are written to:

```text
target/harness/self_eval/latest/
```

Coding reference artifacts are written to:

```text
target/harness/coding_reference/latest/
```

## Executable Scenario Matrix (MVP)

Start with an automatically executable batch (each must include input script, assertions, and artifacts):

1. `protocol/input_modes`
   - Goal: validate `steer/follow_up/next_turn` protocol and queue semantics.
   - Assertions: apply order, queue limits, observable drop behavior.
2. `loop/steer_during_tool_batch`
   - Goal: validate steer interruption and remaining-tool skip semantics during tool batches.
   - Assertions: skip markers, replanning behavior, turn consistency.
3. `autonomy/scheduler_wake`
   - Goal: validate `sleep_until/schedule_at` trigger timing.
   - Assertions: trigger timing, run-state transitions, audit field completeness.
4. `autonomy/reboot_resume`
   - Goal: validate run recovery after daemon restart.
   - Assertions: recovery of non-terminal runs, checkpoint continuity.
5. `autonomy/dedup_side_effect`
   - Goal: validate side-effect dedupe under redelivery.
   - Assertions: irreversible actions with same idempotency key execute once.
6. `governance/recovery_boundary`
   - Goal: validate boundary enforcement during recovery paths.
   - Assertions: no automatic boundary bypass, traceable yield/resume.
7. `self_eval/profile_regression`
   - Goal: compare baseline vs candidate prompt profiles.
   - Assertions: promotion only if thresholds pass (success rate, cost, boundary violations).
8. `coding/minimum_loop`
   - Goal: validate reference coding scaffold executes task -> plan -> edit -> verify -> deliver.
   - Assertions: scaffold completeness, loop artifacts, verification success.
9. `coding/input_modes_stability`
   - Goal: validate `steer/follow_up/next_turn` semantics in coding-flow paths.
   - Assertions: queue behavior, turn-driving semantics, and buffering boundaries.
10. `coding/recovery_dedupe`
   - Goal: validate restart restore and irreversible side-effect dedupe in coding flow.
   - Assertions: checkpoint continuity and dedupe continuity after recovery.
11. `autonomy/mobile_reconnect_snapshot`
   - Goal: validate reconnect snapshot contains dedupe and actionable resume state.
   - Assertions: latest submission hint, run resume action, pending-yield signal visibility.
12. `autonomy/mobile_notification_signal`
   - Goal: validate structured-input yield signal typing for reconnect notification UX.
   - Assertions: signal type mapping and informational-only semantics.
13. `autonomy/mobile_flaky_network_recovery`
   - Goal: validate gap handling and reconnect snapshot fallback under flaky connectivity.
   - Assertions: deterministic `gap=true` detection and non-mutating recovery reads.

Current fixture-backed executable scenarios in repository:

1. `autonomy/scheduler_wake`
2. `autonomy/reboot_resume`
3. `autonomy/dedup_side_effect`
4. `governance/recovery_boundary`
5. `coding/minimum_loop`
6. `coding/input_modes_stability`
7. `coding/recovery_dedupe`
8. `coding/governance_boundary`
9. `autonomy/mobile_reconnect_snapshot`
10. `autonomy/mobile_notification_signal`
11. `autonomy/mobile_flaky_network_recovery`

## Release Gate Recommendations

Treat these as blocking checks:

1. `protocol/input_modes`
2. `autonomy/reboot_resume`
3. `autonomy/dedup_side_effect`
4. `governance/recovery_boundary`
5. `self_eval/profile_regression`
6. `coding/minimum_loop`
7. `coding/input_modes_stability`
8. `coding/recovery_dedupe`

## Acceptance Criteria

1. Critical regression scenarios are repeatable.
2. Failures are attributable to specific layers (protocol/policy/tool/compaction).
3. Harness outputs are usable as release gate inputs.
