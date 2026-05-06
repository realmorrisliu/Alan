import Darwin
import Foundation
import AppKit

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
        verifiesSearchActionsReachSurfaceEngine()
        verifiesScrollbackActionsReachSurfaceEngine()
        verifiesSurfaceSnapshotEqualityIgnoresTimestamp()
        verifiesClipboardDeliveryStates()
        verifiesSelectionCopyAndPasteUseController()
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

    private static func verifiesSearchActionsReachSurfaceEngine() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        let controller = AlanTerminalSurfaceController()
        var searchStateChangeCount = 0
        controller.onSearchStateChange = {
            searchStateChangeCount += 1
        }
        controller.bind(surfaceHandle: handle, paneID: "pane_1")

        expect(controller.beginSearch(), "controller must start search through the surface engine")
        expect(handle.searchActions == ["start_search"], "begin search must invoke Ghostty search action")
        expect(controller.searchAdapter?.state.isActive == true, "started search must become active")

        expect(
            controller.updateSearchQuery("alan"),
            "query changes must be accepted by the surface search engine"
        )
        expect(
            handle.searchActions.suffix(1) == ["search:alan"],
            "query changes must reach the terminal search action"
        )

        let countBeforeEngineUpdates = searchStateChangeCount
        handle.emitSearchUpdate(.matches(total: 3))
        handle.emitSearchUpdate(.selected(index: 1))
        expect(controller.searchAdapter?.state.totalMatches == 3, "search totals must update from engine callbacks")
        expect(controller.searchAdapter?.state.selectedIndex == 1, "selected match must update from engine callbacks")
        expect(
            searchStateChangeCount == countBeforeEngineUpdates + 2,
            "engine search callbacks must notify host UI refresh"
        )

        controller.nextSearchMatch()
        controller.previousSearchMatch()
        expect(
            handle.searchActions.suffix(2) == ["navigate_search:next", "navigate_search:previous"],
            "search navigation must be delegated to the terminal search engine"
        )

        controller.dismissSearch()
        expect(handle.searchActions.last == "end_search", "dismissed search must end the terminal search")
        expect(controller.searchAdapter?.state.isActive == false, "dismissed search must deactivate local state")

        handle.searchActionsShouldSucceed = false
        expect(
            controller.beginSearch() == false,
            "failed engine start must not pretend search is available"
        )
    }

    private static func verifiesScrollbackActionsReachSurfaceEngine() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        let controller = AlanTerminalSurfaceController()
        var stateChangeCount = 0
        controller.onSurfaceStateChange = {
            stateChangeCount += 1
        }
        controller.bind(surfaceHandle: handle, paneID: "pane_1")

        handle.emitScrollbackUpdate(
            AlanTerminalScrollbackMetrics(
                totalRows: 220,
                visibleRows: 40,
                firstVisibleRow: 120,
                mode: .normalBuffer
            )
        )

        expect(controller.scrollbackAdapter.state.nativeScrollbarVisible, "Ghostty scrollbar updates must expose native scrollbar state")
        expect(
            controller.scrollbackAdapter.state.thumbRange == 120..<160,
            "Ghostty scrollbar updates must set the visible terminal row range"
        )
        expect(stateChangeCount == 1, "scrollbar updates must notify host UI/runtime refresh")

        let normalRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: true)
        )
        expect(
            normalRoute == .nativeScroll(row: 128),
            "normal-buffer scroll input must become a native row scroll"
        )
        expect(
            handle.scrollActions == ["scroll_to_row:128"],
            "native row scroll must reach the terminal surface"
        )

        handle.emitScrollbackUpdate(
            AlanTerminalScrollbackMetrics(
                totalRows: 220,
                visibleRows: 40,
                firstVisibleRow: 120,
                mode: .alternateScreen
            )
        )
        let alternateRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: true)
        )
        expect(
            alternateRoute == .terminalScroll,
            "alternate-screen scroll input must stay routed to the terminal app"
        )
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

    private static func verifiesSelectionCopyAndPasteUseController() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        handle.selectedText = "selected terminal text"
        let controller = AlanTerminalSurfaceController()
        controller.bind(surfaceHandle: handle, paneID: "pane_1")

        let pasteboard = RecordingTerminalPasteboardWriter()
        expect(
            controller.copySelection(to: pasteboard),
            "controller copy must write selected terminal text to the pasteboard"
        )
        expect(
            pasteboard.string == "selected terminal text",
            "copied terminal selection must be available as pasteboard text"
        )

        let pasteResult = controller.paste("pasted text")
        expect(pasteResult.applied, "controller paste must use failure-aware delivery")
        expect(handle.deliveredText.last == "pasted text", "controller paste must reach the surface handle")

        _ = handle.teardown()
        let rejected = controller.paste("after close")
        expect(rejected.applied == false, "closed controller paste must not report success")
        expect(
            rejected.errorCode == "terminal_clipboard_unavailable",
            "closed controller paste must use the stable clipboard error"
        )
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

@MainActor
private final class RecordingTerminalPasteboardWriter: AlanTerminalPasteboardWriting {
    private(set) var string: String?

    func writeString(_ text: String) -> Bool {
        string = text
        return true
    }
}
#endif
