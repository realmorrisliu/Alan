import SwiftUI

#if os(macOS)
import AppKit

struct ShellWindowPlacementView: NSViewRepresentable {
    @Binding var metrics: ShellWindowChromeMetrics
    let appearanceMode: ShellAppearanceMode
    @Binding var systemColorScheme: ColorScheme

    func makeNSView(context: Context) -> ShellWindowPlacementNSView {
        let metricsBinding = _metrics
        let systemColorSchemeBinding = _systemColorScheme
        return ShellWindowPlacementNSView(
            appearanceMode: appearanceMode,
            metricsHandler: metricsHandler(metricsBinding),
            systemAppearanceHandler: systemAppearanceHandler(systemColorSchemeBinding)
        )
    }

    func updateNSView(_ nsView: ShellWindowPlacementNSView, context: Context) {
        let metricsBinding = _metrics
        let systemColorSchemeBinding = _systemColorScheme
        nsView.updateAppearanceMode(appearanceMode)
        nsView.updateMetricsHandler(metricsHandler(metricsBinding))
        nsView.updateSystemAppearanceHandler(systemAppearanceHandler(systemColorSchemeBinding))
        nsView.resolveWindowIfNeeded()
    }

    private func metricsHandler(
        _ metricsBinding: Binding<ShellWindowChromeMetrics>
    ) -> (ShellWindowChromeMetrics) -> Void {
        { metrics in
            DispatchQueue.main.async {
                guard metricsBinding.wrappedValue != metrics else { return }
                metricsBinding.wrappedValue = metrics
            }
        }
    }

    private func systemAppearanceHandler(
        _ systemColorSchemeBinding: Binding<ColorScheme>
    ) -> (ColorScheme) -> Void {
        { colorScheme in
            DispatchQueue.main.async {
                guard systemColorSchemeBinding.wrappedValue != colorScheme else { return }
                systemColorSchemeBinding.wrappedValue = colorScheme
            }
        }
    }
}

struct ShellWindowChromeMetrics: Equatable {
    var trafficLightGroupFrame = CGRect(
        x: ShellSidebarMetrics.trafficLightLeadingInset,
        y: ShellSidebarMetrics.trafficLightTopInset,
        width: ShellSidebarMetrics.trafficLightFallbackGroupWidth,
        height: ShellSidebarMetrics.trafficLightFallbackButtonHeight
    )

    var titlebarToolLeadingInset: CGFloat {
        trafficLightGroupFrame.maxX + ShellSidebarMetrics.titlebarToolGapAfterTrafficLights
    }

    var titlebarToolTopInset: CGFloat {
        max(
            0,
            trafficLightGroupFrame.midY - (ShellSidebarMetrics.titlebarToolHeight / 2)
        )
    }

    var commandLauncherTopInset: CGFloat {
        trafficLightGroupFrame.maxY + ShellSidebarMetrics.commandLauncherGapBelowTrafficLights
    }

    var collapsedRevealHeaderHeight: CGFloat {
        commandLauncherTopInset + ShellSidebarMetrics.commandLauncherHeight + 8
    }
}

final class ShellWindowPlacementNSView: NSView {
    private var appearanceMode: ShellAppearanceMode
    private var metricsHandler: (ShellWindowChromeMetrics) -> Void
    private var systemAppearanceHandler: (ColorScheme) -> Void
    private var lastPublishedMetrics: ShellWindowChromeMetrics?

