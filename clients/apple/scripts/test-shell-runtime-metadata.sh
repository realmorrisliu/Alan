#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BUILD_DIR="${TMPDIR:-/tmp}/alan-shell-runtime-metadata-tests"
MODULE_CACHE_DIR="${BUILD_DIR}/clang-module-cache"
TEST_BINARY="${BUILD_DIR}/shell-runtime-metadata-tests"

mkdir -p "$MODULE_CACHE_DIR"

CLANG_MODULE_CACHE_PATH="$MODULE_CACHE_DIR" swiftc \
    "$REPO_ROOT/clients/apple/AlanNative/ShellModel.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/ShellControlPlane.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/TerminalHostRuntime.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/TerminalRuntimeService.swift" \
    "$REPO_ROOT/clients/apple/scripts/test-shell-runtime-metadata-host-stubs.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/TerminalRuntimeRegistry.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/ShellHostController.swift" \
    "$REPO_ROOT/clients/apple/scripts/test-shell-runtime-metadata.swift" \
    -o "$TEST_BINARY"

"$TEST_BINARY"
