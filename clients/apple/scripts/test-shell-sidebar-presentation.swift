import CoreGraphics
import Foundation

@main
struct ShellSidebarPresentationTestRunner {
    static func main() throws {
        try ShellSidebarPresentationTests.run()
    }
}

private enum ShellSidebarPresentationTests {
    private static let configuration = ShellSidebarPresentationConfiguration(
        sidebarWidth: 264,
        floatingSidebarInset: 6,
        floatingCornerRadius: 18
    )

    static func run() throws {
        try verifiesCollapsedHiddenHasNoVisibleSurface()
        try verifiesFloatingRevealUsesOverlaySurfaceWithoutLayoutReservation()
        try verifiesFloatingRevealKeepsBalancedVerticalInsets()
        try verifiesFloatingToPinnedMorphKeepsOneVisibleSurface()
        try verifiesPinnedPhaseUsesPinnedLayoutSurface()
        print("Shell sidebar presentation tests passed.")
    }

    private static func verifiesCollapsedHiddenHasNoVisibleSurface() throws {
        let snapshot = ShellSidebarPresentationSnapshot(
            phase: .collapsedHidden,
            configuration: configuration
        )

        expect(snapshot.layoutProgress == 0, "collapsed hidden state must not reserve sidebar layout")
        expect(!snapshot.isSurfaceVisible, "collapsed hidden state must not render a sidebar surface")
        expect(snapshot.visibleSurfaceCount == 0, "collapsed hidden state must render zero surfaces")
        expect(!snapshot.chromeSurface.isVisible, "collapsed hidden state must hide window chrome")
    }

    private static func verifiesFloatingRevealUsesOverlaySurfaceWithoutLayoutReservation() throws {
        let snapshot = ShellSidebarPresentationSnapshot(
            phase: .floatingRevealed(showsTrafficLights: true),
            configuration: configuration
        )

        expect(snapshot.layoutProgress == 0, "floating reveal must not resize terminal layout")
        expect(snapshot.showsOverlaySurface, "floating reveal must render the overlay surface")
        expect(!snapshot.showsPinnedSurfaceContent, "floating reveal must not render pinned content")
        expect(snapshot.visibleSurfaceCount == 1, "floating reveal must render exactly one surface")
        expect(
            snapshot.surfaceOrigin.isApproximately(CGPoint(x: 6, y: 6)),
            "floating reveal origin must use the floating inset"
        )
        expect(snapshot.floatingTreatmentProgress == 1, "floating reveal must use floating treatment")
        expect(snapshot.chromeSurface.isVisible, "floating reveal must expose the sidebar chrome surface")
        expect(
            snapshot.chromeSurface.showsStandardTrafficLights,
            "floating reveal must be able to show native traffic lights"
        )
    }

    private static func verifiesFloatingRevealKeepsBalancedVerticalInsets() throws {
        let snapshot = ShellSidebarPresentationSnapshot(
            phase: .floatingRevealed(showsTrafficLights: true),
            configuration: configuration
        )
        let frame = snapshot.visibleSurfaceFrame(in: CGSize(width: 1200, height: 800))

        expect(
            frame.minY.isApproximately(6),
            "floating reveal frame must start at the floating top inset"
        )
        expect(
            frame.maxY.isApproximately(794),
            "floating reveal frame must preserve the floating bottom inset"
        )
        expect(
            frame.height.isApproximately(788),
            "floating reveal frame height must subtract top and bottom insets"
        )

        let midpoint = ShellSidebarPresentationSnapshot(
            phase: .morphingFloatingToPinned(progress: 0.5),
            configuration: configuration
        )
        let midpointFrame = midpoint.visibleSurfaceFrame(in: CGSize(width: 1200, height: 800))

        expect(
            midpointFrame.minY.isApproximately(3),
            "morph frame must interpolate the top inset"
        )
        expect(
            midpointFrame.maxY.isApproximately(797),
            "morph frame must interpolate the bottom inset"
        )
    }

