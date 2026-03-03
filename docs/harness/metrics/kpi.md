# Harness KPI Output

Autonomy harness runner writes KPI output to:

- `target/harness/autonomy/latest/kpi.json`

Current KPI fields:

1. `suite`
2. `mode` (`all` or `ci_blocking`)
3. `total`
4. `passed`
5. `failed`
6. `skipped`
7. `pass_rate_percent`
8. `duration_secs`

Per-scenario artifacts (under `target/harness/autonomy/latest/<scenario>/`):

1. `input_script.json`
2. `event_trace.log`
3. `decision_trace.jsonl`
4. `assertion_report.json`
