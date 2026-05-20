import AppKit
import Foundation

#if os(macOS)
@MainActor
protocol TerminalHostActivationDelegate: AnyObject {
    func terminalHostDidRequestActivation(paneID: String)
}

@MainActor
final class AlanTerminalHostNSView: NSView {
    private(set) var teardownCount = 0
    private(set) var focusCount = 0

    func configure(
        pane: ShellPane?,
        bootProfile: AlanShellBootProfile?,
        isSelected: Bool,
        surfaceHandle: AlanTerminalSurfaceHandle?,
        activationDelegate: TerminalHostActivationDelegate?,
        onShellAction: ((ShellActionID, ShellActionTarget) -> Void)?,
        onCommandInput: (() -> Void)?,
        onCloseRequest: ((Bool) -> Void)?,
        onRuntimeUpdate: @escaping (TerminalHostRuntimeSnapshot) -> Void,
        onMetadataUpdate: @escaping (TerminalPaneMetadataSnapshot) -> Void
    ) {}

    func teardownTerminalRuntime() {
        teardownCount += 1
    }

    func beginFindInteraction() -> Bool {
        false
    }

    func updateFindQuery(_ query: String) -> Bool {
        false
    }

    func selectNextFindMatch() {}

    func selectPreviousFindMatch() {}

    func dismissFindInteraction(refocusTerminal: Bool = true) {}

    func focusTerminal() {
        focusCount += 1
    }
}
#endif
