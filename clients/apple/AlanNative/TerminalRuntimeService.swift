import Foundation

#if os(macOS)
import AppKit
#if canImport(GhosttyKit)
import GhosttyKit
#endif

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
    let runtimePhase: String?
    let errorCode: String?
    let errorMessage: String?

    var applied: Bool {
        code == .accepted
    }

    static func accepted(
        byteCount: Int,
        runtimePhase: String? = nil
    ) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .accepted,
            acceptedBytes: byteCount,
            runtimePhase: runtimePhase,
            errorCode: nil,
            errorMessage: nil
        )
    }

    static func queued(
        byteCount: Int,
        runtimePhase: String? = nil
    ) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .queued,
            acceptedBytes: byteCount,
            runtimePhase: runtimePhase,
            errorCode: nil,
            errorMessage: nil
        )
    }

    static func rejected(
        errorCode: String,
        errorMessage: String,
        runtimePhase: String? = nil
    ) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .rejected,
            acceptedBytes: 0,
            runtimePhase: runtimePhase,
            errorCode: errorCode,
            errorMessage: errorMessage
        )
    }

    static func missingTarget(
        errorMessage: String,
        runtimePhase: String? = nil
    ) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .missingTarget,
            acceptedBytes: 0,
            runtimePhase: runtimePhase,
            errorCode: "terminal_runtime_missing",
            errorMessage: errorMessage
        )
    }

    static func unavailable(
        errorMessage: String,
        runtimePhase: String? = nil
    ) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .unavailableRuntime,
            acceptedBytes: 0,
            runtimePhase: runtimePhase,
            errorCode: "terminal_runtime_unavailable",
            errorMessage: errorMessage
        )
    }

    static func timeout(
        errorMessage: String,
        runtimePhase: String? = nil
    ) -> TerminalRuntimeDeliveryResult {
        TerminalRuntimeDeliveryResult(
            code: .timeout,
            acceptedBytes: 0,
            runtimePhase: runtimePhase,
            errorCode: "terminal_runtime_timeout",
            errorMessage: errorMessage
        )
    }
}

enum AlanGhosttyBootstrapPhase: String, Equatable {
    case pending
    case ready
    case failed
}

struct AlanGhosttyBootstrapDiagnostics: Equatable {
    let phase: AlanGhosttyBootstrapPhase
    let summary: String
    let detail: String?
    let failureReason: String?
    let dependencies: GhosttyIntegrationStatus
    let lastUpdatedAt: Date

    var isReady: Bool {
        phase == .ready
    }

    static func pending(
        dependencies: GhosttyIntegrationStatus = GhosttyIntegrationStatus.discover()
    ) -> AlanGhosttyBootstrapDiagnostics {
        AlanGhosttyBootstrapDiagnostics(
            phase: .pending,
            summary: "Ghostty process bootstrap has not started.",
            detail: nil,
            failureReason: nil,
            dependencies: dependencies,
            lastUpdatedAt: .now
        )
    }
}

@MainActor
protocol AlanGhosttyProcessBootstrap: AnyObject {
    var diagnostics: AlanGhosttyBootstrapDiagnostics { get }
    func ensureReady() -> AlanGhosttyBootstrapDiagnostics
}

@MainActor
final class AlanDefaultGhosttyProcessBootstrap: AlanGhosttyProcessBootstrap {
    static let shared = AlanDefaultGhosttyProcessBootstrap()

    private var cachedDiagnostics = AlanGhosttyBootstrapDiagnostics.pending()

    var diagnostics: AlanGhosttyBootstrapDiagnostics {
        cachedDiagnostics
    }

