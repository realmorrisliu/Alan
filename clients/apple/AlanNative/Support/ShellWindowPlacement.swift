import SwiftUI

#if os(macOS)
import AppKit

struct ShellWindowPlacementView: NSViewRepresentable {
    @Binding var metrics: ShellWindowChromeMetrics

    func makeNSView(context: Context) -> ShellWindowPlacementNSView {
        let metricsBinding = _metrics
        return ShellWindowPlacementNSView { metrics in
            let newMetrics = metrics
            DispatchQueue.main.async {
                guard metricsBinding.wrappedValue != newMetrics else { return }
                metricsBinding.wrappedValue = newMetrics
            }
        }
    }

    func updateNSView(_ nsView: ShellWindowPlacementNSView, context: Context) {
        let metricsBinding = _metrics
        nsView.updateMetricsHandler { metrics in
            let newMetrics = metrics
            DispatchQueue.main.async {
                guard metricsBinding.wrappedValue != newMetrics else { return }
                metricsBinding.wrappedValue = newMetrics
            }
        }
        nsView.resolveWindowIfNeeded()
    }
}

struct ShellWindowChromeMetrics: Equatable {
    var trafficLightsTopInset: CGFloat = 0
}

final class ShellWindowPlacementNSView: NSView {
    private var metricsHandler: (ShellWindowChromeMetrics) -> Void
    private var lastPublishedMetrics: ShellWindowChromeMetrics?

    init(metricsHandler: @escaping (ShellWindowChromeMetrics) -> Void) {
        self.metricsHandler = metricsHandler
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

    func updateMetricsHandler(_ handler: @escaping (ShellWindowChromeMetrics) -> Void) {
        metricsHandler = handler
    }

    func resolveWindowIfNeeded() {
        DispatchQueue.main.async { [weak self] in
            guard let window = self?.window else { return }
            AlanShellWindowPlacement.apply(to: window)
            let metrics = AlanShellWindowPlacement.chromeMetrics(for: window)
            guard self?.lastPublishedMetrics != metrics else { return }
            self?.lastPublishedMetrics = metrics
            self?.metricsHandler(metrics)
        }
    }
}

private enum AlanShellWindowPlacement {
    private static var positionedWindowNumbers: Set<Int> = []

    static func apply(to window: NSWindow) {
        window.title = "Alan"
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
    }

    static func chromeMetrics(for window: NSWindow) -> ShellWindowChromeMetrics {
        let buttonFrames = [
            NSWindow.ButtonType.closeButton,
            .miniaturizeButton,
            .zoomButton,
        ].compactMap { buttonType -> NSRect? in
            window.standardWindowButton(buttonType)?.frame
        }

        guard let firstFrame = buttonFrames.first else {
            return ShellWindowChromeMetrics()
        }

        let trafficLightsFrame = buttonFrames.dropFirst().reduce(firstFrame) { partialResult, frame in
            partialResult.union(frame)
        }
        let topInset = max(0, trafficLightsFrame.maxY + 10)

        return ShellWindowChromeMetrics(
            trafficLightsTopInset: ceil(topInset)
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
