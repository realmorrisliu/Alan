# Harness KPI Output

Harness runners write KPI output to:

- `target/harness/autonomy/latest/kpi.json`
- `target/harness/coding_steward/latest/kpi.json`
- `target/harness/repo_worker/latest/kpi.json`
- `target/harness/compaction/latest/kpi.json`

Current shared KPI fields:

1. `suite`
2. `mode` (`all` or `ci_blocking`)
3. `total`
4. `passed`
5. `failed`
6. `skipped`
7. `pass_rate_percent`
8. `duration_secs`
9. `executed_scenarios`
10. `kpi_tag_counts`

Additional suite-specific fields may appear. For example, autonomy also records `profile`.

`executed_scenarios` is the ordered list of scenario ids that actually ran for
that suite invocation.

`kpi_tag_counts` aggregates the `kpi_tags` declared on executed scenario
fixtures so later dashboards can slice results without reparsing every fixture.

Per-scenario artifacts (under `target/harness/<suite>/latest/<scenario>/`):

1. `input_script.json`
2. `event_trace.log`
3. `decision_trace.jsonl`
4. `assertion_report.json`