    func ensureReady() -> AlanGhosttyBootstrapDiagnostics {
        if cachedDiagnostics.phase == .ready || cachedDiagnostics.phase == .failed {
            return cachedDiagnostics
        }

        let dependencies = GhosttyIntegrationStatus.discover()
#if canImport(GhosttyKit)
        scrubInheritedTerminalEnvironment()
        configureGhosttyProcessEnvironment(from: dependencies)

        let result = ghostty_init(UInt(CommandLine.argc), CommandLine.unsafeArgv)
        guard result == GHOSTTY_SUCCESS else {
            cachedDiagnostics = AlanGhosttyBootstrapDiagnostics(
                phase: .failed,
                summary: "ghostty_init failed.",
                detail: "libghostty returned \(result).",
                failureReason: "Ghostty library initialization failed.",
                dependencies: dependencies,
                lastUpdatedAt: .now
            )
            return cachedDiagnostics
        }

        cachedDiagnostics = AlanGhosttyBootstrapDiagnostics(
            phase: .ready,
            summary: "Ghostty process bootstrap initialized.",
            detail: dependencies.summary,
            failureReason: nil,
            dependencies: dependencies,
            lastUpdatedAt: .now
        )
#else
        cachedDiagnostics = AlanGhosttyBootstrapDiagnostics(
            phase: .failed,
            summary: "GhosttyKit is not linked into this build.",
            detail: dependencies.summary,
            failureReason: "GhosttyKit framework is unavailable at compile time.",
            dependencies: dependencies,
            lastUpdatedAt: .now
        )
#endif
        return cachedDiagnostics
    }

#if canImport(GhosttyKit)
    private func configureGhosttyProcessEnvironment(from integration: GhosttyIntegrationStatus) {
        guard let resourcesPath = integration.resourcesPath else { return }
        let shouldOverride = getenv("ALAN_GHOSTTY_RESOURCES_DIR") != nil
            || getenv("GHOSTTY_RESOURCES_DIR") == nil
        guard shouldOverride else { return }
        _ = resourcesPath.withCString { path in
            setenv("GHOSTTY_RESOURCES_DIR", path, 1)
        }
    }

    private func scrubInheritedTerminalEnvironment() {
        let exactKeys = [
            "TERM",
            "TERM_PROGRAM",
            "TERM_PROGRAM_VERSION",
            "COLORTERM",
            "TERMINFO",
            "TERMINFO_DIRS",
            "VTE_VERSION",
            "PWD",
            "SHLVL",
            "_",
            "STARSHIP_SHELL",
            "STARSHIP_SESSION_KEY",
            "RBENV_SHELL",
            "GHOSTTY_SURFACE_ID",
            "GHOSTTY_SHELL_FEATURES",
            "GHOSTTY_SHELL_INTEGRATION_XDG_DIR",
            "GHOSTTY_BIN_DIR",
            "NO_COLOR",
        ]
        exactKeys.forEach { unsetenv($0) }

        for key in ProcessInfo.processInfo.environment.keys {
            if key.hasPrefix("WARP_") || key.hasPrefix("CODEX_") {
                unsetenv(key)
            }
        }
    }
#endif
}

enum AlanTerminalSurfaceLifecyclePhase: String, Equatable {
    case pending
    case bootstrapping
    case attachable
    case attached
    case closing
    case closed
    case failed
}

enum AlanTerminalSurfaceTeardownStatus: String, Equatable {
    case notStarted = "not_started"
    case closing
    case completed
    case interrupted
}

struct AlanTerminalSurfaceSnapshot: Equatable {
    let paneID: String
    let lifecyclePhase: AlanTerminalSurfaceLifecyclePhase
    let renderer: TerminalRendererSnapshot
    let metadata: TerminalPaneMetadataSnapshot
    let lastDelivery: TerminalRuntimeDeliveryResult?
    let teardownStatus: AlanTerminalSurfaceTeardownStatus
    let attachedViewCount: Int
    let lastUpdatedAt: Date

    var runtimePhase: String {
        renderer.phase.rawValue
    }

    static func pending(paneID: String) -> AlanTerminalSurfaceSnapshot {
        AlanTerminalSurfaceSnapshot(
            paneID: paneID,
            lifecyclePhase: .pending,
            renderer: .placeholder,
            metadata: .placeholder,
            lastDelivery: nil,
            teardownStatus: .notStarted,
            attachedViewCount: 0,
            lastUpdatedAt: .now
        )
    }
}

