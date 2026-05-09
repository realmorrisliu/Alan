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

reject_active_shell_radius_drift() {
    local matched=0
    local file

    for file in \
        "clients/apple/AlanNative/MacShellRootView.swift" \
        "clients/apple/AlanNative/Views/Shell/ShellSidebarView.swift" \
        "clients/apple/AlanNative/Views/Shell/ShellCommandTabView.swift" \
        "clients/apple/AlanNative/TerminalPaneView.swift" \
        "clients/apple/AlanNative/TerminalHostView.swift"
    do
        if grep -En 'RoundedRectangle\(cornerRadius: (1[4-9]|[2-9][0-9])|cornerRadius = (1[4-9]|[2-9][0-9])' "$REPO_ROOT/$file" >&2; then
            matched=1
        fi

        if grep -En 'Capsule\(style: \.continuous\)' "$REPO_ROOT/$file" >&2; then
            matched=1
        fi
    done

    if [[ "$matched" -ne 0 ]]; then
        printf 'error: active macOS shell chrome must use ShellRadii tokens and avoid default Capsule chrome\n' >&2
        exit 1
    fi
}

"$SCRIPT_DIR/setup-local-ghosttykit.sh" --check >/dev/null
"$SCRIPT_DIR/check-architecture-maintainability.sh" >/dev/null
reject_active_shell_radius_drift

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
    "final class AlanTerminalPointerAdapter" \
    "terminal mouse and pointer behavior must be normalized through a pointer adapter"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "final class AlanTerminalScrollbackAdapter" \
    "terminal scrollback behavior must be normalized through a scrollback adapter"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "final class AlanTerminalNativeScrollViewAdapter" \
    "terminal scrollback must have an AppKit scroll view adapter"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "protocol AlanTerminalScrollbackEngine" \
    "terminal scrollback must delegate native row scrolls to a surface engine"

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
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellFindBarView" \
    "pane-scoped Find must render as a real SwiftUI find bar"

reject_pattern \
    "clients/apple/AlanNative/Views/Shell" \
    "alanShellShowsInspector|showsInspector|ShellInspectorView|ShellInspectorSection|InspectorCard|toggleInspector|Show Inspector|Hide Inspector|right-side shell inspector" \
    "default macOS shell must not expose the removed inspector product surface"

reject_pattern \
    "clients/apple/AlanNative/Views/Shell" \
    "show inspector|hide inspector|open inspector|close inspector|toggle inspector" \
    "legacy shell voice commands must not expose inspector commands"

reject_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "handleSearchKeyIfNeeded|current \\+ characters|dropLast\\(\\)" \
    "Find query editing must be owned by the SwiftUI Find bar instead of terminal key capture"

reject_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "Search terminal|Find text in this pane|Type to search this pane" \
    "Find UI must render through ShellFindBarView instead of the passive terminal overlay card"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.swift" \
    "verifiesScrollbackActionsReachSurfaceEngine" \
    "surface controller tests must prove scrollback actions reach the surface engine"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.swift" \
    "verifiesPointerRoutingFollowsTerminalMouseModes" \
    "surface controller tests must prove pointer routing follows terminal mouse modes"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.swift" \
    "verifiesPointerButtonMappingMatchesGhostty" \
    "surface controller tests must prove other-button mapping matches Ghostty"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.swift" \
    "verifiesSelectionCopyAndPasteUseController" \
    "surface controller tests must prove copy and paste use controller-owned clipboard paths"

require_pattern \
    "clients/apple/scripts/test-shell-runtime-metadata.swift" \
    "verifiesRuntimeProjectsTerminalStatusIntoPaneMetadata" \
    "shell runtime tests must prove terminal status projects into pane metadata"

require_pattern \
    "clients/apple/scripts/test-shell-runtime-metadata.swift" \
    "verifiesTerminalStatusSummaryPrioritizesExitAndRendererHealth" \
    "shell runtime tests must prove sidebar status prioritizes exit and renderer health"

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
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellSplitDividerView" \
    "split panes must use an explicit divider instead of visual spacing gaps"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellSplitDividerTint" \
    "split divider tint must stay subtle instead of rendering as a hard line"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellSplitDividerMetrics\\.thickness" \
    "split divider must use an intentional seam thickness instead of a hard 1px line"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellSplitDividerTint\\.shadow" \
    "split divider must use a subtle bevel seam rather than a single flat line"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "dragPreviewRatio" \
    "split divider drag must track the live preview ratio until drag end"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "resizeSplit\\(splitNodeID: node\\.nodeID, ratio: nextRatio, persist: false\\)" \
    "split divider drag previews must not persist every pointer sample"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "resizeSplit\\(splitNodeID: node\\.nodeID, ratio: finalRatio, persist: true\\)" \
    "split divider drag end must persist the final ratio"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "func resizeSplit\\(splitNodeID: String, ratio: Double, persist: Bool = true\\)" \
    "shell split resize must expose a non-persisting preview path"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "applyMutationResult\\(result, publish: persist\\)" \
    "split resize preview persistence must be controlled at mutation application"

