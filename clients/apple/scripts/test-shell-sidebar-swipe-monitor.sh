#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BUILD_DIR="${TMPDIR:-/tmp}/alan-shell-sidebar-swipe-monitor-tests"
MODULE_CACHE_DIR="${BUILD_DIR}/clang-module-cache"
TEST_BINARY="${BUILD_DIR}/shell-sidebar-swipe-monitor-tests"

mkdir -p "$MODULE_CACHE_DIR"

CLANG_MODULE_CACHE_PATH="$MODULE_CACHE_DIR" swiftc \
    -D ALAN_TESTING \
    "$REPO_ROOT/clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift" \
    "$REPO_ROOT/clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift" \
    "$REPO_ROOT/clients/apple/scripts/test-shell-sidebar-swipe-monitor.swift" \
    -o "$TEST_BINARY"

"$TEST_BINARY"