@MainActor
protocol AlanTerminalSurfaceHandle: AnyObject {
    var paneID: String { get }
    var snapshot: AlanTerminalSurfaceSnapshot { get }
    var isSurfaceReady: Bool { get }

    func configure(bootProfile: AlanShellBootProfile?)
    func attach(
        to canvasView: NSView,
        focused: Bool,
        onDiagnosticsChange: @escaping (TerminalRendererSnapshot) -> Void,
        onMetadataChange: @escaping (TerminalPaneMetadataSnapshot) -> Void
    )
    func detach()
    func updateHostRuntimeSnapshot(_ snapshot: TerminalHostRuntimeSnapshot)
    func sendControlText(_ text: String) -> TerminalRuntimeDeliveryResult
    @discardableResult
    func teardown() -> AlanTerminalSurfaceTeardownStatus
}

#if canImport(GhosttyKit)
@MainActor
protocol AlanGhosttyEventSurfaceHandle: AlanTerminalSurfaceHandle {
    func keyTranslationMods(for mods: ghostty_input_mods_e) -> ghostty_input_mods_e
    func sendKey(_ keyEvent: ghostty_input_key_s) -> Bool
    func keyIsBinding(
        _ keyEvent: ghostty_input_key_s,
        flags: UnsafeMutablePointer<ghostty_binding_flags_e>?
    ) -> Bool
    func sendText(_ text: String)
    func sendPreedit(_ text: String?)
    func sendMousePosition(x: Double, y: Double, mods: ghostty_input_mods_e)
    func sendMouseButton(
        state: ghostty_input_mouse_state_e,
        button: ghostty_input_mouse_button_e,
        mods: ghostty_input_mods_e
    ) -> Bool
    func sendMouseScroll(x: Double, y: Double, mods: ghostty_input_scroll_mods_t)
    func sendMousePressure(stage: UInt32, pressure: Double)
    func readSelectionText() -> String?
    func hasSelection() -> Bool
    func imeRect(in view: NSView) -> NSRect?
}
#endif

@MainActor
final class AlanGhosttySurfaceHandle: AlanTerminalSurfaceHandle {
    let paneID: String

    private let bootstrap: AlanGhosttyProcessBootstrap
    private var bootProfile: AlanShellBootProfile?
    private var currentSnapshot: AlanTerminalSurfaceSnapshot
    private var latestHostRuntime: TerminalHostRuntimeSnapshot?
#if canImport(GhosttyKit)
    private let liveHost = AlanGhosttyLiveHost()
#endif

    init(paneID: String, bootstrap: AlanGhosttyProcessBootstrap) {
        self.paneID = paneID
        self.bootstrap = bootstrap
        self.currentSnapshot = .pending(paneID: paneID)
    }

    var snapshot: AlanTerminalSurfaceSnapshot {
        currentSnapshot
    }

    var isSurfaceReady: Bool {
#if canImport(GhosttyKit)
        return currentSnapshot.teardownStatus != .completed && liveHost.isSurfaceReady
#else
        return false
#endif
    }

    func configure(bootProfile: AlanShellBootProfile?) {
        self.bootProfile = bootProfile
        guard currentSnapshot.teardownStatus != .completed else { return }
        updateSnapshot(
            lifecyclePhase: bootProfile == nil ? .pending : .attachable,
            metadata: metadataWithBootProfile(bootProfile)
        )
    }

