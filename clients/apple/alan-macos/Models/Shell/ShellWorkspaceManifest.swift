import Foundation

struct ShellWorkspaceManifest: Codable, Equatable {
    static let currentSchemaVersion = 1

    var schemaVersion: Int
    var windowID: String
    var selectedSpaceID: String?
    var selectedTabID: String?
    var spaces: [ShellWorkspaceSpaceRecord]

    private enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case windowID = "window_id"
        case selectedSpaceID = "selected_space_id"
        case selectedTabID = "selected_tab_id"
        case spaces
    }

    static func defaultManifest(
        windowID: String,
        defaultWorkingDirectory: String,
        now: Date
    ) -> ShellWorkspaceManifest {
        let spaceID = "space_main"
        let tabID = "tab_main"
        let paneID = "pane_1"
        let snapshot = ShellTabRestoreSnapshot(
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
                    cwd: defaultWorkingDirectory,
                    title: "Shell"
                )
            ]
        )

        return ShellWorkspaceManifest(
            schemaVersion: currentSchemaVersion,
            windowID: windowID,
            selectedSpaceID: spaceID,
            selectedTabID: tabID,
            spaces: [
                ShellWorkspaceSpaceRecord(
                    spaceID: spaceID,
                    title: "Terminal",
                    order: 0,
                    createdAt: now,
                    updatedAt: now,
                    tabs: [
                        ShellWorkspaceTabRecord(
                            tabID: tabID,
                            title: "Shell",
                            kind: .terminal,
                            createdAt: now,
                            lastActivatedAt: now,
                            lastActivityAt: now,
                            isPinned: false,
                            pinSnapshot: nil,
                            liveSnapshot: snapshot,
                            activeTask: .inactive
                        )
                    ]
                )
            ]
        )
    }
}

struct ShellWorkspaceSpaceRecord: Codable, Equatable, Identifiable {
    var spaceID: String
    var title: String
    var order: Int
    var createdAt: Date
    var updatedAt: Date
    var tabs: [ShellWorkspaceTabRecord]

    var id: String { spaceID }

    private enum CodingKeys: String, CodingKey {
        case spaceID = "space_id"
        case title
        case order
        case createdAt = "created_at"
        case updatedAt = "updated_at"
        case tabs
    }
}

struct ShellWorkspaceTabRecord: Codable, Equatable, Identifiable {
    var tabID: String
    var title: String?
    var kind: ShellTabKind
    var createdAt: Date
    var lastActivatedAt: Date
    var lastActivityAt: Date
    var isPinned: Bool
    var pinSnapshot: ShellTabRestoreSnapshot?
    var liveSnapshot: ShellTabRestoreSnapshot?
    var activeTask: ShellTabActiveTaskState

    var id: String { tabID }

    private enum CodingKeys: String, CodingKey {
        case tabID = "tab_id"
        case title
        case kind
        case createdAt = "created_at"
        case lastActivatedAt = "last_activated_at"
        case lastActivityAt = "last_activity_at"
        case isPinned = "is_pinned"
        case pinSnapshot = "pin_snapshot"
        case liveSnapshot = "live_snapshot"
        case activeTask = "active_task"
    }

    func restoreSnapshot(defaultWorkingDirectory: String) -> ShellTabRestoreSnapshot {
        if isPinned, let pinSnapshot {
            return pinSnapshot
        }

        if let liveSnapshot {
            return liveSnapshot
        }

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
                    cwd: defaultWorkingDirectory,
                    title: title
                )
            ]
        )
    }

    func shouldRetain(now: Date, ttl: TimeInterval) -> Bool {
        if isPinned {
            return true
        }

        if activeTask.protectsFromPruning {
            return true
        }

        return now.timeIntervalSince(max(lastActivatedAt, lastActivityAt)) <= ttl
    }
}

struct ShellTabRestoreSnapshot: Codable, Equatable {
    var paneTree: ShellPaneTreeNode
    var panes: [ShellPaneRestoreRecord]

    private enum CodingKeys: String, CodingKey {
        case paneTree = "pane_tree"
        case panes
    }
}

struct ShellPaneRestoreRecord: Codable, Equatable, Identifiable {
    var paneID: String
    var launchTarget: ShellLaunchTarget
    var cwd: String?
    var title: String?

    var id: String { paneID }

    private enum CodingKeys: String, CodingKey {
        case paneID = "pane_id"
        case launchTarget = "launch_target"
        case cwd
        case title
    }
}

extension ShellWorkspaceManifest {
    func pruningExpiredTabs(now: Date, ttl: TimeInterval) -> ShellWorkspaceManifest {
        var pruned = self
        pruned.spaces = spaces.map { space in
            var space = space
            space.tabs = space.tabs.filter { $0.shouldRetain(now: now, ttl: ttl) }
            space.updatedAt = now
            return space
        }
        pruned.repairSelection()
        return pruned
    }