require_pattern \
    "clients/apple/AlanNative/Models/Shell/ShellValueTypes.swift" \
    "enum ShellPaneSplitDirection" \
    "split commands must model left/right/up/down placement separately from split axis"

require_pattern \
    "clients/apple/AlanNative/Models/Shell/ShellValueTypes.swift" \
    "enum ShellSpatialFocusDirection" \
    "spatial focus commands must use explicit left/right/up/down directions"

require_pattern \
    "clients/apple/AlanNative/Models/Shell/ShellValueTypes.swift" \
    "enum ShellWorkspaceCommand: String, CaseIterable, Identifiable" \
    "shell workspace commands must remain a centralized shared vocabulary"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "func performShellWorkspaceCommand\\(_ command: ShellWorkspaceCommand\\)" \
    "menu, keyboard, and command UI actions must route through one shell command entry point"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "case \\.splitRight:" \
    "shared shell workspace command routing must be exhaustively owned by ShellHostController"

require_pattern \
    "clients/apple/AlanNative/AlanNativeApp.swift" \
    "AlanMacShellCommands\\(host: primaryShellOwner\\.host\\)" \
    "native menu commands must receive the primary shell host"

require_pattern \
    "clients/apple/AlanNative/App/AlanMacShellCommands.swift" \
    "CommandMenu\\(\"Shell\"\\)" \
    "split workspace actions must be exposed through a native Shell menu"

require_pattern \
    "clients/apple/AlanNative/App/AlanMacShellCommands.swift" \
    "\\.keyboardShortcut\\(\"d\", modifiers: \\.command\\)" \
    "split right must have a native command-key shortcut"

require_pattern \
    "clients/apple/AlanNative/App/AlanMacShellCommands.swift" \
    "\\.keyboardShortcut\\(\"d\", modifiers: \\[\\.command, \\.shift\\]\\)" \
    "split down must have a native command-shift shortcut"

require_pattern \
    "clients/apple/AlanNative/App/AlanMacShellCommands.swift" \
    "host\\.performShellWorkspaceCommand\\(\\.closeTab\\)" \
    "native menu close actions must use the shared shell workspace command vocabulary"

require_pattern \
    "clients/apple/AlanNative/Views/Shell/ShellCommandTabView.swift" \
    "ShellWorkspaceCommand\\.splitRight" \
    "command UI split actions must call the same shell command router as native menus"

require_pattern \
    "clients/apple/AlanNative/Views/Shell/ShellCommandTabView.swift" \
    "host\\.performShellWorkspaceCommand\\(\\.newTerminalTab\\)" \
    "command UI tab actions must use the shared shell workspace command vocabulary"

require_pattern \
    "clients/apple/AlanNative/Views/Shell/ShellSidebarView.swift" \
    "Go to or Command\\.\\.\\." \
    "command entry copy must match the accepted shell command UI label"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "func routeWorkspaceCommand\\(_ input: AlanTerminalKeyInput\\) -> ShellWorkspaceCommand\\?" \
    "terminal input routing must recognize Alan workspace shortcuts before terminal bindings"

require_pattern \
    "clients/apple/AlanNative/TerminalSurfaceController.swift" \
    "return \\.newTerminalTab" \
    "terminal keyboard tab shortcuts must map to the shared shell workspace command vocabulary"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "routeWorkspaceKeyCommandIfNeeded\\(event\\)" \
    "terminal host key equivalents must give Alan workspace shortcuts priority over Ghostty bindings"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "private let runtimeReporter = TerminalHostRuntimeReporter\\(\\)" \
    "terminal host runtime snapshot publication must be owned by a focused collaborator"

require_pattern \
    "clients/apple/AlanNative/Services/Terminal/TerminalHostRuntimeReporter.swift" \
    "snapshotsEqualIgnoringTimestamp" \
    "terminal runtime reporter must preserve timestamp-insensitive snapshot deduplication"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "private let windowObserver = TerminalHostWindowObserver\\(\\)" \
    "terminal host window notifications must be owned by a focused collaborator"

