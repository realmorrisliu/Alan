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
            Button(host.shellActionTitle(.newTerminalTab)) {
                host.performShellAction(.newTerminalTab)
            }
            .keyboardShortcut("t", modifiers: .command)

            Button(host.shellActionTitle(.newAlanTab)) {
                host.performShellAction(.newAlanTab)
            }
            .keyboardShortcut("t", modifiers: [.command, .option])

            Button("Ask alan...") {
                host.requestCommandInput()
            }
            .keyboardShortcut("p", modifiers: .command)

            Divider()

            Button(host.shellActionTitle(.paneSplitRight)) {
                host.performShellAction(.paneSplitRight)
            }
            .keyboardShortcut("d", modifiers: .command)
            .disabled(!host.shellActionAvailability(.paneSplitRight).isAvailable)

            Button(host.shellActionTitle(.paneSplitDown)) {
                host.performShellAction(.paneSplitDown)
            }
            .keyboardShortcut("d", modifiers: [.command, .shift])
            .disabled(!host.shellActionAvailability(.paneSplitDown).isAvailable)

            Button(host.shellActionTitle(.paneSplitLeft)) {
                host.performShellAction(.paneSplitLeft)
            }
            .keyboardShortcut("d", modifiers: [.command, .option])
            .disabled(!host.shellActionAvailability(.paneSplitLeft).isAvailable)

            Button(host.shellActionTitle(.paneSplitUp)) {
                host.performShellAction(.paneSplitUp)
            }
            .keyboardShortcut("d", modifiers: [.command, .option, .shift])
            .disabled(!host.shellActionAvailability(.paneSplitUp).isAvailable)

            Button(host.shellActionTitle(.paneEqualizeSplits)) {
                host.performShellAction(.paneEqualizeSplits)
            }
            .keyboardShortcut("=", modifiers: [.command, .option])

            Divider()

            Button(host.shellActionTitle(.paneFocusLeft)) {
                host.performShellAction(.paneFocusLeft)
            }
            .keyboardShortcut(.leftArrow, modifiers: [.command, .control])
            .disabled(!host.shellActionAvailability(.paneFocusLeft).isAvailable)

            Button(host.shellActionTitle(.paneFocusRight)) {
                host.performShellAction(.paneFocusRight)
            }
            .keyboardShortcut(.rightArrow, modifiers: [.command, .control])
            .disabled(!host.shellActionAvailability(.paneFocusRight).isAvailable)

            Button(host.shellActionTitle(.paneFocusUp)) {
                host.performShellAction(.paneFocusUp)
            }
            .keyboardShortcut(.upArrow, modifiers: [.command, .control])
            .disabled(!host.shellActionAvailability(.paneFocusUp).isAvailable)

            Button(host.shellActionTitle(.paneFocusDown)) {
                host.performShellAction(.paneFocusDown)
            }
            .keyboardShortcut(.downArrow, modifiers: [.command, .control])
            .disabled(!host.shellActionAvailability(.paneFocusDown).isAvailable)

            Divider()

            Button(host.shellActionTitle(.paneClose)) {
                host.performShellAction(.paneClose)
            }
            .keyboardShortcut("w", modifiers: [.command, .shift])
            .disabled(!host.shellActionAvailability(.paneClose).isAvailable)

            Button(host.shellActionTitle(.tabClose)) {
                host.performShellAction(.tabClose)
            }
            .keyboardShortcut("w", modifiers: .command)
            .disabled(!host.shellActionAvailability(.tabClose).isAvailable)
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
