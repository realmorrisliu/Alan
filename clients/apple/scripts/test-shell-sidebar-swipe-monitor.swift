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

    func event(deltaX: CGFloat, deltaY: CGFloat) -> NSEvent {
        eventTime += 0.016
        return RecordingSidebarScrollWheelEvent(
            window: window,
            locationInWindow: NSPoint(x: 120, y: 240),
            deltaX: deltaX,
            deltaY: deltaY,
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
    private let recordedTimestamp: TimeInterval

    init(
        window: NSWindow,
        locationInWindow: NSPoint,
        deltaX: CGFloat,
        deltaY: CGFloat,
        timestamp: TimeInterval
    ) {
        recordedWindow = window
        recordedLocationInWindow = locationInWindow
        recordedDeltaX = deltaX
        recordedDeltaY = deltaY
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
    override var phase: NSEvent.Phase { [] }
    override var momentumPhase: NSEvent.Phase { [] }
    override var timestamp: TimeInterval { recordedTimestamp }
}
#endif
