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
    "clients/apple/AlanNative/TerminalRuntimeService.swift" \
    "protocol AlanGhosttyProcessBootstrap: AnyObject" \
    "Ghostty initialization must have an injectable process bootstrap boundary"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeService.swift" \
    "final class AlanWindowTerminalRuntimeService" \
    "terminal runtime services must be window-scoped production owners"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeService.swift" \
    "protocol AlanTerminalSurfaceHandle: AnyObject" \
    "terminal panes must be represented by stable service-owned surface handles"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeService.swift" \
    "final class FakeAlanTerminalSurfaceHandle" \
    "runtime service tests must have fake pane surface handles"

require_pattern \
    "clients/apple/scripts/test-terminal-runtime-service.sh" \
    "TerminalRuntimeService.swift" \
    "runtime service behavior tests must compile the service boundary"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeRegistry.swift" \
    "private let runtimeService: AlanTerminalRuntimeService" \
    "terminal runtime registry must delegate runtime authority to the service"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "final class AlanTerminalSurfaceController" \
    "terminal surface behavior must be owned by a controller boundary"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "final class AlanTerminalInputAdapter" \
    "terminal keyboard and IME behavior must be normalized through an input adapter"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "final class AlanTerminalScrollbackAdapter" \
    "terminal scrollback behavior must be normalized through a scrollback adapter"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "final class AlanTerminalSearchAdapter" \
    "terminal search state must be pane scoped and adapter-owned"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "protocol AlanTerminalSearchEngine" \
    "terminal search queries must be delegated to a real surface search engine"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.swift" \
    "verifiesSearchActionsReachSurfaceEngine" \
    "surface controller tests must prove search actions reach the surface engine"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.sh" \
    "TerminalSurfaceController.swift" \
    "surface controller behavior tests must compile the controller boundary"

require_pattern \
    "clients/apple/AlanNative/ShellControlPlane.swift" \
    "deliveryCode: String?" \
    "pane.send_text responses must expose service delivery state"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "runtimePhase: delivery.runtimePhase" \
    "pane.send_text responses must expose the service runtime phase"

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
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "shouldAutoFocusAfterConfigure" \
    "terminal auto-focus must only be requested on initial attachment or selected-pane transitions"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "previousPaneID != paneID \\|\\| !wasSelected" \
    "terminal auto-focus must not refocus the same selected pane on every SwiftUI update"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "guard !pendingFocusRequest else \\{ return \\}" \
    "terminal auto-focus must coalesce pending first-responder requests"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "protocol TerminalHostActivationDelegate: AnyObject" \
    "terminal activation must use a narrow class-bound delegate"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "TerminalHostActivationDelegate" \
    "shell host controller must own terminal activation requests"

require_pattern \
    "clients/apple/AlanNative/TerminalRuntimeRegistry.swift" \
    "activationDelegate: TerminalHostActivationDelegate\\?" \
    "terminal runtime registry must thread the weak activation boundary"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "weak var activationDelegate" \
    "registry-owned terminal host views must not strongly retain activation owners"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "terminalHostDidRequestActivation\\(paneID:" \
    "terminal host mouse events must request pane activation through the delegate"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "private let overlayCard = AlanTerminalPassiveOverlayView\\(\\)" \
    "passive terminal overlays must use a non-interactive overlay view"

reject_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "onTapGesture\\(perform: onSelect\\)" \
    "terminal leaf selection must not be owned by a SwiftUI tap wrapper"

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
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "override func hitTest\\(_ point: NSPoint\\) -> NSView\\? \\{ nil \\}" \
    "fallback terminal canvas views must be transparent to AppKit hit-testing"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "override var mouseDownCanMoveWindow: Bool \\{ false \\}" \
    "Ghostty canvas views must not allow terminal pane clicks to drag the shell window"

require_pattern \
    "clients/apple/AlanNative/GhosttyLiveHost.swift" \
    "override func hitTest\\(_ point: NSPoint\\) -> NSView\\? \\{ nil \\}" \
    "Ghostty canvas views must be transparent to AppKit hit-testing"

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
