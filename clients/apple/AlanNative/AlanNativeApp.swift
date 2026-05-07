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
    @ObservedObject var host: ShellHostController
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("Show Alan Window") {
                openWindow(id: "main")
                AlanMacPrimaryWindowPresenter.focusExistingWindowSoon()
            }
            .keyboardShortcut("n", modifiers: .command)
        }

        CommandMenu("Shell") {
            Button("New Terminal Tab") {
                host.performShellWorkspaceCommand(.newTerminalTab)
            }
            .keyboardShortcut("t", modifiers: .command)

            Button("New Alan Tab") {
                host.performShellWorkspaceCommand(.newAlanTab)
            }
            .keyboardShortcut("t", modifiers: [.command, .option])

            Divider()

            Button("Split Right") {
                host.performShellWorkspaceCommand(.splitRight)
            }
            .keyboardShortcut("d", modifiers: .command)

            Button("Split Down") {
                host.performShellWorkspaceCommand(.splitDown)
            }
            .keyboardShortcut("d", modifiers: [.command, .shift])

            Button("Split Left") {
                host.performShellWorkspaceCommand(.splitLeft)
            }
            .keyboardShortcut("d", modifiers: [.command, .option])

            Button("Split Up") {
                host.performShellWorkspaceCommand(.splitUp)
            }
            .keyboardShortcut("d", modifiers: [.command, .option, .shift])

            Button("Equalize Splits") {
                host.performShellWorkspaceCommand(.equalizeSplits)
            }
            .keyboardShortcut("=", modifiers: [.command, .option])

            Divider()

            Button("Focus Pane Left") {
                host.performShellWorkspaceCommand(.focusLeft)
            }
            .keyboardShortcut(.leftArrow, modifiers: [.command, .control])

            Button("Focus Pane Right") {
                host.performShellWorkspaceCommand(.focusRight)
            }
            .keyboardShortcut(.rightArrow, modifiers: [.command, .control])

            Button("Focus Pane Up") {
                host.performShellWorkspaceCommand(.focusUp)
            }
            .keyboardShortcut(.upArrow, modifiers: [.command, .control])

            Button("Focus Pane Down") {
                host.performShellWorkspaceCommand(.focusDown)
            }
            .keyboardShortcut(.downArrow, modifiers: [.command, .control])

            Divider()

            Button("Close Pane") {
                host.performShellWorkspaceCommand(.closePane)
            }
            .keyboardShortcut("w", modifiers: [.command, .shift])

            Button("Close Tab") {
                host.performShellWorkspaceCommand(.closeTab)
            }
            .keyboardShortcut("w", modifiers: .command)
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
