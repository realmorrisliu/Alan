import Foundation

#if os(macOS) && canImport(GhosttyKit)
import AppKit
import GhosttyKit
import OSLog
import QuartzCore

final class AlanGhosttyLiveHost: NSObject {
    var onDiagnosticsChange: ((TerminalRendererSnapshot) -> Void)?
    var onMetadataChange: ((TerminalPaneMetadataSnapshot) -> Void)?

    private let logger = Logger(
        subsystem: "com.realmorrisliu.AlanNative",
        category: "GhosttyLiveHost"
    )

    private weak var canvasView: AlanGhosttyCanvasView?
    private var config: ghostty_config_t?
    private var app: ghostty_app_t?
    private var surface: ghostty_surface_t?
    private var bootProfile: AlanShellBootProfile?
    private var envStorage: [(UnsafeMutablePointer<CChar>, UnsafeMutablePointer<CChar>)] = []
    private let tickScheduleLock = NSLock()
    private var tickScheduled = false
    private var appObservers: [NSObjectProtocol] = []
    private var didEmitFirstRefresh = false
    private var diagnostics = TerminalRendererSnapshot.placeholder
    private var metadata = TerminalPaneMetadataSnapshot.placeholder

    func attach(
        to canvasView: AlanGhosttyCanvasView,
        bootProfile: AlanShellBootProfile?,
        focused: Bool
    ) {
        let canvasChanged = self.canvasView !== canvasView
        let bootProfileChanged = self.bootProfile != bootProfile

        self.canvasView = canvasView
        self.bootProfile = bootProfile

        guard let bootProfile else {
            teardownSurface()
            transition(
                kind: .ghosttyLive,
                phase: .pending,
                summary: "Ghostty host is ready but no pane is selected.",
                detail: "Select a pane to build a terminal boot contract."
            )
            return
        }

        guard ensureApp() else {
            return
        }

        if canvasView.window == nil {
            transition(
                kind: .ghosttyLive,
                phase: app == nil ? .pending : .appReady,
                summary: "Ghostty app is ready and waiting for a window attachment.",
                detail: bootProfile.command.summary
            )
            return
        }

        if surface == nil || bootProfileChanged || canvasChanged {
            createSurface(on: canvasView, bootProfile: bootProfile)
        }

        synchronizeViewState(focused: focused)
    }

    func synchronizeViewState(focused: Bool) {
        guard let canvasView, let surface else { return }
        synchronizeDrawableMetrics(for: canvasView)
        ghostty_surface_set_focus(surface, focused)
        let visible = canvasView.window?.occlusionState.contains(.visible) ?? false
        ghostty_surface_set_occlusion(surface, visible)
        ghostty_surface_refresh(surface)
        markFirstRefreshIfNeeded(on: canvasView)
    }

    var latestMetadata: TerminalPaneMetadataSnapshot {
        metadata
    }

    var isSurfaceReady: Bool {
        surface != nil
    }

    func keyTranslationMods(for mods: ghostty_input_mods_e) -> ghostty_input_mods_e {
        guard let surface else { return mods }
        return ghostty_surface_key_translation_mods(surface, mods)
    }

    func sendKey(_ keyEvent: ghostty_input_key_s) -> Bool {
        guard let surface else { return false }
        let handled = ghostty_surface_key(surface, keyEvent)
        ghostty_surface_refresh(surface)
        return handled
    }

    func keyIsBinding(_ keyEvent: ghostty_input_key_s, flags: UnsafeMutablePointer<ghostty_binding_flags_e>?) -> Bool {
        guard let surface else { return false }
        return ghostty_surface_key_is_binding(surface, keyEvent, flags)
    }

    func sendText(_ text: String) {
        guard let surface, !text.isEmpty else { return }
        text.withCString { cString in
            ghostty_surface_text(surface, cString, UInt(strlen(cString)))
        }
        ghostty_surface_refresh(surface)
        updateMetadata(summary: "input committed", attention: .active)
    }

    func sendPreedit(_ text: String?) {
        guard let surface else { return }
        guard let text, !text.isEmpty else {
            ghostty_surface_preedit(surface, nil, 0)
            return
        }

        text.withCString { cString in
            ghostty_surface_preedit(surface, cString, UInt(strlen(cString)))
        }
    }

