import SwiftUI

#if os(macOS)
struct ShellWorkspaceView: View {
    @ObservedObject var host: ShellHostController
    let expandedSidebarProgress: CGFloat

    var body: some View {
        TerminalPaneView(
            host: host,
            tab: host.selectedTab,
            spaceID: host.selectedSpace?.spaceID,
            selectedPaneID: host.selectedPane?.paneID,
            terminalSurfaceInsets: ShellWorkspaceMetrics.terminalSurfaceInsets(
                expandedSidebarProgress: expandedSidebarProgress
            )
        )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
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
