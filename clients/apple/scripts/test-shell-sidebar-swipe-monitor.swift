import AppKit
import Foundation

#if os(macOS)
@main
struct ShellSidebarSwipeMonitorTestRunner {
    @MainActor
    static func main() {
        ShellSidebarSwipeMonitorTests.run()
    }
}

@MainActor
private enum ShellSidebarSwipeMonitorTests {
    static func run() {
        verifiesPhaseLessVerticalGestureResetsAfterIdle()
        verifiesPhaseLessUndecidedGestureResetsAfterIdle()
        verifiesPhaseLessHorizontalGestureStillEndsAfterIdle()
        verifiesPhasefulHorizontalGestureEndsOnRelease()
        verifiesPhasefulHorizontalGestureCancelsOnCancelPhase()
        verifiesFastFlickUsesTerminalDelta()
        verifiesPagerOffsetsPagesFromSharedDragOffset()
        verifiesPagerEdgeResistanceDoesNotRenderWrappedTarget()
        verifiesPagerSettlementPhaseExposesCommitAndCancelState()
        print("Shell sidebar swipe monitor tests passed.")
    }

    private static func verifiesPhaseLessVerticalGestureResetsAfterIdle() {
        let harness = MonitorHarness()

        let verticalEvent = harness.event(deltaX: 0, deltaY: 8)
        let verticalResult = harness.coordinator.handleForTesting(verticalEvent)
        expect(verticalResult === verticalEvent, "vertical wheel input must pass through")
        expect(harness.updates.isEmpty, "vertical wheel input must not emit space swipe updates")

        drainMainQueue()

        let horizontalEvent = harness.event(deltaX: 8, deltaY: 0)
        let horizontalResult = harness.coordinator.handleForTesting(horizontalEvent)
        expect(horizontalResult == nil, "horizontal input after idle vertical input must be consumed")
        expect(
            harness.updates.map(\.phase) == [.began, .changed],
            "phase-less vertical intent must reset before the next horizontal swipe"
        )

        harness.uninstall()
    }

    private static func verifiesPhaseLessUndecidedGestureResetsAfterIdle() {
        let harness = MonitorHarness()

        let undecidedEvent = harness.event(deltaX: 0, deltaY: 4.8)
        let undecidedResult = harness.coordinator.handleForTesting(undecidedEvent)
        expect(undecidedResult == nil, "undecided wheel input inside the sidebar should be consumed")
        expect(harness.updates.isEmpty, "undecided wheel input must not emit space swipe updates")

        drainMainQueue()

        let horizontalEvent = harness.event(deltaX: 5.1, deltaY: 0)
        let horizontalResult = harness.coordinator.handleForTesting(horizontalEvent)
        expect(horizontalResult == nil, "horizontal input after idle undecided input must be consumed")
        expect(
            harness.updates.map(\.phase) == [.began, .changed],
            "phase-less undecided input must not bias the next horizontal swipe"
        )

        harness.uninstall()
    }

    private static func verifiesPhaseLessHorizontalGestureStillEndsAfterIdle() {
        let harness = MonitorHarness()

        let horizontalEvent = harness.event(deltaX: 8, deltaY: 0)
        let horizontalResult = harness.coordinator.handleForTesting(horizontalEvent)
        expect(horizontalResult == nil, "horizontal wheel input must be consumed")
        expect(
            harness.updates.map(\.phase) == [.began, .changed],
            "horizontal wheel input must begin and change immediately"
        )

        drainMainQueue()

        expect(
            harness.updates.map(\.phase) == [.began, .changed, .ended],
            "phase-less horizontal input must still synthesize an idle end"
        )

        harness.uninstall()
    }

    private static func verifiesPhasefulHorizontalGestureEndsOnRelease() {
        let harness = MonitorHarness()

        let beganEvent = harness.event(deltaX: 8, deltaY: 0, phase: .began)
        let beganResult = harness.coordinator.handleForTesting(beganEvent)
        expect(beganResult == nil, "phaseful horizontal input must be consumed")

        let endedEvent = harness.event(deltaX: 0, deltaY: 0, phase: .ended)
        let endedResult = harness.coordinator.handleForTesting(endedEvent)
        expect(endedResult == nil, "phaseful horizontal release must be consumed")
        expect(
            harness.updates.map(\.phase) == [.began, .changed, .ended],
            "phaseful horizontal release must emit began, changed, and ended"
        )

        harness.uninstall()
    }

    private static func verifiesPhasefulHorizontalGestureCancelsOnCancelPhase() {
        let harness = MonitorHarness()

        _ = harness.coordinator.handleForTesting(
            harness.event(deltaX: 8, deltaY: 0, phase: .began)
        )
        let cancelledResult = harness.coordinator.handleForTesting(
            harness.event(deltaX: 0, deltaY: 0, phase: .cancelled)
        )

        expect(cancelledResult == nil, "phaseful horizontal cancel must be consumed")
        expect(
            harness.updates.map(\.phase) == [.began, .changed, .cancelled],
            "phaseful horizontal cancel must emit a cancel update"
        )

        harness.uninstall()
    }

    private static func verifiesFastFlickUsesTerminalDelta() {
        let harness = MonitorHarness()

        let flickEvent = harness.event(deltaX: 8, deltaY: 0, phase: .ended)
        let flickResult = harness.coordinator.handleForTesting(flickEvent)

        expect(flickResult == nil, "terminal horizontal flick input must be consumed")
        expect(
            harness.updates.map(\.phase) == [.began, .changed, .ended],
            "terminal horizontal flick must begin, change, and end from one terminal delta"
        )

        harness.uninstall()
    }

