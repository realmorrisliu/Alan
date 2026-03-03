#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  scripts/reference/run_coding_reference_smoke.sh [--mode local|ci]

Modes:
  local  Run scaffold checks and deterministic coding loop (default).
  ci     Same checks with CI-friendly artifact output.
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
    local|ci) ;;
    *)
        usage
        exit 2
        ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
reference_root="$repo_root/reference/coding-agent"
artifact_root="$repo_root/target/reference/coding-agent/smoke/latest"
workspace_dir="$artifact_root/workspace"
trace_file="$artifact_root/loop_trace.log"

required_files=(
    "README.md"
    "profile.toml"
    "skills/decompose/SKILL.md"
    "skills/edit-verify/SKILL.md"
    "skills/deliver/SKILL.md"
    "extensions/code-index.yaml"
    "extensions/test-analyzer.yaml"
    "extensions/pr-helper.yaml"
)

missing=0
for rel in "${required_files[@]}"; do
    if [[ ! -f "$reference_root/$rel" ]]; then
        echo "Missing reference artifact: reference/coding-agent/$rel" >&2
        missing=1
    fi
done
if [[ $missing -ne 0 ]]; then
    exit 1
fi

while IFS= read -r skill_file; do
    if ! grep -q '^---$' "$skill_file"; then
        echo "Invalid skill frontmatter (missing delimiter): $skill_file" >&2
        exit 1
    fi
    if ! grep -q '^name:' "$skill_file"; then
        echo "Invalid skill frontmatter (missing name): $skill_file" >&2
        exit 1
    fi
    if ! grep -q '^description:' "$skill_file"; then
        echo "Invalid skill frontmatter (missing description): $skill_file" >&2
        exit 1
    fi
done < <(find "$reference_root/skills" -name SKILL.md -type f | sort)

rm -rf "$artifact_root"
mkdir -p "$workspace_dir/src"
cp -R "$reference_root" "$artifact_root/reference_snapshot"

cat >"$workspace_dir/Cargo.toml" <<'CARGO'
[package]
name = "coding-loop-fixture"
version = "0.1.0"
edition = "2024"

[lib]
path = "src/lib.rs"

[workspace]
CARGO

cat >"$workspace_dir/src/lib.rs" <<'RS'
pub fn add(a: i32, b: i32) -> i32 {
    a - b
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn add_returns_sum() {
        assert_eq!(add(2, 3), 5);
    }
}
RS

cat >"$artifact_root/input_script.json" <<JSON
{"id":"coding/minimum_loop_smoke","mode":"$mode","steps":["receive_task","plan","edit","verify","deliver"]}
JSON

: >"$trace_file"

echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] receive_task: fix add() implementation" >>"$trace_file"
cat >"$artifact_root/plan.md" <<'PLAN'
1. Locate failing implementation in src/lib.rs.
2. Replace subtraction with addition.
3. Run cargo test.
4. Emit delivery summary.
PLAN

echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] plan: wrote plan.md" >>"$trace_file"
perl -0pi -e 's/a - b/a + b/' "$workspace_dir/src/lib.rs"
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] edit: patched src/lib.rs" >>"$trace_file"

set +e
(cd "$workspace_dir" && cargo test --quiet) >"$artifact_root/verify.log" 2>&1
verify_exit=$?
set -e

echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] verify: cargo test exit=$verify_exit" >>"$trace_file"

verified=false
if [[ $verify_exit -eq 0 ]]; then
    verified=true
fi

change_line="$(grep -n "a + b" "$workspace_dir/src/lib.rs" | head -n1 | cut -d: -f1 || true)"
edit_applied=false
if [[ -n "$change_line" ]]; then
    edit_applied=true
fi

cat >"$artifact_root/delivery_summary.md" <<SUMMARY
# Coding Reference Smoke Summary

- mode: $mode
- edit_applied: $edit_applied
- edit_line: ${change_line:-unknown}
- verify_exit: $verify_exit
- verified: $verified
SUMMARY

cat >"$artifact_root/assertion_report.json" <<ASSERT
{"scenario":"coding/minimum_loop_smoke","passed":$verified,"assertions":[{"name":"scaffold_present","passed":true},{"name":"edit_applied","passed":$edit_applied},{"name":"verify_command_exit_zero","passed":$verified}]}
ASSERT

cat >"$artifact_root/summary.json" <<REPORT
{"mode":"$mode","verify_exit":$verify_exit,"verified":$verified,"artifact_root":"target/reference/coding-agent/smoke/latest"}
REPORT

echo "Coding reference smoke summary:"
echo "  mode: $mode"
echo "  verify_exit: $verify_exit"
echo "  artifacts: $artifact_root"

if [[ $verify_exit -ne 0 ]]; then
    exit 1
fi
