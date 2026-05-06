import Foundation

#if os(macOS)
import AppKit
#if canImport(GhosttyKit)
import GhosttyKit
#endif

enum AlanTerminalMode: String, Equatable {
    case normalBuffer = "normal_buffer"
    case alternateScreen = "alternate_screen"
    case mouseReporting = "mouse_reporting"
}

struct AlanTerminalScrollbackMetrics: Equatable {
    let totalRows: Int
    let visibleRows: Int
    let firstVisibleRow: Int
    let mode: AlanTerminalMode

    static let empty = AlanTerminalScrollbackMetrics(
        totalRows: 0,
        visibleRows: 0,
        firstVisibleRow: 0,
        mode: .normalBuffer
    )
}

struct AlanTerminalScrollbackState: Equatable {
    let metrics: AlanTerminalScrollbackMetrics
    let nativeScrollbarVisible: Bool
    let thumbRange: Range<Int>

    static let empty = AlanTerminalScrollbackState(
        metrics: .empty,
        nativeScrollbarVisible: false,
        thumbRange: 0..<0
    )
}

@MainActor
final class AlanTerminalScrollbackAdapter {
    private(set) var state = AlanTerminalScrollbackState.empty

    @discardableResult
    func updateMetrics(_ metrics: AlanTerminalScrollbackMetrics) -> AlanTerminalScrollbackState {
        let totalRows = max(0, metrics.totalRows)
        let visibleRows = max(0, min(metrics.visibleRows, totalRows))
        let firstVisibleRow = max(0, min(metrics.firstVisibleRow, max(totalRows - visibleRows, 0)))
        let hasScrollableNormalBuffer = metrics.mode == .normalBuffer && totalRows > visibleRows
        let nextMetrics = AlanTerminalScrollbackMetrics(
            totalRows: totalRows,
            visibleRows: visibleRows,
            firstVisibleRow: firstVisibleRow,
            mode: metrics.mode
        )
        state = AlanTerminalScrollbackState(
            metrics: nextMetrics,
            nativeScrollbarVisible: hasScrollableNormalBuffer,
            thumbRange: firstVisibleRow..<(firstVisibleRow + visibleRows)
        )
        return state
    }

    func shouldForwardScrollToTerminal() -> Bool {
        state.metrics.mode == .alternateScreen || state.metrics.mode == .mouseReporting
    }
}

struct AlanTerminalKeyModifiers: OptionSet, Equatable {
    let rawValue: Int

    static let shift = AlanTerminalKeyModifiers(rawValue: 1 << 0)
    static let control = AlanTerminalKeyModifiers(rawValue: 1 << 1)
    static let option = AlanTerminalKeyModifiers(rawValue: 1 << 2)
    static let command = AlanTerminalKeyModifiers(rawValue: 1 << 3)
}

enum AlanTerminalKeyPhase: Equatable {
    case down
    case up
    case flagsChanged
}

struct AlanTerminalKeyInput: Equatable {
    let characters: String?
    let keyCode: UInt16
    let modifiers: AlanTerminalKeyModifiers
    let phase: AlanTerminalKeyPhase
    let isRepeat: Bool
}

enum AlanTerminalInputRoutingDecision: Equatable {
    case nativeCommand(String)
    case terminalText(String)
    case terminalKey
    case ignored
}

@MainActor
final class AlanTerminalInputAdapter {
    func routeKey(_ input: AlanTerminalKeyInput) -> AlanTerminalInputRoutingDecision {
        if input.phase == .down,
           input.modifiers == .command,
           input.characters?.lowercased() == "q"
        {
            return .nativeCommand("quit")
        }

        if input.phase == .down,
           input.modifiers == .command,
           input.characters?.lowercased() == "f"
        {
            return .nativeCommand("find")
        }

        guard input.phase == .down else { return .terminalKey }
        guard input.modifiers.subtracting([.shift, .option]).isEmpty else {
            return .terminalKey
        }
        guard let characters = input.characters, !characters.isEmpty else { return .terminalKey }
        if characters.count == 1, let scalar = characters.unicodeScalars.first {
            if scalar.value < 0x20 || scalar.value == 0x7F {
                return .terminalKey
            }
            if scalar.value >= 0xF700 && scalar.value <= 0xF8FF {
                return .terminalKey
            }
        }
        return .terminalText(characters)
    }
}

