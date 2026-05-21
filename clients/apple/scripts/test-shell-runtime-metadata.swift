import CoreGraphics
import Darwin
import Foundation

#if os(macOS)
@main
struct ShellRuntimeMetadataTestRunner {
    static func main() async {
        await MainActor.run {
            ShellRuntimeMetadataTests.run()
        }
    }
}

@MainActor
private enum ShellRuntimeMetadataTests {
    static func run() {
        verifiesRuntimeProjectsTerminalStatusIntoPaneMetadata()
        verifiesSurfaceExitClosesFinalPaneWithoutRestarting()
        verifiesTerminalStatusSummaryPrioritizesExitAndRendererHealth()
        verifiesPaneTitleBarPrefersTerminalTitle()
        verifiesPaneTitleBarFallbackOrdering()
        verifiesPaneTitleBarSuppressesInternalTitles()
        verifiesOpeningTabSkipsStaleRuntimePaneIDs()
        verifiesOpeningTerminalTabInheritsFocusedRuntimeCwd()
        verifiesShellActionNewTerminalTabInheritsFocusedRuntimeCwd()
        verifiesOpeningTerminalTabFallsBackToFocusedPaneSnapshotCwd()
        verifiesOpeningTerminalTabHonorsExplicitCwd()
        verifiesQuickTerminalShowCreatesAndReusesGlobalPane()
        verifiesQuickTerminalActionsAndControlCommandsShareControllerPath()
        verifiesQuickTerminalPromotionMovesExistingPaneIntoSpace()
        verifiesQuickTerminalPeakPresenterShowsDetachedTerminalWindow()
        verifiesQuickTerminalPeakPresenterPreservesRuntimeOnExplicitHide()
        verifiesQuickTerminalPeakPresenterDoesNotRefocusOnVisibleRefresh()
        verifiesQuickTerminalPeakPlacementFitsActiveDisplay()
        verifiesQuickTerminalPeakEscapePolicyBelongsToTerminal()
        verifiesSplitZoomLeavesCanonicalTreeAndKeepsSiblingRuntimes()
        verifiesSplitZoomIsTabScopedAndPrunedWhenPaneDisappears()
        verifiesInTabPaneMovementPreservesRuntimeContinuity()
        verifiesPaneMovementDragPolicyProtectsTerminalSelection()
        verifiesTerminalActivityProjectsByPaneID()
        verifiesProgressActivityFactoryUsesSourceFirstDisplay()
        verifiesCommandCompletionActivityFactory()
        verifiesTerminalActivityCodableUsesSnakeCase()
        verifiesTerminalActivitySidebarPriority()
        verifiesStaleProgressIsNotSidebarWorthy()
        verifiesDefaultSidebarActivitySelectionHonorsFreshness()
        verifiesSuccessfulCommandIsNotSidebarWorthy()
        verifiesCodexAgentActivityAdapterMapsSupportedStates()
        verifiesAgentActivityAdapterSanitizesDefaultUIPayload()
        verifiesAgentActivityAdapterRejectsMalformedPayloadAndFallsBackForUnsupportedAgent()
        verifiesAgentActivityControlCommandProjectsOntoPane()
        verifiesClearingActivityRemovesPaneActivity()
        verifiesPublishedStateMergeClearsActivity()
        verifiesPaneRebuildMutationsPreserveActivity()
        verifiesTabSidebarActivityProjectionUsesHighestPriorityPane()
        verifiesTabSidebarProjectionFallsBackToRepositoryBranch()
        verifiesTabSidebarProjectionPreservesTerminalStatusBeforeContext()
        verifiesTabSidebarProjectionDoesNotResurrectStaleCommandFailure()
        verifiesSidebarProgressRailBelongsToDisplayedActivity()
        verifiesFocusedCommandFailureDemotesFromSidebarProjection()
        verifiesCommandFailureAcknowledgementSticksAfterFocus()
        verifiesActivityFreshnessPolicies()
        verifiesActivityAttentionIsReadTimeOnly()
        verifiesPaneTitleActivityAccessoryLabel()
        verifiesPaneTitleDetailProjectionIncludesContextBranchAndProcess()
        verifiesPaneTitleDetailProjectionPreservesResponsivePriority()
        verifiesPaneTitleDetailProjectionAvoidsDuplicateAgentAndAlan()
        verifiesActivityNotificationPolicyIsLowNoise()
        verifiesControllerRoutesActivityNotificationsOnce()
        verifiesControllerRoutesDistinctActivityPayloadsInSameSecond()
        verifiesInactiveAppRoutesFocusedPaneNotifications()
        verifiesHiddenQuickTerminalRoutesUserActionableActivityNotifications()
        verifiesProcessExitNotificationRoutesBeforeAutoClose()
        verifiesProcessExitRuntimeNotificationRoutesBeforeAutoClose()
        verifiesTerminalChildExitClosesSplitPane()
        verifiesTerminalChildExitClosesSinglePaneTab()
        verifiesTerminalChildExitCanLeaveEmptyFocusedSpace()
        verifiesClosingTabReleasesTerminalRuntime()
        verifiesTabSelectionCommitsAuthoritativeFocus()
        verifiesShellActionTabNavigationTargetsCurrentSelection()
        verifiesSpaceSelectionCommitsAuthoritativeFocus()
        verifiesShellActionSpaceSelectionReportsMissingTargets()
        verifiesSplitTabSelectionUsesStablePaneWithoutChangingLayout()
        verifiesWorkspaceManifestStartupRestoresPinnedSnapshot()
        verifiesClosingLastTabLeavesSelectedSpaceEmptyAndPersistsManifest()
        verifiesExplicitSpaceDeletionRemovesManifestSpace()
        verifiesPinSnapshotIsExplicitAndDoesNotTrackTransientChanges()
        verifiesTabOrganizationPersistsOrderPinAndSpaceOwnership()
        verifiesManifestActiveTaskProjection()
        print("Shell runtime metadata tests passed.")
    }

    private static func verifiesRuntimeProjectsTerminalStatusIntoPaneMetadata() {
        let controller = makeController()
        guard let pane = controller.selectedPane else {
            fail("bootstrap shell must expose a selected pane")
        }

        controller.updateTerminalRuntime(
            TerminalHostRuntimeSnapshot(
                stage: .windowAttached,
                paneID: pane.paneID,
                tabID: pane.tabID,
                logicalSize: .zero,
                backingSize: .zero,
                displayName: "Studio Display",
                displayID: "display_1",
                attachedWindowTitle: "alan",
                isFocused: false,
                renderer: TerminalRendererSnapshot(
                    kind: .ghosttyLive,
                    phase: .failed,
                    summary: "renderer failed",
                    detail: "lost drawable",
                    failureReason: "lost device",
                    recentEvents: ["device lost"]
                ),
                paneMetadata: TerminalPaneMetadataSnapshot(
                    title: "vim main.rs",
                    workingDirectory: "/Users/morris/Developer/Alan",
                    summary: "terminal bell",
                    attention: .notable,
                    processExited: false,
                    lastCommandExitCode: nil,
                    lastUpdatedAt: Date(timeIntervalSince1970: 1_000)
                ),
                surfaceState: AlanTerminalSurfaceStateSnapshot(
                    readiness: .unready(reason: .rendererFailed),
                    terminalMode: .normalBuffer,
                    scrollback: .empty,
                    search: nil,
                    semanticCommands: .placeholder,
                    readonly: false,
                    secureInput: false,
                    inputReady: false,
                    rendererHealth: "failed",
                    childExited: false,
                    lastUpdatedAt: Date(timeIntervalSince1970: 1_001)
                ),
                lastUpdatedAt: Date(timeIntervalSince1970: 1_002)
            )
        )

        let updated = controller.shellState.panes.first { $0.paneID == pane.paneID }
        expect(updated?.context?.rendererHealth == "failed", "pane context must record renderer health")
        expect(updated?.context?.surfaceReadiness == "renderer_failed", "pane context must record surface readiness")
        expect(updated?.context?.inputReady == false, "pane context must record input readiness")
        expect(updated?.context?.terminalMode == "normal_buffer", "pane context must record terminal mode")
        expect(updated?.viewport?.title == "vim main.rs", "pane viewport must record terminal title")
        expect(updated?.viewport?.summary == "Renderer failed", "pane viewport must expose renderer status")
        expect(updated?.attention == .notable, "pane attention must reflect terminal attention")
        expect(controller.shellState.spaces.first?.attention == .notable, "space attention must track pane attention")
    }

    private static func verifiesSurfaceExitClosesFinalPaneWithoutRestarting() {
        let controller = makeController()
        guard let pane = controller.selectedPane else {
            fail("bootstrap shell must expose a selected pane")
        }

        controller.updateTerminalRuntime(
            TerminalHostRuntimeSnapshot(
                stage: .windowAttached,
                paneID: pane.paneID,
                tabID: pane.tabID,
                logicalSize: .zero,
                backingSize: .zero,
                displayName: "Studio Display",
                displayID: "display_1",
                attachedWindowTitle: "alan",
                isFocused: false,
                renderer: TerminalRendererSnapshot(
                    kind: .ghosttyLive,
                    phase: .surfaceReady,
                    summary: "surface ready",
                    detail: nil,
                    failureReason: nil,
                    recentEvents: []
                ),
                paneMetadata: TerminalPaneMetadataSnapshot(
                    title: "fish",
                    workingDirectory: "/Users/morris/Developer/Alan",
                    summary: "terminal rendering",
                    attention: .idle,
                    processExited: false,
                    lastCommandExitCode: 7,
                    lastUpdatedAt: Date(timeIntervalSince1970: 2_000)
                ),
                surfaceState: AlanTerminalSurfaceStateSnapshot(
                    readiness: .unready(reason: .childExited),
                    terminalMode: .normalBuffer,
                    scrollback: .empty,
                    search: nil,
                    semanticCommands: .placeholder,
                    readonly: false,
                    secureInput: false,
                    inputReady: false,
                    rendererHealth: "surface_ready",
                    childExited: true,
                    lastUpdatedAt: Date(timeIntervalSince1970: 2_001)
                ),
                lastUpdatedAt: Date(timeIntervalSince1970: 2_002)
            )
        )

        expect(
            controller.shellState.pane(paneID: pane.paneID) == nil,
            "surface child exit must close the owning final pane"
        )
        expect(
            controller.shellState.spaces.first?.tabs.isEmpty == true,
            "surface child exit must leave the focused space empty instead of restarting a terminal"
        )
        expect(controller.shellState.focusedPaneID == nil, "surface child exit must clear pane focus")
    }

