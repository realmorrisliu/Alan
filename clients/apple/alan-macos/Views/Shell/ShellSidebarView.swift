import SwiftUI

#if os(macOS)
struct ShellSidebarView: View {
    @ObservedObject var host: ShellHostController
    let chromeMetrics: ShellWindowChromeMetrics
    let spaceTransition: ShellSpaceTransition?
    let isSpaceSwipeGestureLocked: Bool
    let onSpaceSwipe: (ShellSidebarSwipeUpdate) -> Void
    let openCommandTab: () -> Void
    @State private var hoveredTabID: String?
    @State private var hoveredSpaceID: String?
    @State private var isCommandLauncherHovered = false
    @State private var tabListScrollOffsetY: CGFloat = 0

    var body: some View {
        GeometryReader { proxy in
            sidebarContent(pageWidth: max(proxy.size.width, 1))
        }
        .background {
            ShellSidebarSwipeMonitor(onUpdate: onSpaceSwipe)
        }
        .scrollDisabled(isTabListScrollDisabled)
        .onChange(of: sourceSpaceID) { _, _ in
            tabListScrollOffsetY = 0
        }
    }

    private func sidebarContent(pageWidth: CGFloat) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            commandLauncher
                .padding(.horizontal, ShellSidebarMetrics.edgeInset)
                .padding(.bottom, 10)
            spaceLabelRow(pageWidth: pageWidth)
                .padding(.bottom, 2)
            tabSection(pageWidth: pageWidth)
            spaceDock
                .padding(.horizontal, ShellSidebarMetrics.edgeInset)
                .padding(.top, 10)
        }
        .padding(.top, chromeMetrics.commandLauncherTopInset)
        .padding(.bottom, ShellSidebarMetrics.spaceDockOuterBottomInset)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func spaceLabelRow(pageWidth: CGFloat) -> some View {
        ShellSidebarSpaceHeaderPager(
            host: host,
            transition: activeTransition,
            pageWidth: pageWidth
        )
        .frame(maxWidth: .infinity)
        .frame(height: 28)
    }

    private var commandLauncher: some View {
        Button(action: openCommandTab) {
            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: ShellSidebarMetrics.iconPointSize, weight: .semibold))
                    .foregroundStyle(commandLauncherForeground)
                    .frame(width: ShellSidebarMetrics.iconColumnWidth)
                Text("Ask alan...")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(commandLauncherForeground)
                    .lineLimit(1)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, ShellSidebarMetrics.rowInset)
            .frame(maxWidth: .infinity, minHeight: 34, alignment: .leading)
            .background {
                ShellLiquidGlassSurface(
                    shape: Capsule(),
                    tint: ShellPalette.commandGlassTint,
                    tintOpacity: isCommandLauncherHovered ? 0.22 : 0.18,
                    strokeOpacity: isCommandLauncherHovered ? 0.22 : 0.16
                )
            }
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .onHover { isHovering in
            isCommandLauncherHovered = isHovering
        }
        .help("Ask alan, Command-P")
        .accessibilityLabel("Ask alan")
    }

    private var commandLauncherForeground: Color {
        ShellPalette.sidebarMutedInk.opacity(isCommandLauncherHovered ? 0.92 : 0.80)
    }

    private func tabSection(pageWidth: CGFloat) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            GeometryReader { proxy in
                ZStack(alignment: .topLeading) {
                    tabListPage(for: sourceSpaceID)
                        .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                        .offset(x: sourceOffset(in: pageWidth))

                    if let targetSpaceID = activeTransition?.targetSpaceID {
                        tabListPage(for: targetSpaceID)
                            .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                            .offset(x: targetOffset(in: pageWidth))
                            .allowsHitTesting(false)
                    }
                }
                .clipped()
                .overlay(alignment: .top) {
                    ShellSidebarScrollBoundary(progress: tabListBoundaryProgress)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func tabListPage(for spaceID: String?) -> some View {
        ScrollView(.vertical, showsIndicators: false) {
            GeometryReader { proxy in
                Color.clear.preference(
                    key: ShellSidebarTabListOffsetPreferenceKey.self,
                    value: proxy.frame(in: .named(tabListCoordinateSpaceName(for: spaceID))).minY
                )
            }
            .frame(height: 0)

            VStack(alignment: .leading, spacing: 4) {
                if let space = space(for: spaceID) {
                    if space.tabs.isEmpty {
                        ShellCompactEmptyAction(
                            title: "New Tab",
                            systemImage: "plus",
                            action: {
                                _ = host.openTerminalTab()
                            }
                        )
                        .help("Create a tab in this space")
                    } else {
                        ForEach(space.tabs) { tab in
                            tabListRow(for: tab)
                        }
                    }
                } else {
                    ShellCompactEmptyAction(
                        title: "New Space",
                        systemImage: "plus",
                        action: {
                            _ = host.createTerminalSpace()
                        }
                    )
                    .help("Create a space")
                }
            }
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .padding(.top, 2)
            .padding(.horizontal, ShellSidebarMetrics.edgeInset)
        }
        .coordinateSpace(name: tabListCoordinateSpaceName(for: spaceID))
        .onPreferenceChange(ShellSidebarTabListOffsetPreferenceKey.self) { offsetY in
            guard spaceID == sourceSpaceID else { return }
            tabListScrollOffsetY = offsetY
        }
    }

    private var spaceDock: some View {
        HStack(spacing: 8) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 4) {
                    ForEach(host.spaces) { space in
                        Button {
                            host.select(spaceID: space.spaceID)
                        } label: {
                            ShellSpaceSwitcherItem(
                                title: space.title,
                                symbolName: spaceSymbol(for: space),
                                attention: space.attention,
                                tabCount: space.tabs.count,
                                isSelected: host.selectedSpace?.spaceID == space.spaceID,
                                isPreviewed: activeTransition?.targetSpaceID == space.spaceID,
                                isHovered: hoveredSpaceID == space.spaceID
                            )
                        }
                        .buttonStyle(.plain)
                        .onHover { isHovering in
                            hoveredSpaceID = isHovering ? space.spaceID : nil
                        }
                    }
                }
                .padding(.vertical, ShellSidebarMetrics.spaceDockInternalVerticalPadding)
            }

            Button(action: createSpaceFromDock) {
                Image(systemName: "plus")
                    .font(.system(size: 12.5, weight: .semibold))
                    .foregroundStyle(ShellPalette.sidebarInk.opacity(0.76))
                    .frame(width: 30, height: 30)
                    .background {
                        if hoveredSpaceID == "__new_space__" {
                            ShellMaterialShape(
                                role: .controlGlassHover,
                                shape: RoundedRectangle(cornerRadius: ShellRadii.control, style: .continuous)
                            )
                        }
                    }
            }
            .buttonStyle(.plain)
            .help("Create a new space")
            .accessibilityLabel("Create space")
            .onHover { isHovering in
                hoveredSpaceID = isHovering ? "__new_space__" : nil
            }
        }
    }

    private func createSpaceFromDock() {
        _ = host.createTerminalSpace()
    }

    private var activeTransition: ShellSpaceTransition? {
        guard let spaceTransition,
              spaceTransition.sourceSpaceID == host.selectedSpace?.spaceID
        else {
            return nil
        }
        return spaceTransition
    }

    private var isTabListScrollDisabled: Bool {
        isSpaceSwipeGestureLocked || activeTransition != nil
    }

    private var sourceSpaceID: String? {
        activeTransition?.sourceSpaceID ?? host.selectedSpace?.spaceID
    }

    private func sourceOffset(in width: CGFloat) -> CGFloat {
        activeTransition?.sourceOffset(in: width) ?? 0
    }

    private func targetOffset(in width: CGFloat) -> CGFloat {
        activeTransition?.targetOffset(in: width) ?? 0
    }

    private var tabListBoundaryProgress: CGFloat {
        min(max(-tabListScrollOffsetY / 18, 0), 1)
    }

    private func tabListCoordinateSpaceName(for spaceID: String?) -> String {
        "ShellSidebarTabListScroll-\(spaceID ?? "none")"
    }

    private func space(for spaceID: String?) -> ShellSpace? {
        guard let spaceID else { return host.selectedSpace }
        return host.spaces.first { $0.spaceID == spaceID }
    }

    private func close(tab: ShellTab) {
        host.select(tabID: tab.tabID)
        _ = host.closeSelectedTab()
    }

    private func focusPane(_ paneID: String, in tab: ShellTab) {
        host.select(tabID: tab.tabID)
        host.focus(paneID: paneID)
        host.refocusSelectedTerminalPane()
    }

    private func focusNextSplitPane(in tab: ShellTab, summary: ShellTabSplitSummary) {
        guard let paneID = summary.nextPaneID(after: host.shellState.focusedPaneID) else { return }
        focusPane(paneID, in: tab)
    }

    @ViewBuilder
    private func tabListRow(for tab: ShellTab) -> some View {
        let isSelected = host.selectedTab?.tabID == tab.tabID
        let isHovered = hoveredTabID == tab.tabID

        ShellTabSidebarRow(
            title: tabTitle(for: tab),
            subtitle: tabSubtitle(for: tab),
            iconName: tabIconName(for: tab),
            attention: strongestAttention(for: tab),
            showsAlanMarker: showsAlanMarker(for: tab),
            splitSummary: splitSummary(for: tab),
            isSelected: isSelected,
            isHovered: isHovered,
            showsCloseAffordance: isHovered,
            onFocusPane: { paneID in
                focusPane(paneID, in: tab)
            },
            onFocusNextSplitPane: { summary in
                focusNextSplitPane(in: tab, summary: summary)
            },
            onClose: { close(tab: tab) }
        )
        .contentShape(Rectangle())
        .onTapGesture {
            host.select(tabID: tab.tabID)
        }
        .onHover { isHovering in
            hoveredTabID = isHovering ? tab.tabID : nil
        }
        .contextMenu {
            Button("New Tab") {
                _ = host.openTerminalTab()
            }
            Button("Open in alan") {
                _ = host.openAlanTab()
            }
            Divider()
            Button("Close Tab", role: .destructive) {
                close(tab: tab)
            }
        }
    }

    private func splitSummary(for tab: ShellTab) -> ShellTabSplitSummary? {
        let paneIDs = tab.paneTree.paneIDs.filter { paneID in
            host.shellState.panes.contains { $0.paneID == paneID }
        }
        guard paneIDs.count > 1 else { return nil }

        let children = tab.paneTree.children ?? []
        let isSimpleTwoPaneSplit = paneIDs.count == 2
            && tab.paneTree.kind == .split
            && children.count == 2
            && children.allSatisfy { $0.kind == .pane }

        return ShellTabSplitSummary(
            paneIDs: paneIDs,
            direction: isSimpleTwoPaneSplit ? tab.paneTree.direction : nil,
            focusedPaneID: paneIDs.contains(host.shellState.focusedPaneID ?? "")
                ? host.shellState.focusedPaneID
                : nil,
            isComplex: !isSimpleTwoPaneSplit
        )
    }

    private func fallbackTitle(for tab: ShellTab) -> String {
        switch tab.kind {
        case .terminal:
            return "Terminal"
        case .scratch:
            return "Scratch"
        case .log:
            return "Logs"
        }
    }

    private func tabIconName(for tab: ShellTab) -> String {
        switch tab.kind {
        case .terminal:
            return "terminal"
        case .scratch:
            return "note.text"
        case .log:
            return "doc.text.magnifyingglass"
        }
    }

    private func tabTitle(for tab: ShellTab) -> String {
        let panes = host.shellState.panes.filter { $0.tabID == tab.tabID }
        let primaryPane = panes.first
        return shellDisplayTitle(
            rawTitle: tab.title ?? primaryPane?.viewport?.title,
            workingDirectoryName: primaryPane?.context?.workingDirectoryName,
            cwd: primaryPane?.cwd,
            program: primaryPane?.process?.program,
            launchTarget: primaryPane?.resolvedLaunchTarget ?? .shell,
            fallback: fallbackTitle(for: tab)
        )
    }

    private func tabSubtitle(for tab: ShellTab) -> String {
        let panes = host.shellState.panes.filter { $0.tabID == tab.tabID }
        let primaryPane = panes.first
        let title = tabTitle(for: tab)

        if let primaryPane,
           let status = shellTerminalStatusSummary(for: primaryPane)
        {
            return status
        }

        if let branch = primaryPane?.context?.gitBranch,
           let folder = primaryPane?.context?.workingDirectoryName
        {
            if folder == title {
                return branch
            }
            return "\(folder)  ·  \(branch)"
        }

        if let folder = shellVisibleLabel(primaryPane?.context?.workingDirectoryName) ?? shellPathLeaf(primaryPane?.cwd) {
            if folder == title, let program = shellVisibleLabel(primaryPane?.process?.program) {
                return program
            }
            return folder
        }

        if let program = primaryPane?.process?.program {
            return program
        }

        return tab.kind.rawValue.capitalized
    }

    private func strongestAttention(for tab: ShellTab) -> ShellAttentionState? {
        host.shellState.panes
            .filter { $0.tabID == tab.tabID }
            .map(\.attention)
            .sorted { attentionRank(for: $0) > attentionRank(for: $1) }
            .first(where: { $0 != .idle })
    }

    private func spaceSymbol(for space: ShellSpace) -> String {
        let hasAlan = host.shellState.panes.contains { pane in
            pane.spaceID == space.spaceID && pane.resolvedLaunchTarget == .alan
        }

        if hasAlan {
            return "sparkles"
        }

        if space.tabs.count > 1 {
            return "square.stack.3d.up"
        }

        return "terminal"
    }

    private func showsAlanMarker(for tab: ShellTab) -> Bool {
        host.shellState.panes.contains { pane in
            pane.tabID == tab.tabID && pane.resolvedLaunchTarget == .alan
        }
    }

    private func attentionRank(for attention: ShellAttentionState) -> Int {
        switch attention {
        case .awaitingUser:
            return 3
        case .notable:
            return 2
        case .active:
            return 1
        case .idle:
            return 0
        }
    }
}