    init(
        appearanceMode: ShellAppearanceMode,
        metricsHandler: @escaping (ShellWindowChromeMetrics) -> Void,
        systemAppearanceHandler: @escaping (ColorScheme) -> Void = { _ in }
    ) {
        self.appearanceMode = appearanceMode
        self.metricsHandler = metricsHandler
        self.systemAppearanceHandler = systemAppearanceHandler
        super.init(frame: .zero)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        resolveWindowIfNeeded()
    }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        resolveWindowIfNeeded()
    }

    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        publishEffectiveSystemColorScheme()
    }

    func updateMetricsHandler(_ handler: @escaping (ShellWindowChromeMetrics) -> Void) {
        metricsHandler = handler
    }

    func updateSystemAppearanceHandler(_ handler: @escaping (ColorScheme) -> Void) {
        systemAppearanceHandler = handler
        publishEffectiveSystemColorScheme()
    }

    func updateAppearanceMode(_ mode: ShellAppearanceMode) {
        let didChange = appearanceMode != mode
        appearanceMode = mode
        guard didChange else { return }
        applyAppearanceToAttachedWindow()
        publishEffectiveSystemColorScheme()
    }

    func resolveWindowIfNeeded() {
        DispatchQueue.main.async { [weak self] in
            guard let self, let window = self.window else { return }
            let metrics = AlanShellWindowPlacement.apply(to: window, appearanceMode: self.appearanceMode)
            self.publishEffectiveSystemColorScheme()
            guard self.lastPublishedMetrics != metrics else { return }
            self.lastPublishedMetrics = metrics
            self.metricsHandler(metrics)
        }
    }

    private func applyAppearanceToAttachedWindow() {
        guard let window else { return }
        AlanShellWindowPlacement.applyAppearance(to: window, appearanceMode: appearanceMode)
    }

    private func publishEffectiveSystemColorScheme() {
        let appearance = window?.effectiveAppearance ?? effectiveAppearance
        systemAppearanceHandler(ShellAppearanceMode.colorScheme(for: appearance))
    }
}

private enum AlanShellWindowPlacement {
    private static var positionedWindowNumbers: Set<Int> = []

    static func apply(to window: NSWindow, appearanceMode: ShellAppearanceMode) -> ShellWindowChromeMetrics {
        window.title = "Alan"
        applyAppearance(to: window, appearanceMode: appearanceMode)
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.titlebarSeparatorStyle = .none
        window.styleMask.insert(.fullSizeContentView)
        window.isMovableByWindowBackground = true
        window.minSize = NSSize(width: 1180, height: 760)
        window.tabbingMode = .disallowed

        if positionedWindowNumbers.insert(window.windowNumber).inserted {
            window.setFrame(centeredFrameOnMainScreen(for: window), display: true)
        } else if shouldResetFrame(window.frame) {
            window.setFrame(centeredFrame(for: window), display: true)
        }

        if !window.isVisible {
            window.makeKeyAndOrderFront(nil)
        }

        NSApp.activate(ignoringOtherApps: true)

        return chromeMetrics(for: window)
    }

    static func applyAppearance(to window: NSWindow, appearanceMode: ShellAppearanceMode) {
        let appearance = appearanceMode.nsAppearanceName.flatMap(NSAppearance.init(named:))
        window.appearance = appearance
        window.contentView?.appearance = appearance
        window.contentView?.needsDisplay = true
        window.displayIfNeeded()
    }

    private static func chromeMetrics(for window: NSWindow) -> ShellWindowChromeMetrics {
        var metrics = ShellWindowChromeMetrics()

        if let trafficLightGroupFrame = repositionStandardWindowButtons(in: window) {
            metrics.trafficLightGroupFrame = trafficLightGroupFrame
        }

        return metrics
    }

    private static func repositionStandardWindowButtons(in window: NSWindow) -> CGRect? {
        let buttonTypes: [NSWindow.ButtonType] = [.closeButton, .miniaturizeButton, .zoomButton]
        let buttons = buttonTypes.compactMap { window.standardWindowButton($0) }

        guard buttons.count == buttonTypes.count,
              let superview = buttons.first?.superview,
              buttons.allSatisfy({ $0.superview === superview })
        else {
            return nil
        }

        let currentGroupFrame = buttons
            .map(\.frame)
            .reduce(NSRect.null) { $0.union($1) }
        guard !currentGroupFrame.isNull else { return nil }

        let currentVisualTopInset = visualTopInset(of: currentGroupFrame, in: superview)
        let deltaX = ShellSidebarMetrics.trafficLightLeadingInset - currentGroupFrame.minX
        let deltaTop = ShellSidebarMetrics.trafficLightTopInset - currentVisualTopInset
        let deltaY = superview.isFlipped ? deltaTop : -deltaTop

        for button in buttons {
            var frame = button.frame
            frame.origin.x += deltaX
            frame.origin.y += deltaY
            button.setFrameOrigin(frame.origin)
        }

        let movedGroupFrame = buttons
            .map(\.frame)
            .reduce(NSRect.null) { $0.union($1) }
        guard !movedGroupFrame.isNull else { return nil }

        return topLeadingFrame(for: movedGroupFrame, in: superview, window: window)
    }

