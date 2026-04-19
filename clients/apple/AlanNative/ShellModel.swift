import Foundation

enum ShellAttentionState: String, Codable, CaseIterable {
    case idle
    case active
    case awaitingUser = "awaiting_user"
    case notable
}

enum ShellTabKind: String, Codable, CaseIterable {
    case terminal
    case scratch
    case log
}

enum ShellPaneTreeKind: String, Codable {
    case split
    case pane
}

enum ShellSplitDirection: String, Codable {
    case horizontal
    case vertical
}

enum ShellLaunchTarget: String, Codable, CaseIterable {
    case shell
    case alan
}

struct ShellProcessBinding: Codable, Equatable {
    let program: String
    let argvPreview: [String]?

    private enum CodingKeys: String, CodingKey {
        case program
        case argvPreview = "argv_preview"
    }
}

struct ShellContextSnapshot: Codable, Equatable {
    let workingDirectoryName: String?
    let repositoryRoot: String?
    let gitBranch: String?
    let controlPath: String?
    let socketPath: String?
    let alanBindingFile: String?
    let launchCommand: String?
    let launchStrategy: String?
    let shellIntegrationSource: String?
    let processState: String?
    let rendererPhase: String?
    let displayName: String?
    let displayID: String?
    let windowTitle: String?
    let lastMetadataAt: String?
    let lastCommandExitCode: Int?

    init(
        workingDirectoryName: String?,
        repositoryRoot: String?,
        gitBranch: String?,
        controlPath: String?,
        socketPath: String? = nil,
        alanBindingFile: String?,
        launchCommand: String? = nil,
        launchStrategy: String?,
        shellIntegrationSource: String?,
        processState: String?,
        rendererPhase: String? = nil,
        displayName: String? = nil,
        displayID: String? = nil,
        windowTitle: String? = nil,
        lastMetadataAt: String?,
        lastCommandExitCode: Int?
    ) {
        self.workingDirectoryName = workingDirectoryName
        self.repositoryRoot = repositoryRoot
        self.gitBranch = gitBranch
        self.controlPath = controlPath
        self.socketPath = socketPath
        self.alanBindingFile = alanBindingFile
        self.launchCommand = launchCommand
        self.launchStrategy = launchStrategy
        self.shellIntegrationSource = shellIntegrationSource
        self.processState = processState
        self.rendererPhase = rendererPhase
        self.displayName = displayName
        self.displayID = displayID
        self.windowTitle = windowTitle
        self.lastMetadataAt = lastMetadataAt
        self.lastCommandExitCode = lastCommandExitCode
    }

    private enum CodingKeys: String, CodingKey {
        case workingDirectoryName = "working_directory_name"
        case repositoryRoot = "repository_root"
        case gitBranch = "git_branch"
        case controlPath = "control_path"
        case socketPath = "socket_path"
        case alanBindingFile = "alan_binding_file"
        case launchCommand = "launch_command"
        case launchStrategy = "launch_strategy"
        case shellIntegrationSource = "shell_integration_source"
        case processState = "process_state"
        case rendererPhase = "renderer_phase"
        case displayName = "display_name"
        case displayID = "display_id"
        case windowTitle = "window_title"
        case lastMetadataAt = "last_metadata_at"
        case lastCommandExitCode = "last_command_exit_code"
    }
}

struct ShellViewportSnapshot: Codable, Equatable {
    let title: String?
    let summary: String?
    let visibleExcerpt: String?
    let lastActivityAt: String?

    private enum CodingKeys: String, CodingKey {
        case title
        case summary
        case visibleExcerpt = "visible_excerpt"
        case lastActivityAt = "last_activity_at"
    }
}

struct ShellAlanBinding: Codable, Equatable {
    let sessionID: String
    let runStatus: String
    let pendingYield: Bool
    let source: String?
    let lastProjectedAt: String?

    private enum CodingKeys: String, CodingKey {
        case sessionID = "session_id"
        case runStatus = "run_status"
        case pendingYield = "pending_yield"
        case source
        case lastProjectedAt = "last_projected_at"
    }
}

struct ShellPane: Identifiable, Codable, Equatable {
    let paneID: String
    let tabID: String
    let spaceID: String
    let launchTarget: ShellLaunchTarget?
    let cwd: String?
    let process: ShellProcessBinding?
    let attention: ShellAttentionState
    let context: ShellContextSnapshot?
    let viewport: ShellViewportSnapshot?
    let alanBinding: ShellAlanBinding?

    var id: String { paneID }

    private enum CodingKeys: String, CodingKey {
        case paneID = "pane_id"
        case tabID = "tab_id"
        case spaceID = "space_id"
        case launchTarget = "launch_target"
        case cwd
        case process
        case attention
        case context
        case viewport
        case alanBinding = "alan_binding"
    }
}

extension ShellPane {
    // Preserve window-state compatibility while converging persisted naming on tabs.
    private enum LegacyCodingKeys: String, CodingKey {
        case surfaceID = "surface_id"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let legacyContainer = try decoder.container(keyedBy: LegacyCodingKeys.self)

        let tabID: String
        if let decodedTabID = try container.decodeIfPresent(String.self, forKey: .tabID) {
            tabID = decodedTabID
        } else {
            tabID = try legacyContainer.decode(String.self, forKey: .surfaceID)
        }

