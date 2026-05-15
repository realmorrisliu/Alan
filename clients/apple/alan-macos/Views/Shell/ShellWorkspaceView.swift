import SwiftUI

#if os(macOS)
struct ShellWorkspaceView: View {
    @ObservedObject var host: ShellHostController
    let expandedSidebarProgress: CGFloat
    let spaceID: String?
    let selectedPaneID: String?

    init(
        host: ShellHostController,
        expandedSidebarProgress: CGFloat,
        spaceID: String? = nil,
        selectedPaneID: String? = nil
    ) {
        self.host = host
        self.expandedSidebarProgress = expandedSidebarProgress
        self.spaceID = spaceID
        self.selectedPaneID = selectedPaneID
    }

    var body: some View {
        TerminalPaneView(
            host: host,
            tab: displayTab,
            spaceID: displaySpace?.spaceID,
            selectedPaneID: displaySelectedPaneID,
            terminalSurfaceInsets: ShellWorkspaceMetrics.terminalSurfaceInsets(
                expandedSidebarProgress: expandedSidebarProgress
            )
        )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var displaySpace: ShellSpace? {
        guard let spaceID else { return host.selectedSpace }
        return host.spaces.first { $0.spaceID == spaceID }
    }

    private var displayTab: ShellTab? {
        guard let displaySpace else { return host.selectedTab }
        if displaySpace.spaceID == host.selectedSpace?.spaceID,
           let selectedTab = host.selectedTab,
           displaySpace.tabs.contains(where: { $0.tabID == selectedTab.tabID })
        {
            return selectedTab
        }
        if let selectedPaneID,
           let tab = displaySpace.tabs.first(where: { $0.contains(paneID: selectedPaneID) })
        {
            return tab
        }
        return displaySpace.tabs.first
    }

    private var displaySelectedPaneID: String? {
        if let selectedPaneID,
           displayTab?.contains(paneID: selectedPaneID) == true
        {
            return selectedPaneID
        }
        if displayTab?.tabID == host.selectedTab?.tabID,
           let selectedPaneID = host.selectedPane?.paneID,
           displayTab?.contains(paneID: selectedPaneID) == true
        {
            return selectedPaneID
        }
        return displayTab?.paneTree.paneIDs.first
    }
}

struct ShellSpaceKeyboardShortcuts: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(spacing: 0) {
            Button("") {
                host.selectAdjacentSpace(offset: -1)
            }
            .keyboardShortcut(.leftArrow, modifiers: [.command, .option])

            Button("") {
                host.selectAdjacentSpace(offset: 1)
            }
            .keyboardShortcut(.rightArrow, modifiers: [.command, .option])

            ForEach(Array(host.spaces.prefix(9).enumerated()), id: \.element.spaceID) { index, _ in
                Button("") {
                    host.selectSpace(at: index)
                }
                .keyboardShortcut(
                    KeyEquivalent(Character(String(index + 1))),
                    modifiers: [.command, .option]
                )
            }
        }
        .labelsHidden()
        .buttonStyle(.plain)
        .frame(width: 0, height: 0)
        .opacity(0.001)
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}
#endif
