import SwiftUI

#if os(macOS)
struct AlanMacShellCommands: Commands {
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

            Button("Go to or Command...") {
                host.requestCommandInput()
            }
            .keyboardShortcut("p", modifiers: .command)

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
