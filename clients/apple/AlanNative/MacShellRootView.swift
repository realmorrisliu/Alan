import SwiftUI

#if os(macOS)
struct MacShellRootView: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ObservedObject private var host: ShellHostController
    @Binding private var appearanceMode: ShellAppearanceMode
    @Binding private var isSidebarCollapsed: Bool
    @State private var isCommandTabPresented = false
    @State private var isSidebarPanelRevealed = false
    @State private var sidebarRevealToken = 0
    @State private var isSpaceSwipeGestureLocked = false
    @State private var spaceTransition: ShellSpaceTransition?
    @State private var spaceTransitionToken = 0
    @State private var windowChromeMetrics = ShellWindowChromeMetrics()
    private let sidebarWidth: CGFloat = 264
    private let floatingSidebarInset: CGFloat = 6

    init(
        host: ShellHostController,
        appearanceMode: Binding<ShellAppearanceMode> = .constant(.system),
        isSidebarCollapsed: Binding<Bool> = .constant(false)
    ) {
        self.host = host
        _appearanceMode = appearanceMode
        _isSidebarCollapsed = isSidebarCollapsed
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

    private var isSidebarSurfaceVisible: Bool {
        !isSidebarCollapsed || isSidebarPanelRevealed
    }

    private func revealCollapsedSidebarPanel() {
        guard isSidebarCollapsed else { return }
        sidebarRevealToken += 1
        withAnimation(sidebarPanelAnimation) {
            isSidebarPanelRevealed = true
        }
    }

    private func scheduleCollapsedSidebarHide() {
        guard isSidebarCollapsed else { return }
        sidebarRevealToken += 1
        let token = sidebarRevealToken
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.16) {
            guard sidebarRevealToken == token, isSidebarCollapsed else { return }
            withAnimation(sidebarPanelAnimation) {
                isSidebarPanelRevealed = false
            }
        }
    }

    private var sidebarPanelAnimation: Animation? {
        reduceMotion ? nil : .easeOut(duration: 0.16)
    }

    private func updateSidebarCollapsed(_ collapsed: Bool) {
        withAnimation(sidebarPanelAnimation) {
            isSidebarCollapsed = collapsed
            isSidebarPanelRevealed = false
        }
    }

    var body: some View {
        ZStack {
            ShellSpaceKeyboardShortcuts(host: host)

            ShellMaterialBackgroundView(.windowBackdrop)
                .ignoresSafeArea()

            HStack(spacing: 0) {
                if !isSidebarCollapsed {
                    sidebarContent
                    .frame(width: sidebarWidth)
                    .ignoresSafeArea(edges: .top)
                    .transition(.move(edge: .leading).combined(with: .opacity))
                }

                ShellWorkspaceView(host: host)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .ignoresSafeArea(edges: .top)
            }
            .frame(minWidth: 1260, minHeight: 800)

            if isSidebarCollapsed {
                collapsedSidebarRevealZone

                if isSidebarPanelRevealed {
                    floatingSidebarPanel
                        .transition(.move(edge: .leading).combined(with: .opacity))
                }
            }

            if isSidebarSurfaceVisible {
                sidebarChromeControls
                    .transition(.opacity)
            }

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
                .frame(width: 560)
                .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.easeOut(duration: 0.18), value: isCommandTabPresented)
        .animation(sidebarPanelAnimation, value: isSidebarCollapsed)
        .animation(sidebarPanelAnimation, value: isSidebarPanelRevealed)
        .preferredColorScheme(appearanceMode.colorScheme)
        .onChange(of: isSidebarCollapsed) { _, collapsed in
            if !collapsed {
                isSidebarPanelRevealed = false
            }
        }
        .onChange(of: host.commandInputRequestID) { _, _ in
            presentCommandInput()
        }
        .background(
            ShellWindowPlacementView(
                metrics: $windowChromeMetrics,
                appearanceMode: appearanceMode
            )
        )
    }

    private var sidebarContent: some View {
        ShellSidebarView(
            host: host,
            chromeMetrics: windowChromeMetrics,
            spaceTransition: spaceTransition,
            isSpaceSwipeGestureLocked: isSpaceSwipeGestureLocked,
            onSpaceSwipe: handleSpaceSwipe
        ) {
            presentCommandInput()
        }
    }

    private var collapsedSidebarRevealZone: some View {
        VStack(alignment: .leading, spacing: 0) {
            Color.clear
                .frame(width: sidebarWidth, height: windowChromeMetrics.collapsedRevealHeaderHeight)
            Color.clear
                .frame(width: 18)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .contentShape(Rectangle())
        .onHover { hovering in
            hovering ? revealCollapsedSidebarPanel() : scheduleCollapsedSidebarHide()
        }
        .ignoresSafeArea()
        .zIndex(10)
    }

    private var floatingSidebarPanel: some View {
        ZStack(alignment: .topLeading) {
            RoundedRectangle(cornerRadius: ShellRadii.floatingSidebarPanel, style: .continuous)
                .fill(.clear)
                .background {
                    ShellMaterialBackgroundView(.sidebarGlass)
                        .clipShape(
                            RoundedRectangle(
                                cornerRadius: ShellRadii.floatingSidebarPanel,
                                style: .continuous
                            )
                        )
                }
                .overlay {
                    RoundedRectangle(cornerRadius: ShellRadii.floatingSidebarPanel, style: .continuous)
                        .stroke(ShellPalette.line.opacity(0.22), lineWidth: 0.8)
                }

            sidebarContent
                .clipShape(
                    RoundedRectangle(
                        cornerRadius: ShellRadii.floatingSidebarPanel,
                        style: .continuous
                    )
                )
        }
        .frame(width: sidebarWidth, alignment: .topLeading)
        .frame(maxHeight: .infinity, alignment: .topLeading)
        .padding(.leading, floatingSidebarInset)
        .padding(.top, floatingSidebarInset)
        .padding(.bottom, floatingSidebarInset)
        .shellShadow(ShellShadows.floatingPanel)
        .onHover { hovering in
            hovering ? revealCollapsedSidebarPanel() : scheduleCollapsedSidebarHide()
        }
        .ignoresSafeArea(edges: [.top, .bottom])
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .zIndex(20)
    }

    private var sidebarChromeControls: some View {
        GeometryReader { _ in
            HStack(spacing: ShellSidebarMetrics.titlebarToolSpacing) {
                ShellSidebarCollapseControl(isCollapsed: isSidebarCollapsed) {
                    updateSidebarCollapsed(!isSidebarCollapsed)
                }

                ShellAppearanceModeControl(mode: $appearanceMode)
            }
            .padding(
                .leading,
                windowChromeMetrics.titlebarToolLeadingInset
                    + (isSidebarCollapsed ? floatingSidebarInset : 0)
            )
            .padding(
                .top,
                windowChromeMetrics.titlebarToolTopInset
                    + (isSidebarCollapsed ? floatingSidebarInset : 0)
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .onHover { hovering in
                if isSidebarCollapsed {
                    hovering ? revealCollapsedSidebarPanel() : scheduleCollapsedSidebarHide()
                }
            }
        }
        .ignoresSafeArea(edges: .top)
        .zIndex(30)
    }
}

private struct ShellSidebarCollapseControl: View {
    let isCollapsed: Bool
    let action: () -> Void

    var body: some View {
        ShellGhostChromeButton(
            systemName: "sidebar.left",
            help: isCollapsed ? "Pin Sidebar" : "Hide Sidebar",
            accessibilityLabel: isCollapsed ? "Pin Sidebar" : "Hide Sidebar",
            action: action
        )
    }
}

private struct ShellAppearanceModeControl: View {
    @Binding var mode: ShellAppearanceMode

    var body: some View {
        ShellGhostChromeButton(
            systemName: mode.symbolName,
            help: "Appearance: \(mode.label). Click for \(mode.next.label).",
            accessibilityLabel: "Appearance",
            accessibilityValue: mode.label
        ) {
            mode = mode.next
        }
    }
}

private struct ShellGhostChromeButton: View {
    let systemName: String
    let help: String
    let accessibilityLabel: String
    var accessibilityValue: String?
    let action: () -> Void
    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            Image(systemName: systemName)
                .font(.system(size: 14, weight: .semibold))
                .symbolRenderingMode(.hierarchical)
                .foregroundStyle(iconForeground)
                .frame(
                    width: ShellSidebarMetrics.titlebarToolWidth,
                    height: ShellSidebarMetrics.titlebarToolHeight
                )
                .background {
                    if isHovered {
                        RoundedRectangle(cornerRadius: ShellRadii.titlebarTool, style: .continuous)
                            .fill(ShellPalette.titlebarToolGlassTint.opacity(0.20))
                            .overlay {
                                RoundedRectangle(cornerRadius: ShellRadii.titlebarTool, style: .continuous)
                                    .stroke(ShellPalette.line.opacity(0.18), lineWidth: 0.6)
                            }
                    }
                }
                .contentShape(
                    RoundedRectangle(cornerRadius: ShellRadii.titlebarTool, style: .continuous)
                )
        }
        .buttonStyle(.plain)
        .controlSize(.regular)
        .onHover { hovering in
            isHovered = hovering
        }
        .help(help)
        .accessibilityLabel(accessibilityLabel)
        .accessibilityValue(accessibilityValue ?? "")
    }

    private var iconForeground: Color {
        isHovered
            ? ShellPalette.sidebarInk.opacity(0.84)
            : ShellPalette.sidebarMutedInk.opacity(0.76)
    }
}
#endif
