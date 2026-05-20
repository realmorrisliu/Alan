#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

SURFACE_FILES=(
  "$REPO_ROOT/clients/apple/alan-macos/App/AlanMacShellCommands.swift"
  "$REPO_ROOT/clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift"
  "$REPO_ROOT/clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift"
)

if rg -n \
  "performShellWorkspaceCommand|host\\.openTerminalTab|host\\.openAlanTab|host\\.pinTab|host\\.unpinTab|host\\.updatePinnedTabSnapshot|host\\.selectAdjacentSpace|host\\.selectSpace\\(at:" \
  "${SURFACE_FILES[@]}"; then
  echo "Shared shell menu/context/keyboard surfaces must route through ShellActionRegistry." >&2
  exit 1
fi

if ! rg -q "shellActionKeyboardShortcut\\(host\\.shellActionShortcut\\(\\.newTerminalTab\\)\\)" \
  "$REPO_ROOT/clients/apple/alan-macos/App/AlanMacShellCommands.swift"; then
  echo "Native shell menu shortcut hints must come from ShellActionRegistry descriptors." >&2
  exit 1
fi

if ! rg -q "shellActionShortcut\\(\\.spaceSelectByIndex, target: target\\)" \
  "$REPO_ROOT/clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift"; then
  echo "Numeric Space keyboard shortcuts must use registry-derived shortcut descriptors." >&2
  exit 1
fi

COMMAND_UI_FILE="$REPO_ROOT/clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift"
if rg -n "ShellActionRegistry|performShellAction" "$COMMAND_UI_FILE"; then
  echo "Go to or Command... must stay out of the first shell action registry pass." >&2
  exit 1
fi

echo "Shell action registry integration checks passed."
