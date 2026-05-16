import SwiftUI

#if os(macOS)
struct MacShellRootView: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ObservedObject private var host: ShellHostController
    @Binding private var appearanceMode: ShellAppearanceMode
    @Binding private var isSidebarCollapsed: Bool
    @State private var isCommandTabPresented = false
    @State private var isSidebarPanelRevealed = false
    @State private var areFloatingSidebarTrafficLightsVisible = false
    @State private var sidebarRevealToken = 0
    @State private var floatingSidebarTrafficLightRevealToken = 0
    @State private var pinnedSidebarPresentationProgress: CGFloat
    @State private var windowChromeMetrics = ShellWindowChromeMetrics()
    @State private var systemColorScheme = ShellAppearanceMode.currentSystemColorScheme
    @State private var isCollapsedSidebarPointerRetained = false
    private let sidebarWidth: CGFloat = 264
    private let floatingSidebarInset: CGFloat = 6
    private let floatingSidebarTrafficLightRevealDelay: TimeInterval = 0.08
    private let hiddenCommandInputOpacity = 0.001

    init(
        host: ShellHostController,
        appearanceMode: Binding<ShellAppearanceMode> = .constant(.system),
        isSidebarCollapsed: Binding<Bool> = .constant(false)
    ) {
        self.host = host
        _appearanceMode = appearanceMode
        _isSidebarCollapsed = isSidebarCollapsed
        _pinnedSidebarPresentationProgress = State(
            initialValue: isSidebarCollapsed.wrappedValue ? 0 : 1
        )
    }

    private func toggleCommandInput() {
        if isCommandTabPresented {
            dismissCommandInput()
        } else {
            presentCommandInput()
        }
    }

    private func presentCommandInput() {
        withAnimation(commandInputAnimation) {
            isCommandTabPresented = true
        }
    }

    private func dismissCommandInput() {
        withAnimation(commandInputAnimation) {
            isCommandTabPresented = false
        }
        DispatchQueue.main.async {
            host.refocusSelectedTerminalPane()
        }
    }

    private var isSidebarSurfaceVisible: Bool {
        isPinnedSidebarSurfaceActive || isSidebarPanelRevealed
    }

    private var isPinnedSidebarSurfaceActive: Bool {
        !isSidebarCollapsed || pinnedSidebarPresentationProgress > 0.001
    }

    private var isPinnedSidebarFullyCollapsed: Bool {
        pinnedSidebarPresentationProgress <= 0.001
    }

    private var sidebarPinnedVisibleWidth: CGFloat {
        sidebarWidth * clampedPinnedSidebarPresentationProgress
    }

    private var sidebarPinnedChromeOffsetX: CGFloat {
        -sidebarWidth * (1 - clampedPinnedSidebarPresentationProgress)
    }

    private var sidebarPinnedContentOpacity: Double {
        Double(clampedPinnedSidebarPresentationProgress)
    }

    private var clampedPinnedSidebarPresentationProgress: CGFloat {
        min(max(pinnedSidebarPresentationProgress, 0), 1)
    }

    private var sidebarChromeSurfaceOrigin: CGPoint {
        if isSidebarPanelRevealed {
            return CGPoint(x: floatingSidebarInset, y: floatingSidebarInset)
        }
        guard isPinnedSidebarSurfaceActive else {
            return .zero
        }
        return CGPoint(x: sidebarPinnedChromeOffsetX, y: 0)
    }

    private var commandInputOpacity: Double {
        isCommandTabPresented ? 1 : hiddenCommandInputOpacity
    }

    private var commandInputAnimation: Animation? {
        reduceMotion ? nil : .easeOut(duration: 0.14)
    }

    private var windowChromeSurface: ShellWindowChromeSurface {
        ShellWindowChromeSurface(
            isVisible: isSidebarSurfaceVisible,
            origin: sidebarChromeSurfaceOrigin,
            width: sidebarWidth,
            showsStandardTrafficLights: shouldShowStandardTrafficLights
        )
    }

    private var shouldShowStandardTrafficLights: Bool {
        if isPinnedSidebarSurfaceActive {
            return true
        }
        guard isSidebarCollapsed else { return true }
        return isSidebarPanelRevealed && areFloatingSidebarTrafficLightsVisible
    }

    private var isCollapsedSidebarPointerRetentionActive: Bool {
        isSidebarCollapsed && isPinnedSidebarFullyCollapsed && isSidebarPanelRevealed
    }

    private func revealCollapsedSidebarPanel() {
        guard isSidebarCollapsed, isPinnedSidebarFullyCollapsed else { return }
        sidebarRevealToken += 1
        guard !isSidebarPanelRevealed else { return }

        areFloatingSidebarTrafficLightsVisible = false
        floatingSidebarTrafficLightRevealToken += 1
        let token = floatingSidebarTrafficLightRevealToken
        withAnimation(sidebarPanelRevealAnimation) {
            isSidebarPanelRevealed = true
        }
        scheduleFloatingSidebarTrafficLightReveal(token: token)
    }

    private func scheduleCollapsedSidebarHide() {
        guard isSidebarCollapsed else { return }
        sidebarRevealToken += 1
        let token = sidebarRevealToken
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.16) {
            guard sidebarRevealToken == token, isSidebarCollapsed else { return }
            guard !isCollapsedSidebarPointerRetained else { return }
            areFloatingSidebarTrafficLightsVisible = false
            floatingSidebarTrafficLightRevealToken += 1
            withAnimation(sidebarPanelHideAnimation) {
                isSidebarPanelRevealed = false
            }
        }
    }

    private func scheduleFloatingSidebarTrafficLightReveal(token: Int) {
        let delay = reduceMotion ? 0 : floatingSidebarTrafficLightRevealDelay
        let reveal = {
            guard floatingSidebarTrafficLightRevealToken == token,
                  isSidebarCollapsed,
                  isSidebarPanelRevealed
            else {
                return
            }
            areFloatingSidebarTrafficLightsVisible = true
        }

        if delay <= 0 {
            DispatchQueue.main.async(execute: reveal)
        } else {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: reveal)
        }
    }

    private func handleCollapsedSidebarHover(_ hovering: Bool) {
        hovering ? revealCollapsedSidebarPanel() : scheduleCollapsedSidebarHide()
    }

    private func handleCollapsedSidebarToolbarHover(_ hovering: Bool) {
        guard isSidebarCollapsed, isPinnedSidebarFullyCollapsed else { return }
        handleCollapsedSidebarHover(hovering)
    }

    private func handleCollapsedSidebarPointerRetention(_ retained: Bool) {
        guard isCollapsedSidebarPointerRetentionActive else { return }
        if retained {
            sidebarRevealToken += 1
        } else {
            scheduleCollapsedSidebarHide()
        }
    }

    private var sidebarPanelRevealAnimation: Animation? {
        reduceMotion
            ? nil
            : .interactiveSpring(response: 0.28, dampingFraction: 0.86, blendDuration: 0.02)
    }

    private var sidebarPanelHideAnimation: Animation? {
        reduceMotion ? nil : .easeInOut(duration: 0.18)
    }

    private var sidebarPinnedStateAnimation: Animation? {
        reduceMotion ? nil : .easeOut(duration: 0.16)
    }

    private var resolvedAppearanceColorScheme: ColorScheme {
        appearanceMode.resolvedColorScheme(systemColorScheme: systemColorScheme)
    }

    private func updateSidebarCollapsed(_ collapsed: Bool) {
        withAnimation(sidebarPinnedStateAnimation) {
            isSidebarCollapsed = collapsed
            pinnedSidebarPresentationProgress = collapsed ? 0 : 1
            isSidebarPanelRevealed = false
            areFloatingSidebarTrafficLightsVisible = false
            floatingSidebarTrafficLightRevealToken += 1
        }
    }

    var body: some View {
        ZStack {
            ShellSpaceKeyboardShortcuts(host: host)

            ShellMaterialBackgroundView(.windowBackdrop)
                .ignoresSafeArea()

            HStack(spacing: 0) {
                pinnedSidebarSurface()

                ShellWorkspaceView(
                    host: host,
                    expandedSidebarProgress: clampedPinnedSidebarPresentationProgress
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .ignoresSafeArea(edges: .top)
            }
            .frame(
                minWidth: ShellWindowSizing.minimumSize.width,
                minHeight: ShellWindowSizing.minimumSize.height
            )

            if isSidebarCollapsed && isPinnedSidebarFullyCollapsed {
                collapsedSidebarRevealZone

                if isSidebarPanelRevealed {
                    floatingSidebarPanel
                        .transition(floatingSidebarPanelTransition)
                }
            }

            if isSidebarSurfaceVisible {
                sidebarChromeControls
                    .transition(.opacity)
            }

            if isCommandTabPresented {
                Color.clear
                    .ignoresSafeArea()
                    .contentShape(Rectangle())
                    .transition(.identity)
                    .transaction { transaction in
                        transaction.animation = nil
                        transaction.disablesAnimations = true
                    }
                    .onTapGesture {
                        dismissCommandInput()
                    }
            }

            ShellCommandTabView(
                host: host,
                isPresented: $isCommandTabPresented,
                isActive: isCommandTabPresented
            )
            .frame(width: 560)
            .opacity(commandInputOpacity)
            .allowsHitTesting(isCommandTabPresented)
            .accessibilityHidden(!isCommandTabPresented)
        }
        .animation(sidebarPinnedStateAnimation, value: pinnedSidebarPresentationProgress)
        .environment(\.colorScheme, resolvedAppearanceColorScheme)
        .onAppear {
            pinnedSidebarPresentationProgress = isSidebarCollapsed ? 0 : 1
        }
        .onChange(of: isSidebarCollapsed) { _, collapsed in
            synchronizePinnedSidebarPresentation(collapsed: collapsed)
        }
        .onChange(of: host.commandInputRequestID) { _, _ in
            toggleCommandInput()
        }
        .onChange(of: isCollapsedSidebarPointerRetained) { _, retained in
            handleCollapsedSidebarPointerRetention(retained)
        }
        .background(
            ShellWindowPlacementAnimationSyncView(
                metrics: $windowChromeMetrics,
                appearanceMode: appearanceMode,
                pinnedSidebarPresentationProgress: pinnedSidebarPresentationProgress,
                isSidebarCollapsed: isSidebarCollapsed,
                isSidebarPanelRevealed: isSidebarPanelRevealed,
                areFloatingSidebarTrafficLightsVisible: areFloatingSidebarTrafficLightsVisible,
                sidebarWidth: sidebarWidth,
                floatingSidebarInset: floatingSidebarInset,
                systemColorScheme: $systemColorScheme,
                collapsedSidebarPointerRetentionEnabled: isCollapsedSidebarPointerRetentionActive,
                collapsedSidebarPointerRetained: $isCollapsedSidebarPointerRetained
            )
        )
    }

    private func pinnedSidebarSurface() -> some View {
        sidebarContent(isSwipeEnabled: true)
            .frame(width: sidebarWidth)
            .offset(x: sidebarPinnedChromeOffsetX)
            .opacity(sidebarPinnedContentOpacity)
            .allowsHitTesting(!isSidebarCollapsed)
            .frame(width: sidebarPinnedVisibleWidth, alignment: .leading)
            .clipped()
            .ignoresSafeArea(edges: .top)
    }

    private func synchronizePinnedSidebarPresentation(collapsed: Bool) {
        withAnimation(sidebarPinnedStateAnimation) {
            pinnedSidebarPresentationProgress = collapsed ? 0 : 1
            if collapsed {
                isSidebarPanelRevealed = false
                areFloatingSidebarTrafficLightsVisible = false
                floatingSidebarTrafficLightRevealToken += 1
            } else {
                isSidebarPanelRevealed = false
            }
        }
    }

    private func sidebarContent(isSwipeEnabled: Bool = true) -> some View {
        ShellSidebarView(
            host: host,
            chromeMetrics: windowChromeMetrics,
            displaySpaceID: nil,
            isSwipeEnabled: isSwipeEnabled
        ) {
            presentCommandInput()
        }
    }

    private var collapsedSidebarRevealZone: some View {
        Color.clear
            .frame(width: ShellSidebarMetrics.collapsedRevealEdgeWidth)
            .frame(maxHeight: .infinity, alignment: .topLeading)
            .contentShape(Rectangle())
            .onHover(perform: handleCollapsedSidebarHover)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
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

            sidebarContent(isSwipeEnabled: true)
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
        .onHover(perform: handleCollapsedSidebarHover)
        .ignoresSafeArea(edges: [.top, .bottom])
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .zIndex(20)
    }

    private var floatingSidebarPanelTransition: AnyTransition {
        .asymmetric(
            insertion: .move(edge: .leading).combined(with: .opacity),
            removal: .move(edge: .leading)
        )
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
            )
            .padding(
                .top,
                windowChromeMetrics.titlebarToolTopInset
            )
            .offset(
                x: sidebarChromeSurfaceOrigin.x,
                y: sidebarChromeSurfaceOrigin.y
            )
            .contentShape(Rectangle())
            .onHover(perform: handleCollapsedSidebarToolbarHover)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .ignoresSafeArea(edges: .top)
        .zIndex(30)
    }
}

