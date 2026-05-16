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
        "clients/apple/alan-macos/MacShellRootView.swift" \
        "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
        "clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift" \
        "clients/apple/alan-macos/TerminalPaneView.swift" \
        "clients/apple/alan-macos/TerminalHostView.swift"
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
    "clients/apple/alan-macos/TerminalRuntimeRegistry.swift" \
    "hostViewsByPaneID" \
    "terminal runtimes must be owned by a pane-keyed registry"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeRegistry.swift" \
    "protocol TerminalRuntimeHandle" \
    "terminal runtimes must expose a handle protocol"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeService.swift" \
    "protocol AlanGhosttyProcessBootstrap: AnyObject" \
    "Ghostty initialization must have an injectable process bootstrap boundary"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeService.swift" \
    "final class AlanWindowTerminalRuntimeService" \
    "terminal runtime services must be window-scoped production owners"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeService.swift" \
    "protocol AlanTerminalSurfaceHandle: AnyObject" \
    "terminal panes must be represented by stable service-owned surface handles"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeService.swift" \
    "final class FakeAlanTerminalSurfaceHandle" \
    "runtime service tests must have fake pane surface handles"

require_pattern \
    "clients/apple/scripts/test-terminal-runtime-service.sh" \
    "TerminalRuntimeService.swift" \
    "runtime service behavior tests must compile the service boundary"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeRegistry.swift" \
    "private let runtimeService: AlanTerminalRuntimeService" \
    "terminal runtime registry must delegate runtime authority to the service"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeRegistry.swift" \
    "runtimeService\\.surfaceHandle\\(for: paneID, bootProfile: bootProfile\\)" \
    "terminal runtime registry must resolve service-owned handles by pane ID"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeService.swift" \
    "private var handlesByPaneID: \\[String: AlanTerminalSurfaceHandle\\]" \
    "terminal runtime service must keep runtime identity pane-keyed"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeService.swift" \
    "var registeredPaneIDs: Set<String>" \
    "terminal runtime service must expose pane-keyed registration state"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "final class AlanTerminalSurfaceController" \
    "terminal surface behavior must be owned by a controller boundary"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "final class AlanTerminalInputAdapter" \
    "terminal keyboard and IME behavior must be normalized through an input adapter"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "final class AlanTerminalPointerAdapter" \
    "terminal mouse and pointer behavior must be normalized through a pointer adapter"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "final class AlanTerminalScrollbackAdapter" \
    "terminal scrollback behavior must be normalized through a scrollback adapter"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalNativeScrollViewAdapter.swift" \
    "final class AlanTerminalNativeScrollViewAdapter" \
    "terminal scrollback must have an AppKit scroll view adapter"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "protocol AlanTerminalScrollbackEngine" \
    "terminal scrollback must delegate native row scrolls to a surface engine"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "final class AlanTerminalSearchAdapter" \
    "terminal search state must be pane scoped and adapter-owned"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "protocol AlanTerminalSearchEngine" \
    "terminal search queries must be delegated to a real surface search engine"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.swift" \
    "verifiesSearchActionsReachSurfaceEngine" \
    "surface controller tests must prove search actions reach the surface engine"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellFindBarView" \
    "pane-scoped Find must render as a real SwiftUI find bar"

reject_pattern \
    "clients/apple/alan-macos/Views/Shell" \
    "alanShellShowsInspector|showsInspector|ShellInspectorView|ShellInspectorSection|InspectorCard|toggleInspector|Show Inspector|Hide Inspector|right-side shell inspector" \
    "default macOS shell must not expose the removed inspector product surface"

reject_pattern \
    "clients/apple/alan-macos/Views/Shell" \
    "show inspector|hide inspector|open inspector|close inspector|toggle inspector" \
    "legacy shell voice commands must not expose inspector commands"

reject_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "handleSearchKeyIfNeeded|current \\+ characters|dropLast\\(\\)" \
    "Find query editing must be owned by the SwiftUI Find bar instead of terminal key capture"

reject_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "Search terminal|Find text in this pane|Type to search this pane" \
    "Find UI must render through ShellFindBarView instead of the passive terminal overlay card"

require_pattern \
    "clients/apple/alan-macos/Support/ShellDesignTokens.swift" \
    "spaceDockOuterBottomInset" \
    "bottom space dock must use a tokenized outer inset to align with the sidebar edge"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "padding\\(\\.bottom, ShellSidebarMetrics\\.spaceDockOuterBottomInset\\)" \
    "bottom space dock must align its visible controls to the sidebar bottom edge inset"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "padding\\(\\.vertical, ShellSidebarMetrics\\.spaceDockInternalVerticalPadding\\)" \
    "space dock internal vertical padding must stay paired with its bottom alignment token"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "Button\\(action: createSpaceFromDock\\)" \
    "bottom space dock add control must be a direct button, not a variant menu"