    func sendMousePosition(x: Double, y: Double, mods: ghostty_input_mods_e) {
        guard let surface else { return }
        ghostty_surface_mouse_pos(surface, x, y, mods)
    }

    func sendMouseButton(
        state: ghostty_input_mouse_state_e,
        button: ghostty_input_mouse_button_e,
        mods: ghostty_input_mods_e
    ) -> Bool {
        guard let surface else { return false }
        return ghostty_surface_mouse_button(surface, state, button, mods)
    }

    func sendMouseScroll(x: Double, y: Double, mods: ghostty_input_scroll_mods_t) {
        guard let surface else { return }
        ghostty_surface_mouse_scroll(surface, x, y, mods)
    }

    func sendMousePressure(stage: UInt32, pressure: Double) {
        guard let surface else { return }
        ghostty_surface_mouse_pressure(surface, stage, pressure)
    }

    func readSelectionText() -> String? {
        guard let surface else { return nil }
        var text = ghostty_text_s()
        guard ghostty_surface_read_selection(surface, &text) else { return nil }
        defer { ghostty_surface_free_text(surface, &text) }
        guard let raw = text.text else { return nil }
        return String(cString: raw)
    }

    func hasSelection() -> Bool {
        guard let surface else { return false }
        return ghostty_surface_has_selection(surface)
    }

    func imeRect(in view: NSView) -> NSRect? {
        guard let surface else { return nil }

        var x: Double = 0
        var y: Double = 0
        var width: Double = 0
        var height: Double = 0
        ghostty_surface_ime_point(surface, &x, &y, &width, &height)

        let viewRect = NSRect(
            x: x,
            y: view.bounds.height - y,
            width: width,
            height: max(height, 0)
        )

        guard let window = view.window else { return viewRect }
        let windowRect = view.convert(viewRect, to: nil)
        return window.convertToScreen(windowRect)
    }

    func teardown() {
        teardownSurface()
        removeAppObservers()
        if let app {
            ghostty_app_free(app)
            self.app = nil
        }
        if let config {
            ghostty_config_free(config)
            self.config = nil
        }
        transition(
            kind: .scaffold,
            phase: .pending,
            summary: "Ghostty host has been torn down.",
            detail: nil
        )
        resetMetadata()
    }