private struct ShellSidebarTabListOffsetPreferenceKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = nextValue()
    }
}

private struct ShellSidebarScrollBoundary: View {
    let progress: CGFloat

    var body: some View {
        VStack(spacing: 0) {
            Rectangle()
                .fill(ShellPalette.line.opacity(0.36))
                .frame(height: 0.5)

            LinearGradient(
                colors: [
                    ShellPalette.sidebarInk.opacity(0.10),
                    ShellPalette.sidebarInk.opacity(0.035),
                    ShellPalette.sidebarInk.opacity(0),
                ],
                startPoint: .top,
                endPoint: .bottom
            )
            .frame(height: 14)
        }
        .frame(maxWidth: .infinity, alignment: .top)
        .opacity(progress)
        .allowsHitTesting(false)
    }
}

private struct ShellSidebarSpaceHeaderPager: View {
    @ObservedObject var host: ShellHostController
    let transition: ShellSpaceTransition?
    let pageWidth: CGFloat

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                headerPage(for: sourceSpaceID)
                    .frame(width: proxy.size.width, height: proxy.size.height, alignment: .leading)
                    .offset(x: sourceOffset(in: pageWidth))

                if let targetSpaceID = activeTransition?.targetSpaceID {
                    headerPage(for: targetSpaceID)
                        .frame(width: proxy.size.width, height: proxy.size.height, alignment: .leading)
                        .offset(x: targetOffset(in: pageWidth))
                }
            }
            .clipped()
        }
        .frame(height: 26)
    }

    private var activeTransition: ShellSpaceTransition? {
        guard let transition,
              transition.sourceSpaceID == host.selectedSpace?.spaceID
        else {
            return nil
        }
        return transition
    }

    private var sourceSpaceID: String? {
        activeTransition?.sourceSpaceID ?? host.selectedSpace?.spaceID
    }

    private func headerPage(for spaceID: String?) -> some View {
        let space = space(for: spaceID)
        return HStack(spacing: 10) {
            Image(systemName: symbolName(for: space))
                .font(.system(size: ShellSidebarMetrics.iconPointSize, weight: .semibold))
                .foregroundStyle(ShellPalette.sidebarMutedInk.opacity(0.78))
                .frame(width: ShellSidebarMetrics.iconColumnWidth)

            Text(space?.title ?? "Space")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(ShellPalette.sidebarMutedInk.opacity(0.82))
                .lineLimit(1)
        }
        .padding(.horizontal, ShellSidebarMetrics.rowInset)
        .padding(.vertical, 5)
        .padding(.leading, ShellSidebarMetrics.edgeInset)
        .padding(.trailing, ShellSidebarMetrics.edgeInset)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func sourceOffset(in width: CGFloat) -> CGFloat {
        activeTransition?.sourceOffset(in: width) ?? 0
    }

    private func targetOffset(in width: CGFloat) -> CGFloat {
        activeTransition?.targetOffset(in: width) ?? 0
    }

    private func space(for spaceID: String?) -> ShellSpace? {
        guard let spaceID else { return host.selectedSpace }
        return host.spaces.first { $0.spaceID == spaceID }
    }

    private func symbolName(for space: ShellSpace?) -> String {
        guard let space else { return "terminal" }
        let hasAlan = host.shellState.panes.contains { pane in
            pane.spaceID == space.spaceID && pane.resolvedLaunchTarget == .alan
        }

        if hasAlan {
            return "sparkles"
        }

        if space.tabs.count > 1 {
            return "square.stack.3d.up"
        }

        return "terminal"
    }
}