struct AlanTerminalSearchState: Equatable {
    let paneID: String
    let query: String
    let isActive: Bool
    let totalMatches: Int?
    let selectedIndex: Int?

    static func inactive(paneID: String) -> AlanTerminalSearchState {
        AlanTerminalSearchState(
            paneID: paneID,
            query: "",
            isActive: false,
            totalMatches: nil,
            selectedIndex: nil
        )
    }
}

@MainActor
final class AlanTerminalSearchAdapter {
    private(set) var state: AlanTerminalSearchState

    init(paneID: String) {
        self.state = .inactive(paneID: paneID)
    }

    func updateQuery(_ query: String) {
        state = AlanTerminalSearchState(
            paneID: state.paneID,
            query: query,
            isActive: true,
            totalMatches: state.totalMatches,
            selectedIndex: state.selectedIndex
        )
    }

    func updateMatches(total: Int?, selectedIndex: Int?) {
        let boundedIndex: Int?
        if let total, total > 0, let selectedIndex {
            boundedIndex = max(0, min(selectedIndex, total - 1))
        } else {
            boundedIndex = nil
        }
        state = AlanTerminalSearchState(
            paneID: state.paneID,
            query: state.query,
            isActive: state.isActive,
            totalMatches: total,
            selectedIndex: boundedIndex
        )
    }

    func next() {
        guard let total = state.totalMatches, total > 0 else { return }
        let current = state.selectedIndex ?? -1
        updateMatches(total: total, selectedIndex: (current + 1) % total)
    }

    func previous() {
        guard let total = state.totalMatches, total > 0 else { return }
        let current = state.selectedIndex ?? 0
        updateMatches(total: total, selectedIndex: (current - 1 + total) % total)
    }

    func dismiss() {
        state = .inactive(paneID: state.paneID)
    }
}

@MainActor
final class AlanTerminalSelectionClipboardAdapter {
    private weak var surfaceHandle: AlanTerminalSurfaceHandle?

    init(surfaceHandle: AlanTerminalSurfaceHandle?) {
        self.surfaceHandle = surfaceHandle
    }

    func updateSurfaceHandle(_ surfaceHandle: AlanTerminalSurfaceHandle?) {
        self.surfaceHandle = surfaceHandle
    }

    func paste(_ text: String) -> TerminalRuntimeDeliveryResult {
        guard !text.isEmpty else {
            return .accepted(byteCount: 0)
        }
        guard let surfaceHandle,
              surfaceHandle.isSurfaceReady,
              surfaceHandle.snapshot.teardownStatus != .completed
        else {
            return .rejected(
                errorCode: "terminal_clipboard_unavailable",
                errorMessage: "Paste cannot be delivered because the terminal is not ready.",
                runtimePhase: surfaceHandle?.snapshot.runtimePhase
            )
        }
        return surfaceHandle.sendControlText(text)
    }

    func writeSelectionToPasteboard(_ text: String?, pasteboard: NSPasteboard = .general) -> Bool {
        guard let text, !text.isEmpty else { return false }
        pasteboard.clearContents()
        return pasteboard.setString(text, forType: .string)
    }
}

enum AlanTerminalSurfaceUnreadyReason: String, Equatable {
    case missingSurface = "missing_surface"
    case inputNotReady = "input_not_ready"
    case rendererFailed = "renderer_failed"
    case childExited = "child_exited"
    case readonly
}

enum AlanTerminalSurfaceReadiness: Equatable {
    case ready
    case unready(reason: AlanTerminalSurfaceUnreadyReason)
}

struct AlanTerminalOverlayState: Equatable {
    let title: String
    let message: String
    let badge: String
    let action: String?
    let debugDetail: String?
}

struct AlanTerminalSurfaceStateSnapshot: Equatable {
    let readiness: AlanTerminalSurfaceReadiness
    let terminalMode: AlanTerminalMode
    let scrollback: AlanTerminalScrollbackState
    let search: AlanTerminalSearchState?
    let readonly: Bool
    let secureInput: Bool
    let inputReady: Bool
    let rendererHealth: String
    let childExited: Bool
    let lastUpdatedAt: Date

