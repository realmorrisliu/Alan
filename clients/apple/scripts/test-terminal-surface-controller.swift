import Darwin
import Foundation

#if os(macOS)
@main
struct TerminalSurfaceControllerTestRunner {
    static func main() async {
        await MainActor.run {
            TerminalSurfaceControllerTests.run()
        }
    }
}

@MainActor
private enum TerminalSurfaceControllerTests {
    static func run() {
        verifiesScrollbackMetricsAndTerminalModes()
        verifiesInputCommandRouting()
        verifiesPaneScopedSearchState()
        verifiesSurfaceSnapshotEqualityIgnoresTimestamp()
        verifiesClipboardDeliveryStates()
        verifiesMetadataOverlayProjection()
        print("Terminal surface controller tests passed.")
    }

    private static func verifiesScrollbackMetricsAndTerminalModes() {
        let adapter = AlanTerminalScrollbackAdapter()
        let normal = adapter.updateMetrics(
            AlanTerminalScrollbackMetrics(
                totalRows: 140,
                visibleRows: 40,
                firstVisibleRow: 90,
                mode: .normalBuffer
            )
        )

        expect(normal.nativeScrollbarVisible, "normal-buffer scrollback must expose native scrollbar")
        expect(normal.thumbRange.lowerBound == 90, "scrollbar must track first visible row")
        expect(normal.thumbRange.upperBound == 130, "scrollbar must track visible row range")

        let alternate = adapter.updateMetrics(
            AlanTerminalScrollbackMetrics(
                totalRows: 140,
                visibleRows: 40,
                firstVisibleRow: 90,
                mode: .alternateScreen
            )
        )

        expect(alternate.nativeScrollbarVisible == false, "alternate-screen scrollback must not expose stale normal-buffer scrollbar")
    }

    private static func verifiesInputCommandRouting() {
        let adapter = AlanTerminalInputAdapter()
        let command = adapter.routeKey(
            AlanTerminalKeyInput(
                characters: "q",
                keyCode: 12,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            )
        )
        expect(command == .nativeCommand("quit"), "command-q must route to the native app command")

        let findCommand = adapter.routeKey(
            AlanTerminalKeyInput(
                characters: "f",
                keyCode: 3,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            )
        )
        expect(findCommand == .nativeCommand("find"), "command-f must route to pane search")

        let printable = adapter.routeKey(
            AlanTerminalKeyInput(
                characters: "a",
                keyCode: 0,
                modifiers: [],
                phase: .down,
                isRepeat: false
            )
        )
        expect(printable == .terminalText("a"), "printable key input must become terminal text")
    }

    private static func verifiesPaneScopedSearchState() {
        let search = AlanTerminalSearchAdapter(paneID: "pane_1")
        search.updateQuery("runtime")
        search.updateMatches(total: 3, selectedIndex: 0)
        search.next()
        expect(search.state.selectedIndex == 1, "next search result must advance inside the pane")
        search.previous()
        expect(search.state.selectedIndex == 0, "previous search result must move backward inside the pane")
        search.dismiss()
        expect(search.state.isActive == false, "dismissed search must be inactive")
        expect(search.state.paneID == "pane_1", "search state must remain pane scoped")
    }

    private static func verifiesSurfaceSnapshotEqualityIgnoresTimestamp() {
        let older = AlanTerminalSurfaceStateSnapshot.placeholder
        let newer = AlanTerminalSurfaceStateSnapshot(
            readiness: older.readiness,
            terminalMode: older.terminalMode,
            scrollback: older.scrollback,
            search: older.search,
            readonly: older.readonly,
            secureInput: older.secureInput,
            inputReady: older.inputReady,
            rendererHealth: older.rendererHealth,
            childExited: older.childExited,
            lastUpdatedAt: older.lastUpdatedAt.addingTimeInterval(60)
        )
        expect(older != newer, "regular surface snapshot equality must keep timestamp changes visible")
        expect(
            older.equalsIgnoringTimestamp(newer),
            "runtime snapshot diffing must ignore surface timestamp churn"
        )
    }

    private static func verifiesClipboardDeliveryStates() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        let clipboard = AlanTerminalSelectionClipboardAdapter(surfaceHandle: handle)
        let accepted = clipboard.paste("hello")
        expect(accepted.applied, "ready paste must be delivered")
        expect(handle.deliveredText == ["hello"], "ready paste must reach the surface handle")

        _ = handle.teardown()
        let rejected = clipboard.paste("again")
        expect(rejected.applied == false, "closed paste must not report success")
        expect(rejected.errorCode == "terminal_clipboard_unavailable", "closed paste must use a stable clipboard error")
    }

    private static func verifiesMetadataOverlayProjection() {
        let adapter = AlanTerminalMetadataAdapter()
        let failed = adapter.overlayState(
            renderer: TerminalRendererSnapshot(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "internal callback failed",
                detail: nil,
                failureReason: "renderer lost device",
                recentEvents: ["raw callback name"]
            ),
            metadata: .placeholder,
            surface: .unready(reason: .rendererFailed)
        )
        expect(failed?.title == "Terminal cannot draw", "renderer failure overlay must use user-facing language")
        expect(failed?.debugDetail?.contains("renderer lost device") == true, "debug detail must preserve raw failure context")

        let exited = adapter.overlayState(
            renderer: .placeholder,
            metadata: TerminalPaneMetadataSnapshot(
                title: nil,
                workingDirectory: nil,
                summary: nil,
                attention: .idle,
                processExited: true,
                lastCommandExitCode: 2,
                lastUpdatedAt: .now
            ),
            surface: .ready
        )
        expect(exited?.title == "Process exited", "child exit overlay must be terminal-specific")
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