require_pattern \
    "clients/apple/AlanNative/Services/Terminal/TerminalHostWindowObserver.swift" \
    "NSWindow\\.didChangeOcclusionStateNotification" \
    "terminal host window observer must keep occlusion changes connected to surface/runtime refresh"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "host\\.performShellWorkspaceCommand\\(command\\)" \
    "terminal workspace shortcut routing must enter the shared shell workspace command handler"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "private func handleControlPlaneCommand\\(_ command: AlanShellControlCommand\\)" \
    "control-plane protocol commands must stay separate from UI command vocabulary while sharing shell mutation authority"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellTerminalSurfaceFrame" \
    "terminal panes must share one outer rounded terminal surface frame"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellPaneTitleBarView" \
    "visible terminal panes must render a compact pane title bar"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "shellPaneTitleBarTitle" \
    "pane title bars must use a dedicated title helper with terminal-title-first priority"

require_pattern \
    "clients/apple/AlanNative/ShellHostController.swift" \
    "func closePaneByID\\(_ paneID: String\\) -> Bool" \
    "pane title-bar close must route through a controller-owned targeted pane close path"

require_pattern \
    "clients/apple/scripts/test-shell-split-model.swift" \
    "verifiesPaneScopedCloseKeepsInactivePaneTargeting" \
    "split model tests must cover pane-scoped close targeting"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "ShellInactivePaneDim" \
    "inactive split panes must use a lightweight dim treatment"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "allowsHitTesting\\(false\\)" \
    "inactive pane dimming must not intercept terminal pointer input"

require_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "@AppStorage\\(\"alanShellDimsInactiveSplitPanes\"\\)" \
    "inactive pane dimming must be backed by a user-default preference"

reject_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "splitChildren" \
    "split panes must not leave a fixed gap between adjacent terminal panes"

reject_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "paneSelectorStrip" \
    "split panes must not show a bottom pane tab strip by default"

reject_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "Color\\.primary\\.opacity\\(0\\.16\\)" \
    "split divider must not render as a high-contrast primary-color line"

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
    "override func mouseDown\\(with event: NSEvent\\)" \
    "terminal pointer down events must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "private func routePointer\\(_ input: AlanTerminalPointerInput\\)" \
    "terminal pointer routing must stay behind the AppKit terminal host boundary"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "private func routeScrollWheel\\(_ event: NSEvent\\)" \
    "terminal scroll routing must stay behind the AppKit terminal host boundary"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "override func scrollWheel\\(with event: NSEvent\\)" \
    "terminal scroll wheel events must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "override func keyDown\\(with event: NSEvent\\)" \
    "terminal key events must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "func insertText\\(_ string: Any, replacementRange: NSRange\\)" \
    "terminal IME text insertion must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "workspaceCommandHandler\\?\\(command\\)" \
    "terminal workspace shortcuts must leave the AppKit host through the shared command callback"

require_pattern \
    "clients/apple/AlanNative/TerminalHostView.swift" \
    "private let overlayCard = AlanTerminalPassiveOverlayView\\(\\)" \
    "passive terminal overlays must use a non-interactive overlay view"

reject_pattern \
    "clients/apple/AlanNative/TerminalPaneView.swift" \
    "onTapGesture\\(perform: onSelect\\)" \
    "terminal leaf selection must not be owned by a SwiftUI tap wrapper"

require_pattern \
    "clients/apple/AlanNative/Support/ShellWindowPlacement.swift" \
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
    "clients/apple/AlanNative" \
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
    "shell host must expose a shell context type for the singleton primary window"

reject_pattern \
    "clients/apple/AlanNative/MacShellRootView.swift" \
    "ShellWindowContext\\.make\\(\\)" \
    "macOS root view must use the app-scoped primary shell owner instead of creating a fresh context"

require_pattern \
    "clients/apple/AlanNative/AlanNativeApp.swift" \
    "Window\\(\"Alan\", id: \"main\"\\)" \
    "macOS app scene must use a unique primary Window instead of a repeatable WindowGroup"

require_pattern \
    "clients/apple/AlanNative/App/AlanMacShellCommands.swift" \
    "CommandGroup\\(replacing: \\.newItem\\)" \
    "macOS app must replace New Window with a focus/reopen command"

require_pattern \
    "clients/apple/AlanNative/AlanNativeApp.swift" \
    "AlanMacAppStartup\\.acquireSingletonOrTerminate\\(\\)" \
    "macOS app startup must acquire the singleton guard before creating shell state"

require_pattern \
    "clients/apple/AlanNative/AlanAppSingletonGuard.swift" \
    "flock\\(descriptor, LOCK_EX \\| LOCK_NB\\)" \
    "macOS app singleton guard must use an OS-backed exclusive lock"

require_pattern \
    "clients/apple/README.md" \
    "stable .*window_main.* identity" \
    "Apple client docs must describe the singleton primary shell identity"

reject_pattern \
    "clients/apple/README.md" \
    "Each macOS window creates its own shell context" \
    "Apple client docs must not describe each macOS window as an independent shell context"

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
