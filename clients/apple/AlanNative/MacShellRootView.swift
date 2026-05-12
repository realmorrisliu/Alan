import SwiftUI

#if os(macOS)
struct MacShellRootView: View {
    @ObservedObject private var host: ShellHostController
    @State private var isCommandTabPresented = false
    @State private var windowChromeMetrics = ShellWindowChromeMetrics()

    init(host: ShellHostController) {
        self.host = host
    }

    private func presentCommandInput() {
        withAnimation(.easeOut(duration: 0.18)) {
            isCommandTabPresented = true
        }
    }

    private func dismissCommandInput() {
        withAnimation(.easeOut(duration: 0.18)) {
            isCommandTabPresented = false
        }
        DispatchQueue.main.async {
            host.refocusSelectedTerminalPane()
        }
    }

    var body: some View {
        ZStack {
            ShellSpaceKeyboardShortcuts(host: host)

            ShellMaterialBackgroundView(.windowBackdrop)
                .ignoresSafeArea()

            HStack(spacing: 0) {
                ShellSidebarView(
                    host: host,
                    chromeMetrics: windowChromeMetrics
                ) {
                    presentCommandInput()
                }
                .frame(width: 286)

                ShellWorkspaceView(host: host)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .ignoresSafeArea(edges: .top)
            }
            .frame(minWidth: 1260, minHeight: 800)

            if isCommandTabPresented {
                ShellPalette.overlayScrim
                    .ignoresSafeArea()
                    .onTapGesture {
                        dismissCommandInput()
                    }

                ShellCommandTabView(
                    host: host,
                    isPresented: $isCommandTabPresented
                )
                .frame(width: 520)
                .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.easeOut(duration: 0.18), value: isCommandTabPresented)
        .onChange(of: host.commandInputRequestID) { _, _ in
            presentCommandInput()
        }
        .background(ShellWindowPlacementView(metrics: $windowChromeMetrics))
    }
}
#endif
