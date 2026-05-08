import SwiftUI

#if os(macOS)
struct MacShellRootView: View {
    @ObservedObject private var host: ShellHostController
    @State private var isCommandTabPresented = false
    @State private var windowChromeMetrics = ShellWindowChromeMetrics()

    init(host: ShellHostController) {
        self.host = host
    }

    var body: some View {
        ZStack {
            ShellSpaceKeyboardShortcuts(host: host)

            ShellPalette.canvas
                .ignoresSafeArea()

            HStack(spacing: 0) {
                ShellSidebarView(
                    host: host,
                    chromeMetrics: windowChromeMetrics
                ) {
                    withAnimation(.easeOut(duration: 0.18)) {
                        isCommandTabPresented = true
                    }
                }
                .frame(width: 286)

                ShellWorkspaceView(host: host)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .ignoresSafeArea(edges: .top)
                    .background {
                        ShellMaterialBackgroundView()
                            .ignoresSafeArea(edges: .top)
                    }
            }
            .frame(minWidth: 1260, minHeight: 800)
            .background(ShellPalette.window)

            if isCommandTabPresented {
                Color.black.opacity(0.16)
                    .ignoresSafeArea()
                    .onTapGesture {
                        withAnimation(.easeOut(duration: 0.18)) {
                            isCommandTabPresented = false
                        }
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
        .background(ShellWindowPlacementView(metrics: $windowChromeMetrics))
    }
}
#endif