private struct ShellSpaceSwitcherItem: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let title: String
    let symbolName: String
    let attention: ShellAttentionState
    let tabCount: Int
    let isSelected: Bool
    let isPreviewed: Bool
    let isHovered: Bool

    var body: some View {
        ZStack {
            if isSelected || isHovered || isPreviewed {
                ShellMaterialShape(
                    role: isSelected ? .controlGlassSelected : .controlGlassHover,
                    shape: RoundedRectangle(cornerRadius: ShellRadii.control, style: .continuous)
                )
            }
            Image(systemName: symbolName)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(isSelected || isPreviewed ? ShellPalette.accent : ShellPalette.sidebarInk.opacity(0.74))
        }
        .frame(width: 30, height: 30)
        .scaleEffect(isSelected ? 1 : (isHovered || isPreviewed ? 1.015 : 1))
        .shellShadow(isSelected || isPreviewed ? ShellShadows.navigationSelection : ShellShadows.none)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isHovered)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isSelected)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isPreviewed)
        .help(title)
        .accessibilityLabel(accessibilityLabel)
    }

    private var accessibilityLabel: String {
        var parts = [title, tabCount == 1 ? "1 tab" : "\(tabCount) tabs"]
        if isSelected {
            parts.append("selected")
        }
        if isPreviewed {
            parts.append("preview")
        }
        if attention != .idle {
            parts.append("needs attention")
        }
        return parts.joined(separator: ", ")
    }
}