    func attach(
        to canvasView: NSView,
        focused: Bool,
        onDiagnosticsChange: @escaping (TerminalRendererSnapshot) -> Void,
        onMetadataChange: @escaping (TerminalPaneMetadataSnapshot) -> Void
    ) {
        guard currentSnapshot.teardownStatus != .completed else {
            onDiagnosticsChange(currentSnapshot.renderer)
            onMetadataChange(currentSnapshot.metadata)
            return
        }

        updateSnapshot(lifecyclePhase: .bootstrapping, attachedViewCount: 1)
        let diagnostics = bootstrap.ensureReady()
        guard diagnostics.isReady else {
            let renderer = TerminalRendererSnapshot(
                kind: .ghosttyLive,
                phase: .failed,
                summary: diagnostics.summary,
                detail: diagnostics.detail,
                failureReason: diagnostics.failureReason,
                recentEvents: currentSnapshot.renderer.recentEvents
            )
            updateSnapshot(lifecyclePhase: .failed, renderer: renderer)
            onDiagnosticsChange(renderer)
            onMetadataChange(currentSnapshot.metadata)
            return
        }

#if canImport(GhosttyKit)
        guard let canvasView = canvasView as? AlanGhosttyCanvasView else {
            let renderer = TerminalRendererSnapshot(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "Ghostty surface cannot attach to this canvas.",
                detail: nil,
                failureReason: "Expected AlanGhosttyCanvasView.",
                recentEvents: currentSnapshot.renderer.recentEvents
            )
            updateSnapshot(lifecyclePhase: .failed, renderer: renderer)
            onDiagnosticsChange(renderer)
            return
        }

        liveHost.onDiagnosticsChange = { [weak self] snapshot in
            guard let self else { return }
            updateSnapshot(
                lifecyclePhase: snapshot.phase == .failed ? .failed : .attached,
                renderer: snapshot
            )
            onDiagnosticsChange(snapshot)
        }
        liveHost.onMetadataChange = { [weak self] metadata in
            guard let self else { return }
            updateSnapshot(metadata: metadata)
            onMetadataChange(metadata)
        }
        liveHost.attach(to: canvasView, bootProfile: bootProfile, focused: focused)
        updateSnapshot(
            lifecyclePhase: liveHost.isSurfaceReady ? .attached : .attachable,
            metadata: liveHost.latestMetadata
        )
#else
        let renderer = TerminalRendererSnapshot(
            kind: .scaffold,
            phase: .failed,
            summary: "GhosttyKit is not linked into this build.",
            detail: nil,
            failureReason: "GhosttyKit framework is unavailable at compile time.",
            recentEvents: currentSnapshot.renderer.recentEvents
        )
        updateSnapshot(lifecyclePhase: .failed, renderer: renderer)
        onDiagnosticsChange(renderer)
#endif
    }

    func detach() {
        updateSnapshot(attachedViewCount: 0)
    }

    func updateHostRuntimeSnapshot(_ snapshot: TerminalHostRuntimeSnapshot) {
        latestHostRuntime = snapshot
    }

    func sendControlText(_ text: String) -> TerminalRuntimeDeliveryResult {
        guard !text.isEmpty else {
            return recordDelivery(.accepted(byteCount: 0, runtimePhase: currentSnapshot.runtimePhase))
        }
        guard currentSnapshot.teardownStatus != .completed else {
            return recordDelivery(
                .rejected(
                    errorCode: "terminal_runtime_closed",
                    errorMessage: "The requested pane runtime has already closed.",
                    runtimePhase: currentSnapshot.runtimePhase
                )
            )
        }
        guard bootstrap.ensureReady().isReady else {
            return recordDelivery(
                .unavailable(
                    errorMessage: bootstrap.diagnostics.failureReason ?? bootstrap.diagnostics.summary,
                    runtimePhase: currentSnapshot.runtimePhase
                )
            )
        }
        guard isSurfaceReady else {
            return recordDelivery(
                .unavailable(
                    errorMessage: "The requested pane is not ready to receive terminal input.",
                    runtimePhase: currentSnapshot.runtimePhase
                )
            )
        }

#if canImport(GhosttyKit)
        liveHost.sendText(text)
        return recordDelivery(
            .accepted(
                byteCount: text.lengthOfBytes(using: .utf8),
                runtimePhase: currentSnapshot.runtimePhase
            )
        )
#else
        return recordDelivery(
            .rejected(
                errorCode: "ghostty_unavailable",
                errorMessage: "GhosttyKit is not linked into this build.",
                runtimePhase: currentSnapshot.runtimePhase
            )
        )
#endif
    }