        self.init(
            paneID: try container.decode(String.self, forKey: .paneID),
            tabID: tabID,
            spaceID: try container.decode(String.self, forKey: .spaceID),
            launchTarget: try container.decodeIfPresent(ShellLaunchTarget.self, forKey: .launchTarget),
            cwd: try container.decodeIfPresent(String.self, forKey: .cwd),
            process: try container.decodeIfPresent(ShellProcessBinding.self, forKey: .process),
            attention: try container.decode(ShellAttentionState.self, forKey: .attention),
            context: try container.decodeIfPresent(ShellContextSnapshot.self, forKey: .context),
            viewport: try container.decodeIfPresent(ShellViewportSnapshot.self, forKey: .viewport),
            alanBinding: try container.decodeIfPresent(ShellAlanBinding.self, forKey: .alanBinding)
        )
    }

    var resolvedLaunchTarget: ShellLaunchTarget {
        if let launchTarget {
            return launchTarget
        }

        if let processProgram = process?.program.lowercased(),
           processProgram.contains("alan")
        {
            return .alan
        }

        if process?.argvPreview?.contains(where: { $0.lowercased().contains("alan") }) == true {
            return .alan
        }

        if alanBinding != nil {
            return .alan
        }

        return .shell
    }
}

struct ShellPaneTreeNode: Identifiable, Codable, Equatable {
    let nodeID: String
    let kind: ShellPaneTreeKind
    let direction: ShellSplitDirection?
    let paneID: String?
    let children: [ShellPaneTreeNode]?

    var id: String { nodeID }

    private enum CodingKeys: String, CodingKey {
        case nodeID = "node_id"
        case kind
        case direction
        case paneID = "pane_id"
        case children
    }
}

extension ShellPaneTreeNode {
    var nodeIDs: [String] {
        [nodeID] + (children ?? []).flatMap(\.nodeIDs)
    }

    var paneIDs: [String] {
        switch kind {
        case .pane:
            return paneID.map { [$0] } ?? []
        case .split:
            return (children ?? []).flatMap(\.paneIDs)
        }
    }

    func contains(paneID targetPaneID: String) -> Bool {
        switch kind {
        case .pane:
            return paneID == targetPaneID
        case .split:
            return (children ?? []).contains { $0.contains(paneID: targetPaneID) }
        }
    }

    func splittingPane(
        _ targetPaneID: String,
        direction: ShellSplitDirection,
        splitNodeID: String,
        newLeafNodeID: String,
        newPaneID: String
    ) -> ShellPaneTreeNode {
        switch kind {
        case .pane:
            guard paneID == targetPaneID else { return self }
            let currentLeaf = ShellPaneTreeNode(
                nodeID: nodeID,
                kind: .pane,
                direction: nil,
                paneID: targetPaneID,
                children: nil
            )
            let newLeaf = ShellPaneTreeNode(
                nodeID: newLeafNodeID,
                kind: .pane,
                direction: nil,
                paneID: newPaneID,
                children: nil
            )
            return ShellPaneTreeNode(
                nodeID: splitNodeID,
                kind: .split,
                direction: direction,
                paneID: nil,
                children: [currentLeaf, newLeaf]
            )
        case .split:
            return ShellPaneTreeNode(
                nodeID: nodeID,
                kind: .split,
                direction: self.direction,
                paneID: nil,
                children: (children ?? []).map {
                    $0.splittingPane(
                        targetPaneID,
                        direction: direction,
                        splitNodeID: splitNodeID,
                        newLeafNodeID: newLeafNodeID,
                        newPaneID: newPaneID
                    )
                }
            )
        }
    }

    func removingPane(_ targetPaneID: String) -> ShellPaneTreeNode? {
        switch kind {
        case .pane:
            return paneID == targetPaneID ? nil : self
        case .split:
            let remainingChildren = (children ?? []).compactMap { $0.removingPane(targetPaneID) }
            if remainingChildren.isEmpty {
                return nil
            }
            if remainingChildren.count == 1 {
                return remainingChildren[0]
            }
            return ShellPaneTreeNode(
                nodeID: nodeID,
                kind: .split,
                direction: direction,
                paneID: nil,
                children: remainingChildren
            )
        }
    }

    func attachingPane(
        _ newPaneID: String,
        direction: ShellSplitDirection,
        splitNodeID: String,
        newLeafNodeID: String
    ) -> ShellPaneTreeNode {
        let newLeaf = ShellPaneTreeNode(
            nodeID: newLeafNodeID,
            kind: .pane,
            direction: nil,
            paneID: newPaneID,
            children: nil
        )

        if kind == .split,
           self.direction == direction {
            return ShellPaneTreeNode(
                nodeID: nodeID,
                kind: .split,
                direction: direction,
                paneID: nil,
                children: (children ?? []) + [newLeaf]
            )
        }

        return ShellPaneTreeNode(
            nodeID: splitNodeID,
            kind: .split,
            direction: direction,
            paneID: nil,
            children: [self, newLeaf]
        )
    }
}

enum ShellStateMutationError: String, Error {
    case spaceNotFound = "space_not_found"
    case tabNotFound = "tab_not_found"
    case paneNotFound = "pane_not_found"
    case lastTab = "last_tab"
    case lastPane = "last_pane"
    case invalidMoveTarget = "invalid_move_target"
}

struct ShellStateMutationResult {
    let state: ShellStateSnapshot
    let spaceID: String?
    let tabID: String?
    let paneID: String?
}

struct ShellTab: Identifiable, Codable, Equatable {
    let tabID: String
    let kind: ShellTabKind
    let title: String?
    let paneTree: ShellPaneTreeNode

