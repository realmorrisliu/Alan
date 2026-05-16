import SwiftUI

#if os(macOS)
import AppKit

struct AlanMacShellCommands: Commands {
    @ObservedObject var host: ShellHostController
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("Show alan window") {
                openWindow(id: "main")
                AlanMacPrimaryWindowPresenter.focusExistingWindowSoon()
            }
            .keyboardShortcut("n", modifiers: .command)
        }

        CommandMenu("Tools") {
            Button("Install Command Line Tools...") {
                installCommandLineTools()
            }
        }

        CommandMenu("Shell") {
            Button("New Terminal Tab") {
                host.performShellWorkspaceCommand(.newTerminalTab)
            }
            .keyboardShortcut("t", modifiers: .command)

            Button("New alan tab") {
                host.performShellWorkspaceCommand(.newAlanTab)
            }
            .keyboardShortcut("t", modifiers: [.command, .option])

            Button("Ask alan...") {
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

    private func installCommandLineTools() {
        do {
            let records = try AlanCommandLineToolInstaller.install()
            let installed = records.filter { $0.status == .installed }
            let skipped = records.compactMap { record -> String? in
                guard case .skipped(let reason) = record.status else {
                    return nil
                }
                return "\(record.tool): \(reason)\n\(record.targetPath)"
            }

            let alert = NSAlert()
            alert.messageText = skipped.isEmpty
                ? "Command line tools installed"
                : "Command line tools partially installed"
            alert.informativeText = (
                installed.map { "\($0.tool) -> \($0.targetPath)" } +
                skipped.map { "Skipped \($0)" }
            ).joined(separator: "\n\n")
            alert.alertStyle = skipped.isEmpty ? .informational : .warning
            alert.addButton(withTitle: "OK")
            alert.runModal()
        } catch {
            let targetDirectory = AlanCommandLineToolInstaller.defaultInstallDirectory.path
            let alert = NSAlert()
            alert.messageText = "Command line tools were not installed"
            alert.informativeText = """
            alan tried to create command links in \(targetDirectory).

            \(error.localizedDescription)

            Choose a writable PATH directory or install with Homebrew cask.
            """
            alert.alertStyle = .warning
            alert.addButton(withTitle: "OK")
            alert.runModal()
        }
    }
}
#endif
