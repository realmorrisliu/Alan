import SwiftUI

#if os(macOS)
@MainActor
protocol TerminalRuntimeHandle: AnyObject {
    func sendControlText(_ text: String) -> TerminalRuntimeDeliveryResult
    func teardownTerminalRuntime()
}

@MainActor
final class MockTerminalRuntimeHandle: TerminalRuntimeHandle {
    private(set) var attachedCount = 0
    private(set) var detachedCount = 0
    private(set) var teardownCount = 0
    private(set) var deliveredText: [String] = []
    var deliveryResult: TerminalRuntimeDeliveryResult?

    func attach() {
        attachedCount += 1
    }

    func detach() {
        detachedCount += 1
    }

    func sendControlText(_ text: String) -> TerminalRuntimeDeliveryResult {
        deliveredText.append(text)
        return deliveryResult ?? .accepted(byteCount: text.lengthOfBytes(using: .utf8))
    }

    func teardownTerminalRuntime() {
        teardownCount += 1
    }
}

@MainActor
final class TerminalRuntimeRegistry: ObservableObject {
    typealias MockDeliveryHandler = (String, String) -> TerminalRuntimeDeliveryResult

    private var hostViewsByPaneID: [String: AlanTerminalHostNSView] = [:]
    private var snapshotsByPaneID: [String: TerminalHostRuntimeSnapshot] = [:]
    private let runtimeService: AlanTerminalRuntimeService
    private let mockDeliveryHandler: MockDeliveryHandler?

    init(
        runtimeService: AlanTerminalRuntimeService? = nil,
        mockDeliveryHandler: MockDeliveryHandler? = nil
    ) {
        self.runtimeService = runtimeService ?? AlanWindowTerminalRuntimeService()
        self.mockDeliveryHandler = mockDeliveryHandler
    }

    func hostView(
        for pane: ShellPane?,
        bootProfile: AlanShellBootProfile?,
        isSelected: Bool,
        activationDelegate: TerminalHostActivationDelegate?,
        onWorkspaceCommand: ((ShellWorkspaceCommand) -> Void)?,
        onCommandInput: (() -> Void)?,
        onRuntimeUpdate: @escaping (TerminalHostRuntimeSnapshot) -> Void,
        onMetadataUpdate: @escaping (TerminalPaneMetadataSnapshot) -> Void
    ) -> AlanTerminalHostNSView {
        let hostView: AlanTerminalHostNSView
        if let paneID = pane?.paneID {
            if let existing = hostViewsByPaneID[paneID] {
                hostView = existing
            } else {
                let created = AlanTerminalHostNSView()
                hostViewsByPaneID[paneID] = created
                hostView = created
            }
        } else {
            hostView = AlanTerminalHostNSView()
        }

        let surfaceHandle: AlanTerminalSurfaceHandle?
        if let paneID = pane?.paneID {
            surfaceHandle = runtimeService.surfaceHandle(for: paneID, bootProfile: bootProfile)
        } else {
            surfaceHandle = nil
        }

        hostView.configure(
            pane: pane,
            bootProfile: bootProfile,
            isSelected: isSelected,
            surfaceHandle: surfaceHandle,
            activationDelegate: activationDelegate,
            onWorkspaceCommand: onWorkspaceCommand,
            onCommandInput: onCommandInput,
            onRuntimeUpdate: onRuntimeUpdate,
            onMetadataUpdate: onMetadataUpdate
        )
        return hostView
    }

    func surfaceHandle(
        for pane: ShellPane?,
        bootProfile: AlanShellBootProfile?
    ) -> AlanTerminalSurfaceHandle? {
        guard let paneID = pane?.paneID else { return nil }
        return runtimeService.surfaceHandle(for: paneID, bootProfile: bootProfile)
    }

    func updateSnapshot(_ snapshot: TerminalHostRuntimeSnapshot) {
        guard let paneID = snapshot.paneID else { return }
        snapshotsByPaneID[paneID] = snapshot
        runtimeService.existingSurfaceHandle(for: paneID)?.updateHostRuntimeSnapshot(snapshot)
    }

    func snapshot(for paneID: String?) -> TerminalHostRuntimeSnapshot {
        guard let paneID else { return .placeholder }
        return snapshotsByPaneID[paneID] ?? runtimeSnapshot(from: runtimeService.snapshot(for: paneID))
    }

    func releaseRuntimes(excluding activePaneIDs: Set<String>) {
        let stalePaneIDs = Set(hostViewsByPaneID.keys)
            .union(snapshotsByPaneID.keys)
            .subtracting(activePaneIDs)
        stalePaneIDs.forEach { releaseRuntime($0) }
    }

    func releaseRuntime(for paneID: String) {
        releaseRuntime(paneID)
    }

    func releaseAllRuntimes() {
        registeredPaneIDs.forEach { releaseRuntime($0) }
    }

    func sendText(to paneID: String, text: String) -> TerminalRuntimeDeliveryResult {
        if let mockDeliveryHandler {
            return mockDeliveryHandler(paneID, text)
        }

        return runtimeService.sendText(to: paneID, text: text)
    }

    @discardableResult
    func beginFindInteraction(for paneID: String) -> Bool {
        hostViewsByPaneID[paneID]?.beginFindInteraction() ?? false
    }

    @discardableResult
    func updateFindQuery(for paneID: String, query: String) -> Bool {
        hostViewsByPaneID[paneID]?.updateFindQuery(query) ?? false
    }

    func selectNextFindMatch(for paneID: String) {
        hostViewsByPaneID[paneID]?.selectNextFindMatch()
    }

    func selectPreviousFindMatch(for paneID: String) {
        hostViewsByPaneID[paneID]?.selectPreviousFindMatch()
    }

    func dismissFindInteraction(for paneID: String, refocusTerminal: Bool = true) {
        hostViewsByPaneID[paneID]?.dismissFindInteraction(refocusTerminal: refocusTerminal)
    }

    func requestFocus(for paneID: String) {
        hostViewsByPaneID[paneID]?.focusTerminal()
    }

    var registeredPaneIDs: Set<String> {
        Set(hostViewsByPaneID.keys).union(runtimeService.registeredPaneIDs)
    }

    private func releaseRuntime(_ paneID: String) {
        if let hostView = hostViewsByPaneID.removeValue(forKey: paneID) {
            hostView.teardownTerminalRuntime()
        }
        runtimeService.finalizePane(paneID)
        snapshotsByPaneID.removeValue(forKey: paneID)
    }

    private func runtimeSnapshot(
        from surfaceSnapshot: AlanTerminalSurfaceSnapshot?
    ) -> TerminalHostRuntimeSnapshot {
        guard let surfaceSnapshot else { return .placeholder }
        return TerminalHostRuntimeSnapshot(
            stage: .scaffold,
            paneID: surfaceSnapshot.paneID,
            tabID: nil,
            logicalSize: .zero,
            backingSize: .zero,
            displayName: nil,
            displayID: nil,
            attachedWindowTitle: nil,
            isFocused: false,
            renderer: surfaceSnapshot.renderer,
            paneMetadata: surfaceSnapshot.metadata,
            surfaceState: .placeholder,
            lastUpdatedAt: surfaceSnapshot.lastUpdatedAt
        )
    }
}
#endif
