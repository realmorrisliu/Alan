import SwiftUI

@main
struct AlanNativeApp: App {
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
        Window("Alan", id: "main") {
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
        .defaultSize(width: 1360, height: 860)
        #else
        WindowGroup("Alan") {
            ContentView()
        }
        #endif
    }

}
