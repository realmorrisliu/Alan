import SwiftUI

@main
struct AlanNativeApp: App {
    var body: some Scene {
        WindowGroup {
            #if os(macOS)
            MacShellRootView()
            #else
            ContentView()
            #endif
        }
        #if os(macOS)
        .defaultSize(width: 1360, height: 860)
        #endif
    }
}
