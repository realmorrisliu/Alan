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

### 9) Repo Worker (Product-Layer Validation)

- Validate the first-party repo-worker package behavior without runtime forks.
- Focus: minimum coding loop, input-mode stability, and recovery/dedupe continuity.

### 10) Coding Steward Orchestration

- Validate Alan's home-root orchestration path separately from repo-worker-only checks.
- Focus: delegated launch shape, handle handoff, workspace boundaries, and bounded result integration.

## Unified Artifacts

Each harness scenario should produce:

1. Input script (Op sequence).
2. Event trace (Event JSONL).
3. Decision trace (governance/execution-backend/tool trace).
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
    coding_steward/
    repo_worker/
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

Run repo-worker harness scenarios:

```bash
bash scripts/harness/run_repo_worker_suite.sh
```

Run repo-worker CI-blocking subset:

```bash
bash scripts/harness/run_repo_worker_suite.sh --ci-blocking
```

Run coding-steward harness scenarios:

```bash
bash scripts/harness/run_coding_steward_suite.sh
```

Run coding-steward CI-blocking subset:

```bash
bash scripts/harness/run_coding_steward_suite.sh --ci-blocking
```

Run compaction harness scenarios:

```bash
bash scripts/harness/run_compaction_suite.sh
```

Run compaction CI-blocking subset:

```bash
bash scripts/harness/run_compaction_suite.sh --ci-blocking
```

Artifacts are written to:

```text
target/harness/autonomy/latest/
```

Self-eval artifacts are written to:

```text
target/harness/self_eval/latest/
```

Repo-worker artifacts are written to:

```text
target/harness/repo_worker/latest/
```

Coding-steward artifacts are written to:

```text
target/harness/coding_steward/latest/
```

Compaction artifacts are written to:

```text
target/harness/compaction/latest/
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
8. `repo_worker/minimum_loop`
   - Goal: validate the first-party repo-worker package executes task -> plan -> edit -> verify -> deliver.
   - Assertions: package completeness, loop artifacts, verification success, and delivery-contract validity.
9. `repo_worker/input_modes_stability`
   - Goal: validate `steer/follow_up/next_turn` semantics in repo-worker paths.
   - Assertions: queue behavior, turn-driving semantics, and buffering boundaries.
10. `repo_worker/recovery_dedupe`
   - Goal: validate restart restore and irreversible side-effect dedupe in repo-worker flow.
   - Assertions: checkpoint continuity and dedupe continuity after recovery.
11. `coding_steward/delegated_launch_contract`
   - Goal: validate steward-to-worker delegation resolves the package child launch contract.
   - Assertions: package child target resolution, bounded handles, and launch-shape stability.
12. `coding_steward/workspace_scope_binding`
   - Goal: validate nested cwd execution does not replace the delegated workspace boundary.
   - Assertions: nested cwd preservation and repo-root workspace binding.
13. `coding_steward/handle_handoff_profile`
   - Goal: validate non-inheriting default handoff plus explicit bound-handle transfer.
   - Assertions: no ambient parent transcript leakage and explicit parent context projection.
14. `coding_steward/bounded_result_integration`
   - Goal: validate delegated child results stay bounded in parent rollout and tape surfaces.
   - Assertions: bounded preview, bounded payload, and rollout-backed tool-call records.
15. `coding_steward/delegated_fallback_boundary`
   - Goal: validate unsupported delegated-execution or artifact-routing paths fail safe.
   - Assertions: unavailable delegated status and explicit artifact-routing rejection.
16. `autonomy/mobile_reconnect_snapshot`
   - Goal: validate reconnect snapshot contains dedupe and actionable resume state.
   - Assertions: latest submission hint, run resume action, pending-yield signal visibility.
17. `autonomy/mobile_notification_signal`
   - Goal: validate structured-input yield signal typing for reconnect notification UX.
   - Assertions: signal type mapping and informational-only semantics.
18. `autonomy/mobile_flaky_network_recovery`
   - Goal: validate gap handling and reconnect snapshot fallback under flaky connectivity.
   - Assertions: deterministic `gap=true` detection and non-mutating recovery reads.
19. `compaction/retry_after_trim`
   - Goal: validate trim-and-retry compaction audit semantics after an initial failure.
20. `compaction/soft_flush_success`
   - Goal: validate soft-threshold auto compaction writes durable memory before compacting.
   - Assertions: flush attempt/result/path stay consistent across event, read, reconnect, rollout,
     and recovery.
21. `compaction/soft_flush_skipped_no_durable_content`
   - Goal: validate empty-but-valid flush output becomes a structured skip instead of a failure.
   - Assertions: skip reason is visible, no warning-only fallback, and no daily note is written.
22. `compaction/soft_flush_failed_but_compaction_continues`
   - Goal: validate flush failure warning telemetry does not block compaction or turn completion.
   - Assertions: failure attempt is durable and linked while compaction still succeeds.
23. `compaction/hard_threshold_without_flush`
   - Goal: validate hard-threshold auto compaction bypasses pre-compaction memory flush.
   - Assertions: no flush event is emitted and `latest_memory_flush_attempt` remains empty.
   - Assertions: `Retry` result classification and cross-surface consistency.
24. `compaction/degraded_fallback`
   - Goal: validate degraded fallback remains visible as degraded across runtime surfaces.
   - Assertions: degraded strategy visibility and attempt/summary linkage.
25. `compaction/failure_preserves_tape`
   - Goal: validate failure path preserves tape state while still surfacing the attempt durably.
   - Assertions: no tape mutation, failed attempt remains externally observable.

Current fixture-backed executable scenarios in repository:

1. `autonomy/scheduler_wake`
2. `autonomy/reboot_resume`
3. `autonomy/dedup_side_effect`
4. `governance/recovery_boundary`
5. `repo_worker/minimum_loop`
6. `repo_worker/input_modes_stability`
7. `repo_worker/recovery_dedupe`
8. `repo_worker/governance_boundary`
9. `coding_steward/delegated_launch_contract`
10. `coding_steward/workspace_scope_binding`
11. `coding_steward/handle_handoff_profile`
12. `coding_steward/bounded_result_integration`
13. `coding_steward/delegated_fallback_boundary`
14. `autonomy/mobile_reconnect_snapshot`
15. `autonomy/mobile_notification_signal`
16. `autonomy/mobile_flaky_network_recovery`
17. `compaction/manual_success`
18. `compaction/retry_after_trim`
19. `compaction/degraded_fallback`
20. `compaction/failure_preserves_tape`
21. `compaction/repeated_failure_escalation`

## Release Gate Recommendations

Treat these as blocking checks:

1. `protocol/input_modes`
2. `autonomy/reboot_resume`
3. `autonomy/dedup_side_effect`
4. `governance/recovery_boundary`
5. `self_eval/profile_regression`
6. `repo_worker/minimum_loop`
7. `repo_worker/input_modes_stability`
8. `repo_worker/recovery_dedupe`
9. `repo_worker/governance_boundary`
10. `coding_steward/delegated_launch_contract`
11. `coding_steward/workspace_scope_binding`
12. `coding_steward/bounded_result_integration`
13. `coding_steward/delegated_fallback_boundary`
14. `compaction/retry_after_trim`
15. `compaction/degraded_fallback`
16. `compaction/failure_preserves_tape`
17. `compaction/repeated_failure_escalation`

## Acceptance Criteria

1. Critical regression scenarios are repeatable.
2. Failures are attributable to specific layers (protocol/policy/tool/compaction).
3. Harness outputs are usable as release gate inputs.
