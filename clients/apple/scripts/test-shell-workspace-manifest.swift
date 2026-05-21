import Foundation

@main
struct ShellWorkspaceManifestTestRunner {
    static func main() throws {
        try ShellWorkspaceManifestTests.run()
    }
}

private enum ShellWorkspaceManifestTests {
    private static let referenceDate = Date(timeIntervalSince1970: 1_800_000_000)
    private static let twelveHours: TimeInterval = 12 * 60 * 60

    static func run() throws {
        try verifiesMissingManifestCreatesDefaultWithoutMigratingShellState()
        try verifiesCorruptManifestIsQuarantined()
        try verifiesMaterializerPreservesEmptySelectedSpace()
        try verifiesPinnedSnapshotWinsOverLaterLiveSnapshot()
        try verifiesPinnedSplitSnapshotRestoresSplitTree()
        try verifiesTerminalOnlySnapshotMigratesToContentContainerShape()
        try verifiesContentContainerMigrationPreservesWorkspaceMetadata()
        try verifiesUnpinnedTabPruningUsesTtlAndActiveTask()
        try verifiesSelectedTabPruningCanLeaveSelectedSpaceEmpty()
        print("Shell workspace manifest tests passed.")
    }

    private static func verifiesMissingManifestCreatesDefaultWithoutMigratingShellState() throws {
        let fileManager = FileManager.default
        let tempDirectory = try makeTempDirectory()
        let manifestURL = tempDirectory.appendingPathComponent("shell-workspace-window_main.json")
        let legacyStateURL = tempDirectory.appendingPathComponent("shell-state-window_main.json")
        let legacyState = ShellStateSnapshot.bootstrapDefault(
            windowID: "window_main",
            workingDirectory: "/legacy/project"
        )

        try JSONEncoder().encode(legacyState).write(to: legacyStateURL)

        let store = ShellWorkspaceManifestStore(fileManager: fileManager, manifestURL: manifestURL)
        let result = try store.loadOrCreateDefault(
            windowID: "window_main",
            defaultWorkingDirectory: "/fresh/project",
            now: referenceDate
        )

        expect(result.recovery == .createdDefault, "missing manifest must report default creation")
        expect(fileManager.fileExists(atPath: manifestURL.path), "missing manifest must write a new manifest")

        let tab = try requireOnlyTab(in: result.manifest)
        let pane = try requireOnlyPane(in: tab.liveSnapshot)
        expect(
            pane.cwd == "/fresh/project",
            "default manifest must use the requested working directory"
        )

        let persistedManifestText = try String(contentsOf: manifestURL, encoding: .utf8)
        expect(
            !persistedManifestText.contains("/legacy/project"),
            "workspace manifest startup must not migrate legacy ShellStateSnapshot data"
        )
    }

    private static func verifiesCorruptManifestIsQuarantined() throws {
        let fileManager = FileManager.default
        let tempDirectory = try makeTempDirectory()
        let manifestURL = tempDirectory.appendingPathComponent("shell-workspace-window_main.json")
        try "not json".write(to: manifestURL, atomically: true, encoding: .utf8)

        let store = ShellWorkspaceManifestStore(fileManager: fileManager, manifestURL: manifestURL)
        let result = try store.loadOrCreateDefault(
            windowID: "window_main",
            defaultWorkingDirectory: "/fresh/project",
            now: referenceDate
        )

        guard case .quarantinedCorruptFile(let corruptURL) = result.recovery else {
            throw TestFailure("corrupt manifest must report a quarantine URL")
        }

        expect(fileManager.fileExists(atPath: corruptURL.path), "corrupt manifest must be preserved")
        expect(fileManager.fileExists(atPath: manifestURL.path), "corrupt manifest recovery must write a replacement")
        let quarantinedText = try String(contentsOf: corruptURL, encoding: .utf8)
        expect(
            quarantinedText == "not json",
            "quarantined corrupt file must keep the unreadable payload"
        )
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        _ = try decoder.decode(
            ShellWorkspaceManifest.self,
            from: Data(contentsOf: manifestURL)
        )
    }