    @discardableResult
    func teardown() -> AlanTerminalSurfaceTeardownStatus {
        guard currentSnapshot.teardownStatus != .completed else { return .completed }
        updateSnapshot(lifecyclePhase: .closing, teardownStatus: .closing)
#if canImport(GhosttyKit)
        liveHost.teardown()
#endif
        updateSnapshot(
            lifecyclePhase: .closed,
            metadata: .placeholder,
            teardownStatus: .completed,
            attachedViewCount: 0
        )
        return .completed
    }

    private func metadataWithBootProfile(
        _ bootProfile: AlanShellBootProfile?
    ) -> TerminalPaneMetadataSnapshot {
        guard let bootProfile else { return currentSnapshot.metadata }
        return TerminalPaneMetadataSnapshot(
            title: currentSnapshot.metadata.title,
            workingDirectory: bootProfile.workingDirectory,
            summary: currentSnapshot.metadata.summary,
            attention: currentSnapshot.metadata.attention,
            processExited: currentSnapshot.metadata.processExited,
            lastCommandExitCode: currentSnapshot.metadata.lastCommandExitCode,
            lastUpdatedAt: currentSnapshot.metadata.lastUpdatedAt
        )
    }

    private func recordDelivery(
        _ delivery: TerminalRuntimeDeliveryResult
    ) -> TerminalRuntimeDeliveryResult {
        updateSnapshot(lastDelivery: delivery)
        return delivery
    }

    private func updateSnapshot(
        lifecyclePhase: AlanTerminalSurfaceLifecyclePhase? = nil,
        renderer: TerminalRendererSnapshot? = nil,
        metadata: TerminalPaneMetadataSnapshot? = nil,
        lastDelivery: TerminalRuntimeDeliveryResult? = nil,
        teardownStatus: AlanTerminalSurfaceTeardownStatus? = nil,
        attachedViewCount: Int? = nil
    ) {
        currentSnapshot = AlanTerminalSurfaceSnapshot(
            paneID: paneID,
            lifecyclePhase: lifecyclePhase ?? currentSnapshot.lifecyclePhase,
            renderer: renderer ?? currentSnapshot.renderer,
            metadata: metadata ?? currentSnapshot.metadata,
            lastDelivery: lastDelivery ?? currentSnapshot.lastDelivery,
            teardownStatus: teardownStatus ?? currentSnapshot.teardownStatus,
            attachedViewCount: attachedViewCount ?? currentSnapshot.attachedViewCount,
            lastUpdatedAt: .now
        )
    }
}

#if canImport(GhosttyKit)
extension AlanGhosttySurfaceHandle: AlanGhosttyEventSurfaceHandle {
    func keyTranslationMods(for mods: ghostty_input_mods_e) -> ghostty_input_mods_e {
        liveHost.keyTranslationMods(for: mods)
    }

    func sendKey(_ keyEvent: ghostty_input_key_s) -> Bool {
        liveHost.sendKey(keyEvent)
    }

    func keyIsBinding(
        _ keyEvent: ghostty_input_key_s,
        flags: UnsafeMutablePointer<ghostty_binding_flags_e>?
    ) -> Bool {
        liveHost.keyIsBinding(keyEvent, flags: flags)
    }

    func sendText(_ text: String) {
        liveHost.sendText(text)
    }

    func sendPreedit(_ text: String?) {
        liveHost.sendPreedit(text)
    }

    func sendMousePosition(x: Double, y: Double, mods: ghostty_input_mods_e) {
        liveHost.sendMousePosition(x: x, y: y, mods: mods)
    }

    func sendMouseButton(
        state: ghostty_input_mouse_state_e,
        button: ghostty_input_mouse_button_e,
        mods: ghostty_input_mods_e
    ) -> Bool {
        liveHost.sendMouseButton(state: state, button: button, mods: mods)
    }

