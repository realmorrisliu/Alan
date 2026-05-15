import SwiftUI

#if os(macOS)
import AppKit

enum ShellSidebarSwipePhase {
    case began
    case changed
    case ended
    case cancelled
}

struct ShellSidebarSwipeUpdate {
    let phase: ShellSidebarSwipePhase
    let translationX: CGFloat
    let velocityX: CGFloat
}

enum ShellSpacePagerSettlementPhase: Equatable {
    case dragging
    case settlingToSource
    case settlingToTarget
}

struct ShellSpacePagerState: Equatable {
    let sourceIndex: Int
    var targetIndex: Int?
    var dragOffset: CGFloat
    var pageWidth: CGFloat
    var settlementPhase: ShellSpacePagerSettlementPhase

    var isSettling: Bool {
        settlementPhase != .dragging
    }

    var isEdgeResistance: Bool {
        targetIndex == nil
    }

    var committedTargetIndex: Int? {
        guard settlementPhase == .settlingToTarget else { return nil }
        return targetIndex
    }

    var direction: Int {
        guard let targetIndex else {
            return dragOffset < 0 ? 1 : -1
        }
        return targetIndex >= sourceIndex ? 1 : -1
    }

    var progress: CGFloat {
        let width = max(pageWidth, 1)
        return min(abs(dragOffset) / width, 0.98)
    }

    var pageIndicesForRendering: [Int] {
        guard let targetIndex, targetIndex != sourceIndex else {
            return [sourceIndex]
        }
        return [sourceIndex, targetIndex]
    }

    func offset(for index: Int) -> CGFloat {
        CGFloat(index - sourceIndex) * max(pageWidth, 1) + dragOffset
    }
}

