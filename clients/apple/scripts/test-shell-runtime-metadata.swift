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
        verifiesOpeningTerminalTabFallsBackToFocusedPaneSnapshotCwd()
        verifiesOpeningTerminalTabHonorsExplicitCwd()
        verifiesTerminalChildExitClosesSplitPane()
        verifiesTerminalChildExitClosesSinglePaneTab()
        verifiesTerminalChildExitCanLeaveEmptyFocusedSpace()
        verifiesClosingTabReleasesTerminalRuntime()
        verifiesTabSelectionCommitsAuthoritativeFocus()
        verifiesSpaceSelectionCommitsAuthoritativeFocus()
        verifiesSplitTabSelectionUsesStablePaneWithoutChangingLayout()
        verifiesWorkspaceManifestStartupRestoresPinnedSnapshot()
        verifiesClosingLastTabLeavesSelectedSpaceEmptyAndPersistsManifest()
        verifiesExplicitSpaceDeletionRemovesManifestSpace()
        verifiesPinSnapshotIsExplicitAndDoesNotTrackTransientChanges()
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
            onWorkspaceCommand: nil,
            onCommandInput: nil,
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
            onWorkspaceCommand: nil,
            onCommandInput: nil,
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
        workspaceManifest: ShellWorkspaceManifest? = nil
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
            workspaceManifest: workspaceManifest
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
        attention: ShellAttentionState
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
            alanBinding: nil
        )
    }

    private static func context(
        workingDirectoryName: String? = "alan",
        processState: String,
        rendererHealth: String,
        surfaceReadiness: String,
        lastCommandExitCode: Int?
    ) -> ShellContextSnapshot {
        ShellContextSnapshot(
            workingDirectoryName: workingDirectoryName,
            repositoryRoot: nil,
            gitBranch: nil,
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
        activeTaskState: ShellTabActiveTaskState? = nil
    ) -> TerminalPaneMetadataSnapshot {
        TerminalPaneMetadataSnapshot(
            title: title,
            workingDirectory: cwd,
            summary: nil,
            attention: .idle,
            processExited: processExited,
            lastCommandExitCode: nil,
            lastUpdatedAt: Date(timeIntervalSince1970: 3_000),
            activeTaskState: activeTaskState
        )
    }

    private static func childExitMetadata(
        title: String,
        cwd: String = "/Users/morris/Developer/Alan",
        exitCode: Int
    ) -> TerminalPaneMetadataSnapshot {
        TerminalPaneMetadataSnapshot(
            title: title,
            workingDirectory: cwd,
            summary: "process exited",
            attention: .awaitingUser,
            processExited: true,
            lastCommandExitCode: exitCode,
            lastUpdatedAt: Date(timeIntervalSince1970: 3_100),
            activeTaskState: .inactive
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
        pendingYield: Bool
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
