#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BUILD_DIR="${TMPDIR:-/tmp}/alan-shell-window-placement-tests"
MODULE_CACHE_DIR="${BUILD_DIR}/clang-module-cache"
TEST_BINARY="${BUILD_DIR}/shell-window-placement-tests"

mkdir -p "$MODULE_CACHE_DIR"

CLANG_MODULE_CACHE_PATH="$MODULE_CACHE_DIR" swiftc \
    "$REPO_ROOT/clients/apple/alan-macos/Support/ShellDesignTokens.swift" \
    "$REPO_ROOT/clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "$REPO_ROOT/clients/apple/scripts/test-shell-window-placement.swift" \
    -o "$TEST_BINARY"

"$TEST_BINARY"
