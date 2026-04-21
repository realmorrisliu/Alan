#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  crates/runtime/skills/repo-coding/scripts/score_swebench_predictions.sh <predictions-jsonl> [--dataset-name <name>] [--max-workers <n>] [--run-id <id>]

Defaults:
  dataset-name  princeton-nlp/SWE-bench_Lite
  max-workers   8
  run-id        swebench_eval_<timestamp>

This is a thin wrapper around the official SWE-bench harness entrypoint:
  python -m swebench.harness.run_evaluation
USAGE
}

predictions_path=""
dataset_name="princeton-nlp/SWE-bench_Lite"
max_workers="8"
run_id="swebench_eval_$(date -u +%Y%m%dT%H%M%SZ)"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dataset-name)
            dataset_name="$2"
            shift 2
            ;;
        --max-workers)
            max_workers="$2"
            shift 2
            ;;
        --run-id)
            run_id="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            usage
            exit 2
            ;;
        *)
            if [[ -n "$predictions_path" ]]; then
                usage
                exit 2
            fi
            predictions_path="$1"
            shift
            ;;
    esac
done

if [[ -z "$predictions_path" ]]; then
    usage
    exit 2
fi

if [[ ! -f "$predictions_path" ]]; then
    echo "Predictions file not found: $predictions_path" >&2
    exit 1
fi

python_bin=""
if command -v python >/dev/null 2>&1; then
    python_bin="python"
elif command -v python3 >/dev/null 2>&1; then
    python_bin="python3"
else
    echo "Missing required command: python or python3" >&2
    exit 1
fi

"$python_bin" -m swebench.harness.run_evaluation \
  --dataset_name "$dataset_name" \
  --predictions_path "$predictions_path" \
  --max_workers "$max_workers" \
  --run_id "$run_id"
