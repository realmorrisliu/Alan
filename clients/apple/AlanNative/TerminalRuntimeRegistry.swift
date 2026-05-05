import SwiftUI

#if os(macOS)
import AppKit

enum TerminalRuntimeDeliveryCode: String, Codable, Equatable {
    case accepted
    case queued
    case rejected
    case missingTarget = "missing_target"
    case unavailableRuntime = "unavailable_runtime"
    case timeout
}

struct TerminalRuntimeDeliveryResult: Codable, Equatable {
    let code: TerminalRuntimeDeliveryCode
    let acceptedBytes: Int
    let errorCode: String?
    let errorMessage: String?

    var applied: Bool {
        code == .accepted
    }

    static func accepted(byteCount: Int) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .accepted,
            acceptedBytes: byteCount,
            errorCode: nil,
            errorMessage: nil
        )
    }

    static func rejected(errorCode: String, errorMessage: String) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .rejected,
            acceptedBytes: 0,
            errorCode: errorCode,
            errorMessage: errorMessage
        )
    }

    static func unavailable(errorMessage: String) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .unavailableRuntime,
            acceptedBytes: 0,
            errorCode: "terminal_runtime_unavailable",
            errorMessage: errorMessage
        )
    }
}

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
    private let mockDeliveryHandler: MockDeliveryHandler?

    init(mockDeliveryHandler: MockDeliveryHandler? = nil) {
        self.mockDeliveryHandler = mockDeliveryHandler
    }

    func hostView(
        for pane: ShellPane?,
        bootProfile: AlanShellBootProfile?,
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

        hostView.configure(
            pane: pane,
            bootProfile: bootProfile,
            onRuntimeUpdate: onRuntimeUpdate,
            onMetadataUpdate: onMetadataUpdate
        )
        return hostView
    }

    func updateSnapshot(_ snapshot: TerminalHostRuntimeSnapshot) {
        guard let paneID = snapshot.paneID else { return }
        snapshotsByPaneID[paneID] = snapshot
    }

    func snapshot(for paneID: String?) -> TerminalHostRuntimeSnapshot {
        guard let paneID else { return .placeholder }
        return snapshotsByPaneID[paneID] ?? .placeholder
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

    func sendText(to paneID: String, text: String) -> TerminalRuntimeDeliveryResult {
        if let mockDeliveryHandler {
            return mockDeliveryHandler(paneID, text)
        }

        guard let hostView = hostViewsByPaneID[paneID] else {
            return .unavailable(
                errorMessage: "The requested pane does not have a live terminal runtime."
            )
        }

        return hostView.sendControlText(text)
    }

    var registeredPaneIDs: Set<String> {
        Set(hostViewsByPaneID.keys)
    }

    private func releaseRuntime(_ paneID: String) {
        if let hostView = hostViewsByPaneID.removeValue(forKey: paneID) {
            hostView.teardownTerminalRuntime()
        }
        snapshotsByPaneID.removeValue(forKey: paneID)
    }
}
#endif
