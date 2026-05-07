import SwiftUI

#if os(macOS)
import AppKit
import Darwin

@MainActor
private final class AlanMacPrimaryShellOwner: ObservableObject {
    let host: ShellHostController

    init(fileManager: FileManager = .default) {
        let windowContext = ShellWindowContext.make(
            fileManager: fileManager,
            windowID: "window_main"
        )
        host = ShellHostController.live(
            fileManager: fileManager,
            windowContext: windowContext,
            startupMode: .fresh
        )
    }
}

private enum AlanMacAppStartup {
    static func acquireSingletonOrTerminate() -> AlanAppSingletonGuard {
        do {
            switch try AlanAppSingletonGuard.acquire() {
            case .acquired(let guardHandle):
                return guardHandle
            case .alreadyRunning:
                AlanAppSingletonGuard.activateExistingInstance()
                Darwin.exit(0)
            }
        } catch {
            fatalError("Alan could not acquire the macOS app singleton lock: \(error)")
        }
    }
}

private enum AlanMacPrimaryWindowPresenter {
    static func focusExistingWindow() {
        guard let window = NSApp.windows.first(where: { $0.title == "Alan" }) ?? NSApp.windows.first
        else {
            return
        }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    static func focusExistingWindowSoon() {
        DispatchQueue.main.async {
            focusExistingWindow()
        }
    }
}

private final class AlanMacAppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldHandleReopen(
        _ sender: NSApplication,
        hasVisibleWindows flag: Bool
    ) -> Bool {
        AlanMacPrimaryWindowPresenter.focusExistingWindow()
        return true
    }
}

private struct AlanMacShellCommands: Commands {
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("Show Alan Window") {
                openWindow(id: "main")
                AlanMacPrimaryWindowPresenter.focusExistingWindowSoon()
            }
            .keyboardShortcut("n", modifiers: .command)
        }
    }
}
#endif

@main
struct AlanNativeApp: App {
    #if os(macOS)
    private let singletonGuard: AlanAppSingletonGuard
    @StateObject private var primaryShellOwner: AlanMacPrimaryShellOwner
    @NSApplicationDelegateAdaptor(AlanMacAppDelegate.self) private var appDelegate

    init() {
        singletonGuard = AlanMacAppStartup.acquireSingletonOrTerminate()
        _primaryShellOwner = StateObject(wrappedValue: AlanMacPrimaryShellOwner())
    }
    #endif

    var body: some Scene {
        #if os(macOS)
        Window("Alan", id: "main") {
            MacShellRootView(host: primaryShellOwner.host)
                .toolbarBackgroundVisibility(.hidden, for: .windowToolbar)
                .toolbar(removing: .title)
        }
        .commands {
            AlanMacShellCommands()
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