    mutating func repairSelection() {
        guard !spaces.isEmpty else {
            selectedSpaceID = nil
            selectedTabID = nil
            return
        }

        if selectedSpaceID == nil || !spaces.contains(where: { $0.spaceID == selectedSpaceID }) {
            selectedSpaceID = spaces.first?.spaceID
        }

        guard let selectedSpaceID,
              let selectedSpace = spaces.first(where: { $0.spaceID == selectedSpaceID })
        else {
            selectedTabID = nil
            return
        }

        let selectedTabStillExists = selectedTabID.map { selectedTabID in
            selectedSpace.tabs.contains { $0.tabID == selectedTabID }
        } ?? false

        if !selectedTabStillExists {
            selectedTabID = selectedSpace.tabs.first?.tabID
        }
    }
}

struct ShellWorkspaceMaterializer {
    static func materialize(
        manifest: ShellWorkspaceManifest,
        defaultWorkingDirectory: String,
        now: Date
    ) -> ShellStateSnapshot {
        var repairedManifest = manifest
        repairedManifest.repairSelection()

        let sourceManifest = repairedManifest.spaces.isEmpty
            ? ShellWorkspaceManifest.defaultManifest(
                windowID: manifest.windowID,
                defaultWorkingDirectory: defaultWorkingDirectory,
                now: now
            )
            : repairedManifest

        let spaces = sourceManifest.spaces.sorted { lhs, rhs in
            if lhs.order == rhs.order {
                return lhs.spaceID < rhs.spaceID
            }
            return lhs.order < rhs.order
        }

        var shellSpaces: [ShellSpace] = []
        var panes: [ShellPane] = []

        for space in spaces {
            let shellTabs = organizedTabs(space.tabs).map { tabRecord -> ShellTab in
                let restoreSnapshot = tabRecord.restoreSnapshot(
                    defaultWorkingDirectory: defaultWorkingDirectory
                )

                panes.append(
                    contentsOf: restoreSnapshot.panes.map { paneRecord in
                        makePane(
                            record: paneRecord,
                            tabID: tabRecord.tabID,
                            spaceID: space.spaceID,
                            selectedTabID: sourceManifest.selectedTabID,
                            defaultWorkingDirectory: defaultWorkingDirectory
                        )
                    }
                )

                return ShellTab(
                    tabID: tabRecord.tabID,
                    kind: tabRecord.kind,
                    title: tabRecord.title,
                    paneTree: restoreSnapshot.paneTree,
                    isPinned: tabRecord.isPinned
                )
            }

            shellSpaces.append(
                ShellSpace(
                    spaceID: space.spaceID,
                    title: space.title,
                    attention: strongestAttention(for: shellTabs, panes: panes),
                    tabs: shellTabs
                )
            )
        }

        let focusedSpaceID = sourceManifest.selectedSpaceID
        let focusedTabID = focusedSpaceID.flatMap { spaceID in
            let selectedSpace = shellSpaces.first { $0.spaceID == spaceID }
            if let selectedTabID = sourceManifest.selectedTabID,
               selectedSpace?.tabs.contains(where: { $0.tabID == selectedTabID }) == true
            {
                return selectedTabID
            }
            return selectedSpace?.tabs.first?.tabID
        }
        let focusedPaneID = focusedTabID
            .flatMap { tabID in shellSpaces.lazy.flatMap(\.tabs).first { $0.tabID == tabID } }
            .flatMap { $0.paneTree.paneIDs.first }

        return ShellStateSnapshot(
            contractVersion: "0.1",
            windowID: sourceManifest.windowID,
            focusedSpaceID: focusedSpaceID,
            focusedTabID: focusedTabID,
            focusedPaneID: focusedPaneID,
            spaces: shellSpaces,
            panes: panes
        )
    }

    private static func makePane(
        record: ShellPaneRestoreRecord,
        tabID: String,
        spaceID: String,
        selectedTabID: String?,
        defaultWorkingDirectory: String
    ) -> ShellPane {
        let launchTarget = record.launchTarget
        let title = record.title ?? defaultViewportTitle(for: launchTarget)
        return ShellPane(
            paneID: record.paneID,
            tabID: tabID,
            spaceID: spaceID,
            launchTarget: launchTarget,
            cwd: record.cwd ?? defaultWorkingDirectory,
            process: nil,
            attention: tabID == selectedTabID ? .active : .idle,
            context: nil,
            viewport: ShellViewportSnapshot(
                title: title,
                summary: nil,
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            alanBinding: nil
        )
    }

    private static func organizedTabs(
        _ tabs: [ShellWorkspaceTabRecord]
    ) -> [ShellWorkspaceTabRecord] {
        tabs.filter(\.isPinned) + tabs.filter { !$0.isPinned }
    }

    private static func strongestAttention(
        for tabs: [ShellTab],
        panes: [ShellPane]
    ) -> ShellAttentionState {
        let paneIDs = Set(tabs.flatMap(\.paneTree.paneIDs))
        return panes
            .filter { paneIDs.contains($0.paneID) }
            .map(\.attention)
            .max { attentionRank(for: $0) < attentionRank(for: $1) }
            ?? .idle
    }

    private static func attentionRank(for attention: ShellAttentionState) -> Int {
        switch attention {
        case .idle:
            return 0
        case .active:
            return 1
        case .notable:
            return 2
        case .awaitingUser:
            return 3
        }
    }

    private static func defaultViewportTitle(for launchTarget: ShellLaunchTarget) -> String {
        switch launchTarget {
        case .shell:
            return "Shell"
        case .alan:
            return "alan"
        }
    }
}
