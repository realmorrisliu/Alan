import SwiftUI

#if os(macOS)
struct MacShellRootView: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ObservedObject private var host: ShellHostController
    @State private var isCommandTabPresented = false
    @State private var isSpaceSwipeGestureLocked = false
    @State private var spaceTransition: ShellSpaceTransition?
    @State private var spaceTransitionToken = 0
    @State private var windowChromeMetrics = ShellWindowChromeMetrics()
    private let sidebarWidth: CGFloat = 264

    init(host: ShellHostController) {
        self.host = host
    }

    private func presentCommandInput() {
        withAnimation(.easeOut(duration: 0.18)) {
            isCommandTabPresented = true
        }
    }

    private func dismissCommandInput() {
        withAnimation(.easeOut(duration: 0.18)) {
            isCommandTabPresented = false
        }
        DispatchQueue.main.async {
            host.refocusSelectedTerminalPane()
        }
    }

    private func handleSpaceSwipe(_ update: ShellSidebarSwipeUpdate) {
        switch update.phase {
        case .began:
            guard spaceTransition?.isSettling != true else { return }
            isSpaceSwipeGestureLocked = true
            beginSpaceTransition()
        case .changed:
            guard spaceTransition?.isSettling != true else { return }
            isSpaceSwipeGestureLocked = true
            updateSpaceTransition(translationX: update.translationX)
        case .ended:
            finishSpaceTransition(velocityX: update.velocityX)
        case .cancelled:
            settleSpaceTransition(committing: false)
        }
    }

    private func beginSpaceTransition() {
        guard let sourceSpaceID = host.selectedSpace?.spaceID else { return }
        var transaction = Transaction()
        transaction.disablesAnimations = true
        withTransaction(transaction) {
            spaceTransition = ShellSpaceTransition(
                sourceSpaceID: sourceSpaceID,
                targetSpaceID: nil,
                direction: 1,
                offsetX: 0,
                progress: 0,
                isSettling: false
            )
        }
    }

    private func updateSpaceTransition(translationX: CGFloat) {
        guard abs(translationX) > 0.5 else { return }
        let sourceSpaceID = spaceTransition?.sourceSpaceID ?? host.selectedSpace?.spaceID
        guard let sourceSpaceID else { return }

        let direction = translationX < 0 ? 1 : -1
        let targetSpaceID = adjacentSpaceID(from: sourceSpaceID, direction: direction)
        let offsetX = targetSpaceID == nil ? resistedEdgeOffset(for: translationX) : translationX
        let progress = min(abs(offsetX) / sidebarSwipePageWidth, 0.98)

        var transaction = Transaction()
        transaction.disablesAnimations = true
        withTransaction(transaction) {
            spaceTransition = ShellSpaceTransition(
                sourceSpaceID: sourceSpaceID,
                targetSpaceID: targetSpaceID,
                direction: direction,
                offsetX: offsetX,
                progress: progress,
                isSettling: false
            )
        }
    }

    private func finishSpaceTransition(velocityX: CGFloat) {
        guard let transition = spaceTransition else {
            isSpaceSwipeGestureLocked = false
            return
        }
        guard transition.targetSpaceID != nil else {
            settleSpaceTransition(committing: false)
            return
        }

        let velocityDirection = velocityX < 0 ? 1 : -1
        let fastEnough = abs(velocityX) >= 120 && velocityDirection == transition.direction
        let farEnough = transition.progress >= 0.28
        settleSpaceTransition(committing: farEnough || fastEnough)
    }

    private func settleSpaceTransition(committing: Bool) {
        guard var transition = spaceTransition else {
            isSpaceSwipeGestureLocked = false
            return
        }
        transition.isSettling = true
        transition.offsetX = committing ? -CGFloat(transition.direction) * sidebarSwipePageWidth : 0
        transition.progress = committing ? 1 : 0
        spaceTransitionToken += 1
        let token = spaceTransitionToken
        let duration = reduceMotion ? 0.12 : 0.28

        withAnimation(settleAnimation) {
            spaceTransition = transition
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + duration) {
            guard spaceTransitionToken == token else { return }
            if committing, let targetSpaceID = transition.targetSpaceID {
                host.select(spaceID: targetSpaceID)
                DispatchQueue.main.async {
                    host.refocusSelectedTerminalPane()
                }
            }
            var transaction = Transaction()
            transaction.disablesAnimations = true
            withTransaction(transaction) {
                spaceTransition = nil
                isSpaceSwipeGestureLocked = false
            }
        }
    }

    private var settleAnimation: Animation {
        if reduceMotion {
            return .easeOut(duration: 0.12)
        }
        return .interactiveSpring(response: 0.28, dampingFraction: 0.86, blendDuration: 0.04)
    }

    private var sidebarSwipePageWidth: CGFloat {
        max(sidebarWidth, 1)
    }

    private func resistedEdgeOffset(for translationX: CGFloat) -> CGFloat {
        let edgeLimit = sidebarSwipePageWidth * 0.18
        let distance = abs(translationX)
        let resistedDistance = edgeLimit * distance / (distance + edgeLimit)
        return translationX < 0 ? -resistedDistance : resistedDistance
    }

    private func adjacentSpaceID(from sourceSpaceID: String, direction: Int) -> String? {
        guard let sourceIndex = host.spaces.firstIndex(where: { $0.spaceID == sourceSpaceID }) else {
            return nil
        }
        let targetIndex = sourceIndex + direction
        guard host.spaces.indices.contains(targetIndex) else { return nil }
        return host.spaces[targetIndex].spaceID
    }

    var body: some View {
        ZStack {
            ShellSpaceKeyboardShortcuts(host: host)

            ShellMaterialBackgroundView(.windowBackdrop)
                .ignoresSafeArea()

            HStack(spacing: 0) {
                ShellSidebarView(
                    host: host,
                    chromeMetrics: windowChromeMetrics,
                    spaceTransition: spaceTransition,
                    isSpaceSwipeGestureLocked: isSpaceSwipeGestureLocked,
                    onSpaceSwipe: handleSpaceSwipe
                ) {
                    presentCommandInput()
                }
                .frame(width: sidebarWidth)

                ShellWorkspaceView(host: host)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .ignoresSafeArea(edges: .top)
            }
            .frame(minWidth: 1260, minHeight: 800)

            if isCommandTabPresented {
                ShellPalette.overlayScrim
                    .ignoresSafeArea()
                    .onTapGesture {
                        dismissCommandInput()
                    }

                ShellCommandTabView(
                    host: host,
                    isPresented: $isCommandTabPresented
                )
                .frame(width: 520)
                .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.easeOut(duration: 0.18), value: isCommandTabPresented)
        .onChange(of: host.commandInputRequestID) { _, _ in
            presentCommandInput()
        }
        .background(ShellWindowPlacementView(metrics: $windowChromeMetrics))
    }
}
#endif