private struct ShellTabSplitSummary: Equatable {
    let paneIDs: [String]
    let direction: ShellSplitDirection?
    let focusedPaneID: String?
    let isComplex: Bool

    var paneCount: Int {
        paneIDs.count
    }

    func nextPaneID(after currentPaneID: String?) -> String? {
        guard !paneIDs.isEmpty else { return nil }
        guard let currentPaneID,
              let currentIndex = paneIDs.firstIndex(of: currentPaneID)
        else {
            return paneIDs.first
        }

        return paneIDs[(currentIndex + 1) % paneIDs.count]
    }
}

private enum ShellSidebarRowVisualState: Equatable {
    case normal
    case hover
    case selected

    var cornerRadius: CGFloat {
        switch self {
        case .normal:
            return ShellRadii.row
        case .hover:
            return ShellRadii.surface
        case .selected:
            return ShellRadii.overlay
        }
    }

    var fill: Color? {
        switch self {
        case .normal:
            return nil
        case .hover:
            return ShellPalette.sidebarRowHover
        case .selected:
            return ShellPalette.sidebarRowSelected
        }
    }

    var stroke: Color {
        switch self {
        case .normal:
            return .clear
        case .hover:
            return ShellPalette.line.opacity(0.08)
        case .selected:
            return ShellPalette.line.opacity(0.12)
        }
    }