    private static func visualTopInset(of frame: NSRect, in view: NSView) -> CGFloat {
        if view.isFlipped {
            return frame.minY
        }

        return max(0, view.bounds.height - frame.maxY)
    }

    private static func topLeadingFrame(
        for frame: NSRect,
        in view: NSView,
        window: NSWindow
    ) -> CGRect? {
        guard let contentView = window.contentView else { return nil }
        let windowFrame = view.convert(frame, to: nil)
        let contentFrame = contentView.convert(windowFrame, from: nil)
        let topInset = visualTopInset(of: contentFrame, in: contentView)

        return CGRect(
            x: contentFrame.minX,
            y: topInset,
            width: contentFrame.width,
            height: contentFrame.height
        )
    }

    private static func shouldResetFrame(_ frame: NSRect) -> Bool {
        let screens = NSScreen.screens
        guard !screens.isEmpty else { return false }

        let frameArea = max(frame.width * frame.height, 1)
        let largestVisibleArea = screens
            .map(\.visibleFrame)
            .map { visibleFrame -> CGFloat in
                let intersection = visibleFrame.intersection(frame)
                guard !intersection.isNull else { return 0 }
                return intersection.width * intersection.height
            }
            .max() ?? 0

        let visibleRatio = largestVisibleArea / frameArea
        let targetVisibleFrame = preferredVisibleFrame(for: frame)

        return visibleRatio < 0.55
            || frame.width > targetVisibleFrame.width
            || frame.height > targetVisibleFrame.height
            || !targetVisibleFrame.insetBy(dx: -48, dy: -48).intersects(frame)
    }

    private static func centeredFrame(for window: NSWindow) -> NSRect {
        let visibleFrame = preferredVisibleFrame(for: window.frame)
        return centeredFrame(in: visibleFrame, minSize: window.minSize)
    }

    private static func centeredFrameOnMainScreen(for window: NSWindow) -> NSRect {
        let visibleFrame = primaryVisibleFrame() ?? preferredVisibleFrame(for: window.frame)
        return centeredFrame(in: visibleFrame, minSize: window.minSize)
    }

    private static func centeredFrame(in visibleFrame: NSRect, minSize: NSSize) -> NSRect {
        let targetWidth = min(
            max(visibleFrame.width * 0.84, minSize.width),
            visibleFrame.width - 32
        )
        let targetHeight = min(
            max(visibleFrame.height * 0.86, minSize.height),
            visibleFrame.height - 32
        )

        let origin = CGPoint(
            x: visibleFrame.midX - (targetWidth / 2),
            y: visibleFrame.midY - (targetHeight / 2)
        )

        return NSRect(origin: origin, size: NSSize(width: targetWidth, height: targetHeight))
    }

    private static func preferredVisibleFrame(for frame: NSRect) -> NSRect {
        if let containingScreen = NSScreen.screens.first(where: {
            !$0.visibleFrame.intersection(frame).isNull
        }) {
            return containingScreen.visibleFrame
        }

        if let mainFrame = NSScreen.main?.visibleFrame {
            return mainFrame
        }

        return NSScreen.screens.first?.visibleFrame ?? NSRect(x: 80, y: 80, width: 1440, height: 900)
    }

    private static func primaryVisibleFrame() -> NSRect? {
        if let originScreen = NSScreen.screens.first(where: {
            abs($0.frame.minX) < 1 && abs($0.frame.minY) < 1
        }) {
            return originScreen.visibleFrame
        }

        return NSScreen.screens
            .sorted {
                if $0.frame.minY == $1.frame.minY {
                    return $0.frame.minX < $1.frame.minX
                }
                return $0.frame.minY < $1.frame.minY
            }
            .first?
            .visibleFrame
    }
}
#endif
