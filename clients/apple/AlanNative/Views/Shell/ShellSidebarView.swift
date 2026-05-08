import SwiftUI

#if os(macOS)
struct ShellSidebarView: View {
    @ObservedObject var host: ShellHostController
    let chromeMetrics: ShellWindowChromeMetrics
    let openCommandTab: () -> Void
    @State private var hoveredTabID: String?
    @State private var hoveredSpaceID: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            sidebarHeader
            commandLauncher
            tabSection
            spaceDock
        }
        .padding(.horizontal, 12)
        .padding(.top, chromeMetrics.trafficLightsTopInset)
        .padding(.bottom, 15)
        .frame(maxHeight: .infinity, alignment: .top)
        .background {
            ShellMaterialBackgroundView()
                .ignoresSafeArea(edges: .top)
        }
    }

    private var sidebarHeader: some View {
        HStack(alignment: .center, spacing: 10) {
            ZStack {
                RoundedRectangle(cornerRadius: ShellRadii.row, style: .continuous)
                    .fill(ShellPalette.sidebarControlStrong)
                Text("A")
                    .font(.system(size: 12.5, weight: .bold, design: .rounded))
                    .foregroundStyle(ShellPalette.accent)
            }
            .frame(width: 28, height: 28)

            VStack(alignment: .leading, spacing: 2) {
                Text(host.selectedSpace?.title ?? "Alan")
                    .font(.system(size: 15.5, weight: .semibold))
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                Text(spaceSubtitle)
                    .font(.system(size: 10.5, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            .layoutPriority(1)

            Spacer(minLength: 8)

            Menu {
                Button("New Tab") {
                    _ = host.openTerminalTab()
                }
                Button("Open in Alan") {
                    _ = host.openAlanTab()
                }
            } label: {
                Image(systemName: "plus")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(.primary)
                    .frame(width: 26, height: 26)
                    .background(
                        RoundedRectangle(cornerRadius: ShellRadii.control, style: .continuous)
                            .fill(ShellPalette.sidebarControl)
                    )
            }
            .menuStyle(.borderlessButton)
            .buttonStyle(.plain)
            .menuIndicator(.hidden)
        }
    }

    private var commandLauncher: some View {
        Button(action: openCommandTab) {
            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.secondary)
                Text("Go to or Command...")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer(minLength: 0)
                Text("⌘K")
                    .font(.system(size: 11, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 11)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: ShellRadii.row, style: .continuous)
                    .fill(ShellPalette.sidebarControl)
            )
        }
        .buttonStyle(.plain)
        .keyboardShortcut("k", modifiers: [.command])
    }

    private var tabSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Tabs")
                    .font(.system(size: 11, weight: .semibold))
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)
                Spacer(minLength: 0)
                if let selectedSpace = host.selectedSpace {
                    Text("\(selectedSpace.tabs.count)")
                        .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.secondary)
                }
            }

            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 4) {
                    if let selectedSpace = host.selectedSpace {
                        ForEach(selectedSpace.tabs) { tab in
                            tabListRow(for: tab)
                        }
                    } else {
                        ShellEmptyStateRow(
                            title: "No spaces yet",
                            detail: "Create a space to start a new stack of terminal tabs."
                        )
                    }
                }
            }
        }
        .frame(maxHeight: .infinity, alignment: .top)
    }

    private var spaceDock: some View {
        ShellSidebarSection(title: "Spaces", accessory: "⌥⌘1-9") {
            HStack(spacing: 10) {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(host.spaces) { space in
                            Button {
                                host.select(spaceID: space.spaceID)
                            } label: {
                                ShellSpaceRailItem(
                                    title: space.title,
                                    symbolName: spaceSymbol(for: space),
                                    attention: space.attention,
                                    isSelected: host.selectedSpace?.spaceID == space.spaceID,
                                    isHovered: hoveredSpaceID == space.spaceID
                                )
                            }
                            .buttonStyle(.plain)
                            .onHover { isHovering in
                                hoveredSpaceID = isHovering ? space.spaceID : nil
                            }
                        }
                    }
                    .padding(.vertical, 2)
                }

                Menu {
                    Button("New Space") {
                        _ = host.createTerminalSpace()
                    }
                    Button("New Space with Alan") {
                        _ = host.createAlanSpace()
                    }
                } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 12.5, weight: .semibold))
                        .foregroundStyle(.primary)
                        .frame(width: 30, height: 30)
                        .background(
                            RoundedRectangle(cornerRadius: ShellRadii.control, style: .continuous)
                                .fill(ShellPalette.sidebarControl)
                        )
                }
                .menuStyle(.borderlessButton)
                .buttonStyle(.plain)
                .menuIndicator(.hidden)
                .help("Create a new space")
            }
        }
    }

    private var spaceSubtitle: String {
        guard let selectedSpace = host.selectedSpace else {
            return "Terminal-first on macOS"
        }

        let count = selectedSpace.tabs.count
        return count == 1 ? "1 tab" : "\(count) tabs"
    }

    private func close(tab: ShellTab) {
        host.select(tabID: tab.tabID)
        _ = host.closeSelectedTab()
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
            isSelected: isSelected,
            isHovered: isHovered,
            showsMenuAffordance: isHovered,
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
            Button("Open in Alan") {
                _ = host.openAlanTab()
            }
            Divider()
            Button("Close Tab", role: .destructive) {
                close(tab: tab)
            }
        }
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