    private func ensureApp() -> Bool {
        if app != nil {
            return true
        }

        var runtimeConfig = ghostty_runtime_config_s()
        runtimeConfig.userdata = Unmanaged.passUnretained(self).toOpaque()
        runtimeConfig.supports_selection_clipboard = true
        runtimeConfig.wakeup_cb = { userdata in
            AlanGhosttyLiveHost.from(userdata)?.scheduleTick()
        }
        runtimeConfig.action_cb = { app, target, action in
            guard let host = AlanGhosttyLiveHost.from(ghostty_app_userdata(app)) else {
                return false
            }
            return host.handleAction(target: target, action: action)
        }
        runtimeConfig.read_clipboard_cb = { userdata, location, state in
            guard let host = AlanGhosttyLiveHost.from(userdata),
                  let surface = host.surface
            else { return false }

            let text = AlanGhosttyLiveHost.readClipboardText(location: location) ?? ""
            text.withCString { cString in
                ghostty_surface_complete_clipboard_request(surface, cString, state, false)
            }
            return true
        }
        runtimeConfig.confirm_read_clipboard_cb = { userdata, string, state, _ in
            guard let host = AlanGhosttyLiveHost.from(userdata),
                  let surface = host.surface
            else { return }
            ghostty_surface_complete_clipboard_request(surface, string, state, true)
        }
        runtimeConfig.write_clipboard_cb = { _, location, content, len, _ in
            AlanGhosttyLiveHost.writeClipboard(location: location, content: content, len: len)
        }
        runtimeConfig.close_surface_cb = { userdata, _ in
            AlanGhosttyLiveHost.from(userdata)?.teardownSurface()
        }

        guard let primaryConfig = makePrimaryConfig() else {
            transition(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "Failed to allocate a Ghostty config.",
                detail: nil,
                failureReason: "ghostty_config_new returned nil."
            )
            return false
        }

        if let created = ghostty_app_new(&runtimeConfig, primaryConfig) {
            self.app = created
            self.config = primaryConfig
            installAppObservers()
            ghostty_app_set_focus(created, NSApp.isActive)
            transition(
                kind: .ghosttyLive,
                phase: .appReady,
                summary: "Ghostty app initialized.",
                detail: "Using the user's Ghostty config if present."
            )
            return true
        }

        let primaryDiagnostics = diagnosticMessages(for: primaryConfig)
        logger.error("ghostty_app_new(primary) failed: \(primaryDiagnostics.joined(separator: " | "))")
        ghostty_config_free(primaryConfig)

        guard let fallbackConfig = makeFallbackConfig() else {
            transition(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "Ghostty app initialization failed.",
                detail: primaryDiagnostics.first,
                failureReason: primaryDiagnostics.joined(separator: " | ")
            )
            return false
        }

        guard let created = ghostty_app_new(&runtimeConfig, fallbackConfig) else {
            let fallbackDiagnostics = diagnosticMessages(for: fallbackConfig)
            logger.error("ghostty_app_new(fallback) failed: \(fallbackDiagnostics.joined(separator: " | "))")
            ghostty_config_free(fallbackConfig)
            transition(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "Ghostty app initialization failed for both primary and fallback config.",
                detail: primaryDiagnostics.first ?? fallbackDiagnostics.first,
                failureReason: (primaryDiagnostics + fallbackDiagnostics).joined(separator: " | ")
            )
            return false
        }

        self.app = created
        self.config = fallbackConfig
        installAppObservers()
        ghostty_app_set_focus(created, NSApp.isActive)
        transition(
            kind: .ghosttyLive,
            phase: .appReady,
            summary: "Ghostty app initialized with a minimal fallback config.",
            detail: primaryDiagnostics.first ?? "User config was skipped after diagnostics."
        )
        return true
    }

    private func createSurface(
        on canvasView: AlanGhosttyCanvasView,
        bootProfile: AlanShellBootProfile
    ) {
        guard let app else { return }

        teardownSurface()
        didEmitFirstRefresh = false

        transition(
            kind: .ghosttyLive,
            phase: .appReady,
            summary: "Ghostty app is creating a surface.",
            detail: bootProfile.command.summary
        )
        updateMetadata(
            title: nil,
            workingDirectory: bootProfile.workingDirectory,
            summary: "booting \(bootProfile.command.summary.lowercased())",
            attention: .active,
            processExited: false,
            lastCommandExitCode: nil
        )

        var surfaceConfig = ghostty_surface_config_new()
        surfaceConfig.platform_tag = GHOSTTY_PLATFORM_MACOS
        surfaceConfig.platform = ghostty_platform_u(
            macos: ghostty_platform_macos_s(
                nsview: Unmanaged.passUnretained(canvasView).toOpaque()
            )
        )
        surfaceConfig.userdata = Unmanaged.passUnretained(self).toOpaque()
        surfaceConfig.scale_factor = Double(
            canvasView.window?.backingScaleFactor
                ?? NSScreen.main?.backingScaleFactor
                ?? 2
        )
        surfaceConfig.context = GHOSTTY_SURFACE_CONTEXT_WINDOW

        envStorage = makeEnvStorage(bootProfile.environment)
        var envVars = envStorage.map { ghostty_env_var_s(key: UnsafePointer($0.0), value: UnsafePointer($0.1)) }

        let createSurface = {
            if envVars.isEmpty {
                self.surface = ghostty_surface_new(app, &surfaceConfig)
            } else {
                let envVarsCount = envVars.count
                envVars.withUnsafeMutableBufferPointer { buffer in
                    surfaceConfig.env_vars = buffer.baseAddress
                    surfaceConfig.env_var_count = envVarsCount
                    self.surface = ghostty_surface_new(app, &surfaceConfig)
                }
            }
        }

        bootProfile.workingDirectory.withCString { cwdCString in
            surfaceConfig.working_directory = cwdCString
            if let surfaceCommand = bootProfile.surfaceCommand, !surfaceCommand.isEmpty {
                surfaceCommand.withCString { commandCString in
                    surfaceConfig.command = commandCString
                    createSurface()
                }
            } else {
                surfaceConfig.command = nil
                createSurface()
            }
        }

        guard surface != nil else {
            let diagnostics = diagnosticMessages(for: config)
            let detail = diagnostics.first ?? bootProfile.command.detail
            transition(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "Ghostty surface creation failed.",
                detail: detail,
                failureReason: diagnostics.joined(separator: " | ")
            )
            logger.error("ghostty_surface_new failed: \(diagnostics.joined(separator: " | "))")
            return
        }

        transition(
            kind: .ghosttyLive,
            phase: .surfaceReady,
            summary: "Ghostty surface attached to the macOS canvas.",
            detail: bootProfile.command.launchCommandString
        )
        updateMetadata(
            workingDirectory: bootProfile.workingDirectory,
            summary: "surface ready",
            attention: .active,
            processExited: false,
            lastCommandExitCode: nil
        )
    }

