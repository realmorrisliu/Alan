import AppKit
import Foundation

@main
struct ShellWindowPlacementTestRunner {
    static func main() throws {
        try ShellWindowPlacementTests.run()
    }
}

private enum ShellWindowPlacementTests {
    static func run() throws {
        try verifiesDefaultFrameUsesVisibleRegionRatios()
        try verifiesDefaultFrameFitsSmallVisibleRegions()
        try verifiesZoomFrameUsesEntireVisibleRegion()
        try verifiesApproximateZoomFrameMatching()
        try verifiesTitlebarPointOutsideContentViewCanTriggerDoubleClickZoom()
        try verifiesTrafficLightAreaDoesNotTriggerDoubleClickZoom()
        try verifiesTitlebarOverlayAcceptsTopBlankHit()
        try verifiesTitlebarOverlayRejectsTrafficLightHit()
        try verifiesAppearanceModeAppliesToAttachedWindowImmediately()
        try verifiesSystemModeClearsExplicitWindowAppearanceImmediately()
        print("Shell window placement tests passed.")
    }

    private static func verifiesDefaultFrameUsesVisibleRegionRatios() throws {
        let visibleFrame = CGRect(x: 10, y: 40, width: 1600, height: 1000)

        let frame = ShellWindowSizing.defaultFrame(in: visibleFrame)

        expect(frame.width == 1440, "default window width must use 90% of visible width")
        expect(frame.height == 800, "default window height must use 80% of visible height")
        expect(frame.midX == visibleFrame.midX, "default window frame must be horizontally centered")
        expect(frame.midY == visibleFrame.midY, "default window frame must be vertically centered")
    }

    private static func verifiesDefaultFrameFitsSmallVisibleRegions() throws {
        let visibleFrame = CGRect(x: 0, y: 80, width: 1000, height: 700)

        let frame = ShellWindowSizing.defaultFrame(in: visibleFrame)

        expect(
            visibleFrame.contains(frame),
            "default window frame must stay inside the visible screen region"
        )
        expect(frame.width == 968, "default window width must clamp to visible width with margins")
        expect(frame.height == 668, "default window height must clamp to visible height with margins")
    }

    private static func verifiesZoomFrameUsesEntireVisibleRegion() throws {
        let visibleFrame = CGRect(x: -120, y: 60, width: 1920, height: 1080)

        let frame = ShellWindowSizing.zoomFrame(in: visibleFrame)

        expect(frame == visibleFrame, "zoom frame must match the screen visible region exactly")
    }

    private static func verifiesApproximateZoomFrameMatching() throws {
        let visibleFrame = CGRect(x: 0, y: 60, width: 1512, height: 889)
        let nearlyZoomed = CGRect(x: 0.5, y: 59.5, width: 1511.5, height: 889.5)
        let notZoomed = CGRect(x: 20, y: 80, width: 1200, height: 760)

        expect(
            ShellWindowSizing.frame(nearlyZoomed, approximatelyMatches: visibleFrame),
            "zoom frame matching must allow AppKit rounding differences"
        )
        expect(
            !ShellWindowSizing.frame(notZoomed, approximatelyMatches: visibleFrame),
            "zoom frame matching must reject clearly different frames"
        )
    }

    private static func verifiesTitlebarPointOutsideContentViewCanTriggerDoubleClickZoom() throws {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 800, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )

        let location = CGPoint(x: 420, y: window.frame.height - 12)

        expect(
            ShellWindowDoubleClickZoomHitTesting.isWindowTopChromeZoomCandidate(
                locationInWindow: location,
                in: window
            ),
            "top titlebar blank points must trigger double-click visible-frame zoom"
        )
    }

    private static func verifiesTrafficLightAreaDoesNotTriggerDoubleClickZoom() throws {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 800, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )

        let location = CGPoint(x: 42, y: window.frame.height - 18)

        expect(
            !ShellWindowDoubleClickZoomHitTesting.isWindowTopChromeZoomCandidate(
                locationInWindow: location,
                in: window
            ),
            "traffic light region must keep its standard button behavior"
        )
    }

    private static func verifiesTitlebarOverlayAcceptsTopBlankHit() throws {
        let window = makeTestWindow()
        let overlay = ShellWindowDoubleClickZoomOverlayView(
            frame: CGRect(origin: .zero, size: window.frame.size)
        )
        window.contentView?.addSubview(overlay)

        let windowPoint = CGPoint(x: 420, y: window.frame.height - 12)
        let overlayPoint = overlay.convert(windowPoint, from: nil)

        expect(
            overlay.hitTest(overlayPoint) === overlay,
            "titlebar overlay must accept top blank titlebar hits"
        )
    }

    private static func verifiesTitlebarOverlayRejectsTrafficLightHit() throws {
        let window = makeTestWindow()
        let overlay = ShellWindowDoubleClickZoomOverlayView(
            frame: CGRect(origin: .zero, size: window.frame.size)
        )
        window.contentView?.addSubview(overlay)

        let windowPoint = CGPoint(x: 42, y: window.frame.height - 18)
        let overlayPoint = overlay.convert(windowPoint, from: nil)

        expect(
            overlay.hitTest(overlayPoint) == nil,
            "titlebar overlay must leave traffic light hits to standard window buttons"
        )
    }

    private static func verifiesAppearanceModeAppliesToAttachedWindowImmediately() throws {
        let (window, placementView) = makeAttachedPlacementView()

        placementView.updateAppearanceMode(.dark)

        expect(
            window.appearance?.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua,
            "dark appearance mode must apply to the attached window immediately"
        )
    }

    private static func verifiesSystemModeClearsExplicitWindowAppearanceImmediately() throws {
        let (window, placementView) = makeAttachedPlacementView()
        placementView.updateAppearanceMode(.dark)

        placementView.updateAppearanceMode(.system)

        expect(
            window.appearance == nil,
            "system appearance mode must clear the explicit window appearance immediately"
        )
    }

    private static func makeAttachedPlacementView() -> (
        window: NSWindow,
        placementView: ShellWindowPlacementNSView
    ) {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 320, height: 240),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        let contentView = NSView(frame: window.contentView?.bounds ?? .zero)
        let placementView = ShellWindowPlacementNSView(appearanceMode: .system) { _ in }

        contentView.addSubview(placementView)
        window.contentView = contentView

        return (window, placementView)
    }

    private static func makeTestWindow() -> NSWindow {
        NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 800, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: @autoclosure () -> String
    ) {
        guard condition() else {
            fatalError(message())
        }
    }
}
