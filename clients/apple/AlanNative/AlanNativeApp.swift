import SwiftUI

@main
struct AlanNativeApp: App {
    var body: some Scene {
        WindowGroup("Alan") {
            #if os(macOS)
            MacShellRootView()
                .toolbarBackgroundVisibility(.hidden, for: .windowToolbar)
                .toolbar(removing: .title)
            #else
            ContentView()
            #endif
        }
        #if os(macOS)
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1360, height: 860)
        #endif
    }
}