reject_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "New Space with alan|createAlanSpace|menuIndicator\\(\\.hidden\\)" \
    "bottom space dock add control must not expose the removed New Space with alan menu path"

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
    "clients/apple/alan-macos/Services/Shell/ShellWorkspaceManifestStore.swift" \
    "shell-workspace-" \
    "workspace restore authority must use the ShellWorkspaceManifest store filename"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "case \\.workspaceManifest:" \
    "shell host startup must have a workspace-manifest restore path"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "ShellWorkspaceMaterializer\\.materialize" \
    "workspace-manifest startup must materialize shell state from the manifest"

require_pattern \
    "clients/apple/alan-macos/App/AlanMacPrimaryShellOwner.swift" \
    "startupMode: \\.workspaceManifest" \
    "primary macOS shell must start from the workspace manifest"

reject_pattern \
    "clients/apple/alan-macos/App/AlanMacPrimaryShellOwner.swift" \
    "startupMode: \\.fresh|startupMode: \\.restorePrevious|restoreShellState|ShellStatePersistenceStore" \
    "primary macOS shell must not restore workspace identity from ShellStateSnapshot"

require_pattern \
    "clients/apple/scripts/test-shell-workspace-manifest.swift" \
    "verifiesMissingManifestCreatesDefaultWithoutMigratingShellState" \
    "workspace manifest tests must prove legacy ShellStateSnapshot is not migrated"

require_pattern \
    "clients/apple/scripts/test-terminal-surface-controller.sh" \
    "TerminalSurfaceController.swift" \
    "surface controller behavior tests must compile the controller boundary"

require_pattern \
    "clients/apple/alan-macos/Models/Shell/ShellControlPlaneDTOs.swift" \
    "struct AlanShellControlCommand: Codable" \
    "shell control-plane protocol DTOs must live in the shell model boundary"

require_pattern \
    "clients/apple/alan-macos/Models/Shell/ShellControlPlaneDTOs.swift" \
    "deliveryCode: String?" \
    "pane.send_text responses must expose service delivery state"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellLocalCommandExecutor.swift" \
    "enum AlanShellLocalCommandExecutor" \
    "shell local command execution must live outside the socket server boundary"

reject_pattern \
    "clients/apple/alan-macos/ShellControlPlane.swift" \
    "enum AlanShellLocalCommandExecutor|struct AlanShellLocalCommandResult" \
    "shell control plane transport must not own local command execution"

require_pattern \
    "clients/apple/alan-macos/Controllers/Shell/ShellHostControlCommandHandling.swift" \
    "runtimePhase: delivery.runtimePhase" \
    "pane.send_text responses must expose the service runtime phase"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeRegistry.swift" \
    "final class MockTerminalRuntimeHandle" \
    "terminal runtime delivery must have a mock handle for contract tests"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeRegistry.swift" \
    "func sendText\\(to paneID: String, text: String\\)" \
    "terminal text delivery must go through the runtime registry"

require_pattern \
    "clients/apple/alan-macos/Controllers/Shell/ShellHostControlCommandHandling.swift" \
    "terminalRuntimeRegistry\\.sendText\\(to: paneID, text: text\\)" \
    "pane.send_text must use the registry delivery result"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "\\.id\\(pane\\.paneID\\)" \
    "terminal host views must be keyed by stable pane identity"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellSplitDividerView" \
    "split panes must use an explicit divider instead of visual spacing gaps"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellSplitDividerTint" \
    "split divider tint must stay subtle instead of rendering as a hard line"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellSplitDividerMetrics\\.thickness" \
    "split divider must use an intentional seam thickness instead of a hard 1px line"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellSplitDividerTint\\.shadow" \
    "split divider must use a subtle bevel seam rather than a single flat line"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "dragPreviewRatio" \
    "split divider drag must track the live preview ratio until drag end"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "resizeSplit\\(splitNodeID: node\\.nodeID, ratio: nextRatio, persist: false\\)" \
    "split divider drag previews must not persist every pointer sample"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "resizeSplit\\(splitNodeID: node\\.nodeID, ratio: finalRatio, persist: true\\)" \
    "split divider drag end must persist the final ratio"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "func resizeSplit\\(splitNodeID: String, ratio: Double, persist: Bool = true\\)" \
    "shell split resize must expose a non-persisting preview path"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "applyMutationResult\\(result, publish: persist\\)" \
    "split resize preview persistence must be controlled at mutation application"

require_pattern \
    "clients/apple/alan-macos/Models/Shell/ShellValueTypes.swift" \
    "enum ShellPaneSplitDirection" \
    "split commands must model left/right/up/down placement separately from split axis"

require_pattern \
    "clients/apple/alan-macos/Models/Shell/ShellValueTypes.swift" \
    "enum ShellSpatialFocusDirection" \
    "spatial focus commands must use explicit left/right/up/down directions"