    func sendMouseScroll(x: Double, y: Double, mods: ghostty_input_scroll_mods_t) {
        liveHost.sendMouseScroll(x: x, y: y, mods: mods)
    }

    func sendMousePressure(stage: UInt32, pressure: Double) {
        liveHost.sendMousePressure(stage: stage, pressure: pressure)
    }

    func readSelectionText() -> String? {
        liveHost.readSelectionText()
    }

    func hasSelection() -> Bool {
        liveHost.hasSelection()
    }

    func imeRect(in view: NSView) -> NSRect? {
        liveHost.imeRect(in: view)
    }
}
#endif

@MainActor
protocol AlanTerminalRuntimeService: AnyObject {
    var diagnostics: AlanGhosttyBootstrapDiagnostics { get }
    var registeredPaneIDs: Set<String> { get }

    @discardableResult
    func ensureReady() -> AlanGhosttyBootstrapDiagnostics
    func surfaceHandle(for paneID: String, bootProfile: AlanShellBootProfile?) -> AlanTerminalSurfaceHandle
    func existingSurfaceHandle(for paneID: String) -> AlanTerminalSurfaceHandle?
    func snapshot(for paneID: String) -> AlanTerminalSurfaceSnapshot?
    func sendText(to paneID: String, text: String) -> TerminalRuntimeDeliveryResult
    @discardableResult
    func finalizePane(_ paneID: String) -> AlanTerminalSurfaceTeardownStatus
    func finalizePanes(excluding activePaneIDs: Set<String>)
}

@MainActor
final class AlanWindowTerminalRuntimeService: AlanTerminalRuntimeService {
    typealias SurfaceFactory = (String, AlanGhosttyProcessBootstrap) -> AlanTerminalSurfaceHandle

    private let bootstrap: AlanGhosttyProcessBootstrap
    private let makeSurfaceHandle: SurfaceFactory
    private var handlesByPaneID: [String: AlanTerminalSurfaceHandle] = [:]

    init(surfaceFactory: SurfaceFactory? = nil) {
        self.bootstrap = AlanDefaultGhosttyProcessBootstrap.shared
        self.makeSurfaceHandle = surfaceFactory ?? { paneID, bootstrap in
            AlanGhosttySurfaceHandle(paneID: paneID, bootstrap: bootstrap)
        }
    }

    init(
        bootstrap: AlanGhosttyProcessBootstrap,
        surfaceFactory: SurfaceFactory? = nil
    ) {
        self.bootstrap = bootstrap
        self.makeSurfaceHandle = surfaceFactory ?? { paneID, bootstrap in
            AlanGhosttySurfaceHandle(paneID: paneID, bootstrap: bootstrap)
        }
    }

    var diagnostics: AlanGhosttyBootstrapDiagnostics {
        bootstrap.diagnostics
    }

    var registeredPaneIDs: Set<String> {
        Set(handlesByPaneID.keys)
    }

    @discardableResult
    func ensureReady() -> AlanGhosttyBootstrapDiagnostics {
        bootstrap.ensureReady()
    }

    func surfaceHandle(
        for paneID: String,
        bootProfile: AlanShellBootProfile?
    ) -> AlanTerminalSurfaceHandle {
        ensureReady()
        if let handle = handlesByPaneID[paneID] {
            handle.configure(bootProfile: bootProfile)
            return handle
        }
        let handle = makeSurfaceHandle(paneID, bootstrap)
        handle.configure(bootProfile: bootProfile)
        handlesByPaneID[paneID] = handle
        return handle
    }

    func existingSurfaceHandle(for paneID: String) -> AlanTerminalSurfaceHandle? {
        handlesByPaneID[paneID]
    }

    func snapshot(for paneID: String) -> AlanTerminalSurfaceSnapshot? {
        handlesByPaneID[paneID]?.snapshot
    }

