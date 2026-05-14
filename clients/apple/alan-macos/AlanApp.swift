import SwiftUI

@main
struct AlanApp: App {
    #if os(macOS)
    private let singletonGuard: AlanAppSingletonGuard
    @StateObject private var primaryShellOwner: AlanMacPrimaryShellOwner
    @NSApplicationDelegateAdaptor(AlanMacAppDelegate.self) private var appDelegate
    @AppStorage("alanShellAppearanceMode") private var appearanceMode = ShellAppearanceMode.system
    @AppStorage("alanShellSidebarCollapsed") private var isSidebarCollapsed = false

    init() {
        singletonGuard = AlanMacAppStartup.acquireSingletonOrTerminate()
        _primaryShellOwner = StateObject(wrappedValue: AlanMacPrimaryShellOwner())
    }
    #endif

    var body: some Scene {
        #if os(macOS)
        Window("alan", id: "main") {
            MacShellRootView(
                host: primaryShellOwner.host,
                appearanceMode: $appearanceMode,
                isSidebarCollapsed: $isSidebarCollapsed
            )
                .toolbarBackgroundVisibility(.hidden, for: .windowToolbar)
                .toolbar(removing: .title)
        }
        .commands {
            AlanMacShellCommands(host: primaryShellOwner.host)
        }
        .windowStyle(.hiddenTitleBar)
        .defaultWindowPlacement { _, context in
            let frame = ShellWindowSizing.defaultFrame(in: context.defaultDisplay.visibleRect)
            return WindowPlacement(frame.origin, size: frame.size)
        }
        .windowResizability(.contentMinSize)
        .restorationBehavior(.disabled)
        .defaultLaunchBehavior(.presented)
        #else
        WindowGroup("alan") {
            ContentView()
        }
        #endif
    }

}