    private static func verifiesFloatingToPinnedMorphKeepsOneVisibleSurface() throws {
        let start = ShellSidebarPresentationSnapshot(
            phase: .morphingFloatingToPinned(progress: 0),
            configuration: configuration
        )
        let midpoint = ShellSidebarPresentationSnapshot(
            phase: .morphingFloatingToPinned(progress: 0.5),
            configuration: configuration
        )
        let end = ShellSidebarPresentationSnapshot(
            phase: .morphingFloatingToPinned(progress: 1),
            configuration: configuration
        )

        expect(start.visibleSurfaceCount == 1, "morph start must render exactly one surface")
        expect(midpoint.visibleSurfaceCount == 1, "morph midpoint must render exactly one surface")
        expect(end.visibleSurfaceCount == 1, "morph end must render exactly one surface")

        expect(start.showsOverlaySurface, "morph must keep the visible overlay surface mounted")
        expect(!start.showsPinnedSurfaceContent, "morph start must not duplicate pinned content")
        expect(!midpoint.showsPinnedSurfaceContent, "morph midpoint must not duplicate pinned content")
        expect(!end.showsPinnedSurfaceContent, "morph end must not duplicate pinned content before settlement")

        expect(start.layoutProgress == 0, "morph start must not reserve pinned layout")
        expect(midpoint.layoutProgress == 0.5, "morph midpoint must open layout continuously")
        expect(end.layoutProgress == 1, "morph end must reserve full pinned layout")

        expect(
            start.surfaceOrigin.isApproximately(CGPoint(x: 6, y: 6)),
            "morph start must begin at the floating origin"
        )
        expect(
            midpoint.surfaceOrigin.isApproximately(CGPoint(x: 3, y: 3)),
            "morph midpoint must interpolate surface origin"
        )
        expect(
            end.surfaceOrigin.isApproximately(.zero),
            "morph end must settle at the pinned origin"
        )

        expect(
            start.floatingTreatmentProgress == 1,
            "morph start must use floating visual treatment"
        )
        expect(
            midpoint.floatingTreatmentProgress == 0.5,
            "morph midpoint must fade floating treatment continuously"
        )
        expect(end.floatingTreatmentProgress == 0, "morph end must remove floating treatment")

        expect(
            midpoint.chromeSurface.origin.isApproximately(midpoint.surfaceOrigin),
            "window chrome must use the same interpolated origin as the visible surface"
        )
        expect(
            midpoint.chromeSurface.showsStandardTrafficLights,
            "pin morph must preserve native traffic lights"
        )
    }

    private static func verifiesPinnedPhaseUsesPinnedLayoutSurface() throws {
        let snapshot = ShellSidebarPresentationSnapshot(
            phase: .pinned(progress: 1),
            configuration: configuration
        )

        expect(snapshot.layoutProgress == 1, "pinned state must reserve full sidebar layout")
        expect(snapshot.showsPinnedSurfaceContent, "pinned state must render pinned content")
        expect(!snapshot.showsOverlaySurface, "pinned state must not render an overlay surface")
        expect(snapshot.visibleSurfaceCount == 1, "pinned state must render exactly one surface")
        expect(snapshot.surfaceOrigin.isApproximately(.zero), "pinned state origin must be zero")
        expect(snapshot.chromeSurface.isVisible, "pinned state must expose window chrome")
    }

    private static func expect(_ condition: @autoclosure () -> Bool, _ message: String) {
        guard condition() else {
            fatalError(message)
        }
    }
}

private extension CGFloat {
    func isApproximately(_ other: CGFloat, tolerance: CGFloat = 0.001) -> Bool {
        abs(self - other) <= tolerance
    }
}

private extension CGPoint {
    func isApproximately(_ other: CGPoint, tolerance: CGFloat = 0.001) -> Bool {
        x.isApproximately(other.x, tolerance: tolerance)
            && y.isApproximately(other.y, tolerance: tolerance)
    }
}