    private static func verifiesMaterializerPreservesEmptySelectedSpace() throws {
        let manifest = ShellWorkspaceManifest(
            schemaVersion: ShellWorkspaceManifest.currentSchemaVersion,
            windowID: "window_main",
            selectedSpaceID: "space_empty",
            selectedTabID: nil,
            spaces: [
                ShellWorkspaceSpaceRecord(
                    spaceID: "space_empty",
                    title: "Empty",
                    order: 0,
                    createdAt: referenceDate,
                    updatedAt: referenceDate,
                    tabs: []
                )
            ]
        )

        let state = ShellWorkspaceMaterializer.materialize(
            manifest: manifest,
            defaultWorkingDirectory: "/tmp",
            now: referenceDate
        )

        expect(state.spaces.count == 1, "materializer must preserve empty spaces")
        expect(state.focusedSpaceID == "space_empty", "selected empty space must remain selected")
        expect(state.focusedTabID == nil, "selected empty space must not fabricate a tab selection")
        expect(state.focusedPaneID == nil, "selected empty space must not fabricate a pane selection")
        expect(state.panes.isEmpty, "selected empty space must not fabricate panes")
    }

    private static func verifiesPinnedSnapshotWinsOverLaterLiveSnapshot() throws {
        let tab = makeTab(
            tabID: "tab_pinned",
            title: "Pinned",
            isPinned: true,
            pinCwd: "/pinned/project",
            liveCwd: "/later/project",
            lastActivatedAt: referenceDate,
            lastActivityAt: referenceDate,
            activeTask: .inactive
        )
        let manifest = makeManifest(selectedTabID: tab.tabID, tabs: [tab])

        let state = ShellWorkspaceMaterializer.materialize(
            manifest: manifest,
            defaultWorkingDirectory: "/tmp",
            now: referenceDate
        )

        let pane = try requirePane("pane_tab_pinned", in: state)
        expect(
            pane.cwd == "/pinned/project",
            "pinned restore must use the explicit pin snapshot, not later live cwd"
        )
    }

    private static func verifiesPinnedSplitSnapshotRestoresSplitTree() throws {
        var tab = makeTab(
            tabID: "tab_split",
            title: "Pinned Split",
            isPinned: true,
            pinCwd: nil,
            liveCwd: "/live/single",
            lastActivatedAt: referenceDate,
            lastActivityAt: referenceDate,
            activeTask: .inactive
        )
        tab.pinSnapshot = makeSplitSnapshot(tabID: tab.tabID)
        let manifest = makeManifest(selectedTabID: tab.tabID, tabs: [tab])

        let state = ShellWorkspaceMaterializer.materialize(
            manifest: manifest,
            defaultWorkingDirectory: "/tmp",
            now: referenceDate
        )

        let restoredTab = try requireTab("tab_split", in: state)
        expect(restoredTab.paneTree.paneIDs == ["pane_tab_split_left", "pane_tab_split_right"], "pinned split restore must keep the split pane order")
        expect(state.panes(in: "tab_split").count == 2, "pinned split restore must restore both panes")
        expect(
            state.pane(paneID: "pane_tab_split_left")?.cwd == "/pinned/left",
            "pinned split restore must keep left pane cwd"
        )
        expect(
            state.pane(paneID: "pane_tab_split_right")?.cwd == "/pinned/right",
            "pinned split restore must keep right pane cwd"
        )
    }