    static let placeholder = AlanTerminalSurfaceStateSnapshot(
        readiness: .unready(reason: .missingSurface),
        terminalMode: .normalBuffer,
        scrollback: .empty,
        search: nil,
        readonly: false,
        secureInput: false,
        inputReady: false,
        rendererHealth: "pending",
        childExited: false,
        lastUpdatedAt: .now
    )

    func equalsIgnoringTimestamp(_ other: AlanTerminalSurfaceStateSnapshot) -> Bool {
        readiness == other.readiness
            && terminalMode == other.terminalMode
            && scrollback == other.scrollback
            && search == other.search
            && readonly == other.readonly
            && secureInput == other.secureInput
            && inputReady == other.inputReady
            && rendererHealth == other.rendererHealth
            && childExited == other.childExited
    }
}

@MainActor
final class AlanTerminalMetadataAdapter {
    func overlayState(
        renderer: TerminalRendererSnapshot,
        metadata: TerminalPaneMetadataSnapshot,
        surface: AlanTerminalSurfaceReadiness
    ) -> AlanTerminalOverlayState? {
        if metadata.processExited {
            let status = metadata.lastCommandExitCode.map { "Exit status \($0)." }
                ?? "The shell process ended."
            return AlanTerminalOverlayState(
                title: "Process exited",
                message: status,
                badge: "Exited",
                action: "Open a new pane or tab to continue.",
                debugDetail: metadata.summary
            )
        }

        if renderer.phase == .failed || surface == .unready(reason: .rendererFailed) {
            return AlanTerminalOverlayState(
                title: "Terminal cannot draw",
                message: "The terminal renderer is not available for this pane.",
                badge: "Renderer failed",
                action: "Close and reopen the pane if it does not recover.",
                debugDetail: renderer.failureReason ?? renderer.detail ?? renderer.summary
            )
        }

        switch surface {
        case .ready:
            return nil
        case .unready(reason: .missingSurface):
            return AlanTerminalOverlayState(
                title: "Terminal surface missing",
                message: "This pane does not currently have a terminal surface.",
                badge: "Missing",
                action: "Select the pane again or open a new terminal.",
                debugDetail: nil
            )
        case .unready(reason: .inputNotReady):
            return AlanTerminalOverlayState(
                title: "Terminal is starting",
                message: "Input will be available after the terminal finishes attaching.",
                badge: "Starting",
                action: nil,
                debugDetail: renderer.detail
            )
        case .unready(reason: .rendererFailed):
            return AlanTerminalOverlayState(
                title: "Terminal cannot draw",
                message: "The terminal renderer is not available for this pane.",
                badge: "Renderer failed",
                action: "Close and reopen the pane if it does not recover.",
                debugDetail: renderer.failureReason ?? renderer.detail ?? renderer.summary
            )
        case .unready(reason: .childExited):
            return AlanTerminalOverlayState(
                title: "Process exited",
                message: "The shell process ended.",
                badge: "Exited",
                action: "Open a new pane or tab to continue.",
                debugDetail: metadata.summary
            )
        case .unready(reason: .readonly):
            return AlanTerminalOverlayState(
                title: "Terminal is read-only",
                message: "This pane is not accepting input right now.",
                badge: "Read-only",
                action: nil,
                debugDetail: nil
            )
        }
    }
}

@MainActor
final class AlanTerminalSurfaceController {
    let inputAdapter = AlanTerminalInputAdapter()
    let scrollbackAdapter = AlanTerminalScrollbackAdapter()
    let metadataAdapter = AlanTerminalMetadataAdapter()

    private(set) var searchAdapter: AlanTerminalSearchAdapter?
    private(set) var clipboardAdapter = AlanTerminalSelectionClipboardAdapter(surfaceHandle: nil)
    private weak var surfaceHandle: AlanTerminalSurfaceHandle?
    private var latestRenderer = TerminalRendererSnapshot.placeholder
    private var latestMetadata = TerminalPaneMetadataSnapshot.placeholder
    private var readonly = false
    private var secureInput = false

