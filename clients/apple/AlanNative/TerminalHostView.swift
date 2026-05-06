import AppKit
import SwiftUI
#if canImport(QuartzCore)
import QuartzCore
#endif
#if canImport(GhosttyKit)
import GhosttyKit
#endif

#if os(macOS)
struct TerminalHostView: NSViewRepresentable {
    let pane: ShellPane?
    let bootProfile: AlanShellBootProfile?
    let isSelected: Bool
    let runtimeRegistry: TerminalRuntimeRegistry
    let activationDelegate: TerminalHostActivationDelegate?
    let onRuntimeUpdate: (TerminalHostRuntimeSnapshot) -> Void
    let onMetadataUpdate: (TerminalPaneMetadataSnapshot) -> Void

    func makeNSView(context: Context) -> AlanTerminalHostNSView {
        runtimeRegistry.hostView(
            for: pane,
            bootProfile: bootProfile,
            isSelected: isSelected,
            activationDelegate: activationDelegate,
            onRuntimeUpdate: onRuntimeUpdate,
            onMetadataUpdate: onMetadataUpdate
        )
    }

    func updateNSView(_ nsView: AlanTerminalHostNSView, context: Context) {
        nsView.configure(
            pane: pane,
            bootProfile: bootProfile,
            isSelected: isSelected,
            surfaceHandle: runtimeRegistry.surfaceHandle(for: pane, bootProfile: bootProfile),
            activationDelegate: activationDelegate,
            onRuntimeUpdate: onRuntimeUpdate,
            onMetadataUpdate: onMetadataUpdate
        )
    }
}

@MainActor
protocol TerminalHostActivationDelegate: AnyObject {
    func terminalHostDidRequestActivation(paneID: String)
}

final class AlanTerminalHostNSView: NSView, NSTextInputClient, TerminalRuntimeHandle {
    private let canvasView = makeCanvasView()
    private let overlayCard = AlanTerminalPassiveOverlayView()
    private let bodyStack = NSStackView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let subtitleLabel = NSTextField(wrappingLabelWithString: "")
    private let commandLabel = NSTextField(wrappingLabelWithString: "")
    private let footerLabel = NSTextField(wrappingLabelWithString: "")
    private let statusBadge = NSTextField(labelWithString: "")
    private let surfaceController = AlanTerminalSurfaceController()

    private var pane: ShellPane?
    private var bootProfile: AlanShellBootProfile?
    private var isSelected = false
    private weak var activationDelegate: TerminalHostActivationDelegate?
    private var runtimeObserver: ((TerminalHostRuntimeSnapshot) -> Void)?
    private var metadataObserver: ((TerminalPaneMetadataSnapshot) -> Void)?
    private var windowObservers: [NSObjectProtocol] = []
    private var rendererSnapshot: TerminalRendererSnapshot = .placeholder
    private var paneMetadata: TerminalPaneMetadataSnapshot = .placeholder
    private var lastReportedMetadata: TerminalPaneMetadataSnapshot?
    private var lastReportedRuntime: TerminalHostRuntimeSnapshot?
    private var trackingArea: NSTrackingArea?
    private var markedText = NSMutableAttributedString()
    private var keyTextAccumulator: [String]?
    private var previousPressureStage = 0
    private var hasTornDownRuntime = false
    private var pendingFocusRequest = false
    private var needsWindowAttachmentFocus = false

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        configureView()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    deinit {
        teardownRuntimeIfNeeded()
    }

    override var acceptsFirstResponder: Bool {
        true
    }

    override var mouseDownCanMoveWindow: Bool { false }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea {
            removeTrackingArea(trackingArea)
        }

