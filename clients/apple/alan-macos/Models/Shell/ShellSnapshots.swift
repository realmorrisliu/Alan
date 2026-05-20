import Foundation

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
    let activity: TerminalActivitySnapshot?
    let alanBinding: ShellAlanBinding?

    var id: String { paneID }

    init(
        paneID: String,
        tabID: String,
        spaceID: String,
        launchTarget: ShellLaunchTarget?,
        cwd: String?,
        process: ShellProcessBinding?,
        attention: ShellAttentionState,
        context: ShellContextSnapshot?,
        viewport: ShellViewportSnapshot?,
        activity: TerminalActivitySnapshot? = nil,
        alanBinding: ShellAlanBinding?
    ) {
        self.paneID = paneID
        self.tabID = tabID
        self.spaceID = spaceID
        self.launchTarget = launchTarget
        self.cwd = cwd
        self.process = process
        self.attention = attention
        self.context = context
        self.viewport = viewport
        self.activity = activity
        self.alanBinding = alanBinding
    }

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
        case activity
        case alanBinding = "alan_binding"
    }
}

extension ShellPane {
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
    static let minimumSplitRatio = 0.15
    static let maximumSplitRatio = 0.85

    let nodeID: String
    let kind: ShellPaneTreeKind
    let direction: ShellSplitDirection?
    let ratio: Double?
    let paneID: String?
    let children: [ShellPaneTreeNode]?

    var id: String { nodeID }

    private enum CodingKeys: String, CodingKey {
        case nodeID = "node_id"
        case kind
        case direction
        case ratio
        case paneID = "pane_id"
        case children
    }

    init(
        nodeID: String,
        kind: ShellPaneTreeKind,
        direction: ShellSplitDirection?,
        ratio: Double? = nil,
        paneID: String?,
        children: [ShellPaneTreeNode]?
    ) {
        self.nodeID = nodeID
        self.kind = kind
        self.direction = direction
        self.ratio = kind == .split
            ? Self.clampedSplitRatio(ratio ?? 0.5)
            : nil
        self.paneID = paneID
        self.children = children
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(ShellPaneTreeKind.self, forKey: .kind)
        let decodedRatio = kind == .split ? try container.decode(Double.self, forKey: .ratio) : nil

        self.init(
            nodeID: try container.decode(String.self, forKey: .nodeID),
            kind: kind,
            direction: try container.decodeIfPresent(ShellSplitDirection.self, forKey: .direction),
            ratio: decodedRatio,
            paneID: try container.decodeIfPresent(String.self, forKey: .paneID),
            children: try container.decodeIfPresent([ShellPaneTreeNode].self, forKey: .children)
        )
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(nodeID, forKey: .nodeID)
        try container.encode(kind, forKey: .kind)
        try container.encodeIfPresent(direction, forKey: .direction)
        if kind == .split {
            try container.encode(ratio ?? 0.5, forKey: .ratio)
        }
        try container.encodeIfPresent(paneID, forKey: .paneID)
        try container.encodeIfPresent(children, forKey: .children)
    }

    static func clampedSplitRatio(_ ratio: Double) -> Double {
        guard ratio.isFinite else { return 0.5 }
        return min(max(ratio, minimumSplitRatio), maximumSplitRatio)
    }

