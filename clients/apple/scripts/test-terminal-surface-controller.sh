#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BUILD_DIR="${TMPDIR:-/tmp}/alan-terminal-surface-controller-tests"
MODULE_CACHE_DIR="${BUILD_DIR}/clang-module-cache"
TEST_BINARY="${BUILD_DIR}/terminal-surface-controller-tests"

mkdir -p "$MODULE_CACHE_DIR"

CLANG_MODULE_CACHE_PATH="$MODULE_CACHE_DIR" swiftc \
    "$REPO_ROOT/clients/apple/AlanNative/Models/Shell/ShellValueTypes.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Models/Shell/ShellSnapshots.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Models/Shell/ShellControlPlaneDTOs.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Models/Shell/ShellTreeMutations.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Models/Shell/ShellStateMutations.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/ShellModel.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Services/Shell/ShellControlFilePoller.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Services/Shell/ShellDiagnostics.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Services/Shell/ShellEventStore.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Services/Shell/ShellLocalCommandExecutor.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Services/Shell/ShellPublishedStateMerger.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Services/Shell/ShellSocketServer.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/ShellControlPlane.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/TerminalHostRuntime.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/Services/Terminal/TerminalNativeScrollViewAdapter.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/TerminalRuntimeService.swift" \
    "$REPO_ROOT/clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "$REPO_ROOT/clients/apple/scripts/test-terminal-surface-controller.swift" \
    -o "$TEST_BINARY"

"$TEST_BINARY"
