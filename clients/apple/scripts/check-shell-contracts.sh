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
    "window\\.isMovableByWindowBackground = true" \
    "hidden-titlebar shell windows must make non-interactive background regions draggable"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "override var mouseDownCanMoveWindow: Bool \\{ false \\}" \
    "terminal host views must not allow terminal pane clicks to drag the shell window"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "final class AlanTerminalFallbackCanvasView" \
    "fallback terminal canvas views must explicitly opt out of background window dragging"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "override var mouseDownCanMoveWindow: Bool \\{ false \\}" \
    "Ghostty canvas views must not allow terminal pane clicks to drag the shell window"

reject_pattern \
    "clients/apple/AlanNative/MacShellRootView.swift" \
    "WindowDragGesture\\(\\)" \
    "shell window dragging should rely on movable background regions, not transparent SwiftUI drag overlays"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "let visible = .*occlusionState\\.contains\\(\\.visible\\) \\?\\? false" \
    "Ghostty occlusion bridge must derive the visible flag from NSWindow occlusion state"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "ghostty_surface_set_occlusion\\(surface, visible\\)" \
    "GhosttyKit bridge must pass the observed visible state used by this linked Ghostty build"

reject_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "let isOccluded =|isSurfaceOccluded|!isVisible" \
    "GhosttyKit bridge must not invert NSWindow visible state for this linked Ghostty build"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "if let surface = self\\.surface" \
    "Ghostty wakeup ticks must look up the current surface before refreshing"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "private var tickScheduled = false" \
    "Ghostty wakeup ticks must be coalesced so repeated wakeups do not flood the main queue"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "guard markTickScheduledIfNeeded\\(\\) else \\{ return \\}" \
    "Ghostty wakeup ticks must skip scheduling when a tick is already pending"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "clearScheduledTick\\(\\)" \
    "Ghostty wakeup ticks must clear their pending marker when the scheduled tick begins"

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
