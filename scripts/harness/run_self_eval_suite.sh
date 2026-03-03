#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  scripts/harness/run_self_eval_suite.sh [--mode local|ci|nightly]

Modes:
  local    Run deterministic blocking subset and always emit report (default).
  ci       Run deterministic blocking subset and fail when promotion gate fails.
  nightly  Run full autonomy suite and emit promotion report for trend tracking.
USAGE
}

mode="local"
if [[ "${1:-}" == "--mode" ]]; then
    if [[ -z "${2:-}" ]]; then
        usage
        exit 2
    fi
    mode="$2"
    shift 2
fi

if [[ $# -gt 0 ]]; then
    usage
    exit 2
fi

case "$mode" in
    local|ci|nightly) ;;
    *)
        usage
        exit 2
        ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
scenario_fixture="$repo_root/docs/harness/scenarios/self_eval/profile_regression.json"
threshold_file="$repo_root/docs/harness/self_eval/promotion_thresholds.v1.env"
artifact_root="$repo_root/target/harness/self_eval/latest"

if [[ ! -f "$scenario_fixture" ]]; then
    echo "Missing self-eval scenario fixture: $scenario_fixture" >&2
    exit 1
fi

if [[ ! -f "$threshold_file" ]]; then
    echo "Missing self-eval threshold config: $threshold_file" >&2
    exit 1
fi

# shellcheck disable=SC1090
source "$threshold_file"

extract_json_string_field() {
    local file="$1"
    local key="$2"
    grep -E "^[[:space:]]*\"${key}\":" "$file" \
        | head -n1 \
        | sed -E 's/^[[:space:]]*"[^"]+":[[:space:]]*"([^"]*)".*$/\1/'
}

extract_json_number_field() {
    local file="$1"
    local key="$2"
    grep -E "\"${key}\"" "$file" \
        | head -n1 \
        | sed -E 's/.*"'"${key}"'":[[:space:]]*([0-9]+(\.[0-9]+)?).*/\1/'
}

scenario_id="$(extract_json_string_field "$scenario_fixture" "id")"
baseline_cmd="$(extract_json_string_field "$scenario_fixture" "baseline_command_${mode}")"
candidate_cmd="$(extract_json_string_field "$scenario_fixture" "candidate_command_${mode}")"

if [[ -z "$scenario_id" || -z "$baseline_cmd" || -z "$candidate_cmd" ]]; then
    echo "Invalid self-eval fixture values in $scenario_fixture" >&2
    exit 1
fi

rm -rf "$artifact_root"
mkdir -p "$artifact_root"
cp "$scenario_fixture" "$artifact_root/input_script.json"
cp "$threshold_file" "$artifact_root/promotion_thresholds.env"

float_ge() {
    local left="$1"
    local right="$2"
    awk "BEGIN { exit !($left >= $right) }"
}

float_le() {
    local left="$1"
    local right="$2"
    awk "BEGIN { exit !($left <= $right) }"
}

compute_boundary_violations() {
    local profile_dir="$1"
    local report="$profile_dir/autonomy/governance__recovery_boundary/assertion_report.json"
    if [[ -f "$report" ]] && grep -q '"passed":true' "$report"; then
        echo 0
    else
        echo 1
    fi
}

compute_recovery_success_rate() {
    local profile_dir="$1"
    local scenarios=(
        "autonomy__reboot_resume"
        "autonomy__dedup_side_effect"
    )
    local passed_count=0
    local total_count="${#scenarios[@]}"
    local scenario report
    for scenario in "${scenarios[@]}"; do
        report="$profile_dir/autonomy/$scenario/assertion_report.json"
        if [[ -f "$report" ]] && grep -q '"passed":true' "$report"; then
            passed_count=$((passed_count + 1))
        fi
    done
    awk "BEGIN { printf \"%.2f\", ($passed_count / $total_count) * 100 }"
}

run_profile() {
    local profile_name="$1"
    local command="$2"
    local profile_dir="$artifact_root/$profile_name"
    local autonomy_artifacts="$profile_dir/autonomy"

    mkdir -p "$profile_dir"
    local started_at
    started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    local start_epoch
    start_epoch="$(date +%s)"

    set +e
    (cd "$repo_root" && bash -lc "$command") >"$profile_dir/runner.log" 2>&1
    local exit_code=$?
    set -e

    local finished_at
    finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    local duration_secs=$(( $(date +%s) - start_epoch ))

    rm -rf "$autonomy_artifacts"
    cp -R "$repo_root/target/harness/autonomy/latest" "$autonomy_artifacts"

    local kpi_file="$autonomy_artifacts/kpi.json"
    if [[ ! -f "$kpi_file" ]]; then
        echo "Missing autonomy KPI output for profile: $profile_name" >&2
        exit 1
    fi

    local pass_rate total passed failed
    pass_rate="$(extract_json_number_field "$kpi_file" "pass_rate_percent")"
    total="$(extract_json_number_field "$kpi_file" "total")"
    passed="$(extract_json_number_field "$kpi_file" "passed")"
    failed="$(extract_json_number_field "$kpi_file" "failed")"

    local boundary_violations recovery_success_rate
    boundary_violations="$(compute_boundary_violations "$profile_dir")"
    recovery_success_rate="$(compute_recovery_success_rate "$profile_dir")"

    cat >"$profile_dir/profile_metrics.json" <<EOF
{"profile":"$profile_name","command":"$command","exit_code":$exit_code,"started_at":"$started_at","finished_at":"$finished_at","duration_secs":$duration_secs,"total":$total,"passed":$passed,"failed":$failed,"success_rate_percent":$pass_rate,"boundary_violations":$boundary_violations,"recovery_success_rate_percent":$recovery_success_rate}
EOF
}