    var id: String { tabID }

    private enum CodingKeys: String, CodingKey {
        case tabID = "tab_id"
        case kind
        case title
        case paneTree = "pane_tree"
    }
}

struct ShellSpace: Identifiable, Codable, Equatable {
    let spaceID: String
    let title: String
    let attention: ShellAttentionState
    let tabs: [ShellTab]

    var id: String { spaceID }

    private enum CodingKeys: String, CodingKey {
        case spaceID = "space_id"
        case title
        case attention
        case tabs
    }
}

struct ShellStateSnapshot: Codable, Equatable {
    let contractVersion: String
    let windowID: String
    let focusedSpaceID: String?
    let focusedTabID: String?
    let focusedPaneID: String?
    let spaces: [ShellSpace]
    let panes: [ShellPane]

    private enum CodingKeys: String, CodingKey {
        case contractVersion = "contract_version"
        case windowID = "window_id"
        case focusedSpaceID = "focused_space_id"
        case focusedTabID = "focused_tab_id"
        case focusedPaneID = "focused_pane_id"
        case spaces
        case panes
    }

    var prettyPrintedJSON: String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]

        guard let data = try? encoder.encode(self),
              let string = String(data: data, encoding: .utf8)
        else {
            return "{\n  \"error\": \"failed to encode shell snapshot\"\n}"
        }

        return string
    }
}

extension ShellTab {
    private enum LegacyCodingKeys: String, CodingKey {
        case surfaceID = "surface_id"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let legacyContainer = try decoder.container(keyedBy: LegacyCodingKeys.self)

        let tabID: String
        if let decodedTabID = try container.decodeIfPresent(String.self, forKey: .tabID) {
            tabID = decodedTabID
        } else {
            tabID = try legacyContainer.decode(String.self, forKey: .surfaceID)
        }

        self.init(
            tabID: tabID,
            kind: try container.decode(ShellTabKind.self, forKey: .kind),
            title: try container.decodeIfPresent(String.self, forKey: .title),
            paneTree: try container.decode(ShellPaneTreeNode.self, forKey: .paneTree)
        )
    }

    func contains(paneID: String) -> Bool {
        paneTree.contains(paneID: paneID)
    }
}

extension ShellSpace {
    private enum LegacyCodingKeys: String, CodingKey {
        case surfaces
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let legacyContainer = try decoder.container(keyedBy: LegacyCodingKeys.self)

        let tabs: [ShellTab]
        if let decodedTabs = try container.decodeIfPresent([ShellTab].self, forKey: .tabs) {
            tabs = decodedTabs
        } else {
            tabs = try legacyContainer.decode([ShellTab].self, forKey: .surfaces)
        }

        self.init(
            spaceID: try container.decode(String.self, forKey: .spaceID),
            title: try container.decode(String.self, forKey: .title),
            attention: try container.decode(ShellAttentionState.self, forKey: .attention),
            tabs: tabs
        )
    }
}

extension ShellStateSnapshot {
    private enum LegacyCodingKeys: String, CodingKey {
        case focusedSurfaceID = "focused_surface_id"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let legacyContainer = try decoder.container(keyedBy: LegacyCodingKeys.self)

        let focusedTabID: String?
        if let decodedTabID = try container.decodeIfPresent(String.self, forKey: .focusedTabID) {
            focusedTabID = decodedTabID
        } else {
            focusedTabID = try legacyContainer.decodeIfPresent(String.self, forKey: .focusedSurfaceID)
        }

        self.init(
            contractVersion: try container.decode(String.self, forKey: .contractVersion),
            windowID: try container.decode(String.self, forKey: .windowID),
            focusedSpaceID: try container.decodeIfPresent(String.self, forKey: .focusedSpaceID),
            focusedTabID: focusedTabID,
            focusedPaneID: try container.decodeIfPresent(String.self, forKey: .focusedPaneID),
            spaces: try container.decode([ShellSpace].self, forKey: .spaces),
            panes: try container.decode([ShellPane].self, forKey: .panes)
        )
    }
}

extension ShellStateSnapshot {
    static func bootstrapDefault(
        windowID: String = "window_main",
        workingDirectory: String = FileManager.default.currentDirectoryPath
    ) -> ShellStateSnapshot {
        let spaceID = "space_main"
        let tabID = "tab_main"
        let paneID = "pane_1"

        return ShellStateSnapshot(
            contractVersion: "0.1",
            windowID: windowID,
            focusedSpaceID: spaceID,
            focusedTabID: tabID,
            focusedPaneID: paneID,
            spaces: [
                ShellSpace(
                    spaceID: spaceID,
                    title: "Terminal",
                    attention: .active,
                    tabs: [
                        ShellTab(
                            tabID: tabID,
                            kind: .terminal,
                            title: "Shell",
                            paneTree: ShellPaneTreeNode(
                                nodeID: "node_\(paneID)",
                                kind: .pane,
                                direction: nil,
                                paneID: paneID,
                                children: nil
                            )
                        )
                    ]
                )
            ],
            panes: [
                ShellPane(
                    paneID: paneID,
                    tabID: tabID,
                    spaceID: spaceID,
                    launchTarget: .shell,
                    cwd: workingDirectory,
                    process: Self.defaultProcessBinding(for: .shell),
                    attention: .active,
                    context: nil,
                    viewport: ShellViewportSnapshot(
                        title: Self.defaultViewportTitle(for: .shell),
                        summary: "ready to launch login shell",
                        visibleExcerpt: nil,
                        lastActivityAt: nil
                    ),
                    alanBinding: nil
                )
            ]
        )
    }