    private static func verifiesTerminalOnlySnapshotMigratesToContentContainerShape() throws {
        var tab = makeTab(
            tabID: "tab_split",
            title: "Pinned Split",
            isPinned: true,
            pinCwd: nil,
            liveCwd: "/live/single",
            lastActivatedAt: referenceDate,
            lastActivityAt: referenceDate,
            activeTask: .inactive
        )
        tab.pinSnapshot = makeSplitSnapshot(tabID: tab.tabID)
        let manifest = makeManifest(selectedTabID: tab.tabID, tabs: [tab])

        let migrated = manifest.migratingTerminalRestoreSnapshotsToContentContainers(
            defaultWorkingDirectory: "/fallback"
        )
        let migratedTab = try requireContentTab("tab_split", in: migrated)
        let snapshot = try requireSnapshot(migratedTab.pinSnapshot)

        expect(
            migrated.contentContractVersion == ShellContentStateSnapshot.currentContractVersion,
            "content manifest migration must use the v0.2 content contract"
        )
        expect(
            snapshot.paneTree.paneSlotIDs == ["pane_tab_split_left", "pane_tab_split_right"],
            "content migration must preserve terminal pane IDs as PaneSlot IDs"
        )
        expect(
            snapshot.paneSlots.map(\.paneSlotID) == ["pane_tab_split_left", "pane_tab_split_right"],
            "content migration must create one PaneSlot per terminal restore pane"
        )
        expect(
            snapshot.paneSlots.map(\.contentID) == [
                "content_pane_tab_split_left",
                "content_pane_tab_split_right",
            ],
            "content migration must assign stable ContentInstance IDs"
        )
        expect(
            snapshot.contents.map(\.kind) == [.terminal, .terminal],
            "terminal-only restore panes must migrate to terminal ContentInstances"
        )
        expect(
            snapshot.contents.map(\.title) == ["Shell", "Shell"],
            "terminal ContentInstances must keep user-facing terminal titles"
        )
        expect(
            snapshot.contents.compactMap(\.payload.terminal?.cwd) == [
                "/pinned/left",
                "/pinned/right",
            ],
            "terminal content payloads must preserve per-pane cwd"
        )
        expect(
            snapshot.contents.allSatisfy { $0.payload.markdown == nil && $0.payload.settings == nil },
            "terminal migration must not fabricate non-terminal payloads"
        )
    }

    private static func verifiesContentContainerMigrationPreservesWorkspaceMetadata() throws {
        let activatedAt = referenceDate.addingTimeInterval(-120)
        let activityAt = referenceDate.addingTimeInterval(-30)
        let tab = makeTab(
            tabID: "tab_active",
            title: "Active",
            isPinned: false,
            pinCwd: nil,
            liveCwd: "/fallback",
            lastActivatedAt: activatedAt,
            lastActivityAt: activityAt,
            activeTask: .alanPendingYield
        )
        let manifest = makeManifest(selectedTabID: tab.tabID, tabs: [tab])

        let migrated = manifest.migratingTerminalRestoreSnapshotsToContentContainers(
            defaultWorkingDirectory: "/fallback"
        )
        let migratedSpace = try requireOnlySpace(in: migrated)
        let migratedTab = try requireOnlyContentTab(in: migrated)

        expect(migrated.selectedSpaceID == manifest.selectedSpaceID, "migration must preserve selected Space")
        expect(migrated.selectedTabID == manifest.selectedTabID, "migration must preserve selected Tab")
        expect(migratedSpace.spaceID == "space_main", "migration must preserve Space identity")
        expect(migratedSpace.order == 0, "migration must preserve Space ordering")
        expect(migratedTab.tabID == tab.tabID, "migration must preserve Tab identity")
        expect(migratedTab.isPinned == tab.isPinned, "migration must preserve pin state")
        expect(
            migratedTab.lastActivatedAt == activatedAt && migratedTab.lastActivityAt == activityAt,
            "migration must preserve TTL anchor timestamps"
        )
        expect(
            migratedTab.activeTask == .alanPendingYield,
            "migration must preserve active-task metadata"
        )
        let snapshot = try requireSnapshot(migratedTab.liveSnapshot)
        expect(
            snapshot.contents.first?.payload.terminal?.cwd == "/fallback",
            "migration must preserve terminal restore payload cwd"
        )
    }

