#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CASK_PATH="${1:-$REPO_ROOT/packaging/homebrew/Casks/alan.rb.template}"

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

[[ -f "$CASK_PATH" ]] || fail "cask template not found: $CASK_PATH"

grep -Eq '^[[:space:]]*app "alan\.app"' "$CASK_PATH" ||
    fail "cask must install alan.app"
grep -Eq 'Contents/Resources/bin/alan", target: "alan"' "$CASK_PATH" ||
    fail "cask must link embedded alan binary"
grep -Eq 'Contents/Resources/bin/alan-tui", target: "alan-tui"' "$CASK_PATH" ||
    fail "cask must link embedded alan-tui binary"
grep -Eq 'brew install --cask alan|--cask alan' "$REPO_ROOT/packaging/homebrew/README.md" ||
    fail "Homebrew docs must use brew install --cask alan"

if grep -Eq 'formula "alan"|depends_on formula:' "$CASK_PATH"; then
    fail "cask must not depend on a separate alan formula for CLI/TUI"
fi

printf 'Homebrew cask template validation passed: %s\n' "$CASK_PATH"