private struct ShellSpaceRailItem: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let title: String
    let symbolName: String
    let attention: ShellAttentionState
    let isSelected: Bool
    let isHovered: Bool

    var body: some View {
        ZStack(alignment: .topTrailing) {
            ZStack {
                RoundedRectangle(cornerRadius: ShellRadii.row, style: .continuous)
                    .fill(
                        isSelected
                            ? ShellPalette.railSelection
                            : (isHovered ? ShellPalette.railHover : ShellPalette.railBase)
                    )
                Image(systemName: symbolName)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(isSelected ? ShellPalette.accent : .primary)
            }
            .frame(width: 34, height: 34)

            if attention != .idle {
                Circle()
                    .fill(attentionColor)
                    .frame(width: 9, height: 9)
                    .overlay {
                        Circle()
                            .stroke(ShellPalette.sidebarCard, lineWidth: 1.5)
                    }
                    .offset(x: 2, y: -2)
            }
        }
        .scaleEffect(isSelected ? 1 : (isHovered ? 1.03 : 1))
        .shadow(color: isSelected ? ShellPalette.accent.opacity(0.12) : .clear, radius: 8, y: 3)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isHovered)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isSelected)
        .help(title)
    }

    private var attentionColor: Color {
        switch attention {
        case .idle:
            return ShellPalette.line
        case .active:
            return ShellPalette.accent
        case .awaitingUser:
            return ShellPalette.attention
        case .notable:
            return Color(red: 0.71, green: 0.58, blue: 0.28)
        }
    }
}

private struct ShellTabSidebarRow: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let title: String
    let subtitle: String
    let iconName: String
    let attention: ShellAttentionState?
    let showsAlanMarker: Bool
    let isSelected: Bool
    let isHovered: Bool
    let showsMenuAffordance: Bool
    let onClose: () -> Void

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: iconName)
                .font(.system(size: 11.5, weight: .semibold))
                .foregroundStyle(isSelected ? ShellPalette.accent : .secondary)
                .frame(width: 14)

            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(title)
                        .font(.system(size: 13, weight: .semibold))
                        .tracking(-0.1)
                        .foregroundStyle(.primary)
                        .lineLimit(1)

                    if showsAlanMarker {
                        Image(systemName: "sparkles")
                            .font(.system(size: 9, weight: .bold))
                            .foregroundStyle(ShellPalette.accent)
                    }
                }

                Text(subtitle)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 8)

            if let attention {
                Circle()
                    .fill(attentionColor(for: attention))
                    .frame(width: 8, height: 8)
            }

            if showsMenuAffordance {
                Button(action: onClose) {
                    Image(systemName: "xmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundStyle(.secondary)
                        .frame(width: 18, height: 18)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .background(
            RoundedRectangle(cornerRadius: ShellRadii.row, style: .continuous)
                .fill(
                    isSelected
                        ? ShellPalette.sidebarSelection
                        : (isHovered ? ShellPalette.sidebarHover : Color.clear)
                )
        )
        .scaleEffect(isHovered && !isSelected ? 1.005 : 1)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isHovered)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isSelected)
    }

    private func attentionColor(for attention: ShellAttentionState) -> Color {
        switch attention {
        case .idle:
            return ShellPalette.line
        case .active:
            return ShellPalette.accent
        case .awaitingUser:
            return ShellPalette.attention
        case .notable:
            return Color(red: 0.71, green: 0.58, blue: 0.28)
        }
    }
}

private struct ShellSidebarSection<Content: View>: View {
    let title: String
    let accessory: String?
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(title)
                    .font(.system(size: 11, weight: .semibold))
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)
                Spacer(minLength: 0)
                if let accessory {
                    Text(accessory)
                        .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.tertiary)
                }
            }

            content
        }
    }
}

private struct ShellEmptyStateRow: View {
    let title: String
    let detail: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.system(size: 13, weight: .semibold))
            Text(detail)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(ShellPalette.mutedInk)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: ShellRadii.row, style: .continuous)
                .fill(ShellPalette.panelSoft)
        )
    }
}
#endif