    var shadow: ShellShadowStyle {
        switch self {
        case .normal, .hover:
            return ShellShadows.none
        case .selected:
            return ShellShadows.navigationSelection
        }
    }
}

private struct ShellSidebarRowBackground: View {
    @Environment(\.colorScheme) private var colorScheme
    let state: ShellSidebarRowVisualState

    var body: some View {
        if let fill = state.fill {
            let shape = RoundedRectangle(cornerRadius: state.cornerRadius, style: .continuous)
            shape
                .fill(fill)
                .overlay {
                    shape.stroke(state.stroke, lineWidth: 0.5)
                }
                .overlay {
                    if colorScheme == .light && state == .selected {
                        shape
                            .stroke(Color.white.opacity(0.34), lineWidth: 0.55)
                            .mask {
                                shape.fill(
                                    LinearGradient(
                                        colors: [
                                            Color.white,
                                            Color.white.opacity(0),
                                        ],
                                        startPoint: .top,
                                        endPoint: .bottom
                                    )
                                )
                            }
                    }
                }
                .shellShadow(state.shadow)
        }
    }
}

private struct ShellTabSidebarRow: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @FocusState private var isKeyboardFocused: Bool
    let title: String
    let subtitle: String
    let iconName: String
    let attention: ShellAttentionState?
    let showsAlanMarker: Bool
    let splitSummary: ShellTabSplitSummary?
    let isSelected: Bool
    let isHovered: Bool
    let showsCloseAffordance: Bool
    let onFocusPane: (String) -> Void
    let onFocusNextSplitPane: (ShellTabSplitSummary) -> Void
    let onClose: () -> Void

    var body: some View {
        ZStack(alignment: .trailing) {
            HStack(spacing: 10) {
                Image(systemName: iconName)
                    .font(.system(size: ShellSidebarMetrics.iconPointSize, weight: .semibold))
                    .foregroundStyle(iconForeground)
                    .frame(width: ShellSidebarMetrics.iconColumnWidth)

                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Text(title)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(titleForeground)
                            .lineLimit(1)

                        if showsAlanMarker {
                            Image(systemName: "sparkles")
                                .font(.system(size: 9, weight: .bold))
                                .foregroundStyle(ShellPalette.accent)
                        }
                    }

                    Text(subtitle)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(subtitleForeground)
                        .lineLimit(1)
                }

                Spacer(minLength: 8)

                if let splitSummary {
                    ShellSplitTopologyIndicator(
                        summary: splitSummary,
                        onFocusPane: onFocusPane,
                        onFocusNextSplitPane: onFocusNextSplitPane
                    )
                }
            }
            .padding(.trailing, 24)

            Button(action: onClose) {
                Image(systemName: "xmark")
                    .font(.system(size: 9.5, weight: .bold))
                    .foregroundStyle(closeForeground)
                    .frame(width: 20, height: 20)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .opacity(showsCloseButton ? 1 : 0)
            .allowsHitTesting(showsCloseButton)
            .accessibilityHidden(!showsCloseButton)
            .help("Close tab")
        }
        .padding(.horizontal, ShellSidebarMetrics.rowInset)
        .padding(.vertical, 7)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            ShellSidebarRowBackground(state: visualState)
        )
        .contentShape(RoundedRectangle(cornerRadius: visualState.cornerRadius, style: .continuous))
        .animation(reduceMotion ? nil : .easeOut(duration: 0.14), value: visualState)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.12), value: showsCloseButton)
        .focusable()
        .focused($isKeyboardFocused)
        .focusEffectDisabled()
        .accessibilityLabel(accessibilityLabel)
        .help("Select tab")
    }

    private var visualState: ShellSidebarRowVisualState {
        if isSelected {
            return .selected
        }

        if isHovered || isKeyboardFocused {
            return .hover
        }

        return .normal
    }

    private var isInteractionActive: Bool {
        isHovered || showsCloseAffordance || isKeyboardFocused
    }

    private var showsCloseButton: Bool {
        isSelected || isInteractionActive
    }

    private var iconForeground: Color {
        isSelected ? ShellPalette.accent : ShellPalette.sidebarMutedInk.opacity(0.84)
    }

    private var titleForeground: Color {
        isSelected ? ShellPalette.sidebarInk : ShellPalette.sidebarInk.opacity(0.88)
    }

    private var subtitleForeground: Color {
        isSelected ? ShellPalette.sidebarMutedInk.opacity(0.90) : ShellPalette.sidebarMutedInk.opacity(0.68)
    }

    private var closeForeground: Color {
        isSelected ? ShellPalette.sidebarInk.opacity(0.50) : ShellPalette.sidebarMutedInk.opacity(0.72)
    }

    private var accessibilityLabel: String {
        var parts = [title, subtitle]
        if isSelected {
            parts.append("selected")
        }
        if attention != nil {
            parts.append("needs attention")
        }
        if let splitSummary {
            parts.append("\(splitSummary.paneCount) panes")
        }
        return parts.joined(separator: ", ")
    }
}