require_pattern \
    "clients/apple/alan-macos/Models/Shell/ShellValueTypes.swift" \
    "enum ShellWorkspaceCommand: String, CaseIterable, Identifiable" \
    "shell workspace commands must remain a centralized shared vocabulary"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "func performShellWorkspaceCommand\\(_ command: ShellWorkspaceCommand\\)" \
    "menu, keyboard, and command UI actions must route through one shell command entry point"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "case \\.splitRight:" \
    "shared shell workspace command routing must be exhaustively owned by ShellHostController"

require_pattern \
    "clients/apple/alan-macos/AlanApp.swift" \
    "AlanMacShellCommands\\(host: primaryShellOwner\\.host\\)" \
    "native menu commands must receive the primary shell host"

require_pattern \
    "clients/apple/alan-macos/App/AlanMacShellCommands.swift" \
    "CommandMenu\\(\"Shell\"\\)" \
    "split workspace actions must be exposed through a native Shell menu"

require_pattern \
    "clients/apple/alan-macos/App/AlanMacShellCommands.swift" \
    "\\.keyboardShortcut\\(\"d\", modifiers: \\.command\\)" \
    "split right must have a native command-key shortcut"

require_pattern \
    "clients/apple/alan-macos/App/AlanMacShellCommands.swift" \
    "\\.keyboardShortcut\\(\"d\", modifiers: \\[\\.command, \\.shift\\]\\)" \
    "split down must have a native command-shift shortcut"

