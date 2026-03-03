#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  scripts/harness/run_coding_reference_suite.sh [--ci-blocking]

Options:
  --ci-blocking   Run only scenarios marked as blocking for CI gates.
USAGE
}

mode="all"
if [[ "${1:-}" == "--ci-blocking" ]]; then
    mode="ci_blocking"
    shift
fi

if [[ $# -gt 0 ]]; then
    usage
    exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
artifact_root="$repo_root/target/harness/coding_reference/latest"
mkdir -p "$artifact_root"
rm -rf "$artifact_root"/*

fixtures=(
    "docs/harness/scenarios/coding/minimum_loop.json"
    "docs/harness/scenarios/coding/input_modes_stability.json"
    "docs/harness/scenarios/coding/recovery_dedupe.json"
    "docs/harness/scenarios/coding/governance_boundary.json"
)

suite_start_epoch="$(date +%s)"
total=0
passed=0
failed=0
skipped=0

extract_json_string_field() {
    local file="$1"
    local key="$2"
    grep -E "^[[:space:]]*\"${key}\":" "$file" \
        | head -n1 \
        | sed -E 's/^[[:space:]]*"[^"]+":[[:space:]]*"([^"]*)".*$/\1/'
}

extract_json_bool_field() {
    local file="$1"
    local key="$2"
    grep -E "^[[:space:]]*\"${key}\":" "$file" \
        | head -n1 \
        | sed -E 's/^[[:space:]]*"[^"]+":[[:space:]]*(true|false).*/\1/'
}

for fixture_rel in "${fixtures[@]}"; do
    fixture_path="$repo_root/$fixture_rel"
    if [[ ! -f "$fixture_path" ]]; then
        echo "Missing harness fixture: $fixture_rel" >&2
        exit 1
    fi

    scenario_id="$(extract_json_string_field "$fixture_path" "id")"
    scenario_cmd="$(extract_json_string_field "$fixture_path" "command")"
    scenario_blocking="$(extract_json_bool_field "$fixture_path" "blocking")"

    if [[ -z "$scenario_id" || -z "$scenario_cmd" || -z "$scenario_blocking" ]]; then
        echo "Invalid harness fixture format: $fixture_rel" >&2
        exit 1
    fi

    if [[ "$mode" == "ci_blocking" && "$scenario_blocking" != "true" ]]; then
        skipped=$((skipped + 1))
        continue
    fi

    scenario_dir="$artifact_root/${scenario_id//\//__}"
    mkdir -p "$scenario_dir"
    cp "$fixture_path" "$scenario_dir/input_script.json"

    started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    scenario_start_epoch="$(date +%s)"
    set +e
    (cd "$repo_root" && bash -lc "$scenario_cmd") >"$scenario_dir/event_trace.log" 2>&1
    exit_code=$?
    set -e
    finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    scenario_duration_secs=$(( $(date +%s) - scenario_start_epoch ))

    total=$((total + 1))
    if [[ $exit_code -eq 0 ]]; then
        passed=$((passed + 1))
        decision="pass"
        reason="command_exit_zero"
        assertion_passed=true
    else
        failed=$((failed + 1))
        decision="fail"
        reason="command_exit_nonzero"
        assertion_passed=false
    fi

    cat >"$scenario_dir/decision_trace.jsonl" <<DECISION
{"scenario":"$scenario_id","decision":"$decision","reason":"$reason","started_at":"$started_at","finished_at":"$finished_at","duration_secs":$scenario_duration_secs}
DECISION

    cat >"$scenario_dir/assertion_report.json" <<ASSERT
{"scenario":"$scenario_id","passed":$assertion_passed,"exit_code":$exit_code,"assertions":[{"name":"command_exit_zero","passed":$assertion_passed,"detail":"$scenario_cmd"}]}
ASSERT

    if [[ $exit_code -ne 0 ]]; then
        echo "Scenario failed: $scenario_id"
        echo "  command: $scenario_cmd"
        echo "  artifact: $scenario_dir"
    fi
done

suite_duration_secs=$(( $(date +%s) - suite_start_epoch ))
if [[ $total -gt 0 ]]; then
    pass_rate_percent="$(awk "BEGIN { printf \"%.2f\", ($passed / $total) * 100 }")"
else
    pass_rate_percent="0.00"
fi

cat >"$artifact_root/kpi.json" <<KPI
{"suite":"coding_reference","mode":"$mode","total":$total,"passed":$passed,"failed":$failed,"skipped":$skipped,"pass_rate_percent":$pass_rate_percent,"duration_secs":$suite_duration_secs}
KPI

echo "Coding reference harness summary:"
echo "  mode: $mode"
echo "  total: $total"
echo "  passed: $passed"
echo "  failed: $failed"
echo "  skipped: $skipped"
echo "  pass_rate_percent: $pass_rate_percent"
echo "  artifacts: $artifact_root"

if [[ $failed -gt 0 ]]; then
    exit 1
fi