private struct ShellSplitTopologyIndicator: View {
    let summary: ShellTabSplitSummary
    let onFocusPane: (String) -> Void
    let onFocusNextSplitPane: (ShellTabSplitSummary) -> Void

    var body: some View {
        if summary.isComplex {
            complexButton
        } else {
            twoPaneIndicator
        }
    }

    private var twoPaneIndicator: some View {
        let panes = Array(summary.paneIDs.prefix(2).enumerated())
        let isVertical = summary.direction == .vertical

        return Group {
            if isVertical {
                HStack(spacing: 2) {
                    ForEach(panes, id: \.element) { index, paneID in
                        segmentButton(index: index, paneID: paneID)
                    }
                }
            } else {
                VStack(spacing: 2) {
                    ForEach(panes, id: \.element) { index, paneID in
                        segmentButton(index: index, paneID: paneID)
                    }
                }
            }
        }
        .frame(width: 25, height: 16)
        .help("Focus split pane")
        .accessibilityLabel("Split tab, \(summary.paneCount) panes")
    }

    private var complexButton: some View {
        Button {
            onFocusNextSplitPane(summary)
        } label: {
            HStack(spacing: 2) {
                Image(systemName: "rectangle.split.3x1")
                    .font(.system(size: 8, weight: .bold))
                Text("\(summary.paneCount)")
                    .font(.system(size: 8, weight: .bold, design: .monospaced))
            }
            .foregroundStyle(summary.focusedPaneID == nil ? .secondary : ShellPalette.accent)
            .frame(width: 28, height: 18)
            .background(
                RoundedRectangle(cornerRadius: ShellRadii.badge, style: .continuous)
                    .fill(ShellPalette.canvas.opacity(0.55))
            )
        }
        .buttonStyle(.plain)
        .help("Focus next split pane")
        .accessibilityLabel("Split tab, \(summary.paneCount) panes")
    }