    var totalTabCount: Int {
        spaces.reduce(into: 0) { partialResult, space in
            partialResult += space.tabs.count
        }
    }

    func space(spaceID: String) -> ShellSpace? {
        spaces.first { $0.spaceID == spaceID }
    }

    func tab(tabID: String) -> ShellTab? {
        spaces.lazy.flatMap(\.tabs).first { $0.tabID == tabID }
    }

    func pane(paneID: String) -> ShellPane? {
        panes.first { $0.paneID == paneID }
    }

    func tabs(in spaceID: String?) -> [ShellTab] {
        guard let spaceID else {
            return spaces.flatMap(\.tabs)
        }
        return space(spaceID: spaceID)?.tabs ?? []
    }

    func panes(in tabID: String?) -> [ShellPane] {
        guard let tabID else {
            return panes
        }
        return panes.filter { $0.tabID == tabID }
    }

    func focusingPane(_ paneID: String) throws -> ShellStateMutationResult {
        guard pane(paneID: paneID) != nil else {
            throw ShellStateMutationError.paneNotFound
        }

        return ShellStateMutationResult(
            state: replacing(
                spaces: spaces,
                panes: panes,
                focusedPaneID: paneID
            ),
            spaceID: pane(paneID: paneID)?.spaceID,
            tabID: pane(paneID: paneID)?.tabID,
            paneID: paneID
        )
    }

    func creatingSpace(
        launchTarget: ShellLaunchTarget,
        title: String?,
        workingDirectory: String?,
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath,
        now: Date = .now
    ) -> ShellStateMutationResult {
        let spaceIndex = spaces.count + 1
        let spaceID = nextID(prefix: "space", existing: spaces.map(\.spaceID))
        let tabID = nextID(prefix: "tab", existing: spaces.flatMap { $0.tabs.map(\.tabID) })
        let paneID = nextID(prefix: "pane", existing: panes.map(\.paneID))
        let pane = makeTerminalPane(
            paneID: paneID,
            tabID: tabID,
            spaceID: spaceID,
            launchTarget: launchTarget,
            workingDirectory: workingDirectory ?? defaultWorkingDirectory,
            summary: launchTarget == .alan ? "new Alan space scaffolded" : "new shell space scaffolded",
            now: now
        )
        let tab = ShellTab(
            tabID: tabID,
            kind: .terminal,
            title: launchTarget == .alan ? "Alan" : "Shell",
            paneTree: ShellPaneTreeNode(
                nodeID: "node_\(paneID)",
                kind: .pane,
                direction: nil,
                paneID: paneID,
                children: nil
            )
        )
        let space = ShellSpace(
            spaceID: spaceID,
            title: title ?? (launchTarget == .alan ? "Alan Space \(spaceIndex)" : "Space \(spaceIndex)"),
            attention: .active,
            tabs: [tab]
        )
        let nextPanes = panes + [pane]
        let nextSpaces = rebuildingAttention(in: spaces + [space], panes: nextPanes)

        return ShellStateMutationResult(
            state: replacing(
                spaces: nextSpaces,
                panes: nextPanes,
                focusedPaneID: paneID
            ),
            spaceID: spaceID,
            tabID: tabID,
            paneID: paneID
        )
    }

    func creatingAlanSpace(
        title: String?,
        workingDirectory: String?,
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath,
        now: Date = .now
    ) -> ShellStateMutationResult {
        creatingSpace(
            launchTarget: .alan,
            title: title,
            workingDirectory: workingDirectory,
            defaultWorkingDirectory: defaultWorkingDirectory,
            now: now
        )
    }

    func creatingTerminalSpace(
        title: String?,
        workingDirectory: String?,
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath,
        now: Date = .now
    ) -> ShellStateMutationResult {
        creatingSpace(
            launchTarget: .shell,
            title: title,
            workingDirectory: workingDirectory,
            defaultWorkingDirectory: defaultWorkingDirectory,
            now: now
        )
    }

    func openingTab(
        launchTarget: ShellLaunchTarget,
        in requestedSpaceID: String?,
        title: String?,
        workingDirectory: String?,
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath,
        now: Date = .now
    ) throws -> ShellStateMutationResult {
        let targetSpaceID = requestedSpaceID ?? focusedSpaceID ?? spaces.first?.spaceID
        guard let targetSpaceID,
              let targetSpace = space(spaceID: targetSpaceID)
        else {
            throw ShellStateMutationError.spaceNotFound
        }

        let tabID = nextID(prefix: "tab", existing: spaces.flatMap { $0.tabs.map(\.tabID) })
        let paneID = nextID(prefix: "pane", existing: panes.map(\.paneID))
        let pane = makeTerminalPane(
            paneID: paneID,
            tabID: tabID,
            spaceID: targetSpaceID,
            launchTarget: launchTarget,
            workingDirectory: workingDirectory ?? defaultWorkingDirectory,
            summary: launchTarget == .alan ? "new Alan tab scaffolded" : "new shell tab scaffolded",
            now: now
        )
        let tab = ShellTab(
            tabID: tabID,
            kind: .terminal,
            title: title ?? (launchTarget == .alan ? "Alan \(targetSpace.tabs.count + 1)" : "Shell \(targetSpace.tabs.count + 1)"),
            paneTree: ShellPaneTreeNode(
                nodeID: "node_\(paneID)",
                kind: .pane,
                direction: nil,
                paneID: paneID,
                children: nil
            )
        )
        let nextSpaces = spaces.map { space in
            guard space.spaceID == targetSpaceID else { return space }
            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: space.attention,
                tabs: space.tabs + [tab]
            )
        }
        let nextPanes = panes + [pane]

