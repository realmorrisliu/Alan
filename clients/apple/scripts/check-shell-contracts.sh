#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

require_pattern() {
    local file="$1"
    local pattern="$2"
    local message="$3"

    if ! grep -Eq "$pattern" "$REPO_ROOT/$file"; then
        printf 'error: %s\n' "$message" >&2
        printf '       expected pattern %s in %s\n' "$pattern" "$file" >&2
        exit 1
    fi
}

reject_pattern() {
    local file="$1"
    local pattern="$2"
    local message="$3"

    if grep -ERq "$pattern" "$REPO_ROOT/$file"; then
        printf 'error: %s\n' "$message" >&2
        printf '       rejected pattern %s in %s\n' "$pattern" "$file" >&2
        exit 1
    fi
}

"$SCRIPT_DIR/setup-local-ghosttykit.sh" --check >/dev/null

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeRegistry.swift" \
    "hostViewsByPaneID" \
    "terminal runtimes must be owned by a pane-keyed registry"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeRegistry.swift" \
    "protocol TerminalRuntimeHandle" \
    "terminal runtimes must expose a handle protocol"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeRegistry.swift" \
    "final class MockTerminalRuntimeHandle" \
    "terminal runtime delivery must have a mock handle for contract tests"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeRegistry.swift" \
    "func sendText\\(to paneID: String, text: String\\)" \
    "terminal text delivery must go through the runtime registry"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "terminalRuntimeRegistry\\.sendText\\(to: paneID, text: text\\)" \
    "pane.send_text must use the registry delivery result"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "\\.id\\(pane\\.paneID\\)" \
    "terminal host views must be keyed by stable pane identity"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "hasTornDownRuntime" \
    "terminal teardown must be idempotent"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "let isSelected: Bool" \
    "terminal hosts must know whether their pane is selected"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "guard isSelected, pane != nil else \\{ return \\}" \
    "terminal auto-focus must be gated to the selected pane"

require_pattern \
    "clients/apple/AlanNative/MacShellRootView.swift" \
    "struct ShellWindowDragRegion" \
    "hidden-titlebar shell windows must expose a replacement drag region"

require_pattern \
    "clients/apple/AlanNative/MacShellRootView.swift" \
    "WindowDragGesture\\(\\)" \
    "hidden-titlebar shell windows must use WindowDragGesture for custom chrome"

require_pattern \
    "clients/apple/AlanNative/MacShellRootView.swift" \
    "allowsWindowActivationEvents\\(true\\)" \
    "custom shell window drag regions must support click-then-drag activation"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "let visible = .*occlusionState\\.contains\\(\\.visible\\) \\?\\? false" \
    "Ghostty occlusion bridge must derive the visible flag from NSWindow occlusion state"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "ghostty_surface_set_occlusion\\(surface, visible\\)" \
    "Ghostty occlusion bridge must pass visible=true for visible windows"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "if let surface = self\\.surface" \
    "Ghostty wakeup ticks must look up the current surface before refreshing"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "struct ShellWindowContext" \
    "shell host must expose a per-window context"

require_pattern \
    "clients/apple/AlanNative/MacShellRootView.swift" \
    "ShellWindowContext\\.make\\(\\)" \
    "each macOS shell window must create its own context"

require_pattern \
    "clients/apple/AlanNative/ShellControlPlane.swift" \
    "private static let maxRequestBytes" \
    "socket server must enforce a bounded request size"

require_pattern \
    "clients/apple/AlanNative/ShellControlPlane.swift" \
    "command_timeout" \
    "socket server must return a stable timeout error"

reject_pattern \
    "clients/apple/AlanNative" \
    "NotificationCenter\\.default\\.post" \
    "control-plane text delivery must not rely on NotificationCenter broadcast success"

printf 'Shell contract checks passed.\n'