    var isSurfaceReady: Bool {
        surfaceReadiness == .ready
    }

    var surfaceStateSnapshot: AlanTerminalSurfaceStateSnapshot {
        AlanTerminalSurfaceStateSnapshot(
            readiness: surfaceReadiness,
            terminalMode: scrollbackAdapter.state.metrics.mode,
            scrollback: scrollbackAdapter.state,
            search: searchAdapter?.state,
            readonly: readonly,
            secureInput: secureInput,
            inputReady: isSurfaceReady,
            rendererHealth: latestRenderer.phase == .failed ? "failed" : latestRenderer.phase.rawValue,
            childExited: latestMetadata.processExited,
            lastUpdatedAt: .now
        )
    }

    func bind(surfaceHandle: AlanTerminalSurfaceHandle?, paneID: String?) {
        if self.surfaceHandle !== surfaceHandle {
            self.surfaceHandle?.detach()
            self.surfaceHandle = surfaceHandle
        }
        clipboardAdapter.updateSurfaceHandle(surfaceHandle)
        if let paneID, searchAdapter?.state.paneID != paneID {
            searchAdapter = AlanTerminalSearchAdapter(paneID: paneID)
        } else if paneID == nil {
            searchAdapter = nil
        }
    }

    func attach(
        to canvasView: NSView,
        bootProfile: AlanShellBootProfile?,
        focused: Bool,
        onDiagnosticsChange: @escaping (TerminalRendererSnapshot) -> Void,
        onMetadataChange: @escaping (TerminalPaneMetadataSnapshot) -> Void
    ) {
        surfaceHandle?.configure(bootProfile: bootProfile)
        surfaceHandle?.attach(
            to: canvasView,
            focused: focused,
            onDiagnosticsChange: { [weak self] snapshot in
                guard let self else { return }
                latestRenderer = snapshot
                onDiagnosticsChange(snapshot)
            },
            onMetadataChange: { [weak self] metadata in
                guard let self else { return }
                latestMetadata = metadata
                onMetadataChange(metadata)
            }
        )
    }

    func detach() {
        surfaceHandle?.detach()
        surfaceHandle = nil
        clipboardAdapter.updateSurfaceHandle(nil)
    }

    func updateRenderer(_ renderer: TerminalRendererSnapshot) {
        latestRenderer = renderer
    }

    func updateMetadata(_ metadata: TerminalPaneMetadataSnapshot) {
        latestMetadata = metadata
    }

    func overlayState(
        renderer: TerminalRendererSnapshot,
        metadata: TerminalPaneMetadataSnapshot,
        bootProfile: AlanShellBootProfile?
    ) -> AlanTerminalOverlayState? {
        if let searchState = searchAdapter?.state,
           searchState.isActive
        {
            let status: String
            if let totalMatches = searchState.totalMatches,
               let selectedIndex = searchState.selectedIndex
            {
                status = "\(selectedIndex + 1) of \(totalMatches)"
            } else if searchState.query.isEmpty {
                status = "Type to search this pane."
            } else {
                status = "Searching this pane."
            }
            return AlanTerminalOverlayState(
                title: "Search terminal",
                message: searchState.query.isEmpty ? "Find text in this pane." : searchState.query,
                badge: "Search",
                action: status,
                debugDetail: "pane=\(searchState.paneID)"
            )
        }

        let readiness: AlanTerminalSurfaceReadiness
        if bootProfile == nil || surfaceHandle == nil {
            readiness = .unready(reason: .missingSurface)
        } else {
            readiness = surfaceReadiness
        }
        return metadataAdapter.overlayState(renderer: renderer, metadata: metadata, surface: readiness)
    }

    func sendControlText(_ text: String) -> TerminalRuntimeDeliveryResult {
        guard !text.isEmpty else {
            return .accepted(byteCount: 0, runtimePhase: surfaceHandle?.snapshot.runtimePhase)
        }
        guard let surfaceHandle else {
            return .rejected(
                errorCode: "terminal_runtime_unavailable",
                errorMessage: "No service-owned terminal surface is attached to this host."
            )
        }
        guard !surfaceHandle.snapshot.metadata.processExited else {
            return .rejected(
                errorCode: "terminal_child_exited",
                errorMessage: "The terminal process has exited.",
                runtimePhase: surfaceHandle.snapshot.runtimePhase
            )
        }
        guard surfaceHandle.isSurfaceReady else {
            return .rejected(
                errorCode: "terminal_runtime_unavailable",
                errorMessage: "The requested pane is not ready to receive terminal input.",
                runtimePhase: surfaceHandle.snapshot.runtimePhase
            )
        }
        return surfaceHandle.sendControlText(text)
    }