    func sendText(to paneID: String, text: String) -> TerminalRuntimeDeliveryResult {
        guard let handle = handlesByPaneID[paneID] else {
            return .missingTarget(
                errorMessage: "The requested pane does not have a service-owned terminal runtime."
            )
        }
        return handle.sendControlText(text)
    }

    @discardableResult
    func finalizePane(_ paneID: String) -> AlanTerminalSurfaceTeardownStatus {
        guard let handle = handlesByPaneID.removeValue(forKey: paneID) else {
            return .notStarted
        }
        return handle.teardown()
    }

    func finalizePanes(excluding activePaneIDs: Set<String>) {
        let stalePaneIDs = Set(handlesByPaneID.keys).subtracting(activePaneIDs)
        stalePaneIDs.forEach { finalizePane($0) }
    }
}

@MainActor
final class FakeAlanGhosttyProcessBootstrap: AlanGhosttyProcessBootstrap {
    private(set) var ensureCallCount = 0
    var nextDiagnostics: AlanGhosttyBootstrapDiagnostics

    init(
        nextDiagnostics: AlanGhosttyBootstrapDiagnostics = AlanGhosttyBootstrapDiagnostics(
            phase: .ready,
            summary: "Fake Ghostty bootstrap ready.",
            detail: nil,
            failureReason: nil,
            dependencies: GhosttyIntegrationStatus.discover(),
            lastUpdatedAt: .now
        )
    ) {
        self.nextDiagnostics = nextDiagnostics
        self.cachedDiagnostics = .pending(dependencies: nextDiagnostics.dependencies)
    }

    private var cachedDiagnostics: AlanGhosttyBootstrapDiagnostics

    var diagnostics: AlanGhosttyBootstrapDiagnostics {
        cachedDiagnostics
    }

    func ensureReady() -> AlanGhosttyBootstrapDiagnostics {
        if cachedDiagnostics.phase == .ready || cachedDiagnostics.phase == .failed {
            return cachedDiagnostics
        }
        ensureCallCount += 1
        cachedDiagnostics = nextDiagnostics
        return cachedDiagnostics
    }
}

@MainActor
final class FakeAlanTerminalSurfaceHandle: AlanTerminalSurfaceHandle {
    let paneID: String
    private(set) var configureCount = 0
    private(set) var attachCount = 0
    private(set) var detachCount = 0
    private(set) var teardownCount = 0
    private(set) var deliveredText: [String] = []
    var deliveryResult: TerminalRuntimeDeliveryResult?
    var ready = true
    private var currentSnapshot: AlanTerminalSurfaceSnapshot

    init(paneID: String) {
        self.paneID = paneID
        self.currentSnapshot = .pending(paneID: paneID)
    }

    var snapshot: AlanTerminalSurfaceSnapshot {
        currentSnapshot
    }

    var isSurfaceReady: Bool {
        ready && currentSnapshot.teardownStatus != .completed
    }

    func configure(bootProfile: AlanShellBootProfile?) {
        configureCount += 1
        updateSnapshot(lifecyclePhase: bootProfile == nil ? .pending : .attachable)
    }

    func attach(
        to canvasView: NSView,
        focused: Bool,
        onDiagnosticsChange: @escaping (TerminalRendererSnapshot) -> Void,
        onMetadataChange: @escaping (TerminalPaneMetadataSnapshot) -> Void
    ) {
        attachCount += 1
        updateSnapshot(lifecyclePhase: .attached, attachedViewCount: 1)
        onDiagnosticsChange(currentSnapshot.renderer)
        onMetadataChange(currentSnapshot.metadata)
    }

    func detach() {
        detachCount += 1
        updateSnapshot(attachedViewCount: 0)
    }

    func updateHostRuntimeSnapshot(_ snapshot: TerminalHostRuntimeSnapshot) {}