require_pattern \
    "clients/apple/alan-macos/App/AlanMacShellCommands.swift" \
    "host\\.performShellWorkspaceCommand\\(\\.closeTab\\)" \
    "native menu close actions must use the shared shell workspace command vocabulary"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift" \
    "ShellWorkspaceCommand\\.splitRight" \
    "command UI split actions must call the same shell command router as native menus"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift" \
    "host\\.performShellWorkspaceCommand\\(\\.newTerminalTab\\)" \
    "command UI tab actions must use the shared shell workspace command vocabulary"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift" \
    "GlassEffectContainer\\(spacing:" \
    "floating Ask alan command input must group custom Liquid Glass surfaces"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift" \
    "\\.glassEffect\\(\\.regular\\.interactive\\(\\), in: shape\\)" \
    "floating Ask alan command input must use the default system Liquid Glass material"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift" \
    "\\.glassEffectTransition\\(\\.identity\\)" \
    "floating Ask alan command input must disable Liquid Glass material insertion animation"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellCommandTabView.swift" \
    "private var commandInputContent: some View" \
    "floating Ask alan command input foreground content must render outside the glass-effect layer"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "private let hiddenCommandInputOpacity = 0\\.001" \
    "floating Ask alan command input must keep the Liquid Glass surface mounted before presentation"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "private func toggleCommandInput\\(\\)" \
    "Command-P must toggle the floating Ask alan command input"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "isActive: isCommandTabPresented" \
    "floating Ask alan command input must separate mounted glass identity from active text focus"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "withAnimation\\(commandInputAnimation\\)" \
    "floating Ask alan command input must fade between hidden and visible states"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "\\.opacity\\(commandInputOpacity\\)" \
    "floating Ask alan command input must use opacity fade instead of moving from an edge"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "Color\\.clear" \
    "floating Ask alan click-away layer must avoid visible dimming under Liquid Glass"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "\\.transition\\(\\.identity\\)" \
    "floating Ask alan click-away layer must not animate behind Liquid Glass on insertion"

reject_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "ShellPalette\\.overlayScrim" \
    "floating Ask alan must not place a visible dimming scrim behind Liquid Glass"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "Ask alan\\.\\.\\." \
    "command entry copy must match the accepted shell command UI label"

require_pattern \
    "clients/apple/alan-macos/Support/ShellDesignTokens.swift" \
    "collapsedRevealEdgeWidth" \
    "collapsed sidebar reveal must use a narrow edge hot zone token"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "sidebarPanelRevealAnimation" \
    "collapsed sidebar reveal must use a dedicated spring reveal animation"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "sidebarPanelHideAnimation" \
    "collapsed sidebar hide must use a dedicated fast exit animation"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "handleCollapsedSidebarToolbarHover" \
    "collapsed sidebar toolbar controls must keep the floating panel revealed while hovered"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "windowChromeSurface" \
    "collapsed sidebar chrome must publish its floating surface state to AppKit"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "isVisible: isSidebarSurfaceVisible" \
    "traffic lights must hide when the collapsed sidebar surface is hidden"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "floatingSidebarTrafficLightRevealDelay" \
    "floating sidebar traffic lights must not appear ahead of panel reveal timing"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "scheduleFloatingSidebarTrafficLightReveal" \
    "floating sidebar traffic-light visibility must be delayed separately from panel insertion"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "guard !isSidebarPanelRevealed else" \
    "repeated floating sidebar hover enters must not reset visible traffic lights"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "floatingSidebarTrafficLightRevealToken" \
    "floating sidebar traffic-light reveal timing must not share the hover-retention token"

reject_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "frame\\(width: sidebarWidth, height: windowChromeMetrics\\.collapsedRevealHeaderHeight\\)" \
    "collapsed sidebar reveal must not use the full titlebar/header width as a hover zone"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "func routeWorkspaceCommand\\(_ input: AlanTerminalKeyInput\\) -> ShellWorkspaceCommand\\?" \
    "terminal input routing must recognize alan workspace shortcuts before terminal bindings"

require_pattern \
    "clients/apple/alan-macos/TerminalSurfaceController.swift" \
    "return \\.newTerminalTab" \
    "terminal keyboard tab shortcuts must map to the shared shell workspace command vocabulary"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "routeWorkspaceKeyCommandIfNeeded\\(event\\)" \
    "terminal host key equivalents must give alan workspace shortcuts priority over Ghostty bindings"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "private let runtimeReporter = TerminalHostRuntimeReporter\\(\\)" \
    "terminal host runtime snapshot publication must be owned by a focused collaborator"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostRuntimeReporter.swift" \
    "snapshotsEqualIgnoringTimestamp" \
    "terminal runtime reporter must preserve timestamp-insensitive snapshot deduplication"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "private let windowObserver = TerminalHostWindowObserver\\(\\)" \
    "terminal host window notifications must be owned by a focused collaborator"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostWindowObserver.swift" \
    "NSWindow\\.didChangeOcclusionStateNotification" \
    "terminal host window observer must keep occlusion changes connected to surface/runtime refresh"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "host\\.performShellWorkspaceCommand\\(command\\)" \
    "terminal workspace shortcut routing must enter the shared shell workspace command handler"

require_pattern \
    "clients/apple/alan-macos/Controllers/Shell/ShellHostControlCommandHandling.swift" \
    "func handleControlPlaneCommand\\(_ command: AlanShellControlCommand\\)" \
    "control-plane protocol commands must stay separate from UI command vocabulary while sharing shell mutation authority"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellTerminalSurfaceFrame" \
    "terminal panes must share one outer rounded terminal surface frame"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "pinnedSidebarPresentationProgress" \
    "pinned sidebar collapse must be driven by continuous presentation progress"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift" \
    "expandedSidebarProgress" \
    "workspace view must expose continuous sidebar progress for terminal surface spacing"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "frame\\(width: sidebarPinnedVisibleWidth" \
    "pinned sidebar must stay mounted while visible width animates"

reject_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "if !isSidebarCollapsed \\{" \
    "pinned sidebar must not be conditionally inserted or removed"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "terminalSurfaceInsets: EdgeInsets" \
    "terminal pane must receive semantic terminal surface edge insets"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "padding\\(terminalSurfaceInsets\\)" \
    "terminal pane must apply state-aware terminal surface edge insets"

reject_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "cornerRadius = ShellRadii\\.terminalSurface" \
    "terminal host view must not apply an inner rounded corner inside the outer terminal surface"

require_pattern \
    "clients/apple/alan-macos/Support/ShellDesignTokens.swift" \
    "terminalSurfaceInsets\\(expandedSidebarProgress" \
    "terminal workspace surface insets must support continuous sidebar progress"

require_pattern \
    "clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift" \
    "struct ShellSidebarSwipeMonitor" \
    "sidebar swipe monitor must remain the input adapter"

require_pattern \
    "clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift" \
    "struct ShellSidebarSwipeUpdate" \
    "sidebar swipe monitor must emit swipe input updates"

require_pattern \
    "clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift" \
    "struct ShellSidebarSpaceContentPagerState" \
    "space swipes must use sidebar content pager state"

require_pattern \
    "clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift" \
    "sourceIndex" \
    "space pager state must track the authoritative source space index"

require_pattern \
    "clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift" \
    "targetIndex" \
    "space pager state must track the adjacent target space index"

require_pattern \
    "clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift" \
    "settlementPhase" \
    "space pager state must model drag, commit, and cancel settlement phases"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "spacePager" \
    "sidebar view must own sidebar-local space pager state"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "ShellSidebarSwipeMonitor\\(onUpdate: handleSpaceSwipe\\)" \
    "sidebar view must install the sidebar swipe monitor as its input adapter"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "spaceContentPager" \
    "sidebar view must render sidebar-local space content pager pages"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "commandLauncher" \
    "sidebar-local pager motion must keep the command launcher owned by the sidebar"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "spaceDock" \
    "sidebar-local pager motion must keep the bottom space dock owned by the sidebar"

reject_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "spacePager|spacePagerPages|spacePage\\(index:" \
    "mac shell root must not reintroduce root full-window space pager semantics"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "HStack\\(spacing: 0\\)" \
    "mac shell root must keep a stable sidebar/workspace HStack layout"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "ShellWorkspaceView\\(" \
    "mac shell root must render the committed workspace surface"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift" \
    "tab: host\\.selectedTab" \
    "workspace view must render committed host tab selection"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift" \
    "spaceID: host\\.selectedSpace\\?\\.spaceID" \
    "workspace view must render committed host space selection"

require_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift" \
    "selectedPaneID: host\\.selectedPane\\?\\.paneID" \
    "workspace view must render committed host pane selection"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "selectedPaneID: String\\?" \
    "terminal pane view must render preview pages without borrowing selected-pane state"

reject_pattern \
    "clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift" \
    "ShellSpaceTransition" \
    "space swipe support must not reintroduce the sidebar-only transition model"

reject_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "ShellSpaceTransition|spaceTransition" \
    "mac shell root must use shared space pager state instead of sidebar-only transition state"

reject_pattern \
    "clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift" \
    "ShellSidebarSpaceHeaderPager|activeTransition|sourceOffset\\(|targetOffset\\(" \
    "sidebar view must not own independent header/tab-list pager semantics"

require_pattern \
    "clients/apple/scripts/test-shell-window-placement.swift" \
    "verifiesSystemModeClearsExplicitWindowAppearanceImmediately" \
    "window-placement tests must cover immediate reset to system appearance mode"

require_pattern \
    "clients/apple/scripts/test-shell-window-placement.sh" \
    "ShellWindowPlacement\\.swift" \
    "window-placement tests must compile the macOS shell appearance bridge"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "resolvedAppearanceColorScheme" \
    "mac shell root must resolve system appearance into an explicit SwiftUI colorScheme environment"

reject_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "preferredColorScheme" \
    "mac shell appearance switching must not rely on clearing a stale SwiftUI preferredColorScheme preference"

reject_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "padding\\(ShellWorkspaceMetrics\\.terminalSurfaceInset\\)" \
    "terminal surface must not apply equal workspace inset when the sidebar is expanded"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellPaneTitleBarView" \
    "visible terminal panes must render a compact pane title bar"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "shellPaneTitleBarTitle" \
    "pane title bars must use a dedicated title helper with terminal-title-first priority"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "func closePaneByID\\(_ paneID: String\\) -> Bool" \
    "pane title-bar close must route through a controller-owned targeted pane close path"

require_pattern \
    "clients/apple/scripts/test-shell-split-model.swift" \
    "verifiesPaneScopedCloseKeepsInactivePaneTargeting" \
    "split model tests must cover pane-scoped close targeting"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "ShellInactivePaneDim" \
    "inactive split panes must use a lightweight dim treatment"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "allowsHitTesting\\(false\\)" \
    "inactive pane dimming must not intercept terminal pointer input"

require_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "@AppStorage\\(\"alanShellDimsInactiveSplitPanes\"\\)" \
    "inactive pane dimming must be backed by a user-default preference"

reject_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "splitChildren" \
    "split panes must not leave a fixed gap between adjacent terminal panes"

reject_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "paneSelectorStrip" \
    "split panes must not show a bottom pane tab strip by default"

reject_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "Color\\.primary\\.opacity\\(0\\.16\\)" \
    "split divider must not render as a high-contrast primary-color line"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "hasTornDownRuntime" \
    "terminal teardown must be idempotent"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostViewSupport.swift" \
    "let isSelected: Bool" \
    "terminal hosts must know whether their pane is selected"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "guard isSelected, pane != nil else \\{ return \\}" \
    "terminal auto-focus must be gated to the selected pane"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostViewSupport.swift" \
    "terminalHostShouldAutoFocusAfterConfigure" \
    "terminal auto-focus must only be requested on initial attachment or selected-pane transitions"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostViewSupport.swift" \
    "previousPaneID != paneID \\|\\| !wasSelected" \
    "terminal auto-focus must not refocus the same selected pane on every SwiftUI update"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "guard !pendingFocusRequest else \\{ return \\}" \
    "terminal auto-focus must coalesce pending first-responder requests"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostViewSupport.swift" \
    "protocol TerminalHostActivationDelegate: AnyObject" \
    "terminal activation must use a narrow class-bound delegate"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "TerminalHostActivationDelegate" \
    "shell host controller must own terminal activation requests"

require_pattern \
    "clients/apple/alan-macos/TerminalRuntimeRegistry.swift" \
    "activationDelegate: TerminalHostActivationDelegate\\?" \
    "terminal runtime registry must thread the weak activation boundary"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "weak var activationDelegate" \
    "registry-owned terminal host views must not strongly retain activation owners"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "targetPaneID\\(forSpaceID: spaceID\\)" \
    "sidebar space selection must resolve a target pane before committing selection"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "targetPaneID\\(forTabID: tabID, in: selectedSpace\\)" \
    "sidebar tab selection must resolve a target pane before committing selection"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "terminalRuntimeRegistry\\.requestFocus\\(for: paneID\\)" \
    "committed sidebar selection must request terminal focus through the runtime registry"

reject_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "selectedSpaceID = spaceID" \
    "sidebar space selection must not be view-local-only"

reject_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "selectedTabID = tabID" \
    "sidebar tab selection must not be view-local-only"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "terminalHostDidRequestActivation\\(paneID:" \
    "terminal host mouse events must request pane activation through the delegate"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "override func mouseDown\\(with event: NSEvent\\)" \
    "terminal pointer down events must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "private func routePointer\\(_ input: AlanTerminalPointerInput\\)" \
    "terminal pointer routing must stay behind the AppKit terminal host boundary"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "private func routeScrollWheel\\(_ event: NSEvent\\)" \
    "terminal scroll routing must stay behind the AppKit terminal host boundary"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "override func scrollWheel\\(with event: NSEvent\\)" \
    "terminal scroll wheel events must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "override func keyDown\\(with event: NSEvent\\)" \
    "terminal key events must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "func insertText\\(_ string: Any, replacementRange: NSRange\\)" \
    "terminal IME text insertion must remain owned by the AppKit terminal host"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "workspaceCommandHandler\\?\\(command\\)" \
    "terminal workspace shortcuts must leave the AppKit host through the shared command callback"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostOverlayPresenter.swift" \
    "let overlayCard = AlanTerminalPassiveOverlayView\\(\\)" \
    "passive terminal overlays must use a non-interactive overlay view"

reject_pattern \
    "clients/apple/alan-macos/TerminalPaneView.swift" \
    "onTapGesture\\(perform: onSelect\\)" \
    "terminal leaf selection must not be owned by a SwiftUI tap wrapper"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "window\\.isMovableByWindowBackground = true" \
    "hidden-titlebar shell windows must make non-interactive background regions draggable"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "contentInteractionTopInset" \
    "window double-click zoom overlay must not cover terminal surface title-bar controls"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "isPointInSidebarChromeBand" \
    "window double-click zoom overlay must include blank sidebar chrome outside real controls"

require_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "width: sidebarWidth" \
    "window double-click zoom hit testing must know the sidebar chrome width"

require_pattern \
    "clients/apple/scripts/test-shell-window-placement.swift" \
    "verifiesTitlebarOverlayRejectsTerminalSurfaceTitleBarHit" \
    "shell window placement tests must prove terminal title-bar controls are not intercepted by zoom overlay"

require_pattern \
    "clients/apple/scripts/test-shell-window-placement.swift" \
    "verifiesTitlebarOverlayAcceptsSidebarChromeBlankHit" \
    "shell window placement tests must prove blank sidebar chrome remains a double-click zoom target"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "NSWindow\\.didResizeNotification" \
    "hidden-titlebar shell windows must resynchronize traffic-light placement after resize"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "NSWindow\\.willStartLiveResizeNotification" \
    "hidden-titlebar shell windows must start continuous traffic-light sync during live resize"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "liveResizeChromeSyncTimer" \
    "hidden-titlebar shell windows must keep a scoped live-resize chrome sync timer"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "RunLoop\\.main\\.add\\(timer, forMode: \\.eventTracking\\)" \
    "live-resize chrome sync must run in the event-tracking run loop mode"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "NSWindow\\.didEnterFullScreenNotification" \
    "hidden-titlebar shell windows must publish native fullscreen chrome state"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "standardTrafficLightsVisible = false" \
    "native fullscreen must stop reserving titlebar space for hidden traffic lights"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "ShellWindowChromeSurface" \
    "window chrome sync must accept sidebar surface visibility and origin"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "setStandardWindowButtons\\(in: window, hidden: true\\)" \
    "standard traffic lights must hide with a hidden sidebar surface"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "chromeSurfaceOrigin" \
    "standard traffic lights must follow the visible floating sidebar surface origin"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "showsStandardTrafficLights" \
    "window chrome sync must distinguish surface layout from actual traffic-light visibility"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "setStandardWindowButtons\\(in: window, hidden: false, alphaValue: 0\\)" \
    "floating sidebar traffic lights must be made invisible before AppKit-visible repositioning"

require_pattern \
    "clients/apple/alan-macos/Support/ShellWindowPlacement.swift" \
    "localTrafficLightGroupFrame" \
    "floating sidebar traffic lights must be rechecked after standard button visibility changes"

require_pattern \
    "clients/apple/alan-macos/TerminalHostView.swift" \
    "override var mouseDownCanMoveWindow: Bool \\{ false \\}" \
    "terminal host views must not allow terminal pane clicks to drag the shell window"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostViewSupport.swift" \
    "final class AlanTerminalFallbackCanvasView" \
    "fallback terminal canvas views must explicitly opt out of background window dragging"

require_pattern \
    "clients/apple/alan-macos/Services/Terminal/TerminalHostViewSupport.swift" \
    "override func hitTest\\(_ point: NSPoint\\) -> NSView\\? \\{ nil \\}" \
    "fallback terminal canvas views must be transparent to AppKit hit-testing"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "override var mouseDownCanMoveWindow: Bool \\{ false \\}" \
    "Ghostty canvas views must not allow terminal pane clicks to drag the shell window"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "override func hitTest\\(_ point: NSPoint\\) -> NSView\\? \\{ nil \\}" \
    "Ghostty canvas views must be transparent to AppKit hit-testing"

reject_pattern \
    "clients/apple/alan-macos" \
    "WindowDragGesture\\(\\)" \
    "shell window dragging should rely on movable background regions, not transparent SwiftUI drag overlays"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "let visible = .*occlusionState\\.contains\\(\\.visible\\) \\?\\? false" \
    "Ghostty occlusion bridge must derive the visible flag from NSWindow occlusion state"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "ghostty_surface_set_occlusion\\(surface, visible\\)" \
    "GhosttyKit bridge must pass the observed visible state used by this linked Ghostty build"

reject_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "let isOccluded =|isSurfaceOccluded|!isVisible" \
    "GhosttyKit bridge must not invert NSWindow visible state for this linked Ghostty build"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "if let surface = self\\.surface" \
    "Ghostty wakeup ticks must look up the current surface before refreshing"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "private var tickScheduled = false" \
    "Ghostty wakeup ticks must be coalesced so repeated wakeups do not flood the main queue"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "guard markTickScheduledIfNeeded\\(\\) else \\{ return \\}" \
    "Ghostty wakeup ticks must skip scheduling when a tick is already pending"

require_pattern \
    "clients/apple/alan-macos/GhosttyLiveHost.swift" \
    "clearScheduledTick\\(\\)" \
    "Ghostty wakeup ticks must clear their pending marker when the scheduled tick begins"

require_pattern \
    "clients/apple/alan-macos/ShellHostController.swift" \
    "struct ShellWindowContext" \
    "shell host must expose a shell context type for the singleton primary window"

reject_pattern \
    "clients/apple/alan-macos/MacShellRootView.swift" \
    "ShellWindowContext\\.make\\(\\)" \
    "macOS root view must use the app-scoped primary shell owner instead of creating a fresh context"

require_pattern \
    "clients/apple/alan-macos/AlanApp.swift" \
    "Window\\(\"alan\", id: \"main\"\\)" \
    "macOS app scene must use a launch-presented singleton primary Window"

reject_pattern \
    "clients/apple/alan-macos/AlanApp.swift" \
    "WindowGroup\\(\"alan\", id: \"main\"\\)" \
    "macOS primary shell scene must not use a WindowGroup that can miss first-launch presentation"

require_pattern \
    "clients/apple/alan-macos/AlanApp.swift" \
    "defaultLaunchBehavior\\(\\.presented\\)" \
    "macOS primary window must be presented when the app launches without restoration"

require_pattern \
    "clients/apple/alan-macos/App/AlanMacShellCommands.swift" \
    "CommandGroup\\(replacing: \\.newItem\\)" \
    "macOS app must replace New Window with a focus/reopen command"

require_pattern \
    "clients/apple/alan-macos/AlanApp.swift" \
    "AlanMacAppStartup\\.acquireSingletonOrTerminate\\(\\)" \
    "macOS app startup must acquire the singleton guard before creating shell state"

require_pattern \
    "clients/apple/alan-macos/AlanAppSingletonGuard.swift" \
    "flock\\(descriptor, LOCK_EX \\| LOCK_NB\\)" \
    "macOS app singleton guard must use an OS-backed exclusive lock"

reject_pattern \
    "justfile" \
    "^app:" \
    "just app must not be reintroduced as the local macOS app workflow"

reject_pattern \
    "justfile" \
    "app-debug-run" \
    "debug app runner recipes must not replace the removed just app workflow"

require_pattern \
    "justfile" \
    "^install:" \
    "just install must remain the local release install workflow"

reject_pattern \
    "scripts/install.sh" \
    "\\.alan/bin" \
    "local install must not write CLI/TUI entries under ~/.alan/bin"

require_pattern \
    "clients/apple/alan-macos/App/AlanMacShellCommands.swift" \
    "Install Command Line Tools" \
    "direct app installs must expose an explicit command-line tools install action"

require_pattern \
    "clients/apple/alan-macos/Support/AlanCommandLineToolInstaller.swift" \
    "defaultInstallDirectory = URL\\(fileURLWithPath: \"/usr/local/bin\"" \
    "direct app command-line tool installer must use a conventional PATH directory instead of ~/.alan/bin"

require_pattern \
    "clients/apple/alan-macos/Support/AlanCommandLineToolInstaller.swift" \
    "homebrewManagedCommandLinks" \
    "direct app command-line tool installer must detect existing Homebrew-managed links before creating alternate PATH links"

require_pattern \
    "scripts/install.sh" \
    "has_homebrew_managed_tool_links" \
    "local install must detect existing Homebrew-managed links before creating alternate PATH links"

require_pattern \
    "clients/apple/alan-macos/TerminalHostRuntime.swift" \
    "bundled_resource_binary" \
    "alan launch resolution must support the app-bundled CLI"

reject_pattern \
    "clients/apple/alan-macos/TerminalHostRuntime.swift" \
    "\\.alan/bin" \
    "alan launch resolution must not use ~/.alan/bin"

require_pattern \
    "clients/apple/scripts/test-command-line-tool-installer.sh" \
    "AlanCommandLineToolInstaller.swift" \
    "command-line tool installer behavior must have a focused test"

require_pattern \
    "scripts/validate-release-app.sh" \
    "Developer ID Application" \
    "release app validation must require Developer ID signatures"

require_pattern \
    "scripts/validate-release-app.sh" \
    "require_manifest_checksum" \
    "release app validation must compare manifest checksums with embedded binaries"

require_pattern \
    "scripts/entitlements/alan-tui.entitlements" \
    "com\\.apple\\.security\\.cs\\.allow-jit" \
    "standalone alan-tui must declare the hardened-runtime JIT entitlement it needs to launch"

require_pattern \
    "scripts/assemble-release-app.sh" \
    "alan-tui\\.entitlements" \
    "release assembly must sign alan-tui with its dedicated hardened-runtime entitlements"

require_pattern \
    "scripts/validate-release-app.sh" \
    "com\\.apple\\.security\\.cs\\.allow-jit" \
    "release app validation must verify alan-tui hardened-runtime launch entitlements"

require_pattern \
    "scripts/release-env.sh" \
    "ALAN_DEVELOPER_ID_APPLICATION" \
    "release env loader must accept canonical alan signing identity variables"

reject_pattern \
    "scripts/release-env.sh" \
    "APPLE_API_KEY" \
    "release env loader must expose only the Apple ID app-specific-password notarization path"

reject_pattern \
    "scripts/assemble-release-app.sh" \
    "key-id" \
    "release assembly must submit notarization through the keychain profile only"

reject_pattern \
    "scripts/ensure-notary-profile.sh" \
    "key-id" \
    "notary profile setup must use only Apple ID app-specific-password credentials"

require_pattern \
    "scripts/ensure-notary-profile.sh" \
    "notarytool store-credentials" \
    "release automation must be able to create or refresh the notary keychain profile"

require_pattern \
    "scripts/release-check.sh" \
    "ensure-notary-profile.sh" \
    "release-check must validate notarization setup before building"

require_pattern \
    "justfile" \
    "^release:" \
    "just release must provide the public signed/notarized release workflow"

require_pattern \
    "scripts/validate-homebrew-cask.sh" \
    "Contents/Resources/bin/alan-tui" \
    "Homebrew cask validation must check embedded CLI/TUI binary links"

require_pattern \
    "clients/apple/README.md" \
    "stable .*window_main.* identity" \
    "Apple client docs must describe the singleton primary shell identity"

reject_pattern \
    "clients/apple/README.md" \
    "Each macOS window creates its own shell context" \
    "Apple client docs must not describe each macOS window as an independent shell context"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellSocketServer.swift" \
    "private static let maxRequestBytes" \
    "socket server must enforce a bounded request size"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellSocketServer.swift" \
    "command_timeout" \
    "socket server must return a stable timeout error"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellSocketServer.swift" \
    "private static let maxConcurrentClients" \
    "socket server must keep concurrency limits in the transport owner"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellPublishedStateMerger.swift" \
    "enum AlanShellPublishedStateMerger" \
    "published state merging must live in a dedicated shell service owner"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellControlFilePoller.swift" \
    "final class AlanShellControlFilePoller" \
    "file-polling control plane must live in a dedicated shell service owner"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellEventStore.swift" \
    "final class AlanShellEventStore" \
    "shell event persistence must live in a dedicated shell service owner"

require_pattern \
    "clients/apple/alan-macos/Services/Shell/ShellDiagnostics.swift" \
    "final class AlanShellDiagnostics" \
    "shell diagnostics routing must live in a dedicated shell service owner"

reject_pattern \
    "clients/apple/alan-macos/ShellControlPlane.swift" \
    "final class AlanShellSocketServer|SO_RCVTIMEO|SO_SNDTIMEO|maxRequestBytes|command_timeout|enum AlanShellPublishedStateMerger|pollCommands\\(|pollBindings\\(|handleCommandFile\\(|appendEvent\\(|readEvents\\(|recordEvents\\(|recordDiagnostic\\(" \
    "shell control-plane coordinator must not own socket transport bounds, state merging, file polling, event persistence, or diagnostic routing"

reject_pattern \
    "clients/apple/alan-macos" \
    "NotificationCenter\\.default\\.post" \
    "control-plane text delivery must not rely on NotificationCenter broadcast success"

printf 'Shell contract checks passed.\n'