        let trackingArea = NSTrackingArea(
            rect: bounds,
            options: [.activeInActiveApp, .inVisibleRect, .mouseEnteredAndExited, .mouseMoved],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(trackingArea)
        self.trackingArea = trackingArea
    }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        publishRuntimeSnapshot()
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        installWindowObservers()
        window?.acceptsMouseMovedEvents = true
        if window != nil, needsWindowAttachmentFocus {
            needsWindowAttachmentFocus = false
            focusTerminalSoon()
        } else if window == nil {
            needsWindowAttachmentFocus = false
            pendingFocusRequest = false
        }
        publishRuntimeSnapshot()
    }

    override func becomeFirstResponder() -> Bool {
        let result = super.becomeFirstResponder()
        if result {
            synchronizeLiveHost()
            publishRuntimeSnapshot()
        }
        return result
    }

    override func resignFirstResponder() -> Bool {
        let result = super.resignFirstResponder()
        if result {
            synchronizeLiveHost()
            publishRuntimeSnapshot()
        }
        return result
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        publishRuntimeSnapshot()
    }

    override func layout() {
        super.layout()
        synchronizeLiveHost()
        publishRuntimeSnapshot()
    }

    func configure(
        pane: ShellPane?,
        bootProfile: AlanShellBootProfile?,
        isSelected: Bool,
        surfaceHandle: AlanTerminalSurfaceHandle?,
        activationDelegate: TerminalHostActivationDelegate?,
        onRuntimeUpdate: @escaping (TerminalHostRuntimeSnapshot) -> Void,
        onMetadataUpdate: @escaping (TerminalPaneMetadataSnapshot) -> Void
    ) {
        let previousPaneID = self.pane?.paneID
        let wasSelected = self.isSelected

        self.pane = pane
        self.bootProfile = bootProfile
        self.isSelected = isSelected
        surfaceController.bind(surfaceHandle: surfaceHandle, paneID: pane?.paneID)
        self.activationDelegate = activationDelegate
        runtimeObserver = onRuntimeUpdate
        metadataObserver = onMetadataUpdate

        let title = pane?.viewport?.title ?? pane?.process?.program ?? "Terminal"
        titleLabel.stringValue = title
        subtitleLabel.stringValue = pane?.viewport?.summary ?? "Preparing the native terminal view."

        if let bootProfile {
            commandLabel.stringValue = "$ \(bootProfile.launchCommandString)"

            let envSummary = bootProfile.environmentPreview
                .prefix(4)
                .map { "\($0.key)=\($0.value)" }
                .joined(separator: "\n")
            footerLabel.stringValue = [
                "launch: \(bootProfile.command.strategy.rawValue)",
                bootProfile.command.detail,
                "cwd: \(bootProfile.workingDirectory)",
                envSummary.isEmpty ? nil : envSummary,
                "setup: \(bootProfile.ghostty.setupCommand)",
            ]
            .compactMap { $0 }
            .joined(separator: "\n")
        } else {
            commandLabel.stringValue = "$ /bin/zsh -l"
            footerLabel.stringValue = "Select a pane to prepare a terminal boot profile."
        }

        synchronizeRendererSnapshot(with: bootProfile)
        syncStatusBadge()
        syncOverlayVisibility()
        synchronizeLiveHost()
        if shouldAutoFocusAfterConfigure(
            previousPaneID: previousPaneID,
            paneID: pane?.paneID,
            wasSelected: wasSelected
        ) {
            focusTerminalSoon()
        }
        reportMetadataIfNeeded(paneMetadata)
        publishRuntimeSnapshot()
    }

    private func reportMetadataIfNeeded(_ snapshot: TerminalPaneMetadataSnapshot) {
        guard lastReportedMetadata != snapshot else { return }
        lastReportedMetadata = snapshot
        guard let metadataObserver else { return }
        DispatchQueue.main.async { [weak self] in
            guard let self, self.lastReportedMetadata == snapshot else { return }
            metadataObserver(snapshot)
        }
    }

    private func configureView() {
        wantsLayer = true
        layer?.backgroundColor = NSColor(calibratedRed: 0.06, green: 0.08, blue: 0.10, alpha: 1).cgColor
        layer?.cornerRadius = 12
        layer?.cornerCurve = .continuous
        layer?.masksToBounds = true
        layer?.borderWidth = 0
        layer?.shadowColor = NSColor.black.cgColor
        layer?.shadowOpacity = 0
        layer?.shadowRadius = 0
        layer?.shadowOffset = .zero

        translatesAutoresizingMaskIntoConstraints = false

        bodyStack.orientation = .vertical
        bodyStack.alignment = .leading
        bodyStack.spacing = 10
        bodyStack.translatesAutoresizingMaskIntoConstraints = false

        overlayCard.material = .hudWindow
        overlayCard.blendingMode = .withinWindow
        overlayCard.state = .active
        overlayCard.translatesAutoresizingMaskIntoConstraints = false
        overlayCard.wantsLayer = true
        overlayCard.layer?.cornerRadius = 18
        overlayCard.layer?.borderWidth = 1
        overlayCard.layer?.borderColor = NSColor.white.withAlphaComponent(0.12).cgColor

        statusBadge.font = .systemFont(ofSize: 11, weight: .semibold)
        statusBadge.textColor = NSColor.white.withAlphaComponent(0.92)
        statusBadge.drawsBackground = true
        statusBadge.backgroundColor = NSColor.white.withAlphaComponent(0.10)
        statusBadge.isBezeled = false
        statusBadge.isEditable = false
        statusBadge.isSelectable = false
        statusBadge.alignment = .center
        statusBadge.lineBreakMode = .byTruncatingTail
        statusBadge.maximumNumberOfLines = 1
        statusBadge.cell?.usesSingleLineMode = true
        statusBadge.cell?.wraps = false
        statusBadge.cell?.backgroundStyle = .raised

        titleLabel.font = .systemFont(ofSize: 18, weight: .semibold)
        titleLabel.textColor = .white

        subtitleLabel.font = .systemFont(ofSize: 12, weight: .medium)
        subtitleLabel.textColor = NSColor.white.withAlphaComponent(0.64)
        subtitleLabel.maximumNumberOfLines = 2

        commandLabel.font = .monospacedSystemFont(ofSize: 12, weight: .medium)
        commandLabel.textColor = NSColor(calibratedRed: 0.81, green: 0.90, blue: 0.98, alpha: 1)
        commandLabel.maximumNumberOfLines = 3

        footerLabel.font = .systemFont(ofSize: 11, weight: .regular)
        footerLabel.textColor = NSColor.white.withAlphaComponent(0.52)
        footerLabel.maximumNumberOfLines = 4

        [statusBadge, titleLabel, subtitleLabel, commandLabel, footerLabel].forEach(bodyStack.addArrangedSubview)

        addSubview(canvasView)
        addSubview(overlayCard)
        overlayCard.addSubview(bodyStack)

        NSLayoutConstraint.activate([
            canvasView.topAnchor.constraint(equalTo: topAnchor),
            canvasView.leadingAnchor.constraint(equalTo: leadingAnchor),
            canvasView.trailingAnchor.constraint(equalTo: trailingAnchor),
            canvasView.bottomAnchor.constraint(equalTo: bottomAnchor),

            overlayCard.centerXAnchor.constraint(equalTo: centerXAnchor),
            overlayCard.centerYAnchor.constraint(equalTo: centerYAnchor),
            overlayCard.widthAnchor.constraint(lessThanOrEqualToConstant: 460),
            overlayCard.leadingAnchor.constraint(greaterThanOrEqualTo: leadingAnchor, constant: 24),
            overlayCard.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -24),

            bodyStack.topAnchor.constraint(equalTo: overlayCard.topAnchor, constant: 18),
            bodyStack.leadingAnchor.constraint(equalTo: overlayCard.leadingAnchor, constant: 18),
            bodyStack.trailingAnchor.constraint(equalTo: overlayCard.trailingAnchor, constant: -18),
            bodyStack.bottomAnchor.constraint(equalTo: overlayCard.bottomAnchor, constant: -18),
        ])
    }

    func sendControlText(_ text: String) -> TerminalRuntimeDeliveryResult {
        guard !text.isEmpty else {
            return .accepted(byteCount: 0)
        }

        guard pane?.paneID != nil else {
            return .rejected(
                errorCode: "terminal_runtime_unavailable",
                errorMessage: "No pane is attached to this terminal runtime."
            )
        }

        return surfaceController.sendControlText(text)
    }

    func teardownTerminalRuntime() {
        teardownRuntimeIfNeeded()
    }

    private func teardownRuntimeIfNeeded() {
        guard !hasTornDownRuntime else { return }
        hasTornDownRuntime = true
        removeWindowObservers()
        surfaceController.detach()
    }

    private func installWindowObservers() {
        removeWindowObservers()
        guard let window else { return }
        let center = NotificationCenter.default
        windowObservers = [
            center.addObserver(
                forName: NSWindow.didBecomeKeyNotification,
                object: window,
                queue: .main
            ) { [weak self] _ in
                self?.publishRuntimeSnapshot()
            },
            center.addObserver(
                forName: NSWindow.didResignKeyNotification,
                object: window,
                queue: .main
            ) { [weak self] _ in
                self?.publishRuntimeSnapshot()
            },
            center.addObserver(
                forName: NSWindow.didChangeScreenNotification,
                object: window,
                queue: .main
            ) { [weak self] _ in
                self?.synchronizeLiveHost()
                self?.publishRuntimeSnapshot()
            },
            center.addObserver(
                forName: NSWindow.didChangeOcclusionStateNotification,
                object: window,
                queue: .main
            ) { [weak self] _ in
                self?.synchronizeLiveHost()
                self?.publishRuntimeSnapshot()
            },
        ]
    }

    private func removeWindowObservers() {
        windowObservers.forEach(NotificationCenter.default.removeObserver)
        windowObservers.removeAll()
    }

    private func publishRuntimeSnapshot() {
        guard let runtimeObserver else { return }

        let logicalSize = bounds.size
        let backingRect = convertToBacking(bounds)
        let screen = window?.screen
        let stage: TerminalHostStage = {
            guard superview != nil else { return .scaffold }
            guard window != nil else { return .viewAttached }
            return isFocused ? .focused : .windowAttached
        }()

        let displayID = (screen?.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)
            .map { "\($0.uint32Value)" }

        let snapshot = TerminalHostRuntimeSnapshot(
            stage: stage,
            paneID: pane?.paneID,
            tabID: pane?.tabID,
            logicalSize: logicalSize,
            backingSize: backingRect.size,
            displayName: screen?.localizedName,
            displayID: displayID,
            attachedWindowTitle: window?.title,
            isFocused: isFocused,
            renderer: rendererSnapshot,
            paneMetadata: paneMetadata,
            surfaceState: surfaceController.surfaceStateSnapshot,
            lastUpdatedAt: .now
        )
        reportRuntimeIfNeeded(snapshot, runtimeObserver: runtimeObserver)
    }

    private func reportRuntimeIfNeeded(
        _ snapshot: TerminalHostRuntimeSnapshot,
        runtimeObserver: @escaping (TerminalHostRuntimeSnapshot) -> Void
    ) {
        if let lastReportedRuntime,
           runtimeSnapshotEqualsIgnoringTimestamp(lastReportedRuntime, snapshot)
        {
            return
        }

        lastReportedRuntime = snapshot
        DispatchQueue.main.async { [weak self] in
            guard let self,
                  let lastReportedRuntime = self.lastReportedRuntime,
                  self.runtimeSnapshotEqualsIgnoringTimestamp(lastReportedRuntime, snapshot)
            else {
                return
            }
            runtimeObserver(snapshot)
        }
    }

    private func runtimeSnapshotEqualsIgnoringTimestamp(
        _ lhs: TerminalHostRuntimeSnapshot,
        _ rhs: TerminalHostRuntimeSnapshot
    ) -> Bool {
        lhs.stage == rhs.stage
            && lhs.paneID == rhs.paneID
            && lhs.tabID == rhs.tabID
            && lhs.logicalSize == rhs.logicalSize
            && lhs.backingSize == rhs.backingSize
            && lhs.displayName == rhs.displayName
            && lhs.displayID == rhs.displayID
            && lhs.attachedWindowTitle == rhs.attachedWindowTitle
            && lhs.isFocused == rhs.isFocused
            && lhs.renderer == rhs.renderer
            && lhs.paneMetadata == rhs.paneMetadata
            && lhs.surfaceState.equalsIgnoringTimestamp(rhs.surfaceState)
    }

    private func synchronizeLiveHost() {
#if canImport(GhosttyKit)
        guard let canvasView = canvasView as? AlanGhosttyCanvasView else { return }
        surfaceController.attach(
            to: canvasView,
            bootProfile: bootProfile,
            focused: isFocused,
            onDiagnosticsChange: { [weak self] snapshot in
                guard let self else { return }
                rendererSnapshot = snapshot
                surfaceController.updateRenderer(snapshot)
                syncStatusBadge()
                syncOverlayVisibility()
                publishRuntimeSnapshot()
            },
            onMetadataChange: { [weak self] snapshot in
                guard let self else { return }
                paneMetadata = snapshot
                surfaceController.updateMetadata(snapshot)
                subtitleLabel.stringValue = snapshot.summary ?? subtitleLabel.stringValue
                reportMetadataIfNeeded(snapshot)
                syncOverlayVisibility()
                publishRuntimeSnapshot()
            }
        )
#endif
    }

    private func synchronizeRendererSnapshot(with bootProfile: AlanShellBootProfile?) {
#if canImport(GhosttyKit)
        if rendererSnapshot.kind == .scaffold {
            rendererSnapshot = TerminalRendererSnapshot(
                kind: .ghosttyLive,
                phase: .pending,
                summary: bootProfile?.ghostty.isReady == true
                    ? "GhosttyKit is linked and waiting for the live host handshake."
                    : "GhosttyKit has not been linked into the local repo yet.",
                detail: bootProfile?.command.summary,
                failureReason: nil,
                recentEvents: rendererSnapshot.recentEvents
            )
        }
#else
        rendererSnapshot = TerminalRendererSnapshot(
            kind: .scaffold,
            phase: .pending,
            summary: bootProfile?.ghostty.isReady == true
                ? "GhosttyKit is available on disk but this build does not import it."
                : "GhosttyKit has not been linked into the local repo yet.",
            detail: bootProfile?.command.summary,
            failureReason: nil,
            recentEvents: []
        )
#endif
    }

    private func syncStatusBadge() {
        if bootProfile == nil {
            statusBadge.stringValue = "Select a pane"
            return
        }

        if let bootProfile, !bootProfile.ghostty.isReady {
            statusBadge.stringValue = "GhosttyKit pending"
            return
        }

        statusBadge.stringValue = rendererSnapshot.kind == .ghosttyLive
            ? "Ghostty live · \(rendererSnapshot.phaseLabel)"
            : "GhosttyKit ready"
    }

    private func syncOverlayVisibility() {
        if let overlayState = surfaceController.overlayState(
            renderer: rendererSnapshot,
            metadata: paneMetadata,
            bootProfile: bootProfile
        ) {
            statusBadge.stringValue = overlayState.badge
            titleLabel.stringValue = overlayState.title
            subtitleLabel.stringValue = overlayState.message
            if let action = overlayState.action {
                commandLabel.stringValue = action
            }
            footerLabel.stringValue = bootProfile.map { "launch: \($0.command.strategy.rawValue)\ncwd: \($0.workingDirectory)" }
                ?? "Select a pane to prepare a terminal boot profile."
            overlayCard.isHidden = false
            return
        }

        overlayCard.isHidden = true
    }

    private var isFocused: Bool {
        window?.firstResponder === self
    }

    private func focusTerminalSoon() {
        guard isSelected, pane != nil else { return }
        guard window != nil else {
            needsWindowAttachmentFocus = true
            return
        }
        guard !pendingFocusRequest else { return }
        pendingFocusRequest = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            pendingFocusRequest = false
            guard isSelected, pane != nil, window != nil else { return }
            requestTerminalFocus()
        }
    }

    private func shouldAutoFocusAfterConfigure(
        previousPaneID: String?,
        paneID: String?,
        wasSelected: Bool
    ) -> Bool {
        guard isSelected, paneID != nil else { return false }
        return previousPaneID != paneID || !wasSelected
    }

    private func requestTerminalFocus() {
        window?.makeFirstResponder(self)
        synchronizeLiveHost()
        publishRuntimeSnapshot()
    }

    private func activateTerminalHostForMouseEvent() {
        if let paneID = pane?.paneID {
            activationDelegate?.terminalHostDidRequestActivation(paneID: paneID)
        }
        requestTerminalFocus()
    }

    private func localPoint(for event: NSEvent) -> CGPoint {
        convert(event.locationInWindow, from: nil)
    }

    private func ghosttyPoint(for event: NSEvent) -> CGPoint {
        let point = localPoint(for: event)
        return CGPoint(x: point.x, y: bounds.height - point.y)
    }

    private func sendMousePosition(for event: NSEvent) {
#if canImport(GhosttyKit)
        let point = ghosttyPoint(for: event)
        surfaceController.sendMousePosition(x: point.x, y: point.y, mods: modsFromEvent(event))
#endif
    }

    override func mouseDown(with event: NSEvent) {
        activateTerminalHostForMouseEvent()
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
        _ = surfaceController.sendMouseButton(state: GHOSTTY_MOUSE_PRESS, button: GHOSTTY_MOUSE_LEFT, mods: modsFromEvent(event))
#endif
    }

    override func mouseUp(with event: NSEvent) {
        previousPressureStage = 0
#if canImport(GhosttyKit)
        _ = surfaceController.sendMouseButton(state: GHOSTTY_MOUSE_RELEASE, button: GHOSTTY_MOUSE_LEFT, mods: modsFromEvent(event))
        surfaceController.sendMousePressure(stage: 0, pressure: 0)
#endif
    }

    override func rightMouseDown(with event: NSEvent) {
        activateTerminalHostForMouseEvent()
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
        _ = surfaceController.sendMouseButton(state: GHOSTTY_MOUSE_PRESS, button: GHOSTTY_MOUSE_RIGHT, mods: modsFromEvent(event))
#else
        super.rightMouseDown(with: event)
#endif
    }

    override func rightMouseUp(with event: NSEvent) {
#if canImport(GhosttyKit)
        _ = surfaceController.sendMouseButton(state: GHOSTTY_MOUSE_RELEASE, button: GHOSTTY_MOUSE_RIGHT, mods: modsFromEvent(event))
#else
        super.rightMouseUp(with: event)
#endif
    }

    override func otherMouseDown(with event: NSEvent) {
        activateTerminalHostForMouseEvent()
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
        let button = event.buttonNumber == 2 ? GHOSTTY_MOUSE_MIDDLE : GHOSTTY_MOUSE_MIDDLE
        _ = surfaceController.sendMouseButton(state: GHOSTTY_MOUSE_PRESS, button: button, mods: modsFromEvent(event))
#else
        super.otherMouseDown(with: event)
#endif
    }

    override func otherMouseUp(with event: NSEvent) {
#if canImport(GhosttyKit)
        let button = event.buttonNumber == 2 ? GHOSTTY_MOUSE_MIDDLE : GHOSTTY_MOUSE_MIDDLE
        _ = surfaceController.sendMouseButton(state: GHOSTTY_MOUSE_RELEASE, button: button, mods: modsFromEvent(event))
#else
        super.otherMouseUp(with: event)
#endif
    }

    override func mouseEntered(with event: NSEvent) {
        super.mouseEntered(with: event)
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
#endif
    }

    override func mouseMoved(with event: NSEvent) {
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
#endif
    }

    override func mouseDragged(with event: NSEvent) {
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
#endif
    }

    override func rightMouseDragged(with event: NSEvent) {
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
#endif
    }

    override func otherMouseDragged(with event: NSEvent) {
#if canImport(GhosttyKit)
        sendMousePosition(for: event)
#endif
    }

    override func mouseExited(with event: NSEvent) {
        super.mouseExited(with: event)
#if canImport(GhosttyKit)
        surfaceController.sendMousePosition(x: -1, y: -1, mods: modsFromEvent(event))
#endif
    }

    override func scrollWheel(with event: NSEvent) {
#if canImport(GhosttyKit)
        guard surfaceController.isSurfaceReady == true else { return super.scrollWheel(with: event) }

        var x = event.scrollingDeltaX
        var y = event.scrollingDeltaY
        let precision = event.hasPreciseScrollingDeltas
        if precision {
            x *= 2
            y *= 2
        }

        var scrollMods: Int32 = 0
        if precision {
            scrollMods |= 0b0000_0001
        }

        let momentum: Int32
        switch event.momentumPhase {
        case .began:
            momentum = Int32(GHOSTTY_MOUSE_MOMENTUM_BEGAN.rawValue)
        case .stationary:
            momentum = Int32(GHOSTTY_MOUSE_MOMENTUM_STATIONARY.rawValue)
        case .changed:
            momentum = Int32(GHOSTTY_MOUSE_MOMENTUM_CHANGED.rawValue)
        case .ended:
            momentum = Int32(GHOSTTY_MOUSE_MOMENTUM_ENDED.rawValue)
        case .cancelled:
            momentum = Int32(GHOSTTY_MOUSE_MOMENTUM_CANCELLED.rawValue)
        case .mayBegin:
            momentum = Int32(GHOSTTY_MOUSE_MOMENTUM_MAY_BEGIN.rawValue)
        default:
            momentum = Int32(GHOSTTY_MOUSE_MOMENTUM_NONE.rawValue)
        }
        scrollMods |= momentum << 1

        surfaceController.sendMouseScroll(x: x, y: y, mods: ghostty_input_scroll_mods_t(scrollMods))
#else
        super.scrollWheel(with: event)
#endif
    }

    override func pressureChange(with event: NSEvent) {
        super.pressureChange(with: event)
#if canImport(GhosttyKit)
        guard surfaceController.isSurfaceReady == true else { return }
        surfaceController.sendMousePressure(stage: UInt32(event.stage), pressure: Double(event.pressure))
        previousPressureStage = event.stage
#endif
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if routeNativeKeyCommandIfNeeded(event) {
            return true
        }

#if canImport(GhosttyKit)
        guard !isApplicationReservedKeyEquivalent(event) else { return false }
        guard event.type == .keyDown, isFocused, surfaceController.isSurfaceReady == true else { return false }

        var keyEvent = ghosttyKeyEvent(for: event, action: GHOSTTY_ACTION_PRESS)
        var flags = ghostty_binding_flags_e(0)
        let text = textForKeyEvent(event) ?? ""
        let isBinding = text.withCString { cString in
            keyEvent.text = cString
            return surfaceController.keyIsBinding(keyEvent, flags: &flags)
        }

        if isBinding {
            keyDown(with: event)
            return true
        }
#endif
        return false
    }

    override func keyDown(with event: NSEvent) {
        if routeNativeKeyCommandIfNeeded(event) {
            return
        }
        if handleSearchKeyIfNeeded(event) {
            return
        }

#if canImport(GhosttyKit)
        if isApplicationReservedKeyEquivalent(event) {
            NSApp.terminate(nil)
            return
        }

        guard surfaceController.isSurfaceReady == true else {
            interpretKeyEvents([event])
            return
        }

        requestTerminalFocus()

        let translationModsGhostty =
            surfaceController.keyTranslationMods(for: modsFromEvent(event))
        var translationMods = event.modifierFlags
        for flag in [NSEvent.ModifierFlags.shift, .control, .option, .command] {
            let shouldInclude: Bool
            switch flag {
            case .shift:
                shouldInclude = (translationModsGhostty.rawValue & GHOSTTY_MODS_SHIFT.rawValue) != 0
            case .control:
                shouldInclude = (translationModsGhostty.rawValue & GHOSTTY_MODS_CTRL.rawValue) != 0
            case .option:
                shouldInclude = (translationModsGhostty.rawValue & GHOSTTY_MODS_ALT.rawValue) != 0
            case .command:
                shouldInclude = (translationModsGhostty.rawValue & GHOSTTY_MODS_SUPER.rawValue) != 0
            default:
                shouldInclude = translationMods.contains(flag)
            }

            if shouldInclude {
                translationMods.insert(flag)
            } else {
                translationMods.remove(flag)
            }
        }

        let translationEvent: NSEvent
        if translationMods == event.modifierFlags {
            translationEvent = event
        } else {
            translationEvent = NSEvent.keyEvent(
                with: event.type,
                location: event.locationInWindow,
                modifierFlags: translationMods,
                timestamp: event.timestamp,
                windowNumber: event.windowNumber,
                context: nil,
                characters: event.characters(byApplyingModifiers: translationMods) ?? "",
                charactersIgnoringModifiers: event.charactersIgnoringModifiers ?? "",
                isARepeat: event.isARepeat,
                keyCode: event.keyCode
            ) ?? event
        }

        keyTextAccumulator = []
        defer { keyTextAccumulator = nil }

        let markedTextBefore = markedText.length > 0
        interpretKeyEvents([translationEvent])
        syncPreedit(clearIfNeeded: markedTextBefore)

        if let keyTextAccumulator, !keyTextAccumulator.isEmpty {
            keyTextAccumulator.forEach { surfaceController.sendText($0) }
            return
        }

        var keyEvent = ghosttyKeyEvent(
            for: event,
            action: event.isARepeat ? GHOSTTY_ACTION_REPEAT : GHOSTTY_ACTION_PRESS,
            translationMods: translationMods
        )
        keyEvent.composing = markedText.length > 0 || markedTextBefore

        if let text = textForKeyEvent(translationEvent), shouldSendText(text) {
            text.withCString { cString in
                keyEvent.text = cString
                _ = surfaceController.sendKey(keyEvent)
            }
        } else {
            _ = surfaceController.sendKey(keyEvent)
        }
#else
        super.keyDown(with: event)
#endif
    }

    override func keyUp(with event: NSEvent) {
#if canImport(GhosttyKit)
        guard surfaceController.isSurfaceReady == true else { return super.keyUp(with: event) }
        let keyEvent = ghosttyKeyEvent(for: event, action: GHOSTTY_ACTION_RELEASE)
        _ = surfaceController.sendKey(keyEvent)
#else
        super.keyUp(with: event)
#endif
    }

    override func flagsChanged(with event: NSEvent) {
#if canImport(GhosttyKit)
        guard surfaceController.isSurfaceReady == true else { return super.flagsChanged(with: event) }

        let modifier: UInt32
        switch event.keyCode {
        case 0x39: modifier = GHOSTTY_MODS_CAPS.rawValue
        case 0x38, 0x3C: modifier = GHOSTTY_MODS_SHIFT.rawValue
        case 0x3B, 0x3E: modifier = GHOSTTY_MODS_CTRL.rawValue
        case 0x3A, 0x3D: modifier = GHOSTTY_MODS_ALT.rawValue
        case 0x37, 0x36: modifier = GHOSTTY_MODS_SUPER.rawValue
        default: return
        }

        let mods = modsFromEvent(event)
        var action = GHOSTTY_ACTION_RELEASE
        if mods.rawValue & modifier != 0 {
            action = GHOSTTY_ACTION_PRESS
        }

        let keyEvent = ghosttyKeyEvent(for: event, action: action)
        _ = surfaceController.sendKey(keyEvent)
        sendMousePosition(for: event)
#else
        super.flagsChanged(with: event)
#endif
    }

    @objc func copy(_ sender: Any?) {
#if canImport(GhosttyKit)
        guard let text = surfaceController.readSelectionText(), !text.isEmpty else { return }
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)
#endif
    }

    @objc func cut(_ sender: Any?) {
        copy(sender)
    }

    @objc func paste(_ sender: Any?) {
#if canImport(GhosttyKit)
        guard let text = NSPasteboard.general.string(forType: .string), !text.isEmpty else { return }
        surfaceController.sendText(text)
#endif
    }

    private func modsFromEvent(_ event: NSEvent) -> ghostty_input_mods_e {
        var mods = GHOSTTY_MODS_NONE.rawValue
        if event.modifierFlags.contains(.shift) { mods |= GHOSTTY_MODS_SHIFT.rawValue }
        if event.modifierFlags.contains(.control) { mods |= GHOSTTY_MODS_CTRL.rawValue }
        if event.modifierFlags.contains(.option) { mods |= GHOSTTY_MODS_ALT.rawValue }
        if event.modifierFlags.contains(.command) { mods |= GHOSTTY_MODS_SUPER.rawValue }
        return ghostty_input_mods_e(rawValue: mods)
    }

    private func consumedModsFromFlags(_ flags: NSEvent.ModifierFlags) -> ghostty_input_mods_e {
        var mods = GHOSTTY_MODS_NONE.rawValue
        if flags.contains(.shift) { mods |= GHOSTTY_MODS_SHIFT.rawValue }
        if flags.contains(.option) { mods |= GHOSTTY_MODS_ALT.rawValue }
        return ghostty_input_mods_e(rawValue: mods)
    }

    private func ghosttyKeyEvent(
        for event: NSEvent,
        action: ghostty_input_action_e,
        translationMods: NSEvent.ModifierFlags? = nil
    ) -> ghostty_input_key_s {
        var keyEvent = ghostty_input_key_s()
        keyEvent.action = action
        keyEvent.keycode = UInt32(event.keyCode)
        keyEvent.mods = modsFromEvent(event)
        keyEvent.consumed_mods = consumedModsFromFlags(
            (translationMods ?? event.modifierFlags).subtracting([.control, .command])
        )
        keyEvent.text = nil
        keyEvent.composing = false
        keyEvent.unshifted_codepoint = unshiftedCodepointFromEvent(event)
        return keyEvent
    }

    private func unshiftedCodepointFromEvent(_ event: NSEvent) -> UInt32 {
        guard event.type != .flagsChanged else {
            return 0
        }
        guard let chars = event.characters(byApplyingModifiers: []) ?? event.charactersIgnoringModifiers ?? event.characters,
              let scalar = chars.unicodeScalars.first else {
            return 0
        }
        return scalar.value
    }

    private func textForKeyEvent(_ event: NSEvent) -> String? {
        guard let chars = event.characters, !chars.isEmpty else { return nil }

        if chars.count == 1, let scalar = chars.unicodeScalars.first {
            if isControlCharacter(scalar) {
                return event.characters(byApplyingModifiers: event.modifierFlags.subtracting(.control))
            }

            if scalar.value >= 0xF700 && scalar.value <= 0xF8FF {
                return nil
            }
        }

        return chars
    }

    private func isControlCharacter(_ scalar: UnicodeScalar) -> Bool {
        scalar.value < 0x20 || scalar.value == 0x7F
    }

    private func shouldSendText(_ text: String) -> Bool {
        guard !text.isEmpty else { return false }
        if text.count == 1, let scalar = text.unicodeScalars.first {
            return !isControlCharacter(scalar)
        }
        return true
    }

    private func routeNativeKeyCommandIfNeeded(_ event: NSEvent) -> Bool {
        switch surfaceController.inputAdapter.routeKey(terminalKeyInput(for: event)) {
        case .nativeCommand("find"):
            surfaceController.beginSearch()
            syncOverlayVisibility()
            publishRuntimeSnapshot()
            return true
        case .nativeCommand("quit"):
            return false
        case .nativeCommand, .terminalText, .terminalKey, .ignored:
            return false
        }
    }

    private func handleSearchKeyIfNeeded(_ event: NSEvent) -> Bool {
        guard surfaceController.searchAdapter?.state.isActive == true else { return false }
        guard event.type == .keyDown else { return true }
        if event.modifierFlags.intersection(.deviceIndependentFlagsMask).contains(.command) {
            return false
        }

        switch event.keyCode {
        case 0x35:
            surfaceController.dismissSearch()
        case 0x24:
            surfaceController.nextSearchMatch()
        case 0x33:
            let current = surfaceController.searchAdapter?.state.query ?? ""
            surfaceController.updateSearchQuery(String(current.dropLast()))
        default:
            if let characters = textForKeyEvent(event), shouldSendText(characters) {
                let current = surfaceController.searchAdapter?.state.query ?? ""
                surfaceController.updateSearchQuery(current + characters)
            }
        }

        syncOverlayVisibility()
        publishRuntimeSnapshot()
        return true
    }

    private func terminalKeyInput(for event: NSEvent) -> AlanTerminalKeyInput {
        let phase: AlanTerminalKeyPhase
        switch event.type {
        case .keyDown:
            phase = .down
        case .keyUp:
            phase = .up
        case .flagsChanged:
            phase = .flagsChanged
        default:
            phase = .down
        }
        return AlanTerminalKeyInput(
            characters: event.charactersIgnoringModifiers ?? event.characters,
            keyCode: event.keyCode,
            modifiers: terminalKeyModifiers(from: event.modifierFlags),
            phase: phase,
            isRepeat: event.isARepeat
        )
    }

    private func terminalKeyModifiers(from flags: NSEvent.ModifierFlags) -> AlanTerminalKeyModifiers {
        var modifiers: AlanTerminalKeyModifiers = []
        if flags.contains(.shift) { modifiers.insert(.shift) }
        if flags.contains(.control) { modifiers.insert(.control) }
        if flags.contains(.option) { modifiers.insert(.option) }
        if flags.contains(.command) { modifiers.insert(.command) }
        return modifiers
    }

    private func isApplicationReservedKeyEquivalent(_ event: NSEvent) -> Bool {
        guard event.type == .keyDown else { return false }

        let flags = event.modifierFlags
            .intersection(.deviceIndependentFlagsMask)
            .subtracting([.capsLock, .numericPad, .function])
        guard flags == .command else { return false }

        return event.charactersIgnoringModifiers?.lowercased() == "q"
    }

    private func syncPreedit(clearIfNeeded: Bool = true) {
#if canImport(GhosttyKit)
        if markedText.length > 0 {
            surfaceController.sendPreedit(markedText.string)
        } else if clearIfNeeded {
            surfaceController.sendPreedit(nil)
        }
#endif
    }

    // MARK: NSTextInputClient

    func insertText(_ string: Any, replacementRange: NSRange) {
        let characters: String
        switch string {
        case let value as NSAttributedString:
            characters = value.string
        case let value as String:
            characters = value
        default:
            return
        }

        unmarkText()

        if var keyTextAccumulator {
            keyTextAccumulator.append(characters)
            self.keyTextAccumulator = keyTextAccumulator
        } else {
#if canImport(GhosttyKit)
            surfaceController.sendText(characters)
#endif
        }
    }

    override func insertText(_ insertString: Any) {
        insertText(insertString, replacementRange: NSRange(location: NSNotFound, length: 0))
    }

    override func doCommand(by selector: Selector) {}

    func setMarkedText(_ string: Any, selectedRange: NSRange, replacementRange: NSRange) {
        switch string {
        case let value as NSAttributedString:
            markedText = NSMutableAttributedString(attributedString: value)
        case let value as String:
            markedText = NSMutableAttributedString(string: value)
        default:
            return
        }

        if keyTextAccumulator == nil {
            syncPreedit()
        }
    }

    func unmarkText() {
        guard markedText.length > 0 else { return }
        markedText.mutableString.setString("")
        syncPreedit()
    }

    func selectedRange() -> NSRange {
#if canImport(GhosttyKit)
        if let selection = surfaceController.readSelectionText() {
            return NSRange(location: 0, length: selection.utf16.count)
        }
#endif
        return NSRange(location: NSNotFound, length: 0)
    }

    func markedRange() -> NSRange {
        markedText.length > 0
            ? NSRange(location: 0, length: markedText.length)
            : NSRange(location: NSNotFound, length: 0)
    }

    func hasMarkedText() -> Bool {
        markedText.length > 0
    }

    func attributedSubstring(forProposedRange range: NSRange, actualRange: NSRangePointer?) -> NSAttributedString? {
#if canImport(GhosttyKit)
        guard let selection = surfaceController.readSelectionText(), !selection.isEmpty else { return nil }
        actualRange?.pointee = NSRange(location: 0, length: selection.utf16.count)
        return NSAttributedString(string: selection)
#else
        return nil
#endif
    }

    func validAttributesForMarkedText() -> [NSAttributedString.Key] {
        []
    }

    func firstRect(forCharacterRange range: NSRange, actualRange: NSRangePointer?) -> NSRect {
#if canImport(GhosttyKit)
        if let imeRect = surfaceController.imeRect(in: self) {
            return imeRect
        }
#endif
        guard let window else { return frame }
        return window.convertToScreen(convert(bounds, to: nil))
    }

    func characterIndex(for point: NSPoint) -> Int {
        0
    }
}

private func makeCanvasView() -> NSView {
#if canImport(GhosttyKit)
    let view = AlanGhosttyCanvasView(frame: .zero)
#else
    let view = AlanTerminalFallbackCanvasView(frame: .zero)
    view.wantsLayer = true
    view.layer?.backgroundColor = NSColor.clear.cgColor
#endif
    view.translatesAutoresizingMaskIntoConstraints = false
    return view
}

final class AlanTerminalFallbackCanvasView: NSView {
    override var mouseDownCanMoveWindow: Bool { false }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}

final class AlanTerminalPassiveOverlayView: NSVisualEffectView {
    override var mouseDownCanMoveWindow: Bool { false }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}
#endif