    private func synchronizeDrawableMetrics(for canvasView: AlanGhosttyCanvasView) {
        guard let surface else { return }
        guard let window = canvasView.window else { return }

        let size = canvasView.bounds.size
        guard size.width > 0, size.height > 0 else { return }

        let backingSize = canvasView.convertToBacking(NSRect(origin: .zero, size: size)).size
        guard backingSize.width > 0, backingSize.height > 0 else { return }

        let xScale = backingSize.width / size.width
        let yScale = backingSize.height / size.height
        let layerScale = max(1.0, window.backingScaleFactor)

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        canvasView.layer?.contentsScale = layerScale
        CATransaction.commit()

        ghostty_surface_set_content_scale(surface, xScale, yScale)
        ghostty_surface_set_size(
            surface,
            UInt32(max(1, Int(floor(backingSize.width)))),
            UInt32(max(1, Int(floor(backingSize.height))))
        )

        if let displayID = (window.screen ?? NSScreen.main)?.displayID, displayID != 0 {
            ghostty_surface_set_display_id(surface, displayID)
        }
    }

    private func markFirstRefreshIfNeeded(on canvasView: AlanGhosttyCanvasView) {
        guard !didEmitFirstRefresh else { return }
        didEmitFirstRefresh = true
        let size = canvasView.convertToBacking(canvasView.bounds).size
        transition(
            kind: .ghosttyLive,
            phase: .firstRefresh,
            summary: "Ghostty surface issued its first refresh.",
            detail: "\(Int(size.width)) × \(Int(size.height)) backing pixels"
        )
        updateMetadata(summary: "terminal rendering", attention: .active, processExited: false)
    }