    var splitRatio: Double {
        Self.clampedSplitRatio(ratio ?? 0.5)
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

    func contains(nodeID targetNodeID: String) -> Bool {
        if nodeID == targetNodeID { return true }
        return (children ?? []).contains { $0.contains(nodeID: targetNodeID) }
    }

    func adjacentPaneID(
        from targetPaneID: String,
        direction: ShellSpatialFocusDirection
    ) -> String? {
        let frames = leafFrames(in: .unit)
        guard let targetFrame = frames.first(where: { $0.paneID == targetPaneID }) else {
            return nil
        }

        return frames
            .filter { $0.paneID != targetPaneID && targetFrame.isAdjacentCandidate($0, direction: direction) }
            .min { lhs, rhs in
                targetFrame.sortsBefore(lhs, rhs, direction: direction)
            }?
            .paneID
    }

    private struct PaneFrame {
        static let unit = PaneFrame(
            paneID: "",
            minX: 0,
            maxX: 1,
            minY: 0,
            maxY: 1
        )

        let paneID: String
        let minX: Double
        let maxX: Double
        let minY: Double
        let maxY: Double

        var width: Double { max(maxX - minX, 0) }
        var height: Double { max(maxY - minY, 0) }
        var midX: Double { (minX + maxX) / 2 }
        var midY: Double { (minY + maxY) / 2 }

        func replacingPaneID(_ paneID: String) -> PaneFrame {
            PaneFrame(
                paneID: paneID,
                minX: minX,
                maxX: maxX,
                minY: minY,
                maxY: maxY
            )
        }

        func isAdjacentCandidate(
            _ candidate: PaneFrame,
            direction: ShellSpatialFocusDirection
        ) -> Bool {
            let epsilon = 0.000_001
            guard perpendicularOverlap(with: candidate, direction: direction) > epsilon else {
                return false
            }

            switch direction {
            case .left:
                return candidate.maxX <= minX + epsilon
            case .right:
                return candidate.minX >= maxX - epsilon
            case .up:
                return candidate.maxY <= minY + epsilon
            case .down:
                return candidate.minY >= maxY - epsilon
            }
        }

        func sortsBefore(
            _ lhs: PaneFrame,
            _ rhs: PaneFrame,
            direction: ShellSpatialFocusDirection
        ) -> Bool {
            let epsilon = 0.000_001
            let lhsDistance = primaryDistance(to: lhs, direction: direction)
            let rhsDistance = primaryDistance(to: rhs, direction: direction)
            if abs(lhsDistance - rhsDistance) > epsilon {
                return lhsDistance < rhsDistance
            }

            let lhsOverlap = perpendicularOverlap(with: lhs, direction: direction)
            let rhsOverlap = perpendicularOverlap(with: rhs, direction: direction)
            if abs(lhsOverlap - rhsOverlap) > epsilon {
                return lhsOverlap > rhsOverlap
            }

            let lhsCenterDistance = perpendicularCenterDistance(to: lhs, direction: direction)
            let rhsCenterDistance = perpendicularCenterDistance(to: rhs, direction: direction)
            if abs(lhsCenterDistance - rhsCenterDistance) > epsilon {
                return lhsCenterDistance < rhsCenterDistance
            }

            return lhs.paneID < rhs.paneID
        }

        private func primaryDistance(
            to candidate: PaneFrame,
            direction: ShellSpatialFocusDirection
        ) -> Double {
            switch direction {
            case .left:
                return max(minX - candidate.maxX, 0)
            case .right:
                return max(candidate.minX - maxX, 0)
            case .up:
                return max(minY - candidate.maxY, 0)
            case .down:
                return max(candidate.minY - maxY, 0)
            }
        }

        private func perpendicularOverlap(
            with candidate: PaneFrame,
            direction: ShellSpatialFocusDirection
        ) -> Double {
            switch direction {
            case .left, .right:
                return max(0, min(maxY, candidate.maxY) - max(minY, candidate.minY))
            case .up, .down:
                return max(0, min(maxX, candidate.maxX) - max(minX, candidate.minX))
            }
        }

        private func perpendicularCenterDistance(
            to candidate: PaneFrame,
            direction: ShellSpatialFocusDirection
        ) -> Double {
            switch direction {
            case .left, .right:
                return abs(midY - candidate.midY)
            case .up, .down:
                return abs(midX - candidate.midX)
            }
        }
    }

    private func leafFrames(in frame: PaneFrame) -> [PaneFrame] {
        switch kind {
        case .pane:
            guard let paneID else { return [] }
            return [frame.replacingPaneID(paneID)]
        case .split:
            let childNodes = children ?? []
            guard !childNodes.isEmpty else { return [] }

            if childNodes.count == 2 {
                let ratio = splitRatio
                switch direction ?? .horizontal {
                case .vertical:
                    let splitX = frame.minX + frame.width * ratio
                    return childNodes[0].leafFrames(
                        in: PaneFrame(
                            paneID: "",
                            minX: frame.minX,
                            maxX: splitX,
                            minY: frame.minY,
                            maxY: frame.maxY
                        )
                    ) + childNodes[1].leafFrames(
                        in: PaneFrame(
                            paneID: "",
                            minX: splitX,
                            maxX: frame.maxX,
                            minY: frame.minY,
                            maxY: frame.maxY
                        )
                    )
                case .horizontal:
                    let splitY = frame.minY + frame.height * ratio
                    return childNodes[0].leafFrames(
                        in: PaneFrame(
                            paneID: "",
                            minX: frame.minX,
                            maxX: frame.maxX,
                            minY: frame.minY,
                            maxY: splitY
                        )
                    ) + childNodes[1].leafFrames(
                        in: PaneFrame(
                            paneID: "",
                            minX: frame.minX,
                            maxX: frame.maxX,
                            minY: splitY,
                            maxY: frame.maxY
                        )
                    )
                }
            }

            let childCount = Double(childNodes.count)
            return childNodes.enumerated().flatMap { index, child in
                let start = Double(index) / childCount
                let end = Double(index + 1) / childCount
                switch direction ?? .horizontal {
                case .vertical:
                    let minX = frame.minX + frame.width * start
                    let maxX = frame.minX + frame.width * end
                    return child.leafFrames(
                        in: PaneFrame(
                            paneID: "",
                            minX: minX,
                            maxX: maxX,
                            minY: frame.minY,
                            maxY: frame.maxY
                        )
                    )
                case .horizontal:
                    let minY = frame.minY + frame.height * start
                    let maxY = frame.minY + frame.height * end
                    return child.leafFrames(
                        in: PaneFrame(
                            paneID: "",
                            minX: frame.minX,
                            maxX: frame.maxX,
                            minY: minY,
                            maxY: maxY
                        )
                    )
                }
            }
        }
    }
}

struct ShellTab: Identifiable, Codable, Equatable {
    let tabID: String
    let kind: ShellTabKind
    let title: String?
    let paneTree: ShellPaneTreeNode
    let isPinned: Bool