    func sendControlText(_ text: String) -> TerminalRuntimeDeliveryResult {
        deliveredText.append(text)
        let result = deliveryResult
            ?? .accepted(
                byteCount: text.lengthOfBytes(using: .utf8),
                runtimePhase: currentSnapshot.runtimePhase
            )
        updateSnapshot(lastDelivery: result)
        return result
    }

    @discardableResult
    func teardown() -> AlanTerminalSurfaceTeardownStatus {
        guard currentSnapshot.teardownStatus != .completed else { return .completed }
        teardownCount += 1
        updateSnapshot(
            lifecyclePhase: .closed,
            teardownStatus: .completed,
            attachedViewCount: 0
        )
        return .completed
    }

    private func updateSnapshot(
        lifecyclePhase: AlanTerminalSurfaceLifecyclePhase? = nil,
        lastDelivery: TerminalRuntimeDeliveryResult? = nil,
        teardownStatus: AlanTerminalSurfaceTeardownStatus? = nil,
        attachedViewCount: Int? = nil
    ) {
        currentSnapshot = AlanTerminalSurfaceSnapshot(
            paneID: paneID,
            lifecyclePhase: lifecyclePhase ?? currentSnapshot.lifecyclePhase,
            renderer: currentSnapshot.renderer,
            metadata: currentSnapshot.metadata,
            lastDelivery: lastDelivery ?? currentSnapshot.lastDelivery,
            teardownStatus: teardownStatus ?? currentSnapshot.teardownStatus,
            attachedViewCount: attachedViewCount ?? currentSnapshot.attachedViewCount,
            lastUpdatedAt: .now
        )
    }
}

@MainActor
final class FakeAlanTerminalRuntimeService: AlanTerminalRuntimeService {
    let bootstrap: FakeAlanGhosttyProcessBootstrap
    private(set) var handlesByPaneID: [String: FakeAlanTerminalSurfaceHandle] = [:]

    init() {
        self.bootstrap = FakeAlanGhosttyProcessBootstrap()
    }

    init(bootstrap: FakeAlanGhosttyProcessBootstrap) {
        self.bootstrap = bootstrap
    }

    var diagnostics: AlanGhosttyBootstrapDiagnostics {
        bootstrap.diagnostics
    }

    var registeredPaneIDs: Set<String> {
        Set(handlesByPaneID.keys)
    }

    @discardableResult
    func ensureReady() -> AlanGhosttyBootstrapDiagnostics {
        bootstrap.ensureReady()
    }

    func surfaceHandle(
        for paneID: String,
        bootProfile: AlanShellBootProfile?
    ) -> AlanTerminalSurfaceHandle {
        ensureReady()
        if let handle = handlesByPaneID[paneID] {
            handle.configure(bootProfile: bootProfile)
            return handle
        }
        let handle = FakeAlanTerminalSurfaceHandle(paneID: paneID)
        handle.configure(bootProfile: bootProfile)
        handlesByPaneID[paneID] = handle
        return handle
    }

    func existingSurfaceHandle(for paneID: String) -> AlanTerminalSurfaceHandle? {
        handlesByPaneID[paneID]
    }

    func snapshot(for paneID: String) -> AlanTerminalSurfaceSnapshot? {
        handlesByPaneID[paneID]?.snapshot
    }

    func sendText(to paneID: String, text: String) -> TerminalRuntimeDeliveryResult {
        guard let handle = handlesByPaneID[paneID] else {
            return .missingTarget(
                errorMessage: "The requested pane does not have a fake terminal runtime."
            )
        }
        return handle.sendControlText(text)
    }

    @discardableResult
    func finalizePane(_ paneID: String) -> AlanTerminalSurfaceTeardownStatus {
        guard let handle = handlesByPaneID.removeValue(forKey: paneID) else {
            return .notStarted
        }
        return handle.teardown()
    }

    func finalizePanes(excluding activePaneIDs: Set<String>) {
        let stalePaneIDs = Set(handlesByPaneID.keys).subtracting(activePaneIDs)
        stalePaneIDs.forEach { finalizePane($0) }
    }
}
#endif