    private func scheduleTick() {
        guard markTickScheduledIfNeeded() else { return }

        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.clearScheduledTick()
            if let app = self.app {
                ghostty_app_tick(app)
            }
            if let surface = self.surface {
                ghostty_surface_refresh(surface)
            }
        }
    }

    private func markTickScheduledIfNeeded() -> Bool {
        tickScheduleLock.lock()
        defer { tickScheduleLock.unlock() }
        guard !tickScheduled else { return false }
        tickScheduled = true
        return true
    }

    private func clearScheduledTick() {
        tickScheduleLock.lock()
        tickScheduled = false
        tickScheduleLock.unlock()
    }

    private func teardownSurface() {
        if let surface {
            ghostty_surface_free(surface)
            self.surface = nil
        }

        envStorage.forEach {
            free($0.0)
            free($0.1)
        }
        envStorage.removeAll()

        if app != nil {
            transition(
                kind: .ghosttyLive,
                phase: .appReady,
                summary: "Ghostty app is idle and waiting for a new surface.",
                detail: bootProfile?.command.summary
            )
        }

        updateMetadata(
            summary: app == nil ? nil : "surface released",
            attention: .idle,
            processExited: false,
            lastCommandExitCode: metadata.lastCommandExitCode
        )
    }

    private func makeEnvStorage(_ environment: [String: String]) -> [(UnsafeMutablePointer<CChar>, UnsafeMutablePointer<CChar>)] {
        environment
            .compactMap { key, value in
                guard let keyPtr = strdup(key), let valuePtr = strdup(value) else {
                    return nil
                }
                return (keyPtr, valuePtr)
            }
    }

    private func makePrimaryConfig() -> ghostty_config_t? {
        guard let config = ghostty_config_new() else {
            return nil
        }
        ghostty_config_load_default_files(config)
        ghostty_config_load_recursive_files(config)
        ghostty_config_finalize(config)
        return config
    }

    private func makeFallbackConfig() -> ghostty_config_t? {
        guard let config = ghostty_config_new() else {
            return nil
        }
        ghostty_config_finalize(config)
        return config
    }

    private func diagnosticMessages(for config: ghostty_config_t?) -> [String] {
        guard let config else { return [] }
        let count = Int(ghostty_config_diagnostics_count(config))
        guard count > 0 else { return [] }
        return (0..<count).compactMap { index in
            let diagnostic = ghostty_config_get_diagnostic(config, UInt32(index))
            guard let message = diagnostic.message else { return nil }
            return String(cString: message)
        }
    }

    private func transition(
        kind: TerminalRendererKind,
        phase: TerminalRendererPhase,
        summary: String,
        detail: String?,
        failureReason: String? = nil
    ) {
        let event = detail.map { "\(summary) \($0)" } ?? summary
        var recentEvents = diagnostics.recentEvents
        if recentEvents.last != event {
            recentEvents.append(event)
            recentEvents = Array(recentEvents.suffix(6))
        }

        let snapshot = TerminalRendererSnapshot(
            kind: kind,
            phase: phase,
            summary: summary,
            detail: detail,
            failureReason: failureReason,
            recentEvents: recentEvents
        )

        guard diagnostics != snapshot else { return }
        diagnostics = snapshot
        onDiagnosticsChange?(snapshot)

        if let failureReason, phase == .failed {
            logger.error("\(summary) \(failureReason)")
        } else {
            logger.info("\(summary)")
        }
    }

    private func handleAction(target: ghostty_target_s, action: ghostty_action_s) -> Bool {
        if target.tag == GHOSTTY_TARGET_SURFACE,
           let surface,
           target.target.surface != surface {
            return false
        }

        switch action.tag {
        case GHOSTTY_ACTION_SET_TITLE:
            let title = action.action.set_title.title.flatMap { String(cString: $0) }
            performOnMain {
                self.updateMetadata(
                    title: title,
                    summary: title.flatMap { !$0.isEmpty ? "title updated" : nil }
                )
            }
            return true

        case GHOSTTY_ACTION_PWD:
            let workingDirectory = action.action.pwd.pwd.flatMap { String(cString: $0) }
            performOnMain {
                self.updateMetadata(
                    workingDirectory: workingDirectory,
                    summary: workingDirectory.flatMap { !$0.isEmpty ? "working directory updated" : nil }
                )
            }
            return true

        case GHOSTTY_ACTION_RING_BELL:
            performOnMain {
                self.updateMetadata(summary: "terminal bell", attention: .notable)
            }
            return true

        case GHOSTTY_ACTION_SHOW_CHILD_EXITED:
            let exitCode = action.action.child_exited.exit_code
            performOnMain {
                self.updateMetadata(
                    summary: "process exited with status \(exitCode)",
                    attention: .awaitingUser,
                    processExited: true,
                    lastCommandExitCode: Int(exitCode)
                )
            }
            return true

        case GHOSTTY_ACTION_COMMAND_FINISHED:
            let exitCode = action.action.command_finished.exit_code
            let summary: String
            if exitCode < 0 {
                summary = "command finished"
            } else if exitCode == 0 {
                summary = "command succeeded"
            } else {
                summary = "command failed (\(exitCode))"
            }
            performOnMain {
                self.updateMetadata(
                    summary: summary,
                    attention: exitCode == 0 ? .active : .notable,
                    processExited: false,
                    lastCommandExitCode: Int(exitCode)
                )
            }
            return true

        case GHOSTTY_ACTION_PROGRESS_REPORT:
            let progress = action.action.progress_report
            let summary = progressSummary(progress)
            performOnMain {
                self.updateMetadata(summary: summary, attention: .active)
            }
            return true

        default:
            return false
        }
    }

    private func progressSummary(_ progress: ghostty_action_progress_report_s) -> String? {
        switch progress.state {
        case GHOSTTY_PROGRESS_STATE_REMOVE:
            return "progress cleared"
        case GHOSTTY_PROGRESS_STATE_SET:
            return progress.progress >= 0 ? "progress \(progress.progress)%" : "progress updated"
        case GHOSTTY_PROGRESS_STATE_ERROR:
            return "progress error"
        case GHOSTTY_PROGRESS_STATE_INDETERMINATE:
            return "progress running"
        case GHOSTTY_PROGRESS_STATE_PAUSE:
            return "progress paused"
        default:
            return nil
        }
    }

    private func updateMetadata(
        title: String? = nil,
        workingDirectory: String? = nil,
        summary: String? = nil,
        attention: ShellAttentionState? = nil,
        processExited: Bool? = nil,
        lastCommandExitCode: Int? = nil
    ) {
        let nextTitle = title ?? metadata.title
        let nextWorkingDirectory = workingDirectory ?? metadata.workingDirectory
        let nextSummary = summary ?? metadata.summary
        let nextAttention = attention ?? metadata.attention
        let nextProcessExited = processExited ?? metadata.processExited
        let nextLastCommandExitCode = lastCommandExitCode ?? metadata.lastCommandExitCode

        guard
            nextTitle != metadata.title
                || nextWorkingDirectory != metadata.workingDirectory
                || nextSummary != metadata.summary
                || nextAttention != metadata.attention
                || nextProcessExited != metadata.processExited
                || nextLastCommandExitCode != metadata.lastCommandExitCode
        else {
            return
        }

        let snapshot = TerminalPaneMetadataSnapshot(
            title: nextTitle,
            workingDirectory: nextWorkingDirectory,
            summary: nextSummary,
            attention: nextAttention,
            processExited: nextProcessExited,
            lastCommandExitCode: nextLastCommandExitCode,
            lastUpdatedAt: .now
        )
        metadata = snapshot
        onMetadataChange?(snapshot)
    }

    private func resetMetadata() {
        guard metadata != .placeholder else { return }
        metadata = .placeholder
        onMetadataChange?(.placeholder)
    }

    private func performOnMain(_ body: @escaping () -> Void) {
        if Thread.isMainThread {
            body()
        } else {
            DispatchQueue.main.async(execute: body)
        }
    }

    private func installAppObservers() {
        removeAppObservers()
        guard let app else { return }
        let center = NotificationCenter.default
        appObservers = [
            center.addObserver(
                forName: NSApplication.didBecomeActiveNotification,
                object: nil,
                queue: .main
            ) { _ in
                ghostty_app_set_focus(app, true)
            },
            center.addObserver(
                forName: NSApplication.didResignActiveNotification,
                object: nil,
                queue: .main
            ) { _ in
                ghostty_app_set_focus(app, false)
            },
        ]
    }

    private func removeAppObservers() {
        appObservers.forEach(NotificationCenter.default.removeObserver)
        appObservers.removeAll()
    }

    private static func from(_ userdata: UnsafeMutableRawPointer?) -> AlanGhosttyLiveHost? {
        guard let userdata else { return nil }
        return Unmanaged<AlanGhosttyLiveHost>.fromOpaque(userdata).takeUnretainedValue()
    }

    private static func readClipboardText(location: ghostty_clipboard_e) -> String? {
        let pasteboard: NSPasteboard?
        switch location {
        case GHOSTTY_CLIPBOARD_STANDARD:
            pasteboard = .general
        default:
            pasteboard = nil
        }

        return pasteboard?.string(forType: .string)
    }

    private static func writeClipboard(
        location: ghostty_clipboard_e,
        content: UnsafePointer<ghostty_clipboard_content_s>?,
        len: Int
    ) {
        guard location == GHOSTTY_CLIPBOARD_STANDARD,
              let content,
              len > 0
        else { return }

        let buffer = UnsafeBufferPointer(start: content, count: len)
        guard let first = buffer.first,
              let data = first.data
        else { return }

        let text = String(cString: data)
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)
    }
}

final class AlanGhosttyCanvasView: NSView {
    override var mouseDownCanMoveWindow: Bool { false }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }
}

extension NSScreen {
    var displayID: UInt32? {
        (deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber).map { $0.uint32Value }
    }
}
#endif