        return ShellStateMutationResult(
            state: replacing(
                spaces: rebuildingAttention(in: nextSpaces, panes: nextPanes),
                panes: nextPanes,
                focusedPaneID: paneID
            ),
            spaceID: targetSpaceID,
            tabID: tabID,
            paneID: paneID
        )
    }

    func openingAlanTab(
        in requestedSpaceID: String?,
        title: String?,
        workingDirectory: String?,
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath,
        now: Date = .now
    ) throws -> ShellStateMutationResult {
        try openingTab(
            launchTarget: .alan,
            in: requestedSpaceID,
            title: title,
            workingDirectory: workingDirectory,
            defaultWorkingDirectory: defaultWorkingDirectory,
            now: now
        )
    }

    func openingTerminalTab(
        in requestedSpaceID: String?,
        title: String?,
        workingDirectory: String?,
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath,
        now: Date = .now
    ) throws -> ShellStateMutationResult {
        try openingTab(
            launchTarget: .shell,
            in: requestedSpaceID,
            title: title,
            workingDirectory: workingDirectory,
            defaultWorkingDirectory: defaultWorkingDirectory,
            now: now
        )
    }

    func splittingPane(
        _ paneID: String,
        direction: ShellSplitDirection,
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath,
        now: Date = .now
    ) throws -> ShellStateMutationResult {
        guard let pane = pane(paneID: paneID),
              let tab = tab(tabID: pane.tabID)
        else {
            throw ShellStateMutationError.paneNotFound
        }

        let newPaneID = nextID(prefix: "pane", existing: panes.map(\.paneID))
        let splitNodeID = nextID(
            prefix: "node",
            existing: spaces.flatMap(\.tabs).flatMap { $0.paneTree.nodeIDs }
        )
        let newPane = makeTerminalPane(
            paneID: newPaneID,
            tabID: pane.tabID,
            spaceID: pane.spaceID,
            launchTarget: pane.resolvedLaunchTarget,
            workingDirectory: pane.cwd ?? defaultWorkingDirectory,
            summary: "new split scaffolded",
            now: now
        )
        let updatedTab = ShellTab(
            tabID: tab.tabID,
            kind: tab.kind,
            title: tab.title,
            paneTree: tab.paneTree.splittingPane(
                paneID,
                direction: direction,
                splitNodeID: splitNodeID,
                newLeafNodeID: "node_\(newPaneID)",
                newPaneID: newPaneID
            )
        )

        let nextSpaces = spaces.map { space in
            guard space.spaceID == pane.spaceID else { return space }
            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: space.attention,
                tabs: space.tabs.map { existingTab in
                    existingTab.tabID == updatedTab.tabID ? updatedTab : existingTab
                }
            )
        }
        let nextPanes = panes + [newPane]

        return ShellStateMutationResult(
            state: replacing(
                spaces: rebuildingAttention(in: nextSpaces, panes: nextPanes),
                panes: nextPanes,
                focusedPaneID: newPaneID
            ),
            spaceID: pane.spaceID,
            tabID: pane.tabID,
            paneID: newPaneID
        )
    }

    func closingPane(_ paneID: String) throws -> ShellStateMutationResult {
        guard let pane = pane(paneID: paneID),
              let tab = tab(tabID: pane.tabID)
        else {
            throw ShellStateMutationError.paneNotFound
        }

        if tab.paneTree.paneIDs.count == 1 {
            return try closingTab(tab.tabID)
        }

        guard let updatedPaneTree = tab.paneTree.removingPane(paneID) else {
            throw ShellStateMutationError.paneNotFound
        }

        let updatedTab = ShellTab(
            tabID: tab.tabID,
            kind: tab.kind,
            title: tab.title,
            paneTree: updatedPaneTree
        )
        let nextSpaces = spaces.map { space in
            guard space.spaceID == pane.spaceID else { return space }
            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: space.attention,
                tabs: space.tabs.map { existingTab in
                    existingTab.tabID == updatedTab.tabID ? updatedTab : existingTab
                }
            )
        }
        let nextPanes = panes.filter { $0.paneID != paneID }
        let preferredPaneID =
            focusedPaneID == paneID
            ? (updatedTab.paneTree.paneIDs.first
                ?? nextPanes.first(where: { $0.spaceID == pane.spaceID })?.paneID
                ?? nextPanes.first?.paneID)
            : focusedPaneID

        let nextState = replacing(
            spaces: rebuildingAttention(in: nextSpaces, panes: nextPanes),
            panes: nextPanes,
            focusedPaneID: preferredPaneID
        )

        return ShellStateMutationResult(
            state: nextState,
            spaceID: pane.spaceID,
            tabID: pane.tabID,
            paneID: nextState.focusedPaneID
        )
    }

    func movingPaneToNewTab(
        _ paneID: String,
        title: String?,
        now: Date = .now
    ) throws -> ShellStateMutationResult {
        guard let pane = pane(paneID: paneID),
              let sourceTab = tab(tabID: pane.tabID)
        else {
            throw ShellStateMutationError.paneNotFound
        }

        guard sourceTab.paneTree.paneIDs.count > 1 else {
            throw ShellStateMutationError.lastPane
        }

        guard let sourcePaneTree = sourceTab.paneTree.removingPane(paneID) else {
            throw ShellStateMutationError.paneNotFound
        }

        let newTabID = nextID(
            prefix: "tab",
            existing: spaces.flatMap { $0.tabs.map(\.tabID) }
        )
        let movedPaneNode = ShellPaneTreeNode(
            nodeID: "node_\(paneID)",
            kind: .pane,
            direction: nil,
            paneID: paneID,
            children: nil
        )
        let updatedSourceTab = ShellTab(
            tabID: sourceTab.tabID,
            kind: sourceTab.kind,
            title: sourceTab.title,
            paneTree: sourcePaneTree
        )
        let newTab = ShellTab(
            tabID: newTabID,
            kind: sourceTab.kind,
            title: title ?? pane.viewport?.title ?? "Lifted Pane",
            paneTree: movedPaneNode
        )

        let nextSpaces = spaces.map { space in
            guard space.spaceID == pane.spaceID else { return space }
            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: space.attention,
                tabs: space.tabs.flatMap { existingTab -> [ShellTab] in
                    guard existingTab.tabID == sourceTab.tabID else {
                        return [existingTab]
                    }
                    return [updatedSourceTab, newTab]
                }
            )
        }
        let formatter = ISO8601DateFormatter()
        let nextPanes = panes.map { current in
            guard current.paneID == paneID else { return current }
            return ShellPane(
                paneID: current.paneID,
                tabID: newTabID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: current.cwd,
                process: current.process,
                attention: current.attention,
                context: current.context,
                viewport: ShellViewportSnapshot(
                    title: current.viewport?.title,
                    summary: current.viewport?.summary ?? "pane moved to its own tab",
                    visibleExcerpt: current.viewport?.visibleExcerpt,
                    lastActivityAt: formatter.string(from: now)
                ),
                alanBinding: current.alanBinding
            )
        }

        return ShellStateMutationResult(
            state: replacing(
                spaces: rebuildingAttention(in: nextSpaces, panes: nextPanes),
                panes: nextPanes,
                focusedPaneID: paneID
            ),
            spaceID: pane.spaceID,
            tabID: newTabID,
            paneID: paneID
        )
    }

    func movingPane(
        _ paneID: String,
        toTab targetTabID: String,
        direction: ShellSplitDirection,
        now: Date = .now
    ) throws -> ShellStateMutationResult {
        guard let pane = pane(paneID: paneID),
              let sourceTab = tab(tabID: pane.tabID)
        else {
            throw ShellStateMutationError.paneNotFound
        }

        guard let targetTab = tab(tabID: targetTabID) else {
            throw ShellStateMutationError.tabNotFound
        }

        guard sourceTab.tabID != targetTab.tabID else {
            throw ShellStateMutationError.invalidMoveTarget
        }

        let targetSpaceID = spaces.first(where: { space in
            space.tabs.contains(where: { $0.tabID == targetTabID })
        })?.spaceID

        guard let targetSpaceID else {
            throw ShellStateMutationError.tabNotFound
        }

        let formatter = ISO8601DateFormatter()
        let moveSummary = "pane moved to \(targetTab.title ?? targetTab.tabID)"
        let newSplitNodeID = nextID(
            prefix: "node",
            existing: spaces.flatMap { $0.tabs.flatMap { $0.paneTree.nodeIDs } }
        )
        let newLeafNodeID = "node_\(paneID)_moved"

        let updatedTargetTab = ShellTab(
            tabID: targetTab.tabID,
            kind: targetTab.kind,
            title: targetTab.title,
            paneTree: targetTab.paneTree.attachingPane(
                paneID,
                direction: direction,
                splitNodeID: newSplitNodeID,
                newLeafNodeID: newLeafNodeID
            )
        )

        let updatedSourcePaneTree = sourceTab.paneTree.removingPane(paneID)

        let nextSpaces = spaces.compactMap { space -> ShellSpace? in
            var nextTabs: [ShellTab] = []
            for tab in space.tabs {
                if tab.tabID == sourceTab.tabID {
                    if let updatedSourcePaneTree {
                        nextTabs.append(
                            ShellTab(
                                tabID: sourceTab.tabID,
                                kind: sourceTab.kind,
                                title: sourceTab.title,
                                paneTree: updatedSourcePaneTree
                            )
                        )
                    }
                    continue
                }

                if tab.tabID == updatedTargetTab.tabID {
                    nextTabs.append(updatedTargetTab)
                } else {
                    nextTabs.append(tab)
                }
            }

            guard !nextTabs.isEmpty else { return nil }
            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: space.attention,
                tabs: nextTabs
            )
        }

        let nextPanes = panes.map { current in
            guard current.paneID == paneID else { return current }
            return ShellPane(
                paneID: current.paneID,
                tabID: updatedTargetTab.tabID,
                spaceID: targetSpaceID,
                launchTarget: current.launchTarget,
                cwd: current.cwd,
                process: current.process,
                attention: current.attention,
                context: current.context,
                viewport: ShellViewportSnapshot(
                    title: current.viewport?.title,
                    summary: current.viewport?.summary ?? moveSummary,
                    visibleExcerpt: current.viewport?.visibleExcerpt,
                    lastActivityAt: formatter.string(from: now)
                ),
                alanBinding: current.alanBinding
            )
        }

        let nextState = replacing(
            spaces: rebuildingAttention(in: nextSpaces, panes: nextPanes),
            panes: nextPanes,
            focusedPaneID: paneID
        )

        return ShellStateMutationResult(
            state: nextState,
            spaceID: targetSpaceID,
            tabID: updatedTargetTab.tabID,
            paneID: paneID
        )
    }

    func settingAttention(
        _ attention: ShellAttentionState,
        for paneID: String
    ) throws -> ShellStateMutationResult {
        guard pane(paneID: paneID) != nil else {
            throw ShellStateMutationError.paneNotFound
        }
        let nextPanes = panes.map { current in
            guard current.paneID == paneID else { return current }
            return ShellPane(
                paneID: current.paneID,
                tabID: current.tabID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: current.cwd,
                process: current.process,
                attention: attention,
                context: current.context,
                viewport: current.viewport,
                alanBinding: current.alanBinding
            )
        }

        return ShellStateMutationResult(
            state: replacing(
                spaces: rebuildingAttention(in: spaces, panes: nextPanes),
                panes: nextPanes,
                focusedPaneID: focusedPaneID ?? paneID
            ),
            spaceID: pane(paneID: paneID)?.spaceID,
            tabID: pane(paneID: paneID)?.tabID,
            paneID: paneID
        )
    }

    func closingTab(_ tabID: String) throws -> ShellStateMutationResult {
        guard totalTabCount > 1 else {
            throw ShellStateMutationError.lastTab
        }
        guard let targetSpace = spaces.first(where: { space in
            space.tabs.contains(where: { $0.tabID == tabID })
        }),
        let targetTab = targetSpace.tabs.first(where: { $0.tabID == tabID })
        else {
            throw ShellStateMutationError.tabNotFound
        }

        let removedPaneIDs = Set(targetTab.paneTree.paneIDs)
        let nextSpaces = spaces.compactMap { space -> ShellSpace? in
            let remainingTabs = space.tabs.filter { $0.tabID != tabID }
            guard !remainingTabs.isEmpty else {
                return space.spaceID == targetSpace.spaceID ? nil : space
            }
            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: space.attention,
                tabs: remainingTabs
            )
        }
        let nextPanes = panes.filter { !removedPaneIDs.contains($0.paneID) }
        let preferredPaneID = nextPanes.first(where: { $0.spaceID == targetSpace.spaceID })?.paneID
            ?? nextPanes.first?.paneID
        let nextState = replacing(
            spaces: rebuildingAttention(in: nextSpaces, panes: nextPanes),
            panes: nextPanes,
            focusedPaneID: preferredPaneID
        )

        return ShellStateMutationResult(
            state: nextState,
            spaceID: nextState.focusedSpaceID,
            tabID: tabID,
            paneID: nextState.focusedPaneID
        )
    }

    private func replacing(
        spaces: [ShellSpace],
        panes: [ShellPane],
        focusedPaneID: String?
    ) -> ShellStateSnapshot {
        let resolvedFocusedPaneID =
            focusedPaneID.flatMap { candidate in
                panes.contains(where: { $0.paneID == candidate }) ? candidate : nil
            } ?? panes.first?.paneID
        let focusedPane = resolvedFocusedPaneID.flatMap { candidate in
            panes.first(where: { $0.paneID == candidate })
        }

        return ShellStateSnapshot(
            contractVersion: contractVersion,
            windowID: windowID,
            focusedSpaceID: focusedPane?.spaceID ?? spaces.first?.spaceID,
            focusedTabID: focusedPane?.tabID ?? spaces.first?.tabs.first?.tabID,
            focusedPaneID: resolvedFocusedPaneID,
            spaces: spaces,
            panes: panes
        )
    }

    private func rebuildingAttention(in spaces: [ShellSpace], panes: [ShellPane]) -> [ShellSpace] {
        spaces.map { space in
            ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: strongestAttention(in: panes.filter { $0.spaceID == space.spaceID }),
                tabs: space.tabs
            )
        }
    }

    private func strongestAttention(in panes: [ShellPane]) -> ShellAttentionState {
        panes
            .map(\.attention)
            .max(by: { Self.attentionRank(for: $0) < Self.attentionRank(for: $1) })
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

    private func nextID(prefix: String, existing: [String]) -> String {
        let nextOrdinal = existing
            .compactMap { identifier -> Int? in
                let components = identifier.split(separator: "_")
                guard let last = components.last else { return nil }
                return Int(last)
            }
            .max()
            .map { $0 + 1 }
            ?? (existing.isEmpty ? 1 : existing.count + 1)

        return "\(prefix)_\(nextOrdinal)"
    }

    private func makeTerminalPane(
        paneID: String,
        tabID: String,
        spaceID: String,
        launchTarget: ShellLaunchTarget,
        workingDirectory: String,
        summary: String,
        now: Date
    ) -> ShellPane {
        let formatter = ISO8601DateFormatter()
        return ShellPane(
            paneID: paneID,
            tabID: tabID,
            spaceID: spaceID,
            launchTarget: launchTarget,
            cwd: workingDirectory,
            process: Self.defaultProcessBinding(for: launchTarget),
            attention: .active,
            context: nil,
            viewport: ShellViewportSnapshot(
                title: Self.defaultViewportTitle(for: launchTarget),
                summary: summary,
                visibleExcerpt: nil,
                lastActivityAt: formatter.string(from: now)
            ),
            alanBinding: nil
        )
    }

    func migratingLegacyAlanBootstrapIfNeeded(
        defaultWorkingDirectory: String = FileManager.default.currentDirectoryPath
    ) -> ShellStateSnapshot {
        guard spaces.count == 1,
              panes.count == 1,
              let space = spaces.first,
              let tab = space.tabs.first,
              let pane = panes.first
        else {
            return self
        }

        let isLegacyBootstrap =
            space.title == "Alan"
            && tab.title == "Main Session"
            && pane.process?.program == "alan-tui"
            && pane.viewport?.title == "Alan"
            && pane.alanBinding == nil
            && pane.launchTarget == nil

        guard isLegacyBootstrap else {
            return self
        }

        return .bootstrapDefault(
            windowID: windowID,
            workingDirectory: pane.cwd ?? defaultWorkingDirectory
        )
    }

    private static func defaultProcessBinding(for launchTarget: ShellLaunchTarget) -> ShellProcessBinding {
        switch launchTarget {
        case .shell:
            let shellPath = defaultShellPath()
            return ShellProcessBinding(
                program: URL(fileURLWithPath: shellPath).lastPathComponent.isEmpty
                    ? "zsh"
                    : URL(fileURLWithPath: shellPath).lastPathComponent,
                argvPreview: ["-l"]
            )
        case .alan:
            return ShellProcessBinding(program: "alan-tui", argvPreview: ["alan", "chat"])
        }
    }

    private static func defaultViewportTitle(for launchTarget: ShellLaunchTarget) -> String {
        switch launchTarget {
        case .shell:
            return "Shell"
        case .alan:
            return "Alan"
        }
    }

    private static func defaultShellPath(
        environment: [String: String] = ProcessInfo.processInfo.environment
    ) -> String {
        let shell = environment["SHELL"]?.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let shell, !shell.isEmpty else {
            return "/bin/zsh"
        }
        return shell
    }
}

