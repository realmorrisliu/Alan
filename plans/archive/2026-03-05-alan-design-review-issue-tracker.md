# Alan Design Review Issue Tracker (2026-03-05)

> Status: historical issue tracker.

## Context
This tracker records all findings from the architecture/design review and links each finding to a GitHub issue.

Confirmed priorities from maintainer:
- Goal: high-autonomy execution tools.
- Durability is critical.
- Rollback recoverability is not required for now (ephemeral rollback is acceptable).

## Findings → Issues

1. Bash capability classification can bypass governance (heuristic misclassification risk).
- Severity: High
- Issue: https://github.com/realmorrisliu/Alan/issues/40
- Labels: `bug`, `area/runtime`, `area/tools`

2. Sandbox implementation is weaker than V2 execution-boundary contract.
- Severity: High
- Issue: https://github.com/realmorrisliu/Alan/issues/41
- Labels: `bug`, `area/runtime`, `area/tools`, `area/docs`

3. Compaction trigger is coupled to generation max_tokens instead of context-window budget.
- Severity: Medium-High
- Issue: https://github.com/realmorrisliu/Alan/issues/42
- Labels: `enhancement`, `area/runtime`, `area/llm`

4. Resume can load wrong rollout via latest-file fallback.
- Severity: Medium
- Issue: https://github.com/realmorrisliu/Alan/issues/43
- Labels: `bug`, `area/cli-daemon`, `area/runtime`

5. Runtime silently degrades to non-durable in-memory session when recorder fails.
- Severity: Medium
- Issue: https://github.com/realmorrisliu/Alan/issues/44
- Labels: `bug`, `area/runtime`, `area/cli-daemon`

6. Per-turn skills/persona filesystem scans increase latency and add prompt-path side effects.
- Severity: Medium
- Issue: https://github.com/realmorrisliu/Alan/issues/45
- Labels: `enhancement`, `area/runtime`

7. Rollback is ephemeral; API/docs should make non-durable semantics explicit.
- Severity: Medium-Low
- Issue: https://github.com/realmorrisliu/Alan/issues/46
- Labels: `enhancement`, `area/runtime`, `area/docs`

8. Session binding persistence is non-atomic and corruption handling is too silent.
- Severity: Medium-Low
- Issue: https://github.com/realmorrisliu/Alan/issues/47
- Labels: `bug`, `area/cli-daemon`

## Suggested Execution Order

1. #40 and #41 (governance + execution boundary hardening)
2. #43 and #44 (recovery correctness + durability guarantees)
3. #42, #45, #46, #47 (predictability, latency, semantics, robustness)