private struct ShellWindowPlacementAnimationSyncView: View, Animatable {
    @Binding var metrics: ShellWindowChromeMetrics
    let appearanceMode: ShellAppearanceMode
    var pinnedSidebarPresentationProgress: CGFloat
    let isSidebarCollapsed: Bool
    let isSidebarPanelRevealed: Bool
    let areFloatingSidebarTrafficLightsVisible: Bool
    let sidebarWidth: CGFloat
    let floatingSidebarInset: CGFloat
    @Binding var systemColorScheme: ColorScheme
    let collapsedSidebarPointerRetentionEnabled: Bool
    @Binding var collapsedSidebarPointerRetained: Bool

    var animatableData: CGFloat {
        get { pinnedSidebarPresentationProgress }
        set { pinnedSidebarPresentationProgress = newValue }
    }

    var body: some View {
        ShellWindowPlacementView(
            metrics: $metrics,
            appearanceMode: appearanceMode,
            chromeSurface: windowChromeSurface,
            systemColorScheme: $systemColorScheme,
            collapsedSidebarPointerRetentionEnabled: collapsedSidebarPointerRetentionEnabled,
            collapsedSidebarPointerRetained: $collapsedSidebarPointerRetained
        )
    }

    private var windowChromeSurface: ShellWindowChromeSurface {
        ShellWindowChromeSurface(
            isVisible: isSidebarSurfaceVisible,
            origin: sidebarChromeSurfaceOrigin,
            width: sidebarWidth,
            showsStandardTrafficLights: shouldShowStandardTrafficLights
        )
    }

    private var isSidebarSurfaceVisible: Bool {
        isPinnedSidebarSurfaceActive || isSidebarPanelRevealed
    }

    private var isPinnedSidebarSurfaceActive: Bool {
        !isSidebarCollapsed || clampedPinnedSidebarPresentationProgress > 0.001
    }

    private var clampedPinnedSidebarPresentationProgress: CGFloat {
        min(max(pinnedSidebarPresentationProgress, 0), 1)
    }

    private var sidebarChromeSurfaceOrigin: CGPoint {
        if isSidebarPanelRevealed {
            return CGPoint(x: floatingSidebarInset, y: floatingSidebarInset)
        }
        guard isPinnedSidebarSurfaceActive else {
            return .zero
        }
        return CGPoint(x: -sidebarWidth * (1 - clampedPinnedSidebarPresentationProgress), y: 0)
    }

    private var shouldShowStandardTrafficLights: Bool {
        if isPinnedSidebarSurfaceActive {
            return true
        }
        guard isSidebarCollapsed else { return true }
        return isSidebarPanelRevealed && areFloatingSidebarTrafficLightsVisible
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
