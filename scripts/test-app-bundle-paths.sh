#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/app-bundle-paths.sh
source "$SCRIPT_DIR/app-bundle-paths.sh"

fail() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/alan-app-bundle-paths.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

mkdir "$WORK_DIR/Alan.app"

if alan_is_distinct_existing_path "$WORK_DIR/Alan.app" "$WORK_DIR/Alan.app"; then
    fail "identical app bundle paths must not be distinct"
fi

if [[ -d "$WORK_DIR/alan.app" ]]; then
    if alan_is_distinct_existing_path "$WORK_DIR/alan.app" "$WORK_DIR/Alan.app"; then
        fail "case-insensitive aliases must not be treated as distinct bundles"
    fi
else
    mkdir "$WORK_DIR/alan.app"
    if ! alan_is_distinct_existing_path "$WORK_DIR/alan.app" "$WORK_DIR/Alan.app"; then
        fail "case-sensitive lowercase app bundle must be treated as distinct"
    fi
fi

if ! alan_is_distinct_existing_path "$WORK_DIR/alan.app" "$WORK_DIR/Missing.app"; then
    fail "existing candidate must be distinct from a missing reference"
fi

printf 'App bundle path checks passed.\n'
