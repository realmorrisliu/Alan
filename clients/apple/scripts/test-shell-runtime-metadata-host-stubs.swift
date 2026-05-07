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

    func configure(
        pane: ShellPane?,
        bootProfile: AlanShellBootProfile?,
        isSelected: Bool,
        surfaceHandle: AlanTerminalSurfaceHandle?,
        activationDelegate: TerminalHostActivationDelegate?,
        onRuntimeUpdate: @escaping (TerminalHostRuntimeSnapshot) -> Void,
        onMetadataUpdate: @escaping (TerminalPaneMetadataSnapshot) -> Void
    ) {}

    func teardownTerminalRuntime() {
        teardownCount += 1
    }
}
#endif