struct ShellSidebarSwipeMonitor: NSViewRepresentable {
    let onUpdate: (ShellSidebarSwipeUpdate) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(onUpdate: onUpdate)
    }

    func makeNSView(context: Context) -> MonitorView {
        let view = MonitorView()
        view.coordinator = context.coordinator
        context.coordinator.install(for: view)
        return view
    }

    func updateNSView(_ nsView: MonitorView, context: Context) {
        context.coordinator.onUpdate = onUpdate
        nsView.coordinator = context.coordinator
        context.coordinator.install(for: nsView)
    }

    final class MonitorView: NSView {
        var coordinator: Coordinator?

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            coordinator?.install(for: self)
        }

        deinit {
            coordinator?.uninstall()
        }
    }

    final class Coordinator {
        var onUpdate: (ShellSidebarSwipeUpdate) -> Void
        private weak var view: NSView?
        private var monitor: Any?
        private var accumulatedX: CGFloat = 0
        private var accumulatedY: CGFloat = 0
        private var lastEmittedX: CGFloat = 0
        private var lastVelocityX: CGFloat = 0
        private var lastEventTime: TimeInterval = 0
        private var isPhasefulGesture = false
        private var intent = GestureIntent.undecided
        private var ignoresHorizontalMomentum = false
        private var phaseLessEndWorkItem: DispatchWorkItem?
        private let intentLockDistance: CGFloat = 5
        private let horizontalIntentBias: CGFloat = 1.14
        private let verticalIntentBias: CGFloat = 1.18

        init(onUpdate: @escaping (ShellSidebarSwipeUpdate) -> Void) {
            self.onUpdate = onUpdate
        }

        func install(for view: NSView) {
            self.view = view
            guard monitor == nil else { return }

            monitor = NSEvent.addLocalMonitorForEvents(matching: .scrollWheel) { [weak self] event in
                self?.handle(event) ?? event
            }
        }

        func uninstall() {
            if let monitor {
                NSEvent.removeMonitor(monitor)
            }
            monitor = nil
        }

#if ALAN_TESTING
        func handleForTesting(_ event: NSEvent) -> NSEvent? {
            handle(event)
        }
#endif

        private func handle(_ event: NSEvent) -> NSEvent? {
            if ignoresHorizontalMomentum, !event.momentumPhase.isEmpty {
                if event.momentumPhase.contains(.ended) || event.momentumPhase.contains(.cancelled) {
                    ignoresHorizontalMomentum = false
                    resetAccumulatedScroll()
                }
                return nil
            }

            guard let view, view.window === event.window else {
                return event
            }

            let point = view.convert(event.locationInWindow, from: nil)
            guard view.bounds.contains(point) || intent == .horizontal else {
                return event
            }

            if event.phase.contains(.began) {
                ignoresHorizontalMomentum = false
                resetAccumulatedScroll()
            }

            if !event.phase.isEmpty || !event.momentumPhase.isEmpty {
                isPhasefulGesture = true
            }

            let horizontal = pageDeltaX(from: event)
            let vertical = pageDeltaY(from: event)
            let hasDelta = abs(horizontal) > 0 || abs(vertical) > 0
            let canUseTerminalDeltaForFlick = intent == .undecided
                && hasDelta
                && (event.phase.contains(.ended) || event.momentumPhase.contains(.began))

            if let shouldConsume = finishIfNeeded(event: event, velocityX: lastVelocityX),
               !canUseTerminalDeltaForFlick
            {
                return shouldConsume ? nil : event
            }

            guard abs(horizontal) > 0 || abs(vertical) > 0 else {
                return event
            }

            accumulatedX += horizontal
            accumulatedY += vertical
            let now = event.timestamp > 0 ? event.timestamp : ProcessInfo.processInfo.systemUptime
            let elapsed = lastEventTime > 0 ? max(now - lastEventTime, 0.001) : 0.016
            let eventVelocityX = horizontal / elapsed

            switch intent {
            case .undecided:
                if abs(accumulatedY) >= intentLockDistance,
                   abs(accumulatedY) > abs(accumulatedX) * verticalIntentBias {
                    intent = .vertical
                    lastEventTime = now
                    schedulePhaseLessEndIfNeeded(for: event, velocityX: eventVelocityX)
                    return event
                }

                guard abs(accumulatedX) >= intentLockDistance,
                      abs(accumulatedX) > abs(accumulatedY) * horizontalIntentBias else {
                    lastEventTime = now
                    schedulePhaseLessEndIfNeeded(for: event, velocityX: eventVelocityX)
                    return nil
                }

                intent = .horizontal
                lastEmittedX = accumulatedX
                lastEventTime = now
                lastVelocityX = eventVelocityX
                onUpdate(ShellSidebarSwipeUpdate(phase: .began, translationX: 0, velocityX: 0))
                onUpdate(
                    ShellSidebarSwipeUpdate(
                        phase: .changed,
                        translationX: accumulatedX,
                        velocityX: eventVelocityX
                    )
                )
                if event.phase.contains(.ended) || event.momentumPhase.contains(.began) {
                    onUpdate(
                        ShellSidebarSwipeUpdate(
                            phase: event.phase.contains(.cancelled) ? .cancelled : .ended,
                            translationX: accumulatedX,
                            velocityX: eventVelocityX
                        )
                    )
                    resetAccumulatedScroll()
                    ignoresHorizontalMomentum = true
                    return nil
                }
                schedulePhaseLessEndIfNeeded(for: event, velocityX: eventVelocityX)
                return nil

            case .vertical:
                if event.phase.contains(.ended) || event.phase.contains(.cancelled) {
                    resetAccumulatedScroll()
                } else {
                    lastEventTime = now
                    schedulePhaseLessEndIfNeeded(for: event, velocityX: eventVelocityX)
                }
                return event

            case .horizontal:
                let velocityX = (accumulatedX - lastEmittedX) / elapsed
                lastEmittedX = accumulatedX
                lastEventTime = now
                lastVelocityX = velocityX

                onUpdate(
                    ShellSidebarSwipeUpdate(
                        phase: .changed,
                        translationX: accumulatedX,
                        velocityX: velocityX
                    )
                )
                schedulePhaseLessEndIfNeeded(for: event, velocityX: velocityX)
                return nil
            }
        }

        private func finishIfNeeded(event: NSEvent, velocityX: CGFloat) -> Bool? {
            let isCancelled = event.phase.contains(.cancelled) || event.momentumPhase.contains(.cancelled)
            let isEnded = event.phase.contains(.ended) || event.momentumPhase.contains(.began)
            guard isCancelled || isEnded else { return nil }

            switch intent {
            case .horizontal:
                onUpdate(
                    ShellSidebarSwipeUpdate(
                        phase: isCancelled ? .cancelled : .ended,
                        translationX: accumulatedX,
                        velocityX: velocityX
                    )
                )
                resetAccumulatedScroll()
                ignoresHorizontalMomentum = true
                return true

            case .vertical, .undecided:
                resetAccumulatedScroll()
                return false
            }
        }

        private func resetAccumulatedScroll() {
            phaseLessEndWorkItem?.cancel()
            phaseLessEndWorkItem = nil
            accumulatedX = 0
            accumulatedY = 0
            lastEmittedX = 0
            lastVelocityX = 0
            lastEventTime = 0
            isPhasefulGesture = false
            intent = .undecided
        }

        private func schedulePhaseLessEndIfNeeded(for event: NSEvent, velocityX: CGFloat) {
            guard !isPhasefulGesture,
                  event.phase.isEmpty,
                  event.momentumPhase.isEmpty
            else {
                return
            }

            phaseLessEndWorkItem?.cancel()
            let workItem = DispatchWorkItem { [weak self] in
                guard let self else { return }
                switch self.intent {
                case .horizontal:
                    self.onUpdate(
                        ShellSidebarSwipeUpdate(
                            phase: .ended,
                            translationX: self.accumulatedX,
                            velocityX: velocityX
                        )
                    )
                    self.resetAccumulatedScroll()
                    self.ignoresHorizontalMomentum = true
                case .vertical, .undecided:
                    self.resetAccumulatedScroll()
                }
            }
            phaseLessEndWorkItem = workItem
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.16, execute: workItem)
        }

        private func pageDeltaX(from event: NSEvent) -> CGFloat {
            if event.hasPreciseScrollingDeltas {
                return event.scrollingDeltaX
            }
            return event.deltaX * 10
        }

        private func pageDeltaY(from event: NSEvent) -> CGFloat {
            if event.hasPreciseScrollingDeltas {
                return event.scrollingDeltaY
            }
            return event.deltaY * 10
        }

        private enum GestureIntent {
            case undecided
            case horizontal
            case vertical
        }
    }
}
#endif