    private func segmentButton(index: Int, paneID: String) -> some View {
        let isFocused = summary.focusedPaneID == paneID

        return Button {
            onFocusPane(paneID)
        } label: {
            RoundedRectangle(cornerRadius: ShellRadii.micro, style: .continuous)
                .fill(isFocused ? ShellPalette.accent.opacity(0.9) : ShellPalette.mutedInk.opacity(0.24))
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Focus pane \(index + 1)")
    }
}

private struct ShellCompactEmptyAction: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @FocusState private var isKeyboardFocused: Bool
    @State private var isHovered = false
    let title: String
    let systemImage: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: systemImage)
                    .font(.system(size: 11, weight: .semibold))
                Text(title)
                    .font(.system(size: 12, weight: .semibold))
                Spacer(minLength: 0)
            }
            .foregroundStyle(foreground)
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                ShellSidebarRowBackground(state: visualState)
            )
        }
        .buttonStyle(.plain)
        .focusable()
        .focused($isKeyboardFocused)
        .focusEffectDisabled()
        .onHover { isHovered = $0 }
        .animation(reduceMotion ? nil : .easeOut(duration: 0.14), value: visualState)
        .accessibilityLabel(title)
    }

    private var visualState: ShellSidebarRowVisualState {
        isHovered || isKeyboardFocused ? .hover : .normal
    }

    private var foreground: Color {
        isHovered || isKeyboardFocused
            ? ShellPalette.sidebarMutedInk.opacity(0.86)
            : ShellPalette.sidebarMutedInk.opacity(0.58)
    }
}
#endif
