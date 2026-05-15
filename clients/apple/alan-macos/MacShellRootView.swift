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
    @State private var isSpaceSwipeGestureLocked = false
    @State private var pinnedSidebarPresentationProgress: CGFloat
    @State private var spacePager: ShellSidebarSpaceContentPagerState?
    @State private var spacePagerToken = 0
    @State private var spacePagerPageWidth: CGFloat = 1
    @State private var spacePagerPageSelectedPaneIDs: [Int: String] = [:]
    @State private var windowChromeMetrics = ShellWindowChromeMetrics()
    @State private var systemColorScheme = ShellAppearanceMode.currentSystemColorScheme
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

    private func handleSpaceSwipe(_ update: ShellSidebarSwipeUpdate) {
        switch update.phase {
        case .began:
            guard spacePager?.isSettling != true else { return }
            isSpaceSwipeGestureLocked = true
            beginSpacePager()
        case .changed:
            guard spacePager?.isSettling != true else { return }
            isSpaceSwipeGestureLocked = true
            updateSpacePager(translationX: update.translationX)
        case .ended:
            finishSpacePager(velocityX: update.velocityX)
        case .cancelled:
            settleSpacePager(committing: false)
        }
    }

    private func beginSpacePager() {
        guard let sourceIndex = selectedSpaceIndex else { return }
        var transaction = Transaction()
        transaction.disablesAnimations = true
        withTransaction(transaction) {
            spacePagerPageSelectedPaneIDs = [
                sourceIndex: host.selectedPane?.paneID,
            ].compactMapValues { $0 }
            spacePager = ShellSidebarSpaceContentPagerState(
                sourceIndex: sourceIndex,
                targetIndex: nil,
                dragOffset: 0,
                pageWidth: sidebarSwipePageWidth,
                settlementPhase: .dragging
            )
        }
    }

    private func updateSpacePager(translationX: CGFloat) {
        guard abs(translationX) > 0.5 else { return }
        guard let sourceIndex = spacePager?.sourceIndex ?? selectedSpaceIndex else { return }

        let direction = translationX < 0 ? 1 : -1
        let targetIndex = adjacentSpaceIndex(from: sourceIndex, direction: direction)
        let dragOffset = targetIndex == nil ? resistedEdgeOffset(for: translationX) : translationX

        var transaction = Transaction()
        transaction.disablesAnimations = true
        withTransaction(transaction) {
            if let targetIndex {
                spacePagerPageSelectedPaneIDs[targetIndex] =
                    spacePagerPageSelectedPaneIDs[targetIndex]
                    ?? firstPaneID(forSpaceAt: targetIndex)
            }
            spacePager = ShellSidebarSpaceContentPagerState(
                sourceIndex: sourceIndex,
                targetIndex: targetIndex,
                dragOffset: dragOffset,
                pageWidth: sidebarSwipePageWidth,
                settlementPhase: .dragging
            )
        }
    }

    private func finishSpacePager(velocityX: CGFloat) {
        guard let pager = spacePager else {
            isSpaceSwipeGestureLocked = false
            return
        }
        guard pager.targetIndex != nil else {
            settleSpacePager(committing: false)
            return
        }

        let velocityDirection = velocityX < 0 ? 1 : -1
        let fastEnough = abs(velocityX) >= 120 && velocityDirection == pager.direction
        let farEnough = pager.progress >= 0.28
        settleSpacePager(committing: farEnough || fastEnough)
    }

    private func settleSpacePager(committing: Bool) {
        guard var pager = spacePager else {
            isSpaceSwipeGestureLocked = false
            return
        }
        let targetIndex = pager.targetIndex
        if committing,
           let targetIndex,
           host.spaces.indices.contains(targetIndex)
        {
            host.select(spaceID: host.spaces[targetIndex].spaceID)
        }

        pager.settlementPhase = committing ? .settlingToTarget : .settlingToSource
        pager.pageWidth = sidebarSwipePageWidth
        pager.dragOffset = committing ? -CGFloat(pager.direction) * sidebarSwipePageWidth : 0
        spacePagerToken += 1
        let token = spacePagerToken
        let duration = reduceMotion ? 0.12 : 0.28

        withAnimation(settleAnimation) {
            spacePager = pager
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + duration) {
            guard spacePagerToken == token else { return }
            var transaction = Transaction()
            transaction.disablesAnimations = true
            withTransaction(transaction) {
                spacePager = nil
                spacePagerPageSelectedPaneIDs = [:]
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
        max(spacePagerPageWidth, 1)
    }

    private func resistedEdgeOffset(for translationX: CGFloat) -> CGFloat {
        let edgeLimit = sidebarSwipePageWidth * 0.18
        let distance = abs(translationX)
        let resistedDistance = edgeLimit * distance / (distance + edgeLimit)
        return translationX < 0 ? -resistedDistance : resistedDistance
    }

    private func adjacentSpaceIndex(from sourceIndex: Int, direction: Int) -> Int? {
        let targetIndex = sourceIndex + direction
        guard host.spaces.indices.contains(targetIndex) else { return nil }
        return targetIndex
    }

    private var selectedSpaceIndex: Int? {
        guard let selectedSpaceID = host.selectedSpace?.spaceID else { return nil }
        return host.spaces.firstIndex { $0.spaceID == selectedSpaceID }
    }

    private var previewedSpaceID: String? {
        guard let targetIndex = spacePager?.targetIndex else { return nil }
        return spaceID(forSpaceAt: targetIndex)
    }

    private var swipeEnabledSpaceIndex: Int? {
        spacePager?.sourceIndex ?? selectedSpaceIndex
    }

    private var floatingSidebarDisplaySpaceID: String? {
        if let sourceIndex = spacePager?.sourceIndex {
            return spaceID(forSpaceAt: sourceIndex)
        }
        return host.selectedSpace?.spaceID
    }

    private func spaceID(forSpaceAt index: Int) -> String? {
        guard host.spaces.indices.contains(index) else { return nil }
        return host.spaces[index].spaceID
    }

    private func firstPaneID(forSpaceAt index: Int) -> String? {
        guard host.spaces.indices.contains(index) else { return nil }
        let space = host.spaces[index]
        return space.tabs
            .flatMap(\.paneTree.paneIDs)
            .first { paneID in
                host.shellState.panes.contains { $0.paneID == paneID }
            }
    }

    private func selectedPaneID(forSpaceAt index: Int) -> String? {
        if let paneID = spacePagerPageSelectedPaneIDs[index] {
            return paneID
        }
        if index == selectedSpaceIndex,
           let paneID = host.selectedPane?.paneID
        {
            return paneID
        }
        return firstPaneID(forSpaceAt: index)
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

            spacePagerPages
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
        .background(
            ShellWindowPlacementView(
                metrics: $windowChromeMetrics,
                appearanceMode: appearanceMode,
                chromeSurface: windowChromeSurface,
                systemColorScheme: $systemColorScheme
            )
        )
    }

    private var spacePagerPages: some View {
        GeometryReader { proxy in
            let pageWidth = max(proxy.size.width, 1)
            ZStack(alignment: .leading) {
                ForEach(spacePageIndices, id: \.self) { index in
                    spacePage(index: index, pageWidth: pageWidth)
                        .frame(width: pageWidth, height: proxy.size.height, alignment: .topLeading)
                        .offset(x: spacePageOffset(for: index, pageWidth: pageWidth))
                        .allowsHitTesting(spacePager == nil && index == selectedSpaceIndex)
                }
            }
            .clipped()
            .onAppear {
                updateSpacePagerPageWidth(pageWidth)
            }
            .onChange(of: proxy.size.width) { _, width in
                updateSpacePagerPageWidth(max(width, 1))
            }
        }
    }

    private var spacePageIndices: [Int] {
        guard let spacePager else {
            return selectedSpaceIndex.map { [$0] } ?? []
        }
        return spacePager.pageIndicesForRendering.filter { host.spaces.indices.contains($0) }
    }

    private func spacePage(index: Int, pageWidth: CGFloat) -> some View {
        HStack(spacing: 0) {
            pinnedSidebarSurface(
                displaySpaceID: spaceID(forSpaceAt: index),
                previewedSpaceID: previewedSpaceID,
                isSwipeEnabled: index == swipeEnabledSpaceIndex
            )

            ShellWorkspaceView(
                host: host,
                expandedSidebarProgress: clampedPinnedSidebarPresentationProgress,
                spaceID: spaceID(forSpaceAt: index),
                selectedPaneID: selectedPaneID(forSpaceAt: index)
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .ignoresSafeArea(edges: .top)
        }
        .frame(width: pageWidth, alignment: .leading)
    }

    private func spacePageOffset(for index: Int, pageWidth: CGFloat) -> CGFloat {
        guard var spacePager else { return 0 }
        spacePager.pageWidth = pageWidth
        return spacePager.offset(for: index)
    }

    private func updateSpacePagerPageWidth(_ pageWidth: CGFloat) {
        let clampedPageWidth = max(pageWidth, 1)
        spacePagerPageWidth = clampedPageWidth
        guard var spacePager,
              spacePager.pageWidth != clampedPageWidth
        else {
            return
        }
        spacePager.pageWidth = clampedPageWidth
        self.spacePager = spacePager
    }

    private func pinnedSidebarSurface(
        displaySpaceID: String?,
        previewedSpaceID: String?,
        isSwipeEnabled: Bool
    ) -> some View {
        sidebarContent(
            displaySpaceID: displaySpaceID,
            previewedSpaceID: previewedSpaceID,
            isSwipeEnabled: isSwipeEnabled
        )
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

    private func sidebarContent(
        displaySpaceID: String? = nil,
        previewedSpaceID: String? = nil,
        isSwipeEnabled: Bool = true
    ) -> some View {
        ShellSidebarView(
            host: host,
            chromeMetrics: windowChromeMetrics,
            displaySpaceID: displaySpaceID,
            previewedSpaceID: previewedSpaceID,
            isSpaceSwipeGestureLocked: isSpaceSwipeGestureLocked,
            isSwipeEnabled: isSwipeEnabled,
            onSpaceSwipe: handleSpaceSwipe
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

            sidebarContent(
                displaySpaceID: floatingSidebarDisplaySpaceID,
                previewedSpaceID: previewedSpaceID,
                isSwipeEnabled: true
            )
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