    var id: String { tabID }

    private enum CodingKeys: String, CodingKey {
        case tabID = "tab_id"
        case kind
        case title
        case paneTree = "pane_tree"
        case isPinned = "is_pinned"
    }

    init(
        tabID: String,
        kind: ShellTabKind,
        title: String?,
        paneTree: ShellPaneTreeNode,
        isPinned: Bool = false
    ) {
        self.tabID = tabID
        self.kind = kind
        self.title = title
        self.paneTree = paneTree
        self.isPinned = isPinned
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            tabID: try container.decode(String.self, forKey: .tabID),
            kind: try container.decode(ShellTabKind.self, forKey: .kind),
            title: try container.decodeIfPresent(String.self, forKey: .title),
            paneTree: try container.decode(ShellPaneTreeNode.self, forKey: .paneTree),
            isPinned: try container.decodeIfPresent(Bool.self, forKey: .isPinned) ?? false
        )
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
    func contains(paneID: String) -> Bool {
        paneTree.contains(paneID: paneID)
    }

    var organizationSection: ShellTabOrganizationSection {
        isPinned ? .pinned : .unpinned
    }
}

extension ShellSpace {
    var pinnedTabs: [ShellTab] {
        tabs.filter(\.isPinned)
    }

    var unpinnedTabs: [ShellTab] {
        tabs.filter { !$0.isPinned }
    }

    func tabs(in section: ShellTabOrganizationSection) -> [ShellTab] {
        switch section {
        case .pinned:
            return pinnedTabs
        case .unpinned:
            return unpinnedTabs
        }
    }
}

extension ShellStateSnapshot {
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

    func tabOrganizationLocation(tabID: String) -> ShellTabOrganizationLocation? {
        for space in spaces {
            if let pinnedIndex = space.pinnedTabs.firstIndex(where: { $0.tabID == tabID }) {
                return ShellTabOrganizationLocation(
                    spaceID: space.spaceID,
                    section: .pinned,
                    index: pinnedIndex
                )
            }
            if let unpinnedIndex = space.unpinnedTabs.firstIndex(where: { $0.tabID == tabID }) {
                return ShellTabOrganizationLocation(
                    spaceID: space.spaceID,
                    section: .unpinned,
                    index: unpinnedIndex
                )
            }
        }
        return nil
    }
}
