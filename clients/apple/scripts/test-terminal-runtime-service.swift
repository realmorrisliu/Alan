import Darwin
import Foundation

#if os(macOS)
@main
struct TerminalRuntimeServiceTestRunner {
    static func main() async {
        await MainActor.run {
            TerminalRuntimeServiceTests.run()
        }
    }
}

@MainActor
private enum TerminalRuntimeServiceTests {
    static func run() {
        verifiesBootstrapReuseAndPaneHandleIdentity()
        verifiesDeliveryAndMissingRuntimeResults()
        verifiesQueuedAndTimeoutDeliveryStates()
        verifiesControlResponseCarriesDeliveryDiagnostics()
        verifiesTeardownOnce()
        verifiesBootstrapFailureDiagnostics()
        print("Terminal runtime service tests passed.")
    }

    private static func verifiesBootstrapReuseAndPaneHandleIdentity() {
        let bootstrap = FakeAlanGhosttyProcessBootstrap()
        let service = AlanWindowTerminalRuntimeService(
            bootstrap: bootstrap,
            surfaceFactory: { paneID, _ in FakeAlanTerminalSurfaceHandle(paneID: paneID) }
        )

        let first = service.surfaceHandle(for: "pane_1", bootProfile: nil)
        let second = service.surfaceHandle(for: "pane_1", bootProfile: nil)
        expect(first === second, "service must preserve pane handle identity")
        expect(bootstrap.ensureCallCount == 1, "bootstrap must run once for repeated pane lookup")

        let secondWindow = AlanWindowTerminalRuntimeService(
            bootstrap: bootstrap,
            surfaceFactory: { paneID, _ in FakeAlanTerminalSurfaceHandle(paneID: paneID) }
        )
        secondWindow.ensureReady()
        expect(bootstrap.ensureCallCount == 1, "shared bootstrap must not reinitialize per window")
    }

    private static func verifiesDeliveryAndMissingRuntimeResults() {
        let service = FakeAlanTerminalRuntimeService()
        let handle = service.surfaceHandle(for: "pane_1", bootProfile: nil) as! FakeAlanTerminalSurfaceHandle

        let accepted = service.sendText(to: "pane_1", text: "hello")
        expect(accepted.applied, "accepted delivery must report applied")
        expect(accepted.acceptedBytes == 5, "accepted delivery must report utf8 byte count")
        expect(handle.deliveredText == ["hello"], "fake handle must observe delivered text")

        let missing = service.sendText(to: "pane_missing", text: "hello")
        expect(missing.code == .missingTarget, "missing pane must report runtime-missing")
        expect(missing.applied == false, "missing pane must not report applied")
    }

    private static func verifiesQueuedAndTimeoutDeliveryStates() {
        let service = FakeAlanTerminalRuntimeService()
        let handle = service.surfaceHandle(for: "pane_1", bootProfile: nil) as! FakeAlanTerminalSurfaceHandle
        handle.deliveryResult = .queued(byteCount: 5, runtimePhase: "attachable")

        let queued = service.sendText(to: "pane_1", text: "hello")
        expect(queued.code == .queued, "queued delivery must preserve queued state")
        expect(queued.acceptedBytes == 5, "queued delivery must preserve byte count")
        expect(queued.runtimePhase == "attachable", "queued delivery must preserve runtime phase")

        let timeout = TerminalRuntimeDeliveryResult.timeout(
            errorMessage: "runtime command exceeded deadline",
            runtimePhase: "bootstrapping"
        )
        expect(timeout.code == .timeout, "timeout delivery must preserve timeout state")
        expect(timeout.errorCode == "terminal_runtime_timeout", "timeout delivery must be stable")
        expect(timeout.runtimePhase == "bootstrapping", "timeout delivery must preserve phase")
    }

    private static func verifiesControlResponseCarriesDeliveryDiagnostics() {
        let response = AlanShellControlResponse(
            requestID: "req_1",
            contractVersion: "0.1",
            applied: false,
            state: nil,
            spaces: nil,
            tabs: nil,
            panes: nil,
            pane: nil,
            items: nil,
            candidates: nil,
            events: nil,
            focusedPaneID: nil,
            spaceID: "space_1",
            tabID: "tab_1",
            paneID: "pane_1",
            acceptedBytes: 0,
            deliveryCode: TerminalRuntimeDeliveryCode.missingTarget.rawValue,
            runtimePhase: "failed",
            latestEventID: nil,
            errorCode: "terminal_runtime_missing",
            errorMessage: "missing"
        )

        let data = try! JSONEncoder().encode(response)
        let json = String(decoding: data, as: UTF8.self)
        expect(json.contains("\"delivery_code\":\"missing_target\""), "control response must encode delivery code")
        expect(json.contains("\"runtime_phase\":\"failed\""), "control response must encode runtime phase")
    }

    private static func verifiesTeardownOnce() {
        let service = FakeAlanTerminalRuntimeService()
        let handle = service.surfaceHandle(for: "pane_1", bootProfile: nil) as! FakeAlanTerminalSurfaceHandle

        expect(service.finalizePane("pane_1") == .completed, "first finalize must complete")
        expect(handle.teardownCount == 1, "first finalize must tear down exactly once")
        expect(service.finalizePane("pane_1") == .notStarted, "missing second finalize must be stable")
        expect(handle.teardownCount == 1, "second finalize must not repeat teardown")
    }

    private static func verifiesBootstrapFailureDiagnostics() {
        let failedDiagnostics = AlanGhosttyBootstrapDiagnostics(
            phase: .failed,
            summary: "Fake Ghostty bootstrap failed.",
            detail: nil,
            failureReason: "missing resources",
            dependencies: GhosttyIntegrationStatus.discover(),
            lastUpdatedAt: .now
        )
        let bootstrap = FakeAlanGhosttyProcessBootstrap(nextDiagnostics: failedDiagnostics)
        let service = AlanWindowTerminalRuntimeService(
            bootstrap: bootstrap,
            surfaceFactory: { paneID, _ in FakeAlanTerminalSurfaceHandle(paneID: paneID) }
        )

        let diagnostics = service.ensureReady()
        expect(diagnostics.phase == .failed, "failed bootstrap must publish failed phase")
        expect(diagnostics.failureReason == "missing resources", "failed bootstrap must retain reason")
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: String
    ) {
        guard condition() else {
            fputs("error: \(message)\n", stderr)
            exit(1)
        }
    }
}
#endif