    private static func verifiesPagerOffsetsPagesFromSharedDragOffset() {
        let pager = ShellSidebarSpaceContentPagerState(
            sourceIndex: 1,
            targetIndex: 2,
            dragOffset: -72,
            pageWidth: 240,
            settlementPhase: .dragging
        )

        expect(
            pager.offset(for: 1).isApproximately(-72),
            "source sidebar content page must move directly with finger translation"
        )
        expect(
            pager.offset(for: 2).isApproximately(168),
            "target sidebar content page must share the same drag offset from the adjacent page position"
        )
        expect(
            pager.pageIndicesForRendering == [1, 2],
            "sidebar content pager must render source and adjacent target pages together"
        )
    }

    private static func verifiesPagerEdgeResistanceDoesNotRenderWrappedTarget() {
        let pager = ShellSidebarSpaceContentPagerState(
            sourceIndex: 0,
            targetIndex: nil,
            dragOffset: 42,
            pageWidth: 500,
            settlementPhase: .dragging
        )

        expect(pager.isEdgeResistance, "edge gestures without a target must enter resistance mode")
        expect(
            pager.pageIndicesForRendering == [0],
            "edge resistance must not synthesize or wrap to a target page"
        )
        expect(
            pager.offset(for: 0).isApproximately(42),
            "edge source page must still move with the bounded resisted offset"
        )
    }

    private static func verifiesPagerSettlementPhaseExposesCommitAndCancelState() {
        let committing = ShellSidebarSpaceContentPagerState(
            sourceIndex: 1,
            targetIndex: 2,
            dragOffset: -500,
            pageWidth: 500,
            settlementPhase: .settlingToTarget
        )
        let cancelling = ShellSidebarSpaceContentPagerState(
            sourceIndex: 1,
            targetIndex: 2,
            dragOffset: 0,
            pageWidth: 500,
            settlementPhase: .settlingToSource
        )

        expect(committing.isSettling, "target settlement must report an active settling phase")
        expect(
            committing.committedTargetIndex == 2,
            "target settlement must expose the committed target index"
        )
        expect(cancelling.isSettling, "source settlement must report an active settling phase")
        expect(
            cancelling.committedTargetIndex == nil,
            "cancelled settlement must not expose a committed target"
        )
    }

    private static func drainMainQueue() {
        RunLoop.current.run(until: Date().addingTimeInterval(0.24))
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

private extension CGFloat {
    func isApproximately(_ other: CGFloat, tolerance: CGFloat = 0.001) -> Bool {
        abs(self - other) <= tolerance
    }
}

@MainActor
private final class MonitorHarness {
    let window: NSWindow
    let view: ShellSidebarSwipeMonitor.MonitorView
    let coordinator: ShellSidebarSwipeMonitor.Coordinator
    private let recorder: SwipeUpdateRecorder
    var updates: [ShellSidebarSwipeUpdate] {
        recorder.updates
    }
    private var eventTime: TimeInterval = 1

    init() {
        let recorder = SwipeUpdateRecorder()
        self.recorder = recorder
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 240, height: 480),
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )
        view = ShellSidebarSwipeMonitor.MonitorView(
            frame: NSRect(x: 0, y: 0, width: 240, height: 480)
        )
        coordinator = ShellSidebarSwipeMonitor.Coordinator { update in
            recorder.updates.append(update)
        }
        view.coordinator = coordinator
        window.contentView = view
        coordinator.install(for: view)
    }

    func event(
        deltaX: CGFloat,
        deltaY: CGFloat,
        phase: NSEvent.Phase = [],
        momentumPhase: NSEvent.Phase = []
    ) -> NSEvent {
        eventTime += 0.016
        return RecordingSidebarScrollWheelEvent(
            window: window,
            locationInWindow: NSPoint(x: 120, y: 240),
            deltaX: deltaX,
            deltaY: deltaY,
            phase: phase,
            momentumPhase: momentumPhase,
            timestamp: eventTime
        )
    }

    func uninstall() {
        coordinator.uninstall()
    }
}

@MainActor
private final class SwipeUpdateRecorder {
    var updates: [ShellSidebarSwipeUpdate] = []
}

private final class RecordingSidebarScrollWheelEvent: NSEvent {
    private weak var recordedWindow: NSWindow?
    private let recordedLocationInWindow: NSPoint
    private let recordedDeltaX: CGFloat
    private let recordedDeltaY: CGFloat
    private let recordedPhase: NSEvent.Phase
    private let recordedMomentumPhase: NSEvent.Phase
    private let recordedTimestamp: TimeInterval

    init(
        window: NSWindow,
        locationInWindow: NSPoint,
        deltaX: CGFloat,
        deltaY: CGFloat,
        phase: NSEvent.Phase = [],
        momentumPhase: NSEvent.Phase = [],
        timestamp: TimeInterval
    ) {
        recordedWindow = window
        recordedLocationInWindow = locationInWindow
        recordedDeltaX = deltaX
        recordedDeltaY = deltaY
        recordedPhase = phase
        recordedMomentumPhase = momentumPhase
        recordedTimestamp = timestamp
        super.init()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    override var window: NSWindow? { recordedWindow }
    override var locationInWindow: NSPoint { recordedLocationInWindow }
    override var scrollingDeltaX: CGFloat { recordedDeltaX }
    override var scrollingDeltaY: CGFloat { recordedDeltaY }
    override var hasPreciseScrollingDeltas: Bool { true }
    override var phase: NSEvent.Phase { recordedPhase }
    override var momentumPhase: NSEvent.Phase { recordedMomentumPhase }
    override var timestamp: TimeInterval { recordedTimestamp }
}
#endif