extension ShellStateSnapshot {
    static let spikePreview = ShellStateSnapshot(
        contractVersion: "0.1",
        windowID: "window_main",
        focusedSpaceID: "space_alan_app",
        focusedTabID: "tab_main",
        focusedPaneID: "pane_1",
        spaces: [
            ShellSpace(
                spaceID: "space_alan_app",
                title: "Alan App",
                attention: .awaitingUser,
                tabs: [
                    ShellTab(
                        tabID: "tab_main",
                        kind: .terminal,
                        title: "Main Session",
                        paneTree: ShellPaneTreeNode(
                            nodeID: "node_root",
                            kind: .split,
                            direction: .vertical,
                            paneID: nil,
                            children: [
                                ShellPaneTreeNode(
                                    nodeID: "pane_1",
                                    kind: .pane,
                                    direction: nil,
                                    paneID: "pane_1",
                                    children: nil
                                ),
                                ShellPaneTreeNode(
                                    nodeID: "pane_2",
                                    kind: .pane,
                                    direction: nil,
                                    paneID: "pane_2",
                                    children: nil
                                ),
                            ]
                        )
                    )
                ]
            )
        ],
        panes: [
            ShellPane(
                paneID: "pane_1",
                tabID: "tab_main",
                spaceID: "space_alan_app",
                launchTarget: .alan,
                cwd: "/Users/morris/Developer/Alan",
                process: ShellProcessBinding(program: "alan-tui", argvPreview: ["alan", "chat"]),
                attention: .awaitingUser,
                context: ShellContextSnapshot(
                    workingDirectoryName: "Alan",
                    repositoryRoot: "/Users/morris/Developer/Alan",
                    gitBranch: "main",
                    controlPath: "/tmp/alan-shell-control/window_main",
                    alanBindingFile: "/tmp/alan-shell-control/window_main/panes/pane_1/alan-binding.json",
                    launchStrategy: "installed_binary",
                    shellIntegrationSource: "ghostty_shell_integration",
                    processState: "running",
                    lastMetadataAt: "2026-04-01T10:30:00Z",
                    lastCommandExitCode: nil
                ),
                viewport: ShellViewportSnapshot(
                    title: "Alan",
                    summary: "waiting for approval",
                    visibleExcerpt: nil,
                    lastActivityAt: "2026-04-01T10:30:00Z"
                ),
                alanBinding: ShellAlanBinding(
                    sessionID: "sess_123",
                    runStatus: "yielded",
                    pendingYield: true,
                    source: "preview",
                    lastProjectedAt: "2026-04-01T10:30:00Z"
                )
            ),
            ShellPane(
                paneID: "pane_2",
                tabID: "tab_main",
                spaceID: "space_alan_app",
                launchTarget: .shell,
                cwd: "/Users/morris/Developer/Alan",
                process: ShellProcessBinding(program: "zsh", argvPreview: nil),
                attention: .idle,
                context: ShellContextSnapshot(
                    workingDirectoryName: "Alan",
                    repositoryRoot: "/Users/morris/Developer/Alan",
                    gitBranch: "main",
                    controlPath: "/tmp/alan-shell-control/window_main",
                    alanBindingFile: "/tmp/alan-shell-control/window_main/panes/pane_2/alan-binding.json",
                    launchStrategy: "path_binary",
                    shellIntegrationSource: "ghostty_shell_integration",
                    processState: "running",
                    lastMetadataAt: "2026-04-01T10:24:00Z",
                    lastCommandExitCode: 0
                ),
                viewport: ShellViewportSnapshot(
                    title: "shell",
                    summary: "idle shell",
                    visibleExcerpt: nil,
                    lastActivityAt: nil
                ),
                alanBinding: nil
            ),
        ]
    )
}
