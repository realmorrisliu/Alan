#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  crates/runtime/skills/repo-coding/scripts/validate_delivery_contract.sh <delivery-contract.json>
USAGE
}

if [[ $# -ne 1 ]]; then
    usage
    exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required to validate the repo-worker delivery contract." >&2
    exit 1
fi

contract_path="$1"
if [[ ! -f "$contract_path" ]]; then
    echo "Missing delivery contract: $contract_path" >&2
    exit 1
fi

jq -e '
  def one_of($value; $choices):
    $choices | index($value) != null;

  (.status | type == "string") and
  (.status as $status | one_of($status; ["completed", "blocked", "failed"])) and
  (.summary | type == "string" and length > 0) and
  (.changed_files | type == "array" and all(.[]; type == "string")) and
  (.verification | type == "array" and length > 0) and
  all(
    .verification[];
    (.command | type == "string" and length > 0) and
    (.scope | type == "string") and
    (.scope as $scope | one_of($scope; ["targeted", "broader"])) and
    (.status | type == "string") and
    (.status as $status | one_of($status; ["passed", "failed", "not_run"])) and
    (.exit_code | type == "number") and
    (.summary | type == "string" and length > 0)
  ) and
  (.residual_risks | type == "array" and all(.[]; type == "string")) and
  (.evaluator | type == "object") and
  (.evaluator.mode | type == "string") and
  (.evaluator.mode as $mode | one_of($mode; ["not_needed", "recommended", "used"])) and
  (.evaluator.reason | type == "string" and length > 0)
' "$contract_path" >/dev/null

echo "Repo-worker delivery contract is valid: $contract_path"