    func beginSearch() {
        guard let paneID = searchAdapter?.state.paneID ?? surfaceHandle?.paneID else { return }
        if searchAdapter == nil {
            searchAdapter = AlanTerminalSearchAdapter(paneID: paneID)
        }
        searchAdapter?.updateQuery(searchAdapter?.state.query ?? "")
    }

    func updateSearchQuery(_ query: String) {
        beginSearch()
        searchAdapter?.updateQuery(query)
    }

    func nextSearchMatch() {
        searchAdapter?.next()
    }

    func previousSearchMatch() {
        searchAdapter?.previous()
    }

    func dismissSearch() {
        searchAdapter?.dismiss()
    }

    private var surfaceReadiness: AlanTerminalSurfaceReadiness {
        guard let surfaceHandle else { return .unready(reason: .missingSurface) }
        if latestMetadata.processExited || surfaceHandle.snapshot.metadata.processExited {
            return .unready(reason: .childExited)
        }
        if latestRenderer.phase == .failed || surfaceHandle.snapshot.renderer.phase == .failed {
            return .unready(reason: .rendererFailed)
        }
        if readonly {
            return .unready(reason: .readonly)
        }
        guard surfaceHandle.isSurfaceReady else {
            return .unready(reason: .inputNotReady)
        }
        return .ready
    }
}

#if canImport(GhosttyKit)
extension AlanTerminalSurfaceController {
    var ghosttySurfaceHandle: AlanGhosttyEventSurfaceHandle? {
        surfaceHandle as? AlanGhosttyEventSurfaceHandle
    }

    func keyTranslationMods(for mods: ghostty_input_mods_e) -> ghostty_input_mods_e {
        ghosttySurfaceHandle?.keyTranslationMods(for: mods) ?? mods
    }

    func sendKey(_ keyEvent: ghostty_input_key_s) -> Bool {
        ghosttySurfaceHandle?.sendKey(keyEvent) ?? false
    }

    func keyIsBinding(
        _ keyEvent: ghostty_input_key_s,
        flags: UnsafeMutablePointer<ghostty_binding_flags_e>?
    ) -> Bool {
        ghosttySurfaceHandle?.keyIsBinding(keyEvent, flags: flags) ?? false
    }

    func sendText(_ text: String) {
        ghosttySurfaceHandle?.sendText(text)
    }

    func sendPreedit(_ text: String?) {
        ghosttySurfaceHandle?.sendPreedit(text)
    }

    func sendMousePosition(x: Double, y: Double, mods: ghostty_input_mods_e) {
        ghosttySurfaceHandle?.sendMousePosition(x: x, y: y, mods: mods)
    }

    func sendMouseButton(
        state: ghostty_input_mouse_state_e,
        button: ghostty_input_mouse_button_e,
        mods: ghostty_input_mods_e
    ) -> Bool {
        ghosttySurfaceHandle?.sendMouseButton(state: state, button: button, mods: mods) ?? false
    }

    func sendMouseScroll(x: Double, y: Double, mods: ghostty_input_scroll_mods_t) {
        ghosttySurfaceHandle?.sendMouseScroll(x: x, y: y, mods: mods)
    }

    func sendMousePressure(stage: UInt32, pressure: Double) {
        ghosttySurfaceHandle?.sendMousePressure(stage: stage, pressure: pressure)
    }

    func readSelectionText() -> String? {
        ghosttySurfaceHandle?.readSelectionText()
    }

    func hasSelection() -> Bool {
        ghosttySurfaceHandle?.hasSelection() ?? false
    }

    func imeRect(in view: NSView) -> NSRect? {
        ghosttySurfaceHandle?.imeRect(in: view)
    }
}
#endif
#endif