    private static func verifiesUnpinnedTabPruningUsesTtlAndActiveTask() throws {
        let expiredAt = referenceDate.addingTimeInterval(-(twelveHours + 60))
        let recentAt = referenceDate.addingTimeInterval(-60)
        let expiredInactive = makeTab(
            tabID: "tab_expired",
            title: "Expired",
            isPinned: false,
            pinCwd: nil,
            liveCwd: "/expired",
            lastActivatedAt: expiredAt,
            lastActivityAt: expiredAt,
            activeTask: .inactive
        )
        let expiredActive = makeTab(
            tabID: "tab_active",
            title: "Active",
            isPinned: false,
            pinCwd: nil,
            liveCwd: "/active",
            lastActivatedAt: expiredAt,
            lastActivityAt: expiredAt,
            activeTask: .foregroundCommand
        )
        let recentInactive = makeTab(
            tabID: "tab_recent",
            title: "Recent",
            isPinned: false,
            pinCwd: nil,
            liveCwd: "/recent",
            lastActivatedAt: recentAt,
            lastActivityAt: recentAt,
            activeTask: .inactive
        )
        let manifest = makeManifest(
            selectedTabID: expiredInactive.tabID,
            tabs: [expiredInactive, expiredActive, recentInactive]
        )

        let pruned = manifest.pruningExpiredTabs(now: referenceDate, ttl: twelveHours)

        expect(findTab("tab_expired", in: pruned) == nil, "expired inactive unpinned tab must be pruned")
        expect(findTab("tab_active", in: pruned) != nil, "active unpinned tab must survive TTL pruning")
        expect(findTab("tab_recent", in: pruned) != nil, "recent unpinned tab must survive TTL pruning")
        expect(pruned.selectedTabID == "tab_active", "selected pruned tab must repair to first retained tab")
    }

    private static func verifiesSelectedTabPruningCanLeaveSelectedSpaceEmpty() throws {
        let expiredAt = referenceDate.addingTimeInterval(-(twelveHours + 60))
        let expiredInactive = makeTab(
            tabID: "tab_expired",
            title: "Expired",
            isPinned: false,
            pinCwd: nil,
            liveCwd: "/expired",
            lastActivatedAt: expiredAt,
            lastActivityAt: expiredAt,
            activeTask: .inactive
        )
        let manifest = makeManifest(selectedTabID: expiredInactive.tabID, tabs: [expiredInactive])

        let pruned = manifest.pruningExpiredTabs(now: referenceDate, ttl: twelveHours)
        let state = ShellWorkspaceMaterializer.materialize(
            manifest: pruned,
            defaultWorkingDirectory: "/tmp",
            now: referenceDate
        )

        expect(pruned.spaces.first?.tabs.isEmpty == true, "pruning must keep the selected space even when empty")
        expect(pruned.selectedSpaceID == "space_main", "pruning must preserve selected space")
        expect(pruned.selectedTabID == nil, "pruning all tabs in a selected space must clear selected tab")
        expect(state.focusedSpaceID == "space_main", "materializer must keep the empty selected space focused")
        expect(state.focusedTabID == nil, "materializer must keep empty selected space tabless")
    }

    private static func makeManifest(
        selectedTabID: String?,
        tabs: [ShellWorkspaceTabRecord]
    ) -> ShellWorkspaceManifest {
        ShellWorkspaceManifest(
            schemaVersion: ShellWorkspaceManifest.currentSchemaVersion,
            windowID: "window_main",
            selectedSpaceID: "space_main",
            selectedTabID: selectedTabID,
            spaces: [
                ShellWorkspaceSpaceRecord(
                    spaceID: "space_main",
                    title: "Main",
                    order: 0,
                    createdAt: referenceDate,
                    updatedAt: referenceDate,
                    tabs: tabs
                )
            ]
        )
    }

    private static func makeTab(
        tabID: String,
        title: String,
        isPinned: Bool,
        pinCwd: String?,
        liveCwd: String?,
        lastActivatedAt: Date,
        lastActivityAt: Date,
        activeTask: ShellTabActiveTaskState
    ) -> ShellWorkspaceTabRecord {
        ShellWorkspaceTabRecord(
            tabID: tabID,
            title: title,
            kind: .terminal,
            createdAt: referenceDate,
            lastActivatedAt: lastActivatedAt,
            lastActivityAt: lastActivityAt,
            isPinned: isPinned,
            pinSnapshot: pinCwd.map { makeSnapshot(tabID: tabID, cwd: $0) },
            liveSnapshot: liveCwd.map { makeSnapshot(tabID: tabID, cwd: $0) },
            activeTask: activeTask
        )
    }