run_profile "baseline" "$baseline_cmd"
run_profile "candidate" "$candidate_cmd"

baseline_metrics="$artifact_root/baseline/profile_metrics.json"
candidate_metrics="$artifact_root/candidate/profile_metrics.json"

baseline_pass_rate="$(extract_json_number_field "$baseline_metrics" "success_rate_percent")"
candidate_pass_rate="$(extract_json_number_field "$candidate_metrics" "success_rate_percent")"
baseline_boundary="$(extract_json_number_field "$baseline_metrics" "boundary_violations")"
candidate_boundary="$(extract_json_number_field "$candidate_metrics" "boundary_violations")"
candidate_recovery_rate="$(extract_json_number_field "$candidate_metrics" "recovery_success_rate_percent")"
baseline_duration="$(extract_json_number_field "$baseline_metrics" "duration_secs")"
candidate_duration="$(extract_json_number_field "$candidate_metrics" "duration_secs")"
candidate_exit_code="$(extract_json_number_field "$candidate_metrics" "exit_code")"

pass_rate_drop="$(awk "BEGIN { printf \"%.2f\", $baseline_pass_rate - $candidate_pass_rate }")"
duration_increase_percent="$(awk "BEGIN {
    if ($baseline_duration <= 0) { print 0; }
    else { printf \"%.2f\", (($candidate_duration - $baseline_duration) / $baseline_duration) * 100; }
}")"
boundary_delta=$(( candidate_boundary - baseline_boundary ))

check_candidate_min=false
check_drop=false
check_boundary=false
check_recovery=false
check_duration=false
check_command=false

if float_ge "$candidate_pass_rate" "$MIN_CANDIDATE_PASS_RATE"; then check_candidate_min=true; fi
if float_le "$pass_rate_drop" "$MAX_ALLOWED_PASS_RATE_DROP"; then check_drop=true; fi
if (( boundary_delta <= MAX_BOUNDARY_VIOLATIONS_DELTA )); then check_boundary=true; fi
if float_ge "$candidate_recovery_rate" "$MIN_RECOVERY_SUCCESS_RATE"; then check_recovery=true; fi
if float_le "$duration_increase_percent" "$MAX_DURATION_INCREASE_PERCENT"; then check_duration=true; fi
if (( candidate_exit_code == 0 )); then check_command=true; fi

gate_pass=false
if [[ "$check_candidate_min" == "true" &&
      "$check_drop" == "true" &&
      "$check_boundary" == "true" &&
      "$check_recovery" == "true" &&
      "$check_duration" == "true" &&
      "$check_command" == "true" ]]; then
    gate_pass=true
fi

cat >"$artifact_root/profile_regression_report.json" <<EOF
{
  "scenario": "$scenario_id",
  "mode": "$mode",
  "threshold_version": "$THRESHOLD_VERSION",
  "baseline_metrics_path": "baseline/profile_metrics.json",
  "candidate_metrics_path": "candidate/profile_metrics.json",
  "comparison": {
    "baseline_success_rate_percent": $baseline_pass_rate,
    "candidate_success_rate_percent": $candidate_pass_rate,
    "pass_rate_drop_percent": $pass_rate_drop,
    "baseline_boundary_violations": $baseline_boundary,
    "candidate_boundary_violations": $candidate_boundary,
    "boundary_violation_delta": $boundary_delta,
    "candidate_recovery_success_rate_percent": $candidate_recovery_rate,
    "duration_increase_percent": $duration_increase_percent
  },
  "thresholds": {
    "min_candidate_pass_rate": $MIN_CANDIDATE_PASS_RATE,
    "max_allowed_pass_rate_drop": $MAX_ALLOWED_PASS_RATE_DROP,
    "max_boundary_violations_delta": $MAX_BOUNDARY_VIOLATIONS_DELTA,
    "min_recovery_success_rate": $MIN_RECOVERY_SUCCESS_RATE,
    "max_duration_increase_percent": $MAX_DURATION_INCREASE_PERCENT
  },
  "checks": {
    "candidate_min_pass_rate": $check_candidate_min,
    "pass_rate_drop": $check_drop,
    "boundary_delta": $check_boundary,
    "recovery_success_rate": $check_recovery,
    "duration_increase": $check_duration,
    "candidate_command_exit_zero": $check_command
  },
  "promotion_recommended": $gate_pass
}
EOF

echo "Self-eval summary:"
echo "  scenario: $scenario_id"
echo "  mode: $mode"
echo "  gate_pass: $gate_pass"
echo "  artifacts: $artifact_root"

if [[ "$mode" == "ci" && "$gate_pass" != "true" ]]; then
    echo "Self-eval promotion gate failed in CI mode" >&2
    exit 1
fi