    private static func verifiesTerminalStatusSummaryPrioritizesExitAndRendererHealth() {
        let exited = pane(
            context: context(
                processState: "exited",
                rendererHealth: "ready",
                surfaceReadiness: "child_exited",
                lastCommandExitCode: 2
            ),
            viewport: ShellViewportSnapshot(
                title: "fish",
                summary: "terminal bell",
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            attention: .awaitingUser
        )
        expect(shellTerminalStatusSummary(for: exited) == "Exited 2", "exit status must outrank cwd or generic summaries")

        let failedRenderer = pane(
            context: context(
                processState: "running",
                rendererHealth: "failed",
                surfaceReadiness: "renderer_failed",
                lastCommandExitCode: nil
            ),
            viewport: ShellViewportSnapshot(
                title: "fish",
                summary: "terminal bell",
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            attention: .notable
        )
        expect(shellTerminalStatusSummary(for: failedRenderer) == "Renderer failed", "renderer failure must outrank generic summaries")

        let ordinary = pane(
            context: context(
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: ShellViewportSnapshot(
                title: "fish",
                summary: "idle shell",
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            attention: .idle
        )
        expect(shellTerminalStatusSummary(for: ordinary) == nil, "ordinary summaries must not hide cwd or branch metadata")
    }

    private static func verifiesPaneTitleBarPrefersTerminalTitle() {
        let title = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: "alan",
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: ShellViewportSnapshot(
                    title: "vim main.rs - fish",
                    summary: nil,
                    visibleExcerpt: nil,
                    lastActivityAt: nil
                ),
                cwd: "/Users/morris/Developer/Alan",
                process: ShellProcessBinding(program: "fish", argvPreview: nil),
                attention: .idle
            )
        )

        expect(title == "vim main.rs", "pane title bar must prefer normalized terminal title over cwd")
    }

    private static func verifiesPaneTitleBarFallbackOrdering() {
        let cwdTitle = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: "Workspace",
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: nil,
                cwd: "/tmp/project",
                process: ShellProcessBinding(program: "fish", argvPreview: nil),
                attention: .idle
            )
        )
        expect(cwdTitle == "project", "pane title bar must use cwd leaf before working-directory name")

        let workingDirectoryTitle = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: "Workspace",
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: nil,
                cwd: nil,
                process: ShellProcessBinding(program: "fish", argvPreview: nil),
                attention: .idle
            )
        )
        expect(workingDirectoryTitle == "Workspace", "pane title bar must use working directory when cwd is missing")

        let alanTitle = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: nil,
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: nil,
                cwd: nil,
                launchTarget: .alan,
                process: ShellProcessBinding(program: "alan", argvPreview: nil),
                attention: .idle
            )
        )
        expect(alanTitle == "alan", "pane title bar must expose alan launch-target fallback")

        let processTitle = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: nil,
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: nil,
                cwd: nil,
                process: ShellProcessBinding(program: "fish", argvPreview: nil),
                attention: .idle
            )
        )
        expect(processTitle == "fish", "pane title bar must use process fallback before generic Terminal")
    }

    private static func verifiesPaneTitleBarSuppressesInternalTitles() {
        let debugTitle = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: "alan",
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: ShellViewportSnapshot(
                    title: "title updated",
                    summary: nil,
                    visibleExcerpt: nil,
                    lastActivityAt: nil
                ),
                cwd: "/Users/morris/Developer/Alan",
                process: ShellProcessBinding(program: "fish", argvPreview: nil),
                attention: .idle
            )
        )
        expect(debugTitle == "alan", "pane title bar must suppress debug title text")

        let rawPaneTitle = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: "Workspace",
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: ShellViewportSnapshot(
                    title: "pane_42",
                    summary: nil,
                    visibleExcerpt: nil,
                    lastActivityAt: nil
                ),
                cwd: nil,
                process: ShellProcessBinding(program: "fish", argvPreview: nil),
                attention: .idle
            )
        )
        expect(rawPaneTitle == "Workspace", "pane title bar must suppress raw pane IDs")

        let longTitle = "ssh production-shell-with-a-very-long-title.example.com"
        let preservedLongTitle = shellPaneTitleBarTitle(
            for: pane(
                context: context(
                    workingDirectoryName: nil,
                    processState: "running",
                    rendererHealth: "ready",
                    surfaceReadiness: "ready",
                    lastCommandExitCode: nil
                ),
                viewport: ShellViewportSnapshot(
                    title: longTitle,
                    summary: nil,
                    visibleExcerpt: nil,
                    lastActivityAt: nil
                ),
                cwd: nil,
                process: nil,
                attention: .idle
            )
        )
        expect(preservedLongTitle == longTitle, "pane title helper must leave long titles available for UI truncation")
    }

    private static func verifiesOpeningTabSkipsStaleRuntimePaneIDs() {
        let windowID = "metadata_test_\(UUID().uuidString)"
        let registry = TerminalRuntimeRegistry(runtimeService: FakeAlanTerminalRuntimeService())
        let context = ShellWindowContext.make(
            windowID: windowID,
            terminalRuntimeRegistry: registry
        )
        let persistenceURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID).json")
        let controller = ShellHostController(
            shellState: .bootstrapDefault(windowID: windowID),
            windowContext: context,
            persistenceURL: persistenceURL,
            terminalRuntimeRegistry: registry
        )
        let stalePane = ShellPane(
            paneID: "pane_2",
            tabID: "tab_stale",
            spaceID: "space_main",
            launchTarget: .shell,
            cwd: "/tmp",
            process: nil,
            attention: .idle,
            context: nil,
            viewport: nil,
            alanBinding: nil
        )
        _ = registry.surfaceHandle(for: stalePane, bootProfile: nil)

        expect(
            registry.registeredPaneIDs.contains("pane_2"),
            "test setup must register a runtime-only stale pane"
        )

        _ = controller.openTerminalTab()

        expect(
            controller.selectedPane?.paneID == "pane_3",
            "opening a tab must skip pane IDs still owned by the terminal runtime registry"
        )
        expect(
            !registry.registeredPaneIDs.contains("pane_2"),
            "opening a tab must release stale runtime-only panes after adopting the new state"
        )
    }

    private static func verifiesOpeningTerminalTabInheritsFocusedRuntimeCwd() {
        let controller = makeController()
        controller.updateTerminalMetadata(metadata(title: "cwd update", cwd: "/repo/app"), for: "pane_1")

        _ = controller.openTerminalTab()

        expect(
            controller.selectedPane?.cwd == "/repo/app",
            "new terminal tabs must inherit the focused pane runtime cwd"
        )
    }

    private static func verifiesShellActionNewTerminalTabInheritsFocusedRuntimeCwd() {
        let controller = makeController()
        controller.updateTerminalMetadata(metadata(title: "cwd update", cwd: "/repo/app"), for: "pane_1")

        let result = controller.performShellAction(.newTerminalTab)

        expect(result == .executed, "registry new-terminal action must execute")
        expect(
            controller.selectedPane?.cwd == "/repo/app",
            "registry new-terminal action must inherit the focused pane runtime cwd"
        )
    }

    private static func verifiesOpeningTerminalTabFallsBackToFocusedPaneSnapshotCwd() {
        let windowID = "metadata_snapshot_cwd_\(UUID().uuidString)"
        let controller = makeController(
            windowID: windowID,
            shellState: .bootstrapDefault(windowID: windowID, workingDirectory: "/snapshot/cwd")
        )

        _ = controller.openTerminalTab()

        expect(
            controller.selectedPane?.cwd == "/snapshot/cwd",
            "new terminal tabs must fall back to the focused pane snapshot cwd"
        )
    }

    private static func verifiesOpeningTerminalTabHonorsExplicitCwd() {
        let controller = makeController()
        controller.updateTerminalMetadata(metadata(title: "cwd update", cwd: "/repo/app"), for: "pane_1")

        _ = controller.openTerminalTab(workingDirectory: "/explicit/cwd")

        expect(
            controller.selectedPane?.cwd == "/explicit/cwd",
            "explicit new-tab cwd must override focused pane cwd"
        )
    }

    private static func verifiesQuickTerminalShowCreatesAndReusesGlobalPane() {
        let controller = makeController()
        controller.updateTerminalMetadata(metadata(title: "cwd update", cwd: "/repo/app"), for: "pane_1")
        let selectedSpaceBefore = controller.selectedSpaceID
        let selectedTabBefore = controller.selectedTabID
        let focusedPaneBefore = controller.shellState.focusedPaneID

        let shownPaneID = controller.showQuickTerminal()
        let hidden = controller.hideQuickTerminal()
        controller.updateTerminalMetadata(metadata(title: "cwd update", cwd: "/repo/other"), for: "pane_1")
        let reshownPaneID = controller.showQuickTerminal()

        expect(shownPaneID == "quick_terminal_pane", "quick terminal must use one stable global pane id")
        expect(hidden == true, "quick terminal hide must apply when the peak is visible")
        expect(reshownPaneID == shownPaneID, "quick terminal show must reuse the existing global pane")
        expect(
            controller.quickTerminalPane?.cwd == "/repo/app",
            "quick terminal must keep the existing instance cwd across hide/show"
        )
        expect(
            controller.quickTerminalPresentation == .visible,
            "reshowing quick terminal must make the global slot visible"
        )
        expect(
            controller.shellState.panes.filter(\.isQuickTerminalPane).count == 1,
            "quick terminal must not create one pane per summon"
        )
        expect(controller.selectedSpaceID == selectedSpaceBefore, "show/hide must not move the selected space")
        expect(controller.selectedTabID == selectedTabBefore, "show/hide must not move the selected tab")
        expect(
            controller.shellState.focusedPaneID == focusedPaneBefore,
            "show/hide must not steal regular pane focus in the shell model"
        )
    }

    private static func verifiesQuickTerminalActionsAndControlCommandsShareControllerPath() {
        let controller = makeController()

        let actionResult = controller.performShellAction(.quickTerminalToggle)
        let paneIDFromAction = controller.quickTerminalPane?.paneID
        let hiddenResponse = controller.handleControlPlaneCommand(
            decodeControlCommand(
                """
                {
                  "request_id": "quick-hide-1",
                  "command": "quick_terminal.hide"
                }
                """
            )
        )

        expect(actionResult == .executed, "quick terminal action must execute through ShellActionRegistry")
        expect(paneIDFromAction == "quick_terminal_pane", "quick terminal action must create the global pane")
        expect(hiddenResponse.applied == true, "quick terminal hide control command must use controller routing")
        expect(
            controller.quickTerminalPresentation == .hidden,
            "quick terminal hide command must preserve the global runtime slot"
        )

        let focusResponse = controller.handleControlPlaneCommand(
            decodeControlCommand(
                """
                {
                  "request_id": "quick-focus-1",
                  "command": "quick_terminal.focus"
                }
                """
            )
        )

        expect(focusResponse.applied == true, "quick terminal focus command must use controller routing")
        expect(focusResponse.paneID == paneIDFromAction, "focus response must identify the quick pane")

        let closeResponse = controller.handleControlPlaneCommand(
            decodeControlCommand(
                """
                {
                  "request_id": "quick-close-1",
                  "command": "quick_terminal.close"
                }
                """
            )
        )

        expect(closeResponse.applied == true, "quick terminal close command must use controller routing")
        expect(
            closeResponse.paneID == controller.shellState.focusedPaneID,
            "quick terminal close response must return the resulting focused pane"
        )
        expect(
            closeResponse.paneID != ShellQuickTerminalSlot.globalPaneID,
            "quick terminal close response must not return the removed quick pane"
        )
        expect(controller.quickTerminalPane == nil, "close must clear the global quick-terminal slot")
        expect(
            !controller.terminalRuntimeRegistry.registeredPaneIDs.contains("quick_terminal_pane"),
            "close must release the quick terminal runtime through regular registry cleanup"
        )
    }

    private static func verifiesQuickTerminalPromotionMovesExistingPaneIntoSpace() {
        let controller = makeController()
        _ = controller.createTerminalSpace(title: "Second", workingDirectory: "/tmp")
        let quickPaneID = controller.showQuickTerminal()
        let response = controller.handleControlPlaneCommand(
            decodeControlCommand(
                """
                {
                  "request_id": "quick-promote-1",
                  "command": "quick_terminal.promote",
                  "target_space_id": "space_2"
                }
                """
            )
        )
        let promotedPane = controller.pane(paneID: ShellQuickTerminalSlot.globalPaneID)
        let targetSpace = controller.shellState.space(spaceID: "space_2")
        let targetTab = targetSpace?.tabs.first { $0.contains(paneID: ShellQuickTerminalSlot.globalPaneID) }

        expect(quickPaneID == ShellQuickTerminalSlot.globalPaneID, "quick setup must create the global pane")
        expect(response.applied == true, "quick terminal promote command must use controller routing")
        expect(response.paneID == ShellQuickTerminalSlot.globalPaneID, "promote response must return the moved pane")
        expect(controller.quickTerminalPane == nil, "promote must clear the quick-terminal slot")
        expect(promotedPane?.spaceID == "space_2", "promote must move the existing pane into the target space")
        expect(promotedPane?.tabID == targetTab?.tabID, "promote must attach the existing pane to the new tab")
        expect(
            controller.shellState.panes.filter { $0.paneID == ShellQuickTerminalSlot.globalPaneID }.count == 1,
            "promote must move the pane instead of copying the process"
        )
    }

    private static func verifiesQuickTerminalPeakPresenterShowsDetachedTerminalWindow() {
        let controller = makeController()
        let window = FakeQuickTerminalPeakWindow()
        let presenter = ShellQuickTerminalPeakPresenter(
            host: controller,
            window: window,
            visibleFrameProvider: {
                CGRect(x: 80, y: 120, width: 1_440, height: 900)
            }
        )
        let selectedSpaceBefore = controller.selectedSpaceID
        let selectedTabBefore = controller.selectedTabID

        let paneID = controller.showQuickTerminal()
        presenter.synchronize()

        expect(paneID == ShellQuickTerminalSlot.globalPaneID, "peak presenter setup must show the global quick pane")
        expect(window.presentedPaneIDs == [ShellQuickTerminalSlot.globalPaneID], "peak presenter must present the quick pane")
        expect(window.lastTabID == ShellQuickTerminalSlot.globalTabID, "peak presenter must wrap the quick pane in the quick tab")
        expect(window.lastPlacement?.requiresMainWindow == false, "peak window must not depend on the main window")
        expect(window.lastPlacement?.followsActiveSpace == true, "peak window must follow the active macOS Space")
        expect(window.lastPlacement?.joinsAllSpaces == true, "peak window must be able to appear across macOS Spaces")
        expect(window.focusedPaneIDs == [ShellQuickTerminalSlot.globalPaneID], "peak presenter must focus terminal input after show")
        expect(controller.selectedSpaceID == selectedSpaceBefore, "peak presenter must not move the selected Alan space")
        expect(controller.selectedTabID == selectedTabBefore, "peak presenter must not move the selected Alan tab")
    }

    private static func verifiesQuickTerminalPeakPresenterPreservesRuntimeOnExplicitHide() {
        let controller = makeController()
        let window = FakeQuickTerminalPeakWindow()
        let presenter = ShellQuickTerminalPeakPresenter(host: controller, window: window)

        _ = controller.showQuickTerminal()
        presenter.synchronize()
        presenter.windowDidResignKey()

        expect(
            controller.quickTerminalPresentation == .visible,
            "peak focus loss must not hide the quick terminal"
        )
        expect(window.dismissalReasons.isEmpty, "peak focus loss must not dismiss the window")

        expect(controller.hideQuickTerminal(), "explicit quick-terminal hide must apply")
        presenter.synchronize()

        expect(
            window.dismissalReasons.last == .hidden,
            "explicit hide must hide the peak without removing the runtime slot"
        )
        expect(controller.quickTerminalPane != nil, "explicit hide must preserve the quick-terminal pane")

        expect(controller.closeQuickTerminal(), "explicit quick-terminal close must apply")
        presenter.synchronize()

        expect(
            window.dismissalReasons.last == .removed,
            "explicit close must release the peak presentation"
        )
        expect(controller.quickTerminalPane == nil, "explicit close must remove the quick-terminal slot")
    }

    private static func verifiesQuickTerminalPeakPresenterDoesNotRefocusOnVisibleRefresh() {
        let controller = makeController()
        let window = FakeQuickTerminalPeakWindow()
        let presenter = ShellQuickTerminalPeakPresenter(host: controller, window: window)

        _ = controller.showQuickTerminal()
        presenter.synchronize()
        controller.updateTerminalMetadata(metadata(title: "regular pane update", cwd: "/repo/app"), for: "pane_1")
        presenter.synchronize()

        expect(
            window.presentedPaneIDs == [ShellQuickTerminalSlot.globalPaneID],
            "visible state refresh must not bring the Peak window forward again"
        )
        expect(
            window.focusedPaneIDs == [ShellQuickTerminalSlot.globalPaneID],
            "visible state refresh must not repeatedly focus terminal input"
        )
    }

    private static func verifiesQuickTerminalPeakPlacementFitsActiveDisplay() {
        let visibleFrame = CGRect(x: 20, y: 40, width: 1_280, height: 760)
        let placement = ShellQuickTerminalPeakPlacement.defaultPlacement(in: visibleFrame)

        expect(visibleFrame.contains(placement.frame), "peak frame must fit inside the active display")
        expect(placement.frame.width >= 720, "normal displays should get a usable terminal width")
        expect(placement.frame.height >= 320, "normal displays should get a usable terminal height")
        expect(placement.requiresMainWindow == false, "peak placement must be detached from the main window")
    }

    private static func verifiesQuickTerminalPeakEscapePolicyBelongsToTerminal() {
        let policy = ShellQuickTerminalPeakInteractionPolicy.terminalFirst

        expect(policy.escapeKeyBehavior == .terminalInput, "Esc must remain terminal input by default")
        expect(policy.hidesOnFocusLoss == false, "focus loss must not auto-hide the peak")
        expect(policy.usesMainWindowParenting == false, "peak must not be parented to the main window")
    }

    private static func verifiesSplitZoomLeavesCanonicalTreeAndKeepsSiblingRuntimes() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        guard let tab = controller.selectedTab else {
            fail("test setup must keep a selected tab")
        }
        let canonicalTree = tab.paneTree
        _ = controller.terminalRuntimeRegistry.surfaceHandle(
            for: controller.pane(paneID: "pane_1"),
            bootProfile: controller.bootProfile(for: controller.pane(paneID: "pane_1"))
        )
        _ = controller.terminalRuntimeRegistry.surfaceHandle(
            for: controller.pane(paneID: "pane_2"),
            bootProfile: controller.bootProfile(for: controller.pane(paneID: "pane_2"))
        )

        expect(controller.zoomPane(paneID: "pane_1"), "zoom must accept a split pane")
        expect(controller.selectedTabZoomedPaneID == "pane_1", "zoom state must be tab scoped")
        expect(
            controller.displayPaneTree(for: controller.selectedTab)?.paneIDs == ["pane_1"],
            "zoomed display tree must project only the zoomed pane"
        )
        expect(
            controller.selectedTab?.paneTree == canonicalTree,
            "zoom must leave the canonical split tree unchanged"
        )
        expect(
            controller.terminalRuntimeRegistry.registeredPaneIDs.isSuperset(of: ["pane_1", "pane_2"]),
            "zoom must not release sibling terminal runtimes"
        )

        expect(controller.unzoomSelectedTab(), "unzoom must clear the selected tab zoom state")
        expect(
            controller.displayPaneTree(for: controller.selectedTab)?.paneIDs == canonicalTree.paneIDs,
            "unzoom must restore the displayed split tree"
        )
    }

    private static func verifiesSplitZoomIsTabScopedAndPrunedWhenPaneDisappears() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        let firstTabID = controller.selectedTabID
        expect(controller.zoomPane(paneID: "pane_1"), "test setup must zoom the first split tab")
        let secondTabID = controller.openTerminalTab(in: controller.selectedSpaceID)
        expect(secondTabID != nil, "test setup must open a second tab")

        expect(controller.selectedTabID == secondTabID, "opening a tab must select it")
        expect(controller.selectedTabZoomedPaneID == nil, "zoom state must not leak to another tab")
        if let firstTabID {
            controller.select(tabID: firstTabID)
        }
        expect(controller.selectedTabZoomedPaneID == "pane_1", "zoom state must remain attached to its tab")

        _ = controller.closePane(paneID: "pane_1")
        expect(controller.selectedTabZoomedPaneID == nil, "closing the zoomed pane must prune zoom state")
    }

    private static func verifiesInTabPaneMovementPreservesRuntimeContinuity() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        let movedPaneBefore = controller.pane(paneID: "pane_2")
        _ = controller.terminalRuntimeRegistry.surfaceHandle(
            for: controller.pane(paneID: "pane_1"),
            bootProfile: controller.bootProfile(for: controller.pane(paneID: "pane_1"))
        )
        _ = controller.terminalRuntimeRegistry.surfaceHandle(
            for: controller.pane(paneID: "pane_2"),
            bootProfile: controller.bootProfile(for: controller.pane(paneID: "pane_2"))
        )
        let registeredBefore = controller.terminalRuntimeRegistry.registeredPaneIDs

        expect(
            controller.movePaneWithinTab(paneID: "pane_2", placement: .left),
            "in-tab movement must accept an adjacent destination"
        )
        expect(
            controller.selectedTab?.paneTree.paneIDs == ["pane_2", "pane_1"],
            "in-tab movement must update PaneSlot placement inside the selected tab"
        )
        expect(
            controller.pane(paneID: "pane_2") == movedPaneBefore,
            "in-tab movement must preserve mounted terminal content metadata"
        )
        expect(
            controller.terminalRuntimeRegistry.registeredPaneIDs == registeredBefore,
            "in-tab movement must not release or recreate terminal runtimes"
        )
    }

    private static func verifiesPaneMovementDragPolicyProtectsTerminalSelection() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        let originalTree = controller.selectedTab?.paneTree

        expect(
            !controller.movePaneWithinTab(
                paneID: "pane_2",
                placement: .left,
                source: .terminalContentDrag
            ),
            "terminal content drags must not start pane movement"
        )
        expect(
            controller.selectedTab?.paneTree == originalTree,
            "rejected terminal-content drag movement must leave layout unchanged"
        )
        expect(
            controller.movePaneWithinTab(
                paneID: "pane_2",
                placement: .left,
                source: .titleBarDragAffordance
            ),
            "drag-backed movement must route through the same controller mutation path"
        )
        expect(
            controller.selectedTab?.paneTree.paneIDs == ["pane_2", "pane_1"],
            "drag-backed movement must preserve the explicit movement result semantics"
        )
    }

    private static func verifiesTerminalActivityProjectsByPaneID() {
        let controller = makeController()
        _ = controller.openTerminalTab(workingDirectory: "/background")
        let activity = progressActivity(
            percent: 42,
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:15Z"
        )

        controller.updateTerminalMetadata(
            metadata(title: "build", cwd: "/repo/app", activity: activity),
            for: "pane_1"
        )

        let activePane = controller.pane(paneID: "pane_1")
        let backgroundPane = controller.pane(paneID: "pane_2")
        expect(
            activePane?.activity == activity,
            "terminal activity metadata must project onto the owning pane"
        )
        expect(
            backgroundPane?.activity == nil,
            "terminal activity metadata must not leak to other panes"
        )
    }

    private static func verifiesTerminalActivitySidebarPriority() {
        let running = activity(status: .running, source: .shell, sourceLabel: "Shell", stateLabel: "Running")
        let progress = activity(
            status: .progress,
            source: .progress,
            sourceLabel: "Progress",
            stateLabel: "42%",
            progress: .percent(42)
        )
        let needsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed"
        )

        expect(
            TerminalActivitySnapshot.primarySidebarActivity([running, progress]) == progress,
            "progress must outrank generic running activity"
        )
        expect(
            TerminalActivitySnapshot.primarySidebarActivity([progress, needsInput]) == needsInput,
            "user-input-required activity must outrank progress"
        )
    }

    private static func verifiesProgressActivityFactoryUsesSourceFirstDisplay() {
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let activity = TerminalActivitySnapshot.progressActivity(percent: 42, now: now)

        expect(activity.source.kind == .progress, "progress factory must label source as progress")
        expect(activity.status == .progress, "determinate progress must use progress status")
        expect(activity.progress == .percent(42), "determinate progress must carry bounded percent")
        expect(
            activity.display.sourceFirstLabel == "Progress · 42%",
            "progress display copy must be source-first"
        )
        expect(
            activity.freshness.staleAt == "2026-05-17T09:00:15Z",
            "progress activity must get the default 15 second stale deadline"
        )
    }

    private static func verifiesCodexAgentActivityAdapterMapsSupportedStates() {
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let updatedAt = "2026-05-17T09:00:00Z"
        let running = TerminalAgentActivityAdapter.activity(
            from: TerminalAgentActivityEvent(
                agentKind: "codex",
                status: "running",
                sessionLabel: nil,
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan",
                detail: nil,
                updatedAt: updatedAt
            ),
            now: now
        )
        let needsInput = TerminalAgentActivityAdapter.activity(
            from: TerminalAgentActivityEvent(
                agentKind: "codex",
                status: "approval_required",
                sessionLabel: nil,
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan",
                detail: nil,
                updatedAt: updatedAt
            ),
            now: now
        )
        let complete = TerminalAgentActivityAdapter.activity(
            from: TerminalAgentActivityEvent(
                agentKind: "codex",
                status: "completed",
                sessionLabel: nil,
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan",
                detail: nil,
                updatedAt: updatedAt
            ),
            now: now
        )
        let failed = TerminalAgentActivityAdapter.activity(
            from: TerminalAgentActivityEvent(
                agentKind: "codex",
                status: "error",
                sessionLabel: nil,
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan",
                detail: nil,
                updatedAt: updatedAt
            ),
            now: now
        )

        expect(running?.source.kind == .codex, "Codex running activity must keep codex as source")
        expect(running?.status == .running, "Codex running event must map to running")
        expect(running?.display.sourceFirstLabel == "Codex · Running", "Codex running copy must be source-first")
        expect(running?.freshness.staleAt == "2026-05-17T09:01:30Z", "running agent activity must have a bounded stale window")
        expect(needsInput?.status == .needsInput, "Codex approval-required event must map to needs-input")
        expect(needsInput?.priority == .awaitingUser, "Codex needs-input event must be awaiting-user priority")
        expect(needsInput?.freshness.staleAt == nil, "Codex needs-input must persist until replaced")
        expect(complete?.status == .done, "Codex completed event must map to done")
        expect(complete?.freshness.expiresAt == "2026-05-17T09:00:08Z", "Codex done activity must be brief")
        expect(failed?.status == .failed, "Codex error event must map to failed")
        expect(failed?.display.sourceFirstLabel == "Codex · Error", "Codex error copy must hide implementation names")
        expect(failed?.freshness.staleAt == nil, "Codex error must persist until replaced")
    }

    private static func verifiesAgentActivityAdapterSanitizesDefaultUIPayload() {
        let activity = TerminalAgentActivityAdapter.activity(
            from: TerminalAgentActivityEvent(
                agentKind: "codex",
                status: "needs_input",
                sessionLabel: "session-1234567890abcdef1234567890",
                projectLabel: "alan\nworkspace",
                workingDirectory: "/Users/morris/Developer/alan",
                detail: #"{"event":"codex.status","session_id":"session-1234567890abcdef1234567890"}"#,
                updatedAt: "2026-05-17T09:00:00Z"
            ),
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )

        expect(activity?.agent?.safeSessionLabel == nil, "agent adapter must not expose raw session ids")
        expect(activity?.agent?.projectLabel == "alan workspace", "agent adapter must collapse control characters in labels")
        expect(activity?.display.detailLabel == nil, "agent adapter must not expose raw hook payloads in default UI detail")
        if let activity {
            do {
                let data = try JSONEncoder().encode(activity)
                let json = String(data: data, encoding: .utf8) ?? ""
                expect(!json.contains("codex.status"), "serialized activity must not retain raw hook event names")
                expect(!json.contains("1234567890abcdef"), "serialized activity must not retain raw session ids")
            } catch {
                fail("agent activity JSON encode failed: \(error)")
            }
        } else {
            fail("valid Codex needs-input activity should adapt")
        }
    }

    private static func verifiesAgentActivityAdapterRejectsMalformedPayloadAndFallsBackForUnsupportedAgent() {
        let malformed = TerminalAgentActivityAdapter.activity(
            from: TerminalAgentActivityEvent(
                agentKind: "codex",
                status: "tool_call_delta",
                sessionLabel: nil,
                projectLabel: nil,
                workingDirectory: nil,
                detail: nil,
                updatedAt: "2026-05-17T09:00:00Z"
            ),
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )
        let unsupported = TerminalAgentActivityAdapter.activity(
            from: TerminalAgentActivityEvent(
                agentKind: "future-agent",
                status: "running",
                sessionLabel: nil,
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan",
                detail: nil,
                updatedAt: "2026-05-17T09:00:00Z"
            ),
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )

        expect(malformed == nil, "unknown implementation event names must not create precise activity")
        expect(unsupported?.source.kind == .unknown, "unsupported agents must fall back to unknown source")
        expect(unsupported?.display.sourceFirstLabel == "Agent · Running", "unsupported agents must use generic UI copy")
    }

    private static func verifiesAgentActivityControlCommandProjectsOntoPane() {
        let controller = makeController(appIsActive: false)
        let json = """
        {
          "request_id": "agent-activity-1",
          "command": "agent.activity",
          "pane_id": "pane_1",
          "agent_kind": "codex",
          "agent_status": "needs_input",
          "session_label": "session-1234567890abcdef1234567890",
          "project_label": "alan",
          "working_directory": "/Users/morris/Developer/alan",
          "detail": "{\\"event\\":\\"codex.status\\"}",
          "updated_at": "2026-05-17T09:00:00Z"
        }
        """
        let command = decodeControlCommand(json)
        let response = controller.handleControlPlaneCommand(command)
        let paneActivity = controller.pane(paneID: "pane_1")?.activity

        expect(response.applied == true, "agent activity command must be applied")
        expect(response.paneID == "pane_1", "agent activity command must identify the updated pane")
        expect(paneActivity?.source.kind == .codex, "agent activity command must project Codex source onto pane")
        expect(paneActivity?.status == .needsInput, "agent activity command must project needs-input status")
        expect(paneActivity?.agent?.safeSessionLabel == nil, "control command projection must not expose raw session ids")
        expect(controller.activityNotifications.first?.kind == .needsInput, "agent command must reuse low-noise notification routing")
    }

    private static func verifiesCommandCompletionActivityFactory() {
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let success = TerminalActivitySnapshot.commandCompletion(exitCode: 0, now: now)
        let failure = TerminalActivitySnapshot.commandCompletion(exitCode: 2, now: now)
        let longSuccess = TerminalActivitySnapshot.commandCompletion(
            exitCode: 0,
            now: now,
            durationMilliseconds: 120_000
        )

        expect(success.status == .done, "zero exit code must produce done status")
        expect(!success.isSidebarWorthy, "successful commands must not be sidebar-worthy")
        expect(
            success.freshness.staleAt == "2026-05-17T09:00:08Z",
            "successful command completion must get a short stale deadline"
        )
        expect(
            !success.isFresh(at: now.addingTimeInterval(9)),
            "successful command completion must become stale after its freshness window"
        )
        expect(
            longSuccess.command?.durationMilliseconds == 120_000,
            "command completion must preserve measured duration"
        )
        expect(failure.status == .failed, "non-zero exit code must produce failed status")
        expect(failure.command?.exitCode == 2, "command completion must preserve exit code")
        expect(
            failure.display.sourceFirstLabel == "Shell · Command failed 2",
            "failed command copy must be source-first"
        )
        expect(
            TerminalActivitySnapshot.primarySidebarActivity([success, failure], now: now) == failure,
            "failed command completion must outrank successful completion"
        )
    }

    private static func verifiesTerminalActivityCodableUsesSnakeCase() {
        let activity = TerminalActivitySnapshot(
            source: TerminalActivitySource(kind: .codex, label: "Codex"),
            status: .failed,
            priority: .notable,
            progress: nil,
            command: TerminalActivityCommandOutcome(
                exitCode: 2,
                durationMilliseconds: 120_000,
                commandText: "just check"
            ),
            agent: TerminalActivityAgentMetadata(
                kind: .codex,
                safeSessionLabel: "Codex",
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            ),
            display: TerminalActivityDisplay(
                sourceLabel: "Codex",
                stateLabel: "Failed",
                detailLabel: "just check",
                paneHint: "1"
            ),
            freshness: TerminalActivityFreshness(
                updatedAt: "2026-05-17T09:00:00Z",
                staleAt: "2026-05-17T09:00:30Z",
                expiresAt: "2026-05-17T09:05:00Z"
            )
        )

        do {
            let data = try JSONEncoder().encode(activity)
            guard
                let root = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                let command = root["command"] as? [String: Any],
                let agent = root["agent"] as? [String: Any],
                let display = root["display"] as? [String: Any],
                let freshness = root["freshness"] as? [String: Any]
            else {
                fail("activity JSON must encode nested objects")
            }

            expect(command["exit_code"] as? Int == 2, "command JSON must use exit_code")
            expect(
                command["duration_milliseconds"] as? Int == 120_000,
                "command JSON must use duration_milliseconds"
            )
            expect(command["command_text"] as? String == "just check", "command JSON must use command_text")
            expect(!command.keys.contains("exitCode"), "command JSON must not use camelCase exitCode")
            expect(
                agent["safe_session_label"] as? String == "Codex",
                "agent JSON must use safe_session_label"
            )
            expect(agent["project_label"] as? String == "alan", "agent JSON must use project_label")
            expect(
                agent["working_directory"] as? String == "/Users/morris/Developer/alan",
                "agent JSON must use working_directory"
            )
            expect(display["source_label"] as? String == "Codex", "display JSON must use source_label")
            expect(display["state_label"] as? String == "Failed", "display JSON must use state_label")
            expect(display["detail_label"] as? String == "just check", "display JSON must use detail_label")
            expect(display["pane_hint"] as? String == "1", "display JSON must use pane_hint")
            expect(
                freshness["updated_at"] as? String == "2026-05-17T09:00:00Z",
                "freshness JSON must use updated_at"
            )
            expect(
                freshness["stale_at"] as? String == "2026-05-17T09:00:30Z",
                "freshness JSON must use stale_at"
            )
            expect(
                freshness["expires_at"] as? String == "2026-05-17T09:05:00Z",
                "freshness JSON must use expires_at"
            )

            let decoded = try JSONDecoder().decode(TerminalActivitySnapshot.self, from: data)
            expect(decoded == activity, "activity JSON must round-trip through snake_case keys")
        } catch {
            fail("activity JSON contract failed: \(error)")
        }
    }

    private static func verifiesSuccessfulCommandIsNotSidebarWorthy() {
        let success = activity(
            status: .done,
            source: .command,
            sourceLabel: "Shell",
            stateLabel: "Command succeeded"
        )

        expect(
            TerminalActivitySnapshot.primarySidebarActivity([success]) == nil,
            "successful command completion must not become sidebar-worthy activity"
        )
    }

    private static func verifiesStaleProgressIsNotSidebarWorthy() {
        let progress = progressActivity(
            percent: 42,
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:15Z"
        )
        let now = Date(timeIntervalSince1970: 1_779_008_416)

        expect(
            !progress.isFresh(at: now),
            "progress must become stale after its freshness deadline"
        )
        expect(
            TerminalActivitySnapshot.primarySidebarActivity([progress], now: now) == nil,
            "stale progress must not remain sidebar-worthy"
        )
    }

    private static func verifiesDefaultSidebarActivitySelectionHonorsFreshness() {
        let staleProgress = progressActivity(
            percent: 42,
            updatedAt: "2000-01-01T00:00:00Z",
            staleAt: "2000-01-01T00:00:15Z"
        )

        expect(
            TerminalActivitySnapshot.primarySidebarActivity([staleProgress]) == nil,
            "default sidebar activity selection must reject stale activity"
        )
    }

    private static func verifiesClearingActivityRemovesPaneActivity() {
        let controller = makeController()
        let progress = progressActivity(
            percent: 64,
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:15Z"
        )

        controller.updateTerminalMetadata(
            metadata(title: "build", cwd: "/repo/app", activity: progress),
            for: "pane_1"
        )
        expect(
            controller.shellState.pane(paneID: "pane_1")?.activity == progress,
            "test setup must project progress activity"
        )

        controller.updateTerminalMetadata(
            metadata(title: "build", cwd: "/repo/app", clearsActivity: true),
            for: "pane_1"
        )
        expect(
            controller.shellState.pane(paneID: "pane_1")?.activity == nil,
            "clear metadata must remove stale pane activity"
        )
    }

    private static func verifiesPublishedStateMergeClearsActivity() {
        let progress = progressActivity(
            percent: 42,
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:15Z"
        )
        let authoritative = stateWithAlanBinding(
            windowID: "window_activity_merge",
            pendingYield: false,
            activity: progress
        )
        let incoming = stateWithAlanBinding(
            windowID: "window_activity_merge",
            pendingYield: false,
            activity: nil
        )

        let merged = AlanShellPublishedStateMerger.merge(
            authoritative: authoritative,
            incoming: incoming
        )

        expect(
            merged.pane(paneID: "pane_1")?.activity == nil,
            "published state merge must allow incoming nil activity to clear stale activity"
        )
    }

    private static func verifiesPaneRebuildMutationsPreserveActivity() {
        let controller = makeController()
        let progress = progressActivity(
            percent: 42,
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:15Z"
        )

        controller.updateTerminalMetadata(
            metadata(title: "build", cwd: "/repo/app", activity: progress),
            for: "pane_1"
        )
        expect(
            controller.pane(paneID: "pane_1")?.activity == progress,
            "test setup must project progress activity before pane rebuilds"
        )

        _ = controller.setAttention(.notable, for: "pane_1")
        expect(
            controller.pane(paneID: "pane_1")?.activity == progress,
            "attention-only pane rebuild must preserve terminal activity"
        )

        guard let targetTabID = controller.openTerminalTab(workingDirectory: "/target") else {
            fail("test setup must create a target tab")
        }
        expect(
            controller.movePane(paneID: "pane_1", toTab: targetTabID, direction: .vertical),
            "test setup must move pane into target tab"
        )
        expect(
            controller.pane(paneID: "pane_1")?.activity == progress,
            "pane move rebuild must preserve terminal activity"
        )

        switch controller.liftPaneToTab(paneID: "pane_1") {
        case .lifted:
            break
        case .paneNotFound, .lastPane:
            fail("test setup must lift moved pane into a new tab")
        }
        expect(
            controller.pane(paneID: "pane_1")?.activity == progress,
            "pane lift rebuild must preserve terminal activity"
        )
    }

    private static func verifiesTabSidebarActivityProjectionUsesHighestPriorityPane() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let progress = progressActivity(
            percent: 42,
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:15Z"
        )
        let needsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed",
            updatedAt: "2026-05-17T09:00:01Z"
        )

        controller.updateTerminalMetadata(
            metadata(title: "build", cwd: "/repo/app", activity: progress),
            for: "pane_1"
        )
        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", activity: needsInput),
            for: "pane_2"
        )

        guard let tab = controller.shellState.tab(tabID: "tab_main") else {
            fail("test setup must keep the split tab")
        }

        let projection = shellSidebarTabProjection(
            for: tab,
            panes: controller.shellState.panes,
            focusedPaneID: "pane_1",
            focusedTabID: "tab_other",
            now: now
        )

        expect(projection.activity?.status == .needsInput, "tab activity must pick the most actionable pane")
        expect(
            projection.secondaryLine == "Pane 2 · Codex · Input needed",
            "background pane activity must include a short pane hint"
        )
        expect(projection.progress == nil, "non-progress displayed activity must not inherit another pane progress")
    }

    private static func verifiesTabSidebarProjectionFallsBackToRepositoryBranch() {
        let tab = ShellTab(
            tabID: "tab_1",
            kind: .terminal,
            title: nil,
            paneTree: ShellPaneTreeNode(
                nodeID: "node_pane_1",
                kind: .pane,
                direction: nil,
                paneID: "pane_1",
                children: nil
            )
        )
        let testPane = pane(
            context: context(
                workingDirectoryName: "src",
                repositoryRoot: "/Users/morris/Developer/alan",
                gitBranch: "main",
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            cwd: "/Users/morris/Developer/alan/crates/runtime",
            attention: .idle
        )

        let projection = shellSidebarTabProjection(
            for: tab,
            panes: [testPane],
            focusedPaneID: "pane_1",
            focusedTabID: "tab_1",
            now: nil
        )

        expect(projection.activity == nil, "idle panes must not produce tab activity")
        expect(
            projection.secondaryLine == "alan · main",
            "sidebar context fallback must prefer repository/worktree leaf plus branch"
        )
    }

    private static func verifiesTabSidebarProjectionPreservesTerminalStatusBeforeContext() {
        let tab = ShellTab(
            tabID: "tab_1",
            kind: .terminal,
            title: nil,
            paneTree: ShellPaneTreeNode(
                nodeID: "node_pane_1",
                kind: .pane,
                direction: nil,
                paneID: "pane_1",
                children: nil
            )
        )
        let failedRenderer = pane(
            context: context(
                workingDirectoryName: "src",
                repositoryRoot: "/Users/morris/Developer/alan",
                gitBranch: "main",
                processState: "running",
                rendererHealth: "failed",
                surfaceReadiness: "renderer_failed",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            cwd: "/Users/morris/Developer/alan/crates/runtime",
            attention: .notable
        )
        let startingPane = pane(
            context: context(
                workingDirectoryName: "src",
                repositoryRoot: "/Users/morris/Developer/alan",
                gitBranch: "main",
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "input_not_ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            cwd: "/Users/morris/Developer/alan/crates/runtime",
            attention: .idle
        )

        let failedProjection = shellSidebarTabProjection(
            for: tab,
            panes: [failedRenderer],
            focusedPaneID: "pane_1",
            focusedTabID: "tab_1",
            now: nil
        )
        let startingProjection = shellSidebarTabProjection(
            for: tab,
            panes: [startingPane],
            focusedPaneID: "pane_1",
            focusedTabID: "tab_1",
            now: nil
        )

        expect(
            failedProjection.secondaryLine == "Renderer failed",
            "sidebar fallback must preserve renderer failures before repository context"
        )
        expect(
            startingProjection.secondaryLine == "Starting",
            "sidebar fallback must preserve startup/input readiness before repository context"
        )
    }

    private static func verifiesTabSidebarProjectionDoesNotResurrectStaleCommandFailure() {
        let tab = ShellTab(
            tabID: "tab_1",
            kind: .terminal,
            title: nil,
            paneTree: ShellPaneTreeNode(
                nodeID: "node_pane_1",
                kind: .pane,
                direction: nil,
                paneID: "pane_1",
                children: nil
            )
        )
        let staleFailure = activity(
            status: .failed,
            source: .command,
            sourceLabel: "Shell",
            stateLabel: "Command failed 2",
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:30Z"
        )
        let testPane = pane(
            context: context(
                workingDirectoryName: "src",
                repositoryRoot: "/Users/morris/Developer/alan",
                gitBranch: "main",
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: 2
            ),
            viewport: ShellViewportSnapshot(
                title: "fish",
                summary: "command failed (2)",
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            cwd: "/Users/morris/Developer/alan/crates/runtime",
            attention: .notable,
            activity: staleFailure
        )

        let projection = shellSidebarTabProjection(
            for: tab,
            panes: [testPane],
            focusedPaneID: "pane_1",
            focusedTabID: "tab_1",
            now: Date(timeIntervalSince1970: 1_779_008_431)
        )

        expect(projection.activity == nil, "stale command failure must not remain sidebar activity")
        expect(
            projection.secondaryLine == "alan · main",
            "stale command failure summary must not hide repository context"
        )
    }

    private static func verifiesSidebarProgressRailBelongsToDisplayedActivity() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let progress = progressActivity(
            percent: 64,
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:15Z"
        )
        let failed = activity(
            status: .failed,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Error",
            updatedAt: "2026-05-17T09:00:01Z"
        )

        controller.updateTerminalMetadata(
            metadata(title: "build", cwd: "/repo/app", activity: progress),
            for: "pane_1"
        )
        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", activity: failed),
            for: "pane_2"
        )

        guard let tab = controller.shellState.tab(tabID: "tab_main") else {
            fail("test setup must keep the split tab")
        }

        let failedProjection = shellSidebarTabProjection(
            for: tab,
            panes: controller.shellState.panes,
            focusedPaneID: "pane_1",
            focusedTabID: "tab_other",
            now: now
        )
        expect(failedProjection.secondaryLine == "Pane 2 · Codex · Error", "failed activity must outrank progress")
        expect(failedProjection.progress == nil, "progress rail must not be shown for a different pane's progress")

        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", clearsActivity: true),
            for: "pane_2"
        )
        let progressProjection = shellSidebarTabProjection(
            for: tab,
            panes: controller.shellState.panes,
            focusedPaneID: "pane_1",
            focusedTabID: "tab_other",
            now: now
        )
        expect(progressProjection.secondaryLine == "Progress · 64%", "progress activity must become visible after failure clears")
        expect(progressProjection.progress == .percent(64), "progress rail must use the displayed activity progress")
    }

    private static func verifiesFocusedCommandFailureDemotesFromSidebarProjection() {
        let controller = makeController()
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let failure = TerminalActivitySnapshot.commandCompletion(exitCode: 2, now: now)
        controller.updateTerminalMetadata(
            metadata(title: "fish", cwd: "/Users/morris/Developer/alan", activity: failure),
            for: "pane_1"
        )

        guard let tab = controller.shellState.tab(tabID: "tab_main") else {
            fail("test setup must keep the tab")
        }

        let focusedProjection = shellSidebarTabProjection(
            for: tab,
            panes: controller.shellState.panes,
            focusedPaneID: "pane_1",
            focusedTabID: "tab_main",
            now: now
        )
        expect(focusedProjection.activity == nil, "focused command failure may demote from sidebar activity")

        let backgroundProjection = shellSidebarTabProjection(
            for: tab,
            panes: controller.shellState.panes,
            focusedPaneID: "pane_1",
            focusedTabID: "tab_other",
            now: now
        )
        expect(
            backgroundProjection.secondaryLine == "Shell · Command failed 2",
            "background command failure must remain sidebar-worthy"
        )
    }

    private static func verifiesCommandFailureAcknowledgementSticksAfterFocus() {
        let controller = makeController()
        _ = controller.openTerminalTab()
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let failure = TerminalActivitySnapshot.commandCompletion(exitCode: 2, now: now)

        controller.updateTerminalMetadata(
            metadata(title: "fish", cwd: "/Users/morris/Developer/alan", activity: failure),
            for: "pane_1"
        )

        guard let backgroundTab = controller.shellState.tab(tabID: "tab_main") else {
            fail("test setup must keep the background tab")
        }
        let unacknowledgedProjection = shellSidebarTabProjection(
            for: backgroundTab,
            panes: controller.shellState.panes,
            focusedPaneID: "pane_2",
            focusedTabID: "tab_2",
            now: now
        )
        expect(
            unacknowledgedProjection.secondaryLine == "Shell · Command failed 2",
            "background command failure must remain visible before focus acknowledgement"
        )

        controller.focus(paneID: "pane_1")
        expect(
            controller.pane(paneID: "pane_1")?.activity == nil,
            "focusing the command-failure tab must acknowledge and clear the retained activity"
        )
        expect(
            controller.pane(paneID: "pane_1")?.attention != .notable,
            "acknowledged focused command failure must stop keeping the pane notable"
        )

        controller.focus(paneID: "pane_2")
        guard let acknowledgedTab = controller.shellState.tab(tabID: "tab_main") else {
            fail("test setup must keep the acknowledged tab")
        }
        let acknowledgedProjection = shellSidebarTabProjection(
            for: acknowledgedTab,
            panes: controller.shellState.panes,
            focusedPaneID: "pane_2",
            focusedTabID: "tab_2",
            now: now.addingTimeInterval(1)
        )
        expect(
            acknowledgedProjection.activity == nil,
            "acknowledged command failure must not become sidebar-worthy again after switching away"
        )
        expect(
            acknowledgedProjection.secondaryLine != "Shell · Command failed 2",
            "acknowledged command failure must fall back to tab context instead of resurfacing"
        )
    }

    private static func verifiesActivityFreshnessPolicies() {
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let bell = TerminalActivitySnapshot.bellActivity(now: now)
        let exited = TerminalActivitySnapshot.processExitedActivity(exitCode: 2, now: now)
        let needsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed"
        )

        expect(
            bell.isFresh(at: now.addingTimeInterval(7)),
            "bell activity must remain briefly visible"
        )
        expect(
            !bell.isFresh(at: now.addingTimeInterval(9)),
            "bell activity must expire after the brief visibility window"
        )
        expect(
            exited.isFresh(at: now.addingTimeInterval(3_600)),
            "process-exited activity must persist until the pane is closed or replaced"
        )
        expect(
            needsInput.isFresh(at: now.addingTimeInterval(3_600)),
            "needs-input activity must persist until replaced"
        )
    }

    private static func verifiesActivityAttentionIsReadTimeOnly() {
        let projection = ShellPaneProjectionService()
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let failure = activity(
            status: .failed,
            source: .command,
            sourceLabel: "Shell",
            stateLabel: "Command failed 2",
            updatedAt: "2026-05-17T09:00:00Z",
            staleAt: "2026-05-17T09:00:30Z"
        )

        let persistedAttention = projection.projectedAttention(
            metadataAttention: .idle,
            processExited: false,
            binding: nil
        )
        let freshPane = pane(
            context: context(
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            attention: persistedAttention,
            activity: failure
        )
        let legacyStalePane = pane(
            context: context(
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            attention: .notable,
            activity: failure
        )
        let rendererFailedPane = pane(
            context: context(
                processState: "running",
                rendererHealth: "failed",
                surfaceReadiness: "renderer_failed",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            attention: .notable,
            activity: failure
        )

        expect(persistedAttention == .idle, "activity attention must not be persisted into pane attention")
        expect(
            shellEffectiveAttention(for: freshPane, now: now.addingTimeInterval(10)) == .notable,
            "fresh failed activity may overlay pane attention at read time"
        )
        expect(
            shellEffectiveAttention(for: freshPane, now: now.addingTimeInterval(31)) == .idle,
            "stale failed activity must not overlay pane attention"
        )
        expect(
            shellEffectiveAttention(for: legacyStalePane, now: now.addingTimeInterval(31)) == .idle,
            "legacy stale activity-derived pane attention must be ignored at read time"
        )
        expect(
            shellEffectiveAttention(for: rendererFailedPane, now: now.addingTimeInterval(31)) == .notable,
            "stale activity must not suppress persistent renderer attention"
        )
    }

    private static func verifiesPaneTitleActivityAccessoryLabel() {
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let paneWithProgress = pane(
            context: context(
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            attention: .idle,
            activity: TerminalActivitySnapshot.progressActivity(
                percent: 42,
                now: now
            )
        )

        expect(
            shellPaneActivityAccessoryLabel(for: paneWithProgress, now: now) == "Progress · 42%",
            "pane title activity accessory must expose source-first activity copy"
        )
        expect(
            shellPaneActivityAccessoryLabel(for: paneWithProgress, now: now.addingTimeInterval(16)) == nil,
            "pane title activity accessory must hide stale progress"
        )
    }

    private static func verifiesPaneTitleDetailProjectionIncludesContextBranchAndProcess() {
        let testPane = pane(
            context: context(
                workingDirectoryName: "alan",
                repositoryRoot: "/Users/morris/Developer/alan",
                gitBranch: "main",
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            cwd: "/Users/morris/Developer/alan",
            process: ShellProcessBinding(program: "fish", argvPreview: nil),
            attention: .idle
        )
        let details = shellPaneTitleBarDetailProjection(
            for: testPane,
            title: "Terminal",
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )

        expect(
            details.map(\.id) == ["worktree", "branch", "process"],
            "pane title details must expose non-redundant worktree, branch, and process"
        )
        expect(details.map(\.title) == ["alan", "main", "fish"], "pane title details must use compact labels")
    }

    private static func verifiesPaneTitleDetailProjectionPreservesResponsivePriority() {
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let progress = TerminalActivitySnapshot.progressActivity(percent: 42, now: now)
        let testPane = ShellPane(
            paneID: "pane_1",
            tabID: "tab_1",
            spaceID: "space_1",
            launchTarget: .shell,
            cwd: "/Users/morris/Developer/alan",
            process: ShellProcessBinding(program: "fish", argvPreview: nil),
            attention: .notable,
            context: context(
                workingDirectoryName: "alan",
                repositoryRoot: "/Users/morris/Developer/alan",
                gitBranch: "feature/title-bar",
                processState: "running",
                rendererHealth: "failed",
                surfaceReadiness: "renderer_failed",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            activity: progress,
            alanBinding: ShellAlanBinding(
                sessionID: "session_1",
                runStatus: "running",
                pendingYield: true,
                source: "test",
                lastProjectedAt: nil
            )
        )

        let details = shellPaneTitleBarDetailProjection(
            for: testPane,
            title: "Editor",
            now: now
        )

        expect(
            details.map(\.id) == ["activity", "status", "worktree", "branch", "process", "alan"],
            "pane title detail projection must preserve responsive priority order"
        )
        expect(
            details.map(\.title) == [
                "Progress · 42%",
                "Renderer failed",
                "alan",
                "feature/title-bar",
                "fish",
                "Input",
            ],
            "pane title detail projection must keep compact labels in priority order"
        )
    }

    private static func verifiesPaneTitleDetailProjectionAvoidsDuplicateAgentAndAlan() {
        let codexActivity = activity(
            status: .running,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Running",
            agent: TerminalActivityAgentMetadata(
                kind: .codex,
                safeSessionLabel: nil,
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            )
        )
        let codexPane = pane(
            context: context(
                workingDirectoryName: "alan",
                repositoryRoot: "/Users/morris/Developer/alan",
                gitBranch: "main",
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            cwd: "/Users/morris/Developer/alan",
            process: ShellProcessBinding(program: "codex", argvPreview: ["codex"]),
            attention: .idle,
            activity: codexActivity
        )
        let codexDetails = shellPaneTitleBarDetailProjection(
            for: codexPane,
            title: "alan",
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )
        expect(codexDetails.map(\.id) == ["activity", "branch"], "Codex activity must not duplicate process")

        let alanActivity = activity(
            status: .running,
            source: .alan,
            sourceLabel: "alan",
            stateLabel: "Running"
        )
        let alanPane = ShellPane(
            paneID: "pane_1",
            tabID: "tab_1",
            spaceID: "space_1",
            launchTarget: .alan,
            cwd: "/Users/morris/Developer/alan",
            process: ShellProcessBinding(program: "alan", argvPreview: ["alan", "chat"]),
            attention: .active,
            context: context(
                workingDirectoryName: "alan",
                repositoryRoot: nil,
                gitBranch: nil,
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            activity: alanActivity,
            alanBinding: ShellAlanBinding(
                sessionID: "session_1",
                runStatus: "running",
                pendingYield: false,
                source: "test",
                lastProjectedAt: nil
            )
        )
        let alanDetails = shellPaneTitleBarDetailProjection(
            for: alanPane,
            title: "alan",
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )
        expect(alanDetails.map(\.id) == ["activity"], "alan activity must not duplicate alan binding or process")
    }

    private static func verifiesActivityNotificationPolicyIsLowNoise() {
        let now = Date(timeIntervalSince1970: 1_779_008_400)
        let testPane = pane(
            context: context(
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: nil,
            attention: .idle
        )
        let testTab = ShellTab(
            tabID: "tab_1",
            kind: .terminal,
            title: "alan",
            paneTree: ShellPaneTreeNode(
                nodeID: "node_pane_1",
                kind: .pane,
                direction: nil,
                paneID: "pane_1",
                children: nil
            )
        )

        let focusedProgress = TerminalActivitySnapshot.progressActivity(percent: 42, now: now)
        expect(
            shellActivityNotificationRoute(
                for: focusedProgress,
                pane: testPane,
                tab: testTab,
                visibility: .focusedVisible,
                now: now
            ) == nil,
            "focused progress must stay visual-only"
        )

        let agentNeedsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed",
            agent: .init(
                kind: .codex,
                safeSessionLabel: "codex",
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            )
        )
        let needsInputRoute = shellActivityNotificationRoute(
            for: agentNeedsInput,
            pane: testPane,
            tab: testTab,
            visibility: .background,
            now: now
        )
        expect(needsInputRoute?.kind == .needsInput, "background agent input must be notification-worthy")
        expect(needsInputRoute?.attention == .awaitingUser, "agent input must mark tab as awaiting user")

        let focusedSuccess = commandActivity(
            exitCode: 0,
            durationMilliseconds: 120_000,
            updatedAt: "2026-05-17T09:00:00Z"
        )
        expect(
            shellActivityNotificationRoute(
                for: focusedSuccess,
                pane: testPane,
                tab: testTab,
                visibility: .focusedVisible,
                now: now
            ) == nil,
            "focused command success must not send a notification"
        )

        let shortBackgroundSuccess = commandActivity(
            exitCode: 0,
            durationMilliseconds: 5_000,
            updatedAt: "2026-05-17T09:00:00Z"
        )
        expect(
            shellActivityNotificationRoute(
                for: shortBackgroundSuccess,
                pane: testPane,
                tab: testTab,
                visibility: .background,
                now: now
            ) == nil,
            "short background command success must remain quiet"
        )

        let longBackgroundSuccess = commandActivity(
            exitCode: 0,
            durationMilliseconds: 120_000,
            updatedAt: "2026-05-17T09:00:00Z"
        )
        let longCommandRoute = shellActivityNotificationRoute(
            for: longBackgroundSuccess,
            pane: testPane,
            tab: testTab,
            visibility: .background,
            now: now
        )
        expect(longCommandRoute?.kind == .commandCompleted, "long background command completion must route")
        expect(longCommandRoute?.attention == .notable, "long command completion must mark the tab notable")

        let realFactoryLongSuccess = TerminalActivitySnapshot.commandCompletion(
            exitCode: 0,
            now: now,
            durationMilliseconds: 120_000
        )
        expect(
            shellActivityNotificationRoute(
                for: realFactoryLongSuccess,
                pane: testPane,
                tab: testTab,
                visibility: .background,
                now: now
            )?.kind == .commandCompleted,
            "factory-produced long command completion must route"
        )

        let exited = TerminalActivitySnapshot.processExitedActivity(exitCode: 9, now: now)
        let exitedRoute = shellActivityNotificationRoute(
            for: exited,
            pane: testPane,
            tab: testTab,
            visibility: .background,
            now: now
        )
        expect(exitedRoute?.kind == .processExited, "background process exit must route")
        expect(exitedRoute?.attention == .awaitingUser, "process exit must mark the tab awaiting user")
    }

    private static func verifiesControllerRoutesActivityNotificationsOnce() {
        let controller = makeController()
        _ = controller.openTerminalTab()
        guard let backgroundPane = controller.shellState.panes.first(where: { $0.paneID != "pane_1" }) else {
            fail("test setup must create a background pane")
        }
        controller.focus(paneID: "pane_1")

        let needsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed",
            agent: .init(
                kind: .codex,
                safeSessionLabel: "codex",
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            )
        )
        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", activity: needsInput),
            for: backgroundPane.paneID
        )
        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", activity: needsInput),
            for: backgroundPane.paneID
        )

        expect(
            controller.activityNotifications.count == 1,
            "controller must route one notification per activity update"
        )
        expect(
            controller.activityNotifications.first?.kind == .needsInput,
            "controller notification must preserve the routed activity kind"
        )
        expect(
            controller.shellState.pane(paneID: backgroundPane.paneID)?.attention == .idle,
            "notification-worthy activity must not persist into pane attention"
        )
        expect(
            controller.shellState.pane(paneID: backgroundPane.paneID).map {
                shellEffectiveAttention(for: $0, now: Date())
            } == .awaitingUser,
            "notification-worthy agent input must overlay its pane awaiting user at read time"
        )
    }

    private static func verifiesControllerRoutesDistinctActivityPayloadsInSameSecond() {
        let controller = makeController()
        _ = controller.openTerminalTab()
        guard let backgroundPane = controller.shellState.panes.first(where: { $0.paneID != "pane_1" }) else {
            fail("test setup must create a background pane")
        }
        controller.focus(paneID: "pane_1")

        let firstNeedsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed",
            detailLabel: "Review plan",
            agent: .init(
                kind: .codex,
                safeSessionLabel: "codex",
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            ),
            updatedAt: "2026-05-17T09:00:00Z"
        )
        let secondNeedsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed",
            detailLabel: "Approve changes",
            agent: .init(
                kind: .codex,
                safeSessionLabel: "codex",
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            ),
            updatedAt: "2026-05-17T09:00:00Z"
        )

        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", activity: firstNeedsInput),
            for: backgroundPane.paneID
        )
        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", activity: secondNeedsInput),
            for: backgroundPane.paneID
        )

        expect(
            controller.activityNotifications.count == 2,
            "distinct same-second activity payloads must each route a notification"
        )
        expect(
            controller.activityNotifications.first?.id != controller.activityNotifications.last?.id,
            "notification ids must include a payload discriminator beyond the second-level timestamp"
        )
    }

    private static func verifiesInactiveAppRoutesFocusedPaneNotifications() {
        let controller = makeController(appIsActive: false)
        let needsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed",
            agent: .init(
                kind: .codex,
                safeSessionLabel: "codex",
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            )
        )

        controller.updateTerminalMetadata(
            metadata(title: "codex", cwd: "/repo/app", activity: needsInput),
            for: "pane_1"
        )

        expect(
            controller.activityNotifications.count == 1,
            "inactive app must route focused pane activity because it is out of view"
        )
        expect(
            controller.activityNotifications.first?.kind == .needsInput,
            "inactive focused pane notification must preserve the routed activity kind"
        )
    }

    private static func verifiesHiddenQuickTerminalRoutesUserActionableActivityNotifications() {
        let controller = makeController()
        let needsInput = activity(
            status: .needsInput,
            source: .codex,
            sourceLabel: "Codex",
            stateLabel: "Input needed",
            agent: .init(
                kind: .codex,
                safeSessionLabel: "codex",
                projectLabel: "alan",
                workingDirectory: "/Users/morris/Developer/alan"
            )
        )

        expect(controller.showQuickTerminal() != nil, "quick terminal must show before hiding")
        expect(controller.hideQuickTerminal(), "quick terminal hide must preserve the runtime slot")
        controller.updateTerminalMetadata(
            metadata(title: "Quick Terminal", cwd: "/repo/app", activity: needsInput),
            for: ShellQuickTerminalSlot.globalPaneID
        )

        expect(
            controller.activityNotifications.count == 1,
            "hidden quick-terminal activity must still route through notification policy"
        )
        expect(
            controller.activityNotifications.first?.paneID == ShellQuickTerminalSlot.globalPaneID,
            "hidden quick-terminal notification must point at the global quick-terminal pane"
        )
        expect(
            controller.activityNotifications.first?.kind == .needsInput,
            "hidden quick-terminal notification must preserve the routed activity kind"
        )
        expect(
            controller.activityNotifications.first?.body == "app",
            "hidden quick-terminal notification must use the standard pane context body"
        )
    }

    private static func verifiesProcessExitNotificationRoutesBeforeAutoClose() {
        let controller = makeController()
        _ = controller.openTerminalTab(workingDirectory: "/second")
        controller.focus(paneID: "pane_1")

        let processExitActivity = TerminalActivitySnapshot.processExitedActivity(
            exitCode: 0,
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )

        controller.updateTerminalMetadata(
            childExitMetadata(title: "fish", exitCode: 0, activity: processExitActivity),
            for: "pane_2"
        )

        expect(controller.pane(paneID: "pane_2") == nil, "child exit must still close the pane")
        expect(
            controller.activityNotifications.count == 1,
            "process-exit activity must route before auto-close removes the pane"
        )
        expect(
            controller.activityNotifications.first?.kind == .processExited,
            "auto-closed process exit must preserve the notification kind"
        )
        expect(
            controller.activityNotifications.first?.paneID == "pane_2",
            "auto-closed process exit notification must point at the exiting pane"
        )
    }

    private static func verifiesProcessExitRuntimeNotificationRoutesBeforeAutoClose() {
        let controller = makeController()
        _ = controller.openTerminalTab(workingDirectory: "/second")
        controller.focus(paneID: "pane_1")
        guard let exitingPane = controller.pane(paneID: "pane_2") else {
            fail("test setup must create a background pane")
        }

        let processExitActivity = TerminalActivitySnapshot.processExitedActivity(
            exitCode: 130,
            now: Date(timeIntervalSince1970: 1_779_008_400)
        )

        controller.updateTerminalRuntime(
            TerminalHostRuntimeSnapshot(
                stage: .windowAttached,
                paneID: exitingPane.paneID,
                tabID: exitingPane.tabID,
                logicalSize: .zero,
                backingSize: .zero,
                displayName: "Studio Display",
                displayID: "display_1",
                attachedWindowTitle: "alan",
                isFocused: false,
                renderer: TerminalRendererSnapshot(
                    kind: .ghosttyLive,
                    phase: .surfaceReady,
                    summary: "surface ready",
                    detail: nil,
                    failureReason: nil,
                    recentEvents: []
                ),
                paneMetadata: childExitMetadata(
                    title: "fish",
                    exitCode: 130,
                    activity: processExitActivity
                ),
                surfaceState: AlanTerminalSurfaceStateSnapshot(
                    readiness: .unready(reason: .childExited),
                    terminalMode: .normalBuffer,
                    scrollback: .empty,
                    search: nil,
                    semanticCommands: .placeholder,
                    readonly: false,
                    secureInput: false,
                    inputReady: false,
                    rendererHealth: "surface_ready",
                    childExited: true,
                    lastUpdatedAt: Date(timeIntervalSince1970: 2_001)
                ),
                lastUpdatedAt: Date(timeIntervalSince1970: 2_002)
            )
        )

        expect(controller.pane(paneID: "pane_2") == nil, "runtime child exit must still close the pane")
        expect(
            controller.activityNotifications.count == 1,
            "runtime process-exit activity must route before auto-close removes the pane"
        )
        expect(
            controller.activityNotifications.first?.kind == .processExited,
            "runtime auto-closed process exit must preserve the notification kind"
        )
    }

    private static func verifiesTerminalChildExitClosesSplitPane() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)

        controller.updateTerminalMetadata(childExitMetadata(title: "fish", exitCode: 0), for: "pane_2")

        expect(controller.pane(paneID: "pane_2") == nil, "child exit must close the owning split pane")
        expect(controller.pane(paneID: "pane_1") != nil, "child exit must preserve sibling panes")
        expect(controller.shellState.focusedPaneID == "pane_1", "child exit must focus the remaining sibling")
    }

    private static func verifiesTerminalChildExitClosesSinglePaneTab() {
        let controller = makeController()
        _ = controller.openTerminalTab(workingDirectory: "/second")

        controller.updateTerminalMetadata(childExitMetadata(title: "fish", exitCode: 0), for: "pane_2")

        expect(controller.shellState.tab(tabID: "tab_2") == nil, "child exit must close the owning single-pane tab")
        expect(controller.pane(paneID: "pane_1") != nil, "child exit must preserve other tabs")
        expect(controller.shellState.focusedPaneID == "pane_1", "child exit must move focus to the next valid pane")
    }

    private static func verifiesTerminalChildExitCanLeaveEmptyFocusedSpace() {
        let controller = makeController()

        controller.updateTerminalMetadata(childExitMetadata(title: "fish", exitCode: 0), for: "pane_1")

        expect(controller.shellState.spaces.count == 1, "final child exit must keep the focused space")
        expect(controller.shellState.spaces.first?.tabs.isEmpty == true, "final child exit may leave an empty space")
        expect(controller.shellState.panes.isEmpty, "final child exit must not restart a replacement pane")
        expect(controller.shellState.focusedPaneID == nil, "final child exit must clear focused pane")
    }

    private static func verifiesClosingTabReleasesTerminalRuntime() {
        let controller = makeController()
        guard let pane = controller.selectedPane else {
            fail("test setup must expose selected pane")
        }
        _ = controller.terminalRuntimeRegistry.surfaceHandle(for: pane, bootProfile: nil)

        expect(
            controller.terminalRuntimeRegistry.registeredPaneIDs.contains(pane.paneID),
            "test setup must register selected pane runtime"
        )

        _ = controller.closeTab(tabID: pane.tabID)

        expect(
            !controller.terminalRuntimeRegistry.registeredPaneIDs.contains(pane.paneID),
            "closing a tab must release its terminal runtime through the registry"
        )
    }

    private static func verifiesTabSelectionCommitsAuthoritativeFocus() {
        let controller = makeController()
        _ = controller.openTerminalTab()
        controller.focus(paneID: "pane_1")

        guard let targetPane = controller.pane(paneID: "pane_2") else {
            fail("test setup must create second tab pane")
        }
        let targetHostView = controller.terminalRuntimeRegistry.hostView(
            for: targetPane,
            bootProfile: controller.bootProfile(for: targetPane),
            isSelected: false,
            activationDelegate: nil,
            onShellAction: nil,
            onCommandInput: nil,
            onCloseRequest: nil,
            onRuntimeUpdate: { _ in },
            onMetadataUpdate: { _ in }
        )

        controller.select(tabID: "tab_2")
        controller.updateTerminalMetadata(
            metadata(title: "old focused pane updated"),
            for: "pane_1"
        )

        expect(
            controller.shellState.focusedPaneID == "pane_2",
            "tab selection must update authoritative focused pane"
        )
        expect(controller.selectedTabID == "tab_2", "runtime metadata must not revert selected tab")
        expect(controller.selectedPane?.paneID == "pane_2", "selected pane must follow selected tab focus")
        expect(
            targetHostView.focusCount == 1,
            "tab selection must request focus for the target terminal runtime"
        )
    }

    private static func verifiesShellActionTabNavigationTargetsCurrentSelection() {
        let controller = makeController()
        _ = controller.openTerminalTab()
        _ = controller.openTerminalTab()

        let result = controller.performShellAction(.tabSelectPrevious, target: .contextTab("tab_main"))

        expect(result == .executed, "previous-tab shortcut action must execute with multiple tabs")
        expect(
            controller.selectedTabID == "tab_2",
            "keyboard tab navigation must use the current selected tab, not a context-menu tab target"
        )
        expect(
            controller.shellState.focusedPaneID == "pane_2",
            "keyboard tab navigation must commit focus for the selected tab"
        )
    }

    private static func verifiesSpaceSelectionCommitsAuthoritativeFocus() {
        let controller = makeController()
        _ = controller.createTerminalSpace(title: "Second", workingDirectory: "/tmp")
        controller.focus(paneID: "pane_1")

        guard let targetPane = controller.pane(paneID: "pane_2") else {
            fail("test setup must create second space pane")
        }
        let targetHostView = controller.terminalRuntimeRegistry.hostView(
            for: targetPane,
            bootProfile: controller.bootProfile(for: targetPane),
            isSelected: false,
            activationDelegate: nil,
            onShellAction: nil,
            onCommandInput: nil,
            onCloseRequest: nil,
            onRuntimeUpdate: { _ in },
            onMetadataUpdate: { _ in }
        )

        controller.select(spaceID: "space_2")
        controller.updateTerminalMetadata(
            metadata(title: "old focused pane updated"),
            for: "pane_1"
        )

        expect(
            controller.shellState.focusedSpaceID == "space_2",
            "space selection must update focused space"
        )
        expect(controller.shellState.focusedTabID == "tab_2", "space selection must update focused tab")
        expect(
            controller.shellState.focusedPaneID == "pane_2",
            "space selection must update authoritative focused pane"
        )
        expect(controller.selectedSpaceID == "space_2", "runtime metadata must not revert selected space")
        expect(controller.selectedTabID == "tab_2", "runtime metadata must not revert selected space tab")
        expect(
            targetHostView.focusCount == 1,
            "space selection must request focus for the target terminal runtime"
        )
    }

    private static func verifiesShellActionSpaceSelectionReportsMissingTargets() {
        let controller = makeController()
        let selectedSpaceBefore = controller.selectedSpaceID

        let result = controller.performShellAction(.spaceSelectByIndex, target: .spaceIndex(8))

        expect(
            result == .unavailable(reason: "Space is not available"),
            "missing numeric space shortcuts must report a stable unavailable reason"
        )
        expect(
            controller.selectedSpaceID == selectedSpaceBefore,
            "missing numeric space shortcuts must not change the selected space"
        )
    }

    private static func verifiesSplitTabSelectionUsesStablePaneWithoutChangingLayout() {
        let controller = makeController()
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        let splitTreeBefore = controller.shellState.tab(tabID: "tab_main")?.paneTree
        _ = controller.openTerminalTab()

        controller.select(tabID: "tab_main")

        let splitTreeAfter = controller.shellState.tab(tabID: "tab_main")?.paneTree
        expect(
            controller.shellState.focusedPaneID == "pane_1",
            "split-tab selection must choose a stable pane from the tab tree"
        )
        expect(
            splitTreeAfter == splitTreeBefore,
            "split-tab selection must not rewrite split tree or divider ratios"
        )
    }

    private static func verifiesWorkspaceManifestStartupRestoresPinnedSnapshot() {
        let windowID = "manifest_startup_\(UUID().uuidString)"
        let manifestURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID)-workspace.json")
        let context = ShellWindowContext.make(windowID: windowID)
        let store = ShellWorkspaceManifestStore(manifestURL: manifestURL)
        let manifest = ShellWorkspaceManifest(
            schemaVersion: ShellWorkspaceManifest.currentSchemaVersion,
            windowID: windowID,
            selectedSpaceID: "space_main",
            selectedTabID: "tab_main",
            spaces: [
                ShellWorkspaceSpaceRecord(
                    spaceID: "space_main",
                    title: "Main",
                    order: 0,
                    createdAt: Date(timeIntervalSince1970: 10),
                    updatedAt: Date(timeIntervalSince1970: 10),
                    tabs: [
                        ShellWorkspaceTabRecord(
                            tabID: "tab_main",
                            title: "Pinned",
                            kind: .terminal,
                            createdAt: Date(timeIntervalSince1970: 10),
                            lastActivatedAt: Date(timeIntervalSince1970: 10),
                            lastActivityAt: Date(timeIntervalSince1970: 10),
                            isPinned: true,
                            pinSnapshot: restoreSnapshot(tabID: "tab_main", paneID: "pane_1", cwd: "/pinned"),
                            liveSnapshot: restoreSnapshot(tabID: "tab_main", paneID: "pane_1", cwd: "/live"),
                            activeTask: .inactive
                        )
                    ]
                )
            ]
        )

        do {
            try store.save(manifest)
        } catch {
            fail("failed to write test manifest: \(error)")
        }

        let controller = ShellHostController.live(
            windowContext: context,
            startupMode: .workspaceManifest,
            workspaceManifestURL: manifestURL,
            defaultWorkingDirectory: "/fallback",
            now: Date(timeIntervalSince1970: 20)
        )

        expect(controller.selectedPane?.cwd == "/pinned", "workspace manifest startup must use pinned cwd")
        expect(
            controller.shellState.focusedSpaceID == "space_main",
            "workspace manifest startup must preserve selected space"
        )
        expect(
            controller.shellState.focusedTabID == "tab_main",
            "workspace manifest startup must preserve selected tab"
        )
    }

    private static func verifiesClosingLastTabLeavesSelectedSpaceEmptyAndPersistsManifest() {
        let windowID = "manifest_close_\(UUID().uuidString)"
        let manifestURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID)-workspace.json")
        let store = ShellWorkspaceManifestStore(manifestURL: manifestURL)
        let controller = makeController(
            windowID: windowID,
            workspaceManifestStore: store,
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: windowID,
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 30)
            )
        )

        let result = controller.closeTab(tabID: "tab_main")

        expect(result == .closed, "closing the last tab in a space must succeed")
        expect(controller.shellState.spaces.count == 1, "closing the last tab must keep its space")
        expect(
            controller.shellState.spaces.first?.tabs.isEmpty == true,
            "closing the last tab must leave the selected space empty"
        )
        expect(controller.shellState.panes.isEmpty, "closing the last tab must remove its panes")
        expect(controller.selectedSpaceID == "space_main", "empty selected space must stay selected")
        expect(controller.selectedTabID == nil, "empty selected space must clear selected tab")

        guard let savedManifest = decodeManifest(at: manifestURL) else {
            fail("closing the last tab must persist workspace manifest")
        }
        expect(savedManifest.spaces.count == 1, "persisted manifest must keep empty space")
        expect(savedManifest.spaces.first?.tabs.isEmpty == true, "persisted manifest must keep space tabless")
        expect(savedManifest.selectedSpaceID == "space_main", "persisted manifest must keep selected space")
        expect(savedManifest.selectedTabID == nil, "persisted manifest must clear selected tab")
    }

    private static func verifiesExplicitSpaceDeletionRemovesManifestSpace() {
        let windowID = "manifest_delete_space_\(UUID().uuidString)"
        let manifestURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID)-workspace.json")
        let store = ShellWorkspaceManifestStore(manifestURL: manifestURL)
        let controller = makeController(
            windowID: windowID,
            workspaceManifestStore: store,
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: windowID,
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 40)
            )
        )
        _ = controller.createTerminalSpace(title: "Delete Me", workingDirectory: "/delete-me")

        expect(controller.deleteSpace(spaceID: "space_2"), "explicit delete-space must be accepted")
        expect(controller.shellState.space(spaceID: "space_2") == nil, "deleted space must leave shell state")

        guard let savedManifest = decodeManifest(at: manifestURL) else {
            fail("delete-space must persist workspace manifest")
        }
        expect(savedManifest.spaces.map(\.spaceID) == ["space_main"], "deleted space must leave manifest")
        expect(
            savedManifest.spaces.flatMap(\.tabs).allSatisfy { $0.tabID != "tab_2" },
            "delete-space must remove deleted space tabs from manifest"
        )
    }

    private static func verifiesPinSnapshotIsExplicitAndDoesNotTrackTransientChanges() {
        let windowID = "manifest_pin_\(UUID().uuidString)"
        let manifestURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID)-workspace.json")
        let store = ShellWorkspaceManifestStore(manifestURL: manifestURL)
        let controller = makeController(
            windowID: windowID,
            workspaceManifestStore: store,
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: windowID,
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 50)
            )
        )

        controller.updateTerminalMetadata(metadata(title: "Pinned", cwd: "/pinned"), for: "pane_1")
        _ = controller.splitPane(paneID: "pane_1", placement: .right)
        expect(controller.pinTab(tabID: "tab_main"), "pin-tab must be accepted")

        controller.updateTerminalMetadata(metadata(title: "Moved", cwd: "/moved"), for: "pane_1")
        _ = controller.splitPane(paneID: "pane_1", placement: .down)

        guard let savedManifest = decodeManifest(at: manifestURL),
              let tab = savedManifest.spaces.flatMap(\.tabs).first(where: { $0.tabID == "tab_main" })
        else {
            fail("pin-tab must persist manifest tab")
        }

        expect(tab.isPinned, "pin-tab must mark the tab as pinned")
        expect(tab.pinSnapshot?.paneTree.paneIDs.count == 2, "pin snapshot must preserve split layout at pin time")
        expect(tab.liveSnapshot?.paneTree.paneIDs.count == 3, "live snapshot must track later transient split changes")
        expect(
            tab.pinSnapshot?.panes.first(where: { $0.paneID == "pane_1" })?.cwd == "/pinned",
            "pin snapshot must keep cwd from pin time"
        )
        expect(
            tab.liveSnapshot?.panes.first(where: { $0.paneID == "pane_1" })?.cwd == "/moved",
            "live snapshot must track later cwd changes without mutating pin snapshot"
        )

        expect(controller.updatePinnedTabSnapshot(tabID: "tab_main"), "update-pin must be accepted")
        let updatedManifest = decodeManifest(at: manifestURL)
        let updatedTab = updatedManifest?.spaces.flatMap(\.tabs).first { $0.tabID == "tab_main" }
        expect(updatedTab?.pinSnapshot?.paneTree.paneIDs.count == 3, "update-pin must replace pin split snapshot")
        expect(
            updatedTab?.pinSnapshot?.panes.first(where: { $0.paneID == "pane_1" })?.cwd == "/moved",
            "update-pin must replace pin cwd snapshot"
        )
    }

    private static func verifiesTabOrganizationPersistsOrderPinAndSpaceOwnership() {
        let windowID = "manifest_tab_org_\(UUID().uuidString)"
        let manifestURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID)-workspace.json")
        let store = ShellWorkspaceManifestStore(manifestURL: manifestURL)
        let controller = makeController(
            windowID: windowID,
            workspaceManifestStore: store,
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: windowID,
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 60)
            )
        )

        guard let secondTabID = controller.openTerminalTab(title: "Second"),
              let targetSpaceID = controller.createTerminalSpace(title: "Target")
        else {
            fail("tab organization setup must create a second tab and target space")
        }

        guard let secondPaneID = controller.shellState.panes(in: secondTabID).first?.paneID else {
            fail("second tab must have a pane")
        }
        controller.focus(paneID: secondPaneID)
        expect(controller.pinTab(tabID: secondTabID), "pinning organization action must be accepted")
        expect(
            controller.moveTabToSpace(tabID: secondTabID, targetSpaceID: targetSpaceID),
            "move-tab-to-space organization action must be accepted"
        )

        guard let savedManifest = decodeManifest(at: manifestURL),
              let targetSpace = savedManifest.spaces.first(where: { $0.spaceID == targetSpaceID }),
              let movedTab = targetSpace.tabs.first(where: { $0.tabID == secondTabID })
        else {
            fail("tab organization must persist the moved tab in the target space")
        }

        expect(movedTab.isPinned, "move-tab-to-space must preserve pin state")
        expect(movedTab.pinSnapshot != nil, "pinning through organization must persist a pin snapshot")
        expect(
            targetSpace.tabs.filter(\.isPinned).map(\.tabID).last == secondTabID,
            "moved pinned tab must be inserted in the target pinned section"
        )
        expect(
            savedManifest.spaces.first?.tabs.allSatisfy { $0.tabID != secondTabID } == true,
            "source space order must remove the moved tab"
        )
        expect(
            savedManifest.selectedSpaceID == targetSpaceID && savedManifest.selectedTabID == secondTabID,
            "moving the selected tab must persist the followed selection"
        )
    }

    private static func verifiesManifestActiveTaskProjection() {
        let foregroundURL = manifestURL("active_foreground")
        let foregroundController = makeController(
            windowID: "active_foreground_\(UUID().uuidString)",
            workspaceManifestStore: ShellWorkspaceManifestStore(manifestURL: foregroundURL),
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: "window_main",
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 60)
            )
        )
        foregroundController.updateTerminalMetadata(
            metadata(title: "make test", activeTaskState: .foregroundCommand),
            for: "pane_1"
        )
        expect(activeTask(in: foregroundURL) == .foregroundCommand, "foreground command must protect tab")
        expect(
            foregroundController.shellState.panes.first?.context?.processState == "foreground_command",
            "foreground command metadata must project into pane process state"
        )

        let idleURL = manifestURL("active_idle")
        let idleController = makeController(
            windowID: "active_idle_\(UUID().uuidString)",
            workspaceManifestStore: ShellWorkspaceManifestStore(manifestURL: idleURL),
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: "window_main",
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 61)
            )
        )
        idleController.updateTerminalMetadata(
            metadata(title: "zsh", activeTaskState: .inactive),
            for: "pane_1"
        )
        expect(activeTask(in: idleURL) == .inactive, "idle shell must be eligible for retirement")
        expect(
            idleController.shellState.panes.first?.context?.processState == "running",
            "idle shell metadata must remain running but not foreground"
        )

        let exitedURL = manifestURL("active_exited")
        let exitedController = makeController(
            windowID: "active_exited_\(UUID().uuidString)",
            workspaceManifestStore: ShellWorkspaceManifestStore(manifestURL: exitedURL),
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: "window_main",
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 62)
            )
        )
        exitedController.updateTerminalMetadata(
            metadata(title: "done", processExited: true, activeTaskState: .foregroundCommand),
            for: "pane_1"
        )
        expect(
            activeTask(in: exitedURL) == nil,
            "exited terminal must not protect a tab after lifecycle close removes it"
        )
        expect(
            exitedController.shellState.panes.isEmpty,
            "exited metadata must close the pane instead of preserving foreground state"
        )

        let activeOnlyURL = manifestURL("active_only")
        let activeOnlyController = makeController(
            windowID: "active_only_\(UUID().uuidString)",
            workspaceManifestStore: ShellWorkspaceManifestStore(manifestURL: activeOnlyURL),
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: "window_main",
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 64)
            )
        )
        activeOnlyController.updateTerminalMetadata(
            activeOnlyMetadata(activeTaskState: .inactive),
            for: "pane_1"
        )
        activeOnlyController.updateTerminalMetadata(
            activeOnlyMetadata(activeTaskState: .unknown),
            for: "pane_1"
        )
        expect(
            activeTask(in: activeOnlyURL) == .unknown,
            "active-task-only metadata changes must persist the manifest"
        )

        let alanPendingURL = manifestURL("active_alan_pending")
        let alanPendingWindowID = "active_alan_pending_\(UUID().uuidString)"
        let alanPendingController = makeController(
            windowID: alanPendingWindowID,
            shellState: stateWithAlanBinding(windowID: alanPendingWindowID, pendingYield: true),
            workspaceManifestStore: ShellWorkspaceManifestStore(manifestURL: alanPendingURL),
            workspaceManifest: ShellWorkspaceManifest.defaultManifest(
                windowID: alanPendingWindowID,
                defaultWorkingDirectory: "/tmp",
                now: Date(timeIntervalSince1970: 63)
            )
        )
        _ = alanPendingController.setAttention(.awaitingUser, for: "pane_1")
        expect(activeTask(in: alanPendingURL) == .alanPendingYield, "alan pending yield must protect tab")
    }

    private static func makeController(
        windowID: String = "metadata_test_\(UUID().uuidString)",
        shellState: ShellStateSnapshot? = nil,
        workspaceManifestStore: ShellWorkspaceManifestStore? = nil,
        workspaceManifest: ShellWorkspaceManifest? = nil,
        appIsActive: Bool = true
    ) -> ShellHostController {
        let registry = TerminalRuntimeRegistry(runtimeService: FakeAlanTerminalRuntimeService())
        let context = ShellWindowContext.make(
            windowID: windowID,
            terminalRuntimeRegistry: registry
        )
        let persistenceURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID).json")
        return ShellHostController(
            shellState: shellState ?? .bootstrapDefault(windowID: windowID),
            windowContext: context,
            persistenceURL: persistenceURL,
            terminalRuntimeRegistry: registry,
            workspaceManifestStore: workspaceManifestStore,
            workspaceManifest: workspaceManifest,
            appIsActiveProvider: { appIsActive }
        )
    }

    private static func manifestURL(_ prefix: String) -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("\(prefix)-\(UUID().uuidString)-workspace.json")
    }

    private static func restoreSnapshot(
        tabID: String,
        paneID: String,
        cwd: String
    ) -> ShellTabRestoreSnapshot {
        ShellTabRestoreSnapshot(
            paneTree: ShellPaneTreeNode(
                nodeID: "node_\(paneID)",
                kind: .pane,
                direction: nil,
                paneID: paneID,
                children: nil
            ),
            panes: [
                ShellPaneRestoreRecord(
                    paneID: paneID,
                    launchTarget: .shell,
                    cwd: cwd,
                    title: tabID
                )
            ]
        )
    }

    private static func decodeManifest(at url: URL) -> ShellWorkspaceManifest? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return try? decoder.decode(ShellWorkspaceManifest.self, from: data)
    }

    private static func pane(
        context: ShellContextSnapshot,
        viewport: ShellViewportSnapshot?,
        cwd: String? = "/Users/morris/Developer/Alan",
        launchTarget: ShellLaunchTarget = .shell,
        process: ShellProcessBinding? = ShellProcessBinding(program: "fish", argvPreview: nil),
        attention: ShellAttentionState,
        activity: TerminalActivitySnapshot? = nil
    ) -> ShellPane {
        ShellPane(
            paneID: "pane_1",
            tabID: "tab_1",
            spaceID: "space_1",
            launchTarget: launchTarget,
            cwd: cwd,
            process: process,
            attention: attention,
            context: context,
            viewport: viewport,
            activity: activity,
            alanBinding: nil
        )
    }

    private static func context(
        workingDirectoryName: String? = "alan",
        repositoryRoot: String? = nil,
        gitBranch: String? = nil,
        processState: String,
        rendererHealth: String,
        surfaceReadiness: String,
        lastCommandExitCode: Int?
    ) -> ShellContextSnapshot {
        ShellContextSnapshot(
            workingDirectoryName: workingDirectoryName,
            repositoryRoot: repositoryRoot,
            gitBranch: gitBranch,
            controlPath: nil,
            alanBindingFile: nil,
            launchStrategy: nil,
            shellIntegrationSource: "ghostty_shell_integration",
            processState: processState,
            rendererHealth: rendererHealth,
            surfaceReadiness: surfaceReadiness,
            inputReady: surfaceReadiness == "ready",
            readonly: false,
            terminalMode: "normal_buffer",
            lastMetadataAt: nil,
            lastCommandExitCode: lastCommandExitCode
        )
    }

    private static func metadata(
        title: String,
        cwd: String = "/Users/morris/Developer/Alan",
        processExited: Bool = false,
        activeTaskState: ShellTabActiveTaskState? = nil,
        activity: TerminalActivitySnapshot? = nil,
        clearsActivity: Bool = false
    ) -> TerminalPaneMetadataSnapshot {
        TerminalPaneMetadataSnapshot(
            title: title,
            workingDirectory: cwd,
            summary: nil,
            attention: .idle,
            processExited: processExited,
            lastCommandExitCode: nil,
            lastUpdatedAt: Date(timeIntervalSince1970: 3_000),
            activeTaskState: activeTaskState,
            activity: activity,
            clearsActivity: clearsActivity
        )
    }

    private static func progressActivity(
        percent: Int,
        updatedAt: String,
        staleAt: String
    ) -> TerminalActivitySnapshot {
        activity(
            status: .progress,
            source: .progress,
            sourceLabel: "Progress",
            stateLabel: "\(percent)%",
            progress: .percent(percent),
            updatedAt: updatedAt,
            staleAt: staleAt
        )
    }

    private static func activity(
        status: TerminalActivityStatus,
        source: TerminalActivitySourceKind,
        sourceLabel: String,
        stateLabel: String,
        detailLabel: String? = nil,
        progress: TerminalActivityProgress? = nil,
        command: TerminalActivityCommandOutcome? = nil,
        agent: TerminalActivityAgentMetadata? = nil,
        updatedAt: String = "2026-05-17T09:00:00Z",
        staleAt: String? = nil
    ) -> TerminalActivitySnapshot {
        TerminalActivitySnapshot(
            source: .init(kind: source, label: sourceLabel),
            status: status,
            priority: priority(for: status),
            progress: progress,
            command: command,
            agent: agent,
            display: TerminalActivityDisplay(
                sourceLabel: sourceLabel,
                stateLabel: stateLabel,
                detailLabel: detailLabel,
                paneHint: nil
            ),
            freshness: TerminalActivityFreshness(
                updatedAt: updatedAt,
                staleAt: staleAt,
                expiresAt: nil
            )
        )
    }

    private static func commandActivity(
        exitCode: Int,
        durationMilliseconds: Int,
        updatedAt: String
    ) -> TerminalActivitySnapshot {
        let succeeded = exitCode == 0
        return activity(
            status: succeeded ? .done : .failed,
            source: .command,
            sourceLabel: "Shell",
            stateLabel: succeeded ? "Command succeeded" : "Command failed \(exitCode)",
            command: TerminalActivityCommandOutcome(
                exitCode: exitCode,
                durationMilliseconds: durationMilliseconds,
                commandText: nil
            ),
            updatedAt: updatedAt
        )
    }

    private static func priority(for status: TerminalActivityStatus) -> TerminalActivityPriority {
        switch status {
        case .needsInput:
            return .awaitingUser
        case .failed, .exited:
            return .notable
        case .paused, .progress, .running, .bell:
            return .active
        case .idle, .done, .stale:
            return .passive
        }
    }

    private static func childExitMetadata(
        title: String,
        cwd: String = "/Users/morris/Developer/Alan",
        exitCode: Int,
        activity: TerminalActivitySnapshot? = nil
    ) -> TerminalPaneMetadataSnapshot {
        TerminalPaneMetadataSnapshot(
            title: title,
            workingDirectory: cwd,
            summary: "process exited",
            attention: .awaitingUser,
            processExited: true,
            lastCommandExitCode: exitCode,
            lastUpdatedAt: Date(timeIntervalSince1970: 3_100),
            activeTaskState: .inactive,
            activity: activity
        )
    }

    private static func activeOnlyMetadata(
        activeTaskState: ShellTabActiveTaskState
    ) -> TerminalPaneMetadataSnapshot {
        TerminalPaneMetadataSnapshot(
            title: nil,
            workingDirectory: nil,
            summary: nil,
            attention: .idle,
            processExited: false,
            lastCommandExitCode: nil,
            lastUpdatedAt: nil,
            activeTaskState: activeTaskState
        )
    }

    private static func stateWithAlanBinding(
        windowID: String,
        pendingYield: Bool,
        activity: TerminalActivitySnapshot? = nil
    ) -> ShellStateSnapshot {
        let pane = ShellPane(
            paneID: "pane_1",
            tabID: "tab_main",
            spaceID: "space_main",
            launchTarget: .alan,
            cwd: "/tmp",
            process: ShellProcessBinding(program: "alan", argvPreview: ["alan", "chat"]),
            attention: pendingYield ? .awaitingUser : .active,
            context: ShellContextSnapshot(
                workingDirectoryName: "tmp",
                repositoryRoot: nil,
                gitBranch: nil,
                controlPath: "/tmp/control",
                alanBindingFile: "/tmp/binding",
                launchStrategy: "login_shell",
                shellIntegrationSource: "ghostty_shell_integration",
                processState: "running",
                lastMetadataAt: nil,
                lastCommandExitCode: nil
            ),
            viewport: nil,
            activity: activity,
            alanBinding: ShellAlanBinding(
                sessionID: "session_1",
                runStatus: pendingYield ? "yielded" : "running",
                pendingYield: pendingYield,
                source: "test",
                lastProjectedAt: nil
            )
        )

        return ShellStateSnapshot(
            contractVersion: "0.1",
            windowID: windowID,
            focusedSpaceID: "space_main",
            focusedTabID: "tab_main",
            focusedPaneID: "pane_1",
            spaces: [
                ShellSpace(
                    spaceID: "space_main",
                    title: "Main",
                    attention: pane.attention,
                    tabs: [
                        ShellTab(
                            tabID: "tab_main",
                            kind: .terminal,
                            title: "alan",
                            paneTree: ShellPaneTreeNode(
                                nodeID: "node_pane_1",
                                kind: .pane,
                                direction: nil,
                                paneID: "pane_1",
                                children: nil
                            )
                        )
                    ]
                )
            ],
            panes: [pane]
        )
    }

    private static func activeTask(in url: URL) -> ShellTabActiveTaskState? {
        decodeManifest(at: url)?.spaces.first?.tabs.first?.activeTask
    }

    @MainActor
    private final class FakeQuickTerminalPeakWindow: ShellQuickTerminalPeakWindowing {
        var onDismissRequest: (() -> Void)?
        private(set) var presentedPaneIDs: [String] = []
        private(set) var focusedPaneIDs: [String] = []
        private(set) var dismissalReasons: [ShellQuickTerminalPeakDismissalReason] = []
        private(set) var lastPlacement: ShellQuickTerminalPeakPlacement?
        private(set) var lastTabID: String?
        private(set) var isVisible = false

        func presentQuickTerminal(
            host: ShellHostController,
            pane: ShellPane,
            tab: ShellTab,
            placement: ShellQuickTerminalPeakPlacement
        ) {
            isVisible = true
            presentedPaneIDs.append(pane.paneID)
            lastTabID = tab.tabID
            lastPlacement = placement
        }

        func dismissQuickTerminalPeak(reason: ShellQuickTerminalPeakDismissalReason) {
            isVisible = false
            dismissalReasons.append(reason)
        }

        func focusTerminal(paneID: String) {
            focusedPaneIDs.append(paneID)
        }
    }

    private static func decodeControlCommand(_ json: String) -> AlanShellControlCommand {
        do {
            let data = Data(json.utf8)
            return try JSONDecoder().decode(AlanShellControlCommand.self, from: data)
        } catch {
            fail("failed to decode control command fixture: \(error)")
        }
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: String
    ) {
        guard condition() else {
            fail(message)
        }
    }

    private static func fail(_ message: String) -> Never {
        fputs("error: \(message)\n", stderr)
        exit(1)
    }
}
#endif