    private static func makeSnapshot(tabID: String, cwd: String) -> ShellTabRestoreSnapshot {
        let paneID = "pane_\(tabID)"
        return ShellTabRestoreSnapshot(
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
                    title: nil
                )
            ]
        )
    }

    private static func makeSplitSnapshot(tabID: String) -> ShellTabRestoreSnapshot {
        let leftPaneID = "pane_\(tabID)_left"
        let rightPaneID = "pane_\(tabID)_right"
        return ShellTabRestoreSnapshot(
            paneTree: ShellPaneTreeNode(
                nodeID: "node_\(tabID)_split",
                kind: .split,
                direction: .vertical,
                ratio: 0.5,
                paneID: nil,
                children: [
                    ShellPaneTreeNode(
                        nodeID: "node_\(leftPaneID)",
                        kind: .pane,
                        direction: nil,
                        paneID: leftPaneID,
                        children: nil
                    ),
                    ShellPaneTreeNode(
                        nodeID: "node_\(rightPaneID)",
                        kind: .pane,
                        direction: nil,
                        paneID: rightPaneID,
                        children: nil
                    ),
                ]
            ),
            panes: [
                ShellPaneRestoreRecord(
                    paneID: leftPaneID,
                    launchTarget: .shell,
                    cwd: "/pinned/left",
                    title: nil
                ),
                ShellPaneRestoreRecord(
                    paneID: rightPaneID,
                    launchTarget: .shell,
                    cwd: "/pinned/right",
                    title: nil
                ),
            ]
        )
    }

    private static func findTab(
        _ tabID: String,
        in manifest: ShellWorkspaceManifest
    ) -> ShellWorkspaceTabRecord? {
        manifest.spaces.flatMap(\.tabs).first { $0.tabID == tabID }
    }

    private static func requireOnlyTab(in manifest: ShellWorkspaceManifest) throws -> ShellWorkspaceTabRecord {
        let tabs = manifest.spaces.flatMap(\.tabs)
        guard tabs.count == 1, let tab = tabs.first else {
            throw TestFailure("expected exactly one tab")
        }
        return tab
    }

    private static func requireOnlySpace(
        in manifest: ShellContentWorkspaceManifest
    ) throws -> ShellContentWorkspaceSpaceRecord {
        guard manifest.spaces.count == 1, let space = manifest.spaces.first else {
            throw TestFailure("expected exactly one content space")
        }
        return space
    }

    private static func requireOnlyContentTab(
        in manifest: ShellContentWorkspaceManifest
    ) throws -> ShellContentWorkspaceTabRecord {
        let tabs = manifest.spaces.flatMap(\.tabs)
        guard tabs.count == 1, let tab = tabs.first else {
            throw TestFailure("expected exactly one content tab")
        }
        return tab
    }

    private static func requireContentTab(
        _ tabID: String,
        in manifest: ShellContentWorkspaceManifest
    ) throws -> ShellContentWorkspaceTabRecord {
        guard let tab = manifest.spaces.flatMap(\.tabs).first(where: { $0.tabID == tabID }) else {
            throw TestFailure("missing content tab \(tabID)")
        }
        return tab
    }

    private static func requireSnapshot(
        _ snapshot: ShellContentTabRestoreSnapshot?
    ) throws -> ShellContentTabRestoreSnapshot {
        guard let snapshot else {
            throw TestFailure("expected content restore snapshot")
        }
        return snapshot
    }

    private static func requireOnlyPane(in snapshot: ShellTabRestoreSnapshot?) throws -> ShellPaneRestoreRecord {
        guard let snapshot, snapshot.panes.count == 1, let pane = snapshot.panes.first else {
            throw TestFailure("expected exactly one restore pane")
        }
        return pane
    }

    private static func requirePane(_ paneID: String, in state: ShellStateSnapshot) throws -> ShellPane {
        guard let pane = state.pane(paneID: paneID) else {
            throw TestFailure("missing pane \(paneID)")
        }
        return pane
    }

    private static func requireTab(_ tabID: String, in state: ShellStateSnapshot) throws -> ShellTab {
        guard let tab = state.tab(tabID: tabID) else {
            throw TestFailure("missing tab \(tabID)")
        }
        return tab
    }

    private static func makeTempDirectory() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("alan-shell-workspace-manifest-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: String
    ) {
        guard condition() else {
            fputs("error: \(message)\n", stderr)
            exit(1)
        }
    }
}

private struct TestFailure: Error, CustomStringConvertible {
    let description: String

    init(_ description: String) {
        self.description = description
    }
}
