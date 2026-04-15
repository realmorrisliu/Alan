import SwiftUI

#if os(macOS)
import AppKit

enum ShellPalette {
    static let canvas = Color(red: 0.94, green: 0.94, blue: 0.965)
    static let window = Color(red: 0.972, green: 0.973, blue: 0.985)
    static let sidebar = Color(red: 0.922, green: 0.924, blue: 0.953)
    static let sidebarRail = Color(red: 0.902, green: 0.907, blue: 0.941)
    static let sidebarCard = Color(red: 0.98, green: 0.98, blue: 0.995)
    static let workspace = Color(red: 0.979, green: 0.98, blue: 0.989)
    static let terminal = Color(red: 0.10, green: 0.12, blue: 0.16)
    static let terminalSoft = Color(red: 0.16, green: 0.18, blue: 0.24)
    static let accent = Color(red: 0.31, green: 0.39, blue: 0.71)
    static let accentSoft = Color(red: 0.90, green: 0.92, blue: 0.98)
    static let ink = Color(red: 0.16, green: 0.18, blue: 0.24)
    static let mutedInk = Color(red: 0.43, green: 0.45, blue: 0.54)
    static let line = Color(red: 0.82, green: 0.83, blue: 0.89)
    static let panel = Color.white.opacity(0.74)
    static let panelSoft = Color.white.opacity(0.6)
    static let attention = Color(red: 0.82, green: 0.55, blue: 0.24)
}

private struct SidebarMaterialView: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .sidebar
        view.blendingMode = .behindWindow
        view.state = .followsWindowActiveState
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {}
}

struct MacShellRootView: View {
    @StateObject private var host: ShellHostController
    @State private var isCommandSurfacePresented = false
    @AppStorage("alanShellShowsInspector")
    private var showsInspector = false

    init() {
        _host = StateObject(
            wrappedValue: ShellHostController.live(startupMode: .fresh)
        )
    }

    var body: some View {
        ZStack {
            ShellSpaceKeyboardShortcuts(host: host)

            ShellPalette.canvas
                .ignoresSafeArea()

            HStack(spacing: 0) {
                ShellSidebarView(host: host) {
                    withAnimation(.easeOut(duration: 0.18)) {
                        isCommandSurfacePresented = true
                    }
                }
                    .frame(width: 286)

                VStack(spacing: 0) {
                    ShellTopBarView(
                        host: host,
                        isCommandSurfacePresented: $isCommandSurfacePresented,
                        showsInspector: $showsInspector
                    )
                    ShellWorkspaceView(host: host)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background {
                    ShellPalette.workspace
                        .ignoresSafeArea(edges: .top)
                }

                if showsInspector {
                    ShellInspectorView(host: host)
                        .frame(width: 336)
                        .transition(.move(edge: .trailing).combined(with: .opacity))
                }
            }
            .frame(minWidth: 1260, minHeight: 800)
            .background(ShellPalette.window)

            if isCommandSurfacePresented {
                Color.black.opacity(0.16)
                    .ignoresSafeArea()
                    .onTapGesture {
                        withAnimation(.easeOut(duration: 0.18)) {
                            isCommandSurfacePresented = false
                        }
                    }

                ShellCommandSurfaceView(
                    host: host,
                    isPresented: $isCommandSurfacePresented
                )
                .frame(width: 520)
                .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(.easeOut(duration: 0.18), value: isCommandSurfacePresented)
        .animation(.easeOut(duration: 0.18), value: showsInspector)
        .background(ShellWindowPlacementView())
    }
}

private struct ShellWindowPlacementView: NSViewRepresentable {
    func makeNSView(context: Context) -> ShellWindowPlacementNSView {
        ShellWindowPlacementNSView()
    }

    func updateNSView(_ nsView: ShellWindowPlacementNSView, context: Context) {
        nsView.resolveWindowIfNeeded()
    }
}

private final class ShellWindowPlacementNSView: NSView {
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        resolveWindowIfNeeded()
    }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        resolveWindowIfNeeded()
    }

    func resolveWindowIfNeeded() {
        DispatchQueue.main.async { [weak self] in
            guard let window = self?.window else { return }
            AlanShellWindowPlacement.apply(to: window)
        }
    }
}

private enum AlanShellWindowPlacement {
    private static var positionedWindowNumbers: Set<Int> = []

    static func apply(to window: NSWindow) {
        window.title = "Alan"
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.styleMask.insert(.fullSizeContentView)
        window.isMovableByWindowBackground = true
        window.minSize = NSSize(width: 1180, height: 760)
        window.tabbingMode = .disallowed
        if #available(macOS 13.0, *) {
            window.toolbarStyle = .unifiedCompact
        }

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

private struct ShellSidebarView: View {
    @ObservedObject var host: ShellHostController
    let openCommandSurface: () -> Void
    @State private var hoveredSurfaceID: String?
    @State private var hoveredSpaceID: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            sidebarHeader
            commandLauncher
            tabSection
            spaceDock
        }
        .padding(.horizontal, 12)
        .padding(.bottom, 15)
        .frame(maxHeight: .infinity, alignment: .top)
        .background {
            ZStack {
                SidebarMaterialView()
                ShellPalette.sidebar.opacity(0.35)
            }
            .ignoresSafeArea(edges: .top)
        }
    }

    private var sidebarHeader: some View {
        HStack(alignment: .center, spacing: 10) {
            ZStack {
                RoundedRectangle(cornerRadius: 11, style: .continuous)
                    .fill(Color.white.opacity(0.18))
                Text("A")
                    .font(.system(size: 12.5, weight: .bold, design: .rounded))
                    .foregroundStyle(ShellPalette.accent)
            }
            .frame(width: 28, height: 28)

            VStack(alignment: .leading, spacing: 2) {
                Text(host.selectedSpace?.title ?? "Alan")
                    .font(.system(size: 15.5, weight: .semibold))
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                Text(spaceSubtitle)
                    .font(.system(size: 10.5, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 8)

            Menu {
                Button("New Tab") {
                    _ = host.openTerminalSurface()
                }
                Button("Open in Alan") {
                    _ = host.openAlanSurface()
                }
            } label: {
                Image(systemName: "plus")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(.primary)
                    .frame(width: 26, height: 26)
                    .background(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .fill(Color.white.opacity(0.12))
                    )
            }
            .menuStyle(.borderlessButton)
            .buttonStyle(.plain)
            .menuIndicator(.hidden)
        }
    }

    private var commandLauncher: some View {
        Button(action: openCommandSurface) {
            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.secondary)
                Text("Go to tab or command")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer(minLength: 0)
                Text("⌘K")
                    .font(.system(size: 11, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 11)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .fill(Color.white.opacity(0.10))
            )
        }
        .buttonStyle(.plain)
        .keyboardShortcut("k", modifiers: [.command])
    }

    private var tabSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Tabs")
                    .font(.system(size: 11, weight: .semibold))
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)
                Spacer(minLength: 0)
                if let selectedSpace = host.selectedSpace {
                    Text("\(selectedSpace.surfaces.count)")
                        .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.secondary)
                }
            }

            ScrollView(.vertical, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 4) {
                    if let selectedSpace = host.selectedSpace {
                        ForEach(selectedSpace.surfaces) { surface in
                            ShellTabRow(
                                title: tabTitle(for: surface),
                                subtitle: tabSubtitle(for: surface),
                                iconName: tabIconName(for: surface),
                                attention: strongestAttention(for: surface),
                                showsAlanMarker: showsAlanMarker(for: surface),
                                isSelected: host.selectedSurface?.surfaceID == surface.surfaceID,
                                isHovered: hoveredSurfaceID == surface.surfaceID,
                                showsMenuAffordance: hoveredSurfaceID == surface.surfaceID,
                                onClose: { close(surface: surface) }
                            )
                            .contentShape(Rectangle())
                            .onTapGesture {
                                host.select(surfaceID: surface.surfaceID)
                            }
                            .onHover { isHovering in
                                hoveredSurfaceID = isHovering ? surface.surfaceID : nil
                            }
                            .contextMenu {
                                Button("New Tab") {
                                    _ = host.openTerminalSurface()
                                }
                                Button("Open in Alan") {
                                    _ = host.openAlanSurface()
                                }
                                Divider()
                                Button("Close Tab", role: .destructive) {
                                    close(surface: surface)
                                }
                            }
                        }
                    } else {
                        ShellEmptyStateRow(
                            title: "No spaces yet",
                            detail: "Create a space to start a new stack of terminal tabs."
                        )
                    }
                }
            }
        }
        .frame(maxHeight: .infinity, alignment: .top)
    }

    private var spaceDock: some View {
        ShellSidebarSection(title: "Spaces", accessory: "⌥⌘1-9") {
            HStack(spacing: 10) {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(host.spaces) { space in
                            Button {
                                host.select(spaceID: space.spaceID)
                            } label: {
                                ShellSpaceRailItem(
                                    title: space.title,
                                    symbolName: spaceSymbol(for: space),
                                    attention: space.attention,
                                    isSelected: host.selectedSpace?.spaceID == space.spaceID,
                                    isHovered: hoveredSpaceID == space.spaceID
                                )
                            }
                            .buttonStyle(.plain)
                            .onHover { isHovering in
                                hoveredSpaceID = isHovering ? space.spaceID : nil
                            }
                        }
                    }
                    .padding(.vertical, 2)
                }

                Menu {
                    Button("New Space") {
                        _ = host.createTerminalSpace()
                    }
                    Button("New Space with Alan") {
                        _ = host.createAlanSpace()
                    }
                } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 12.5, weight: .semibold))
                        .foregroundStyle(.primary)
                        .frame(width: 30, height: 30)
                        .background(
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .fill(Color.white.opacity(0.10))
                        )
                }
                .menuStyle(.borderlessButton)
                .buttonStyle(.plain)
                .menuIndicator(.hidden)
                .help("Create a new space")
            }
        }
    }

    private var spaceSubtitle: String {
        guard let selectedSpace = host.selectedSpace else {
            return "Terminal-first on macOS"
        }

        let count = selectedSpace.surfaces.count
        return count == 1 ? "1 tab" : "\(count) tabs"
    }

    private func close(surface: ShellSurface) {
        host.select(surfaceID: surface.surfaceID)
        _ = host.closeSelectedSurface()
    }

    private func fallbackTitle(for surface: ShellSurface) -> String {
        switch surface.kind {
        case .terminal:
            return "Terminal"
        case .scratch:
            return "Scratch"
        case .log:
            return "Logs"
        }
    }

    private func tabIconName(for surface: ShellSurface) -> String {
        switch surface.kind {
        case .terminal:
            return "terminal"
        case .scratch:
            return "note.text"
        case .log:
            return "doc.text.magnifyingglass"
        }
    }

    private func tabTitle(for surface: ShellSurface) -> String {
        let panes = host.shellState.panes.filter { $0.surfaceID == surface.surfaceID }
        let primaryPane = panes.first
        return shellDisplayTitle(
            rawTitle: surface.title ?? primaryPane?.viewport?.title,
            workingDirectoryName: primaryPane?.context?.workingDirectoryName,
            cwd: primaryPane?.cwd,
            program: primaryPane?.process?.program,
            launchTarget: primaryPane?.resolvedLaunchTarget ?? .shell,
            fallback: fallbackTitle(for: surface)
        )
    }

    private func tabSubtitle(for surface: ShellSurface) -> String {
        let panes = host.shellState.panes.filter { $0.surfaceID == surface.surfaceID }
        let primaryPane = panes.first
        let title = tabTitle(for: surface)

        if let branch = primaryPane?.context?.gitBranch,
           let folder = primaryPane?.context?.workingDirectoryName
        {
            if folder == title {
                return branch
            }
            return "\(folder)  ·  \(branch)"
        }

        if let folder = shellVisibleLabel(primaryPane?.context?.workingDirectoryName) ?? shellPathLeaf(primaryPane?.cwd) {
            if folder == title, let program = shellVisibleLabel(primaryPane?.process?.program) {
                return program
            }
            return folder
        }

        if let program = primaryPane?.process?.program {
            return program
        }

        return surface.kind.rawValue.capitalized
    }

    private func strongestAttention(for surface: ShellSurface) -> ShellAttentionState? {
        host.shellState.panes
            .filter { $0.surfaceID == surface.surfaceID }
            .map(\.attention)
            .sorted { attentionRank(for: $0) > attentionRank(for: $1) }
            .first(where: { $0 != .idle })
    }

    private func spaceSymbol(for space: ShellSpace) -> String {
        let hasAlan = host.shellState.panes.contains { pane in
            pane.spaceID == space.spaceID && pane.resolvedLaunchTarget == .alan
        }

        if hasAlan {
            return "sparkles"
        }

        if space.surfaces.count > 1 {
            return "square.stack.3d.up"
        }

        return "terminal"
    }

    private func showsAlanMarker(for surface: ShellSurface) -> Bool {
        host.shellState.panes.contains { pane in
            pane.surfaceID == surface.surfaceID && pane.resolvedLaunchTarget == .alan
        }
    }

    private func attentionRank(for attention: ShellAttentionState) -> Int {
        switch attention {
        case .awaitingUser:
            return 3
        case .notable:
            return 2
        case .active:
            return 1
        case .idle:
            return 0
        }
    }
}

private struct ShellWorkspaceView: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(spacing: 0) {
            TerminalPaneView(host: host)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .padding(16)
    }
}

private struct ShellTopBarView: View {
    @ObservedObject var host: ShellHostController
    @Binding var isCommandSurfacePresented: Bool
    @Binding var showsInspector: Bool

    private var title: String {
        if let selectedSurface = host.selectedSurface {
            let surfacePanes = host.shellState.panes.filter { $0.surfaceID == selectedSurface.surfaceID }
            return shellSurfaceDisplayTitle(
                surface: selectedSurface,
                panes: surfacePanes,
                fallback: host.selectedSpace?.title ?? "Alan"
            )
        }

        return shellDisplayTitle(
            rawTitle: host.selectedSurface?.title ?? host.selectedSpace?.title,
            workingDirectoryName: nil,
            cwd: nil,
            program: nil,
            launchTarget: .shell,
            fallback: host.selectedSpace?.title ?? "Alan"
        )
    }

    private var subtitle: String? {
        var parts: [String] = []

        if let selectedSpace = host.selectedSpace?.title, selectedSpace != title {
            parts.append(selectedSpace)
        }

        if let branch = host.selectedPane?.context?.gitBranch {
            parts.append(branch)
        }

        if host.panesForSelectedSurface.count > 1 {
            parts.append("\(host.panesForSelectedSurface.count) panes")
        }

        if host.selectedPane?.resolvedLaunchTarget == .alan {
            parts.append("Alan tab")
        }

        return parts.isEmpty ? nil : parts.joined(separator: "  ·  ")
    }

    var body: some View {
        HStack(spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 16.5, weight: .semibold))
                    .tracking(-0.2)
                    .foregroundStyle(ShellPalette.ink)
                if let subtitle {
                    Text(subtitle)
                        .font(.system(size: 10.5, weight: .medium))
                        .foregroundStyle(ShellPalette.mutedInk)
                }
            }

            Spacer(minLength: 14)

            ShellToolbarButton(
                symbol: "magnifyingglass",
                label: "Go to or Command..."
            ) {
                isCommandSurfacePresented = true
            }
            .keyboardShortcut("k", modifiers: [.command])

            if host.awaitingAttentionCount > 0,
               let firstAttention = host.attentionItems.first
            {
                Button {
                    host.focusAttentionItem(firstAttention)
                } label: {
                    ShellToolbarBadgeButton(symbol: "bell", count: host.awaitingAttentionCount)
                }
                .buttonStyle(.plain)
            }

            Menu {
                Button("New Tab") {
                    _ = host.openTerminalSurface()
                }
                Button("Open in Alan") {
                    _ = host.openAlanSurface()
                }
                Divider()
                Button("New Space") {
                    _ = host.createTerminalSpace()
                }
                Button("New Space with Alan") {
                    _ = host.createAlanSpace()
                }
            } label: {
                ShellToolbarGlyph(symbol: "plus")
            }
            .menuStyle(.borderlessButton)
            .buttonStyle(.plain)
            .menuIndicator(.hidden)

            Menu {
                Button("Split Horizontally") {
                    _ = host.splitFocusedPane(direction: .horizontal)
                }
                Button("Split Vertically") {
                    _ = host.splitFocusedPane(direction: .vertical)
                }
                Divider()
                Button("Jump to attention") {
                    _ = host.focusTopRoutingCandidate(preferredPaneID: host.selectedPane?.paneID)
                }
            } label: {
                ShellToolbarGlyph(symbol: "square.split.2x1")
            }
            .menuStyle(.borderlessButton)
            .buttonStyle(.plain)
            .menuIndicator(.hidden)

            ShellToolbarButton(
                symbol: showsInspector ? "sidebar.trailing" : "sidebar.right",
                label: showsInspector ? "Hide Inspector" : "Show Inspector"
            ) {
                showsInspector.toggle()
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 11)
    }
}

private struct ShellSpaceKeyboardShortcuts: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(spacing: 0) {
            Button("") {
                host.selectAdjacentSpace(offset: -1)
            }
            .keyboardShortcut(.leftArrow, modifiers: [.command, .option])

            Button("") {
                host.selectAdjacentSpace(offset: 1)
            }
            .keyboardShortcut(.rightArrow, modifiers: [.command, .option])

            ForEach(Array(host.spaces.prefix(9).enumerated()), id: \.element.spaceID) { index, _ in
                Button("") {
                    host.selectSpace(at: index)
                }
                .keyboardShortcut(
                    KeyEquivalent(Character(String(index + 1))),
                    modifiers: [.command, .option]
                )
            }
        }
        .labelsHidden()
        .buttonStyle(.plain)
        .frame(width: 0, height: 0)
        .opacity(0.001)
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}

private struct ShellSpaceRailItem: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let title: String
    let symbolName: String
    let attention: ShellAttentionState
    let isSelected: Bool
    let isHovered: Bool

    var body: some View {
        ZStack(alignment: .topTrailing) {
            ZStack {
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .fill(
                        isSelected
                            ? Color.white.opacity(0.22)
                            : (isHovered ? Color.white.opacity(0.14) : Color.white.opacity(0.08))
                    )
                Image(systemName: symbolName)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(isSelected ? ShellPalette.accent : .primary)
            }
            .frame(width: 34, height: 34)

            if attention != .idle {
                Circle()
                    .fill(attentionColor)
                    .frame(width: 9, height: 9)
                    .overlay {
                        Circle()
                            .stroke(Color.white.opacity(0.9), lineWidth: 1.5)
                    }
                    .offset(x: 2, y: -2)
            }
        }
        .scaleEffect(isSelected ? 1 : (isHovered ? 1.03 : 1))
        .shadow(color: isSelected ? ShellPalette.accent.opacity(0.12) : .clear, radius: 8, y: 3)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isHovered)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isSelected)
        .help(title)
    }

    private var attentionColor: Color {
        switch attention {
        case .idle:
            return ShellPalette.line
        case .active:
            return ShellPalette.accent
        case .awaitingUser:
            return ShellPalette.attention
        case .notable:
            return Color(red: 0.71, green: 0.58, blue: 0.28)
        }
    }
}

private struct ShellTabRow: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let title: String
    let subtitle: String
    let iconName: String
    let attention: ShellAttentionState?
    let showsAlanMarker: Bool
    let isSelected: Bool
    let isHovered: Bool
    let showsMenuAffordance: Bool
    let onClose: () -> Void

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: iconName)
                .font(.system(size: 11.5, weight: .semibold))
                .foregroundStyle(isSelected ? ShellPalette.accent : .secondary)
                .frame(width: 14)

            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(title)
                        .font(.system(size: 13, weight: .semibold))
                        .tracking(-0.1)
                        .foregroundStyle(.primary)
                        .lineLimit(1)

                    if showsAlanMarker {
                        Image(systemName: "sparkles")
                            .font(.system(size: 9, weight: .bold))
                            .foregroundStyle(ShellPalette.accent)
                    }
                }

                Text(subtitle)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 8)

            if let attention {
                Circle()
                    .fill(attentionColor(for: attention))
                    .frame(width: 8, height: 8)
            }

            if showsMenuAffordance {
                Button(action: onClose) {
                    Image(systemName: "xmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundStyle(.secondary)
                        .frame(width: 18, height: 18)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .background(
            RoundedRectangle(cornerRadius: 11, style: .continuous)
                .fill(
                    isSelected
                        ? Color.white.opacity(0.15)
                        : (isHovered ? Color.white.opacity(0.08) : Color.clear)
                )
        )
        .scaleEffect(isHovered && !isSelected ? 1.005 : 1)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isHovered)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isSelected)
    }

    private func attentionColor(for attention: ShellAttentionState) -> Color {
        switch attention {
        case .idle:
            return ShellPalette.line
        case .active:
            return ShellPalette.accent
        case .awaitingUser:
            return ShellPalette.attention
        case .notable:
            return Color(red: 0.71, green: 0.58, blue: 0.28)
        }
    }
}

private struct ShellSurfaceRailView: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        HStack(spacing: 10) {
            ForEach(host.selectedSpace?.surfaces ?? []) { surface in
                Button {
                    host.select(surfaceID: surface.surfaceID)
                } label: {
                    VStack(alignment: .leading, spacing: 5) {
                        Text(surface.title ?? surface.kind.rawValue.capitalized)
                            .font(.system(size: 13, weight: .semibold, design: .rounded))
                        Text(surface.kind.rawValue)
                            .font(.system(size: 11, weight: .medium, design: .rounded))
                            .textCase(.uppercase)
                            .foregroundStyle(host.selectedSurface?.surfaceID == surface.surfaceID ? ShellPalette.accent.opacity(0.82) : ShellPalette.mutedInk)
                    }
                    .foregroundStyle(host.selectedSurface?.surfaceID == surface.surfaceID ? ShellPalette.ink : ShellPalette.mutedInk)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(
                        RoundedRectangle(cornerRadius: 18, style: .continuous)
                            .fill(host.selectedSurface?.surfaceID == surface.surfaceID ? Color.white.opacity(0.78) : Color.white.opacity(0.38))
                    )
                    .overlay {
                        RoundedRectangle(cornerRadius: 18, style: .continuous)
                            .stroke(ShellPalette.line.opacity(host.selectedSurface?.surfaceID == surface.surfaceID ? 0.55 : 0.24), lineWidth: 1)
                    }
                }
                .buttonStyle(.plain)
            }
        }
    }
}

private struct ShellInspectorView: View {
    @ObservedObject var host: ShellHostController
    @State private var selectedSection: ShellInspectorSection = .overview

    private var selectedTabTitle: String {
        guard let selectedSurface = host.selectedSurface else { return "No tab" }
        let surfacePanes = host.shellState.panes.filter { $0.surfaceID == selectedSurface.surfaceID }
        return shellSurfaceDisplayTitle(
            surface: selectedSurface,
            panes: surfacePanes,
            fallback: host.selectedSpace?.title ?? "Alan"
        )
    }

    private var focusedProgramLabel: String {
        if let program = shellVisibleLabel(host.focusedPane?.process?.program) {
            return program
        }

        if host.focusedPane?.resolvedLaunchTarget == .alan {
            return "Alan"
        }

        return "Shell"
    }

    private var focusedLocationLabel: String {
        if let workingDirectoryName = shellVisibleLabel(host.focusedPane?.context?.workingDirectoryName) {
            return workingDirectoryName
        }

        if let cwd = shellVisibleLabel(host.focusedPane?.cwd) {
            return cwd
        }

        return "Unknown"
    }

    private var focusedAttentionLabel: String {
        host.focusedPane?.attention.rawValue
            .replacingOccurrences(of: "_", with: " ")
            .capitalized ?? "Idle"
    }

    private var inspectorAttentionItem: ShellAttentionItem? {
        host.attentionItems.first { item in
            item.attention == .awaitingUser || item.attention == .notable
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Inspector")
                    .font(.system(size: 20, weight: .semibold))
                    .foregroundStyle(ShellPalette.ink)
                Spacer(minLength: 0)
                Picker("Inspector Section", selection: $selectedSection) {
                    ForEach(ShellInspectorSection.allCases) { section in
                        Text(section.title).tag(section)
                    }
                }
                .labelsHidden()
                .pickerStyle(.segmented)
                .frame(width: 200)
            }

            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    switch selectedSection {
                    case .overview:
                        InspectorCard(title: "Focused Pane") {
                            VStack(alignment: .leading, spacing: 8) {
                                KeyValueRow(
                                    label: "Tab",
                                    value: selectedTabTitle
                                )
                                KeyValueRow(
                                    label: "Program",
                                    value: focusedProgramLabel
                                )
                                if focusedLocationLabel != "Unknown" {
                                    KeyValueRow(
                                        label: "Location",
                                        value: focusedLocationLabel
                                    )
                                }
                                KeyValueRow(
                                    label: "Attention",
                                    value: focusedAttentionLabel
                                )
                            }
                        }

                        InspectorCard(title: "Alan") {
                            if let binding = host.focusedPane?.alanBinding {
                                VStack(alignment: .leading, spacing: 8) {
                                    KeyValueRow(label: "Session", value: binding.sessionID)
                                    KeyValueRow(label: "Run", value: binding.runStatus)
                                    KeyValueRow(label: "Pending Yield", value: binding.pendingYield ? "Yes" : "None")
                                }
                            } else {
                                Text("This pane is available to Alan, but no Alan session is attached right now.")
                                    .font(.system(size: 13, weight: .medium))
                                    .foregroundStyle(ShellPalette.mutedInk)
                                    .fixedSize(horizontal: false, vertical: true)
                            }
                        }

                        InspectorCard(title: "Attention") {
                            VStack(alignment: .leading, spacing: 10) {
                                KeyValueRow(
                                    label: "Waiting",
                                    value: host.awaitingAttentionCount == 0 ? "None" : "\(host.awaitingAttentionCount)"
                                )

                                if let firstAttention = inspectorAttentionItem {
                                    Button {
                                        host.focusAttentionItem(firstAttention)
                                    } label: {
                                        HStack(spacing: 8) {
                                            Circle()
                                                .fill(ShellPalette.attention)
                                                .frame(width: 8, height: 8)
                                            VStack(alignment: .leading, spacing: 2) {
                                                Text(shellNormalizedTitle(firstAttention.title) ?? firstAttention.title)
                                                    .font(.system(size: 12, weight: .semibold))
                                                    .foregroundStyle(ShellPalette.ink)
                                                Text(shellUserFacingSummary(firstAttention.summary) ?? "Needs attention")
                                                    .font(.system(size: 11, weight: .medium))
                                                    .foregroundStyle(ShellPalette.mutedInk)
                                                    .lineLimit(2)
                                            }
                                            Spacer(minLength: 0)
                                        }
                                        .padding(.horizontal, 10)
                                        .padding(.vertical, 9)
                                        .background(
                                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                                .fill(Color.white.opacity(0.72))
                                        )
                                    }
                                    .buttonStyle(.plain)
                                } else {
                                    Text("Nothing currently needs attention.")
                                        .font(.system(size: 13, weight: .medium))
                                        .foregroundStyle(ShellPalette.mutedInk)
                                }
                            }
                        }
                    case .debug:
                        InspectorCard(title: "Debug Snapshot") {
                            VStack(alignment: .leading, spacing: 12) {
                                Text("Canonical local shell state exposed to `alan shell state`.")
                                    .font(.system(size: 13, weight: .medium))
                                    .foregroundStyle(ShellPalette.mutedInk)

                                HStack {
                                    Button("Copy JSON") {
                                        host.copySnapshotJSON()
                                    }
                                    .buttonStyle(.borderless)
                                    .foregroundStyle(ShellPalette.accent)

                                    Spacer(minLength: 0)

                                    if let lastCopiedAt = host.lastCopiedAt {
                                        Text(lastCopiedAt.formatted(date: .omitted, time: .shortened))
                                            .font(.system(size: 11, weight: .medium))
                                            .foregroundStyle(ShellPalette.mutedInk)
                                    }
                                }

                                Text(host.snapshotJSON)
                                    .font(.system(size: 11, weight: .regular, design: .monospaced))
                                    .foregroundStyle(ShellPalette.ink)
                                    .textSelection(.enabled)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .padding(12)
                                    .background(
                                        RoundedRectangle(cornerRadius: 16, style: .continuous)
                                            .fill(Color.white.opacity(0.72))
                                    )
                            }
                        }
                    }
                }
            }
        }
        .padding(16)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(ShellPalette.window.opacity(0.95))
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(ShellPalette.line.opacity(0.15))
                .frame(width: 1)
        }
    }
}

private struct ShellSidebarSection<Content: View>: View {
    let title: String
    let accessory: String?
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(title)
                    .font(.system(size: 11, weight: .semibold))
                    .textCase(.uppercase)
                    .foregroundStyle(.secondary)
                Spacer(minLength: 0)
                if let accessory {
                    Text(accessory)
                        .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.tertiary)
                }
            }

            content
        }
    }
}

private struct ShellSpaceRow: View {
    let space: ShellSpace
    let isSelected: Bool

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Circle()
                .fill(color(for: space.attention))
                .frame(width: 8, height: 8)

            VStack(alignment: .leading, spacing: 4) {
                Text(space.title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(ShellPalette.ink)
                Text("\(space.surfaces.count) surface\(space.surfaces.count == 1 ? "" : "s")")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(ShellPalette.mutedInk)
            }

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(isSelected ? ShellPalette.sidebarCard : Color.clear)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(isSelected ? ShellPalette.line.opacity(0.6) : Color.clear, lineWidth: 1)
        }
    }

    private func color(for attention: ShellAttentionState) -> Color {
        switch attention {
        case .idle:
            return ShellPalette.line
        case .active:
            return ShellPalette.accent
        case .awaitingUser:
            return ShellPalette.attention
        case .notable:
            return Color(red: 0.71, green: 0.58, blue: 0.28)
        }
    }
}

private struct ShellSurfaceRow: View {
    let surface: ShellSurface
    let isSelected: Bool

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: iconName)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(isSelected ? ShellPalette.accent : ShellPalette.mutedInk)
                .frame(width: 14)

            VStack(alignment: .leading, spacing: 2) {
                Text(surface.title ?? surface.kind.rawValue.capitalized)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(ShellPalette.ink)
                    .lineLimit(1)
                Text(surface.kind.rawValue.capitalized)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(ShellPalette.mutedInk)
            }

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(isSelected ? Color.white.opacity(0.95) : Color.clear)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(isSelected ? ShellPalette.line.opacity(0.65) : Color.clear, lineWidth: 1)
        }
    }

    private var iconName: String {
        switch surface.kind {
        case .terminal:
            return "terminal"
        case .log:
            return "doc.text.magnifyingglass"
        case .scratch:
            return "note.text"
        }
    }
}

private struct ShellAttentionRow: View {
    let item: ShellAttentionItem

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(item.title)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(ShellPalette.ink)
                Spacer(minLength: 8)
                Text(item.attention.rawValue.replacingOccurrences(of: "_", with: " "))
                    .font(.system(size: 10, weight: .semibold))
                    .textCase(.uppercase)
                    .foregroundStyle(item.attention == .awaitingUser ? ShellPalette.attention : ShellPalette.accent)
            }
            Text(item.summary)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(ShellPalette.mutedInk)
                .frame(maxWidth: .infinity, alignment: .leading)
                .lineLimit(2)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(Color.white.opacity(0.82))
        )
    }
}

private struct ShellEmptyStateRow: View {
    let title: String
    let detail: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.system(size: 13, weight: .semibold))
            Text(detail)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(ShellPalette.mutedInk)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(Color.white.opacity(0.72))
        )
    }
}

private enum ShellInspectorSection: String, CaseIterable, Identifiable {
    case overview
    case debug

    var id: String { rawValue }

    var title: String {
        switch self {
        case .overview:
            return "Overview"
        case .debug:
            return "Debug"
        }
    }
}

private enum ShellCommandSurfaceAction: String, CaseIterable, Identifiable {
    case newSpace
    case newAlanSpace
    case openSurface
    case openAlanSurface
    case jumpToAttention
    case focusBestPane
    case splitHorizontal
    case splitVertical
    case liftPane
    case closePane
    case closeSurface
    case copySnapshot

    var id: String { rawValue }

    var title: String {
        switch self {
        case .newSpace:
            return "Create Space"
        case .newAlanSpace:
            return "Create Space with Alan"
        case .openSurface:
            return "Open New Tab"
        case .openAlanSurface:
            return "Open In Alan"
        case .jumpToAttention:
            return "Jump To Attention"
        case .focusBestPane:
            return "Focus Best Routing Pane"
        case .splitHorizontal:
            return "Split Focused Pane Horizontally"
        case .splitVertical:
            return "Split Focused Pane Vertically"
        case .liftPane:
            return "Lift Focused Pane To Tab"
        case .closePane:
            return "Close Focused Pane"
        case .closeSurface:
            return "Close Current Tab"
        case .copySnapshot:
            return "Copy Shell Snapshot"
        }
    }

    var detail: String {
        switch self {
        case .newSpace:
            return "Start a fresh space with a plain login shell."
        case .newAlanSpace:
            return "Start a fresh space that opens directly into Alan."
        case .openSurface:
            return "Open another tab inside the current space."
        case .openAlanSurface:
            return "Open another tab that boots directly into Alan."
        case .jumpToAttention:
            return "Jump to the strongest pane that currently needs approval or attention."
        case .focusBestPane:
            return "Use shell routing signals to jump to the strongest candidate pane."
        case .splitHorizontal:
            return "Create a stacked split beneath the focused pane."
        case .splitVertical:
            return "Create a side-by-side split next to the focused pane."
        case .liftPane:
            return "Move the focused pane into its own tab without losing shell identity."
        case .closePane:
            return "Close the focused pane and keep the remaining tab layout intact."
        case .closeSurface:
            return "Close the current tab while preserving the rest of the space."
        case .copySnapshot:
            return "Copy the canonical shell JSON for debugging or agent context."
        }
    }

    func matches(query: String) -> Bool {
        query.isEmpty
            || title.localizedCaseInsensitiveContains(query)
            || detail.localizedCaseInsensitiveContains(query)
            || keywordMatches(query: query)
    }

    private func keywordMatches(query: String) -> Bool {
        let normalizedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalizedQuery.isEmpty else { return false }
        return keywords.contains { normalizedQuery.contains($0) }
    }

    private var keywords: [String] {
        switch self {
        case .newSpace:
            return ["new space", "fresh space", "workspace", "terminal space"]
        case .newAlanSpace:
            return ["new space with alan", "alan space", "agent space"]
        case .openSurface:
            return ["open tab", "new tab", "open terminal tab"]
        case .openAlanSurface:
            return ["open in alan", "alan tab", "new alan tab"]
        case .jumpToAttention:
            return ["jump to attention", "jump attention", "focus waiting pane", "approval", "waiting pane"]
        case .focusBestPane:
            return ["best pane", "route", "routing", "focus best", "jump pane"]
        case .splitHorizontal:
            return ["split horizontal", "split below", "stack split"]
        case .splitVertical:
            return ["split vertical", "split right", "side by side"]
        case .liftPane:
            return ["lift pane", "move pane", "extract pane"]
        case .closePane:
            return ["close pane", "remove pane"]
        case .closeSurface:
            return ["close surface", "close tab"]
        case .copySnapshot:
            return ["copy snapshot", "copy json", "debug snapshot"]
        }
    }
}

private struct ShellCommandSurfaceIntent: Identifiable {
    enum Route {
        case action(ShellCommandSurfaceAction)
        case attention(ShellAttentionItem)
        case candidate(AlanShellRoutingCandidate)
    }

    let title: String
    let detail: String
    let accent: Color
    let route: Route

    var id: String { title }
}

private struct ShellCommandSurfaceView: View {
    @ObservedObject var host: ShellHostController
    @Binding var isPresented: Bool
    @State private var query = ""
    @FocusState private var isQueryFocused: Bool
    @StateObject private var voiceController = ShellVoiceCommandController()

    private var matchingActions: [ShellCommandSurfaceAction] {
        let allActions = ShellCommandSurfaceAction.allCases.filter { $0.matches(query: query) }

        guard query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return allActions
        }

        let defaultActions: [ShellCommandSurfaceAction] = [
            .openSurface,
            .openAlanSurface,
            .splitVertical,
            .splitHorizontal,
            .jumpToAttention,
            .newSpace,
        ]

        return defaultActions.filter { allActions.contains($0) }
    }

    private var matchingAttention: [ShellAttentionItem] {
        let visibleItems = host.attentionItems.filter {
            $0.attention == .awaitingUser || $0.attention == .notable
        }

        guard !query.isEmpty else { return Array(visibleItems.prefix(2)) }
        return visibleItems.filter {
            $0.title.localizedCaseInsensitiveContains(query)
                || $0.summary.localizedCaseInsensitiveContains(query)
        }
    }

    private var matchingRoutingCandidates: [AlanShellRoutingCandidate] {
        let candidates = host.routingCandidates
        guard !query.isEmpty else { return Array(candidates.prefix(2)) }

        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return candidates.filter { candidate in
            guard let pane = host.shellState.panes.first(where: { $0.paneID == candidate.paneID }) else {
                return candidate.paneID.localizedCaseInsensitiveContains(query)
            }

            return candidate.paneID.localizedCaseInsensitiveContains(query)
                || (pane.viewport?.title?.localizedCaseInsensitiveContains(query) ?? false)
                || (pane.viewport?.summary?.localizedCaseInsensitiveContains(query) ?? false)
                || (pane.process?.program.localizedCaseInsensitiveContains(query) ?? false)
                || candidate.reasons.contains { $0.lowercased().contains(normalized) }
        }
    }

    private var primaryIntent: ShellCommandSurfaceIntent? {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return nil }

        if normalized.contains("route")
            || normalized.contains("best pane")
            || normalized.contains("focus best")
        {
            if let candidate = matchingRoutingCandidates.first {
                return ShellCommandSurfaceIntent(
                    title: "Focus \(candidate.paneID)",
                    detail: routingDetail(for: candidate),
                    accent: ShellPalette.accent,
                    route: .candidate(candidate)
                )
            }
        }

        if normalized.contains("attention") || normalized.contains("waiting") || normalized.contains("jump") {
            if let firstAttention = matchingAttention.first {
                return ShellCommandSurfaceIntent(
                    title: "Jump To \(firstAttention.title)",
                    detail: "Focus the pane that currently needs attention first.",
                    accent: firstAttention.attention == .awaitingUser ? ShellPalette.accent : ShellPalette.ink,
                    route: .attention(firstAttention)
                )
            }
        }

        if let action = matchingActions.first {
            return ShellCommandSurfaceIntent(
                title: action.title,
                detail: action.detail,
                accent: ShellPalette.accent,
                route: .action(action)
            )
        }

        return nil
    }

    var body: some View {
        ScrollView(showsIndicators: false) {
            VStack(alignment: .leading, spacing: 14) {
                HStack(alignment: .center, spacing: 12) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(ShellPalette.accent)

                    TextField(
                        "",
                        text: $query,
                        prompt: Text("Go to tab or run command")
                            .foregroundStyle(ShellPalette.mutedInk.opacity(0.9))
                    )
                        .textFieldStyle(.plain)
                        .font(.system(size: 16, weight: .medium))
                        .foregroundStyle(ShellPalette.ink)
                        .focused($isQueryFocused)
                        .onSubmit {
                            executePrimaryIntent()
                        }

                    Text("⌘K")
                        .font(.system(size: 10, weight: .semibold, design: .monospaced))
                        .foregroundStyle(ShellPalette.mutedInk.opacity(0.9))
                        .padding(.horizontal, 7)
                        .padding(.vertical, 4)
                        .background(
                            Capsule(style: .continuous)
                                .fill(ShellPalette.canvas.opacity(0.95))
                        )

                    Button {
                        voiceController.toggleListening { recognizedCommand in
                            query = recognizedCommand
                            executePrimaryIntent()
                        }
                    } label: {
                        Image(systemName: voiceController.isListening ? "mic.fill" : "mic")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(voiceController.isListening ? ShellPalette.accent : ShellPalette.mutedInk)
                            .frame(width: 26, height: 26)
                            .background(
                                RoundedRectangle(cornerRadius: 8, style: .continuous)
                                    .fill(ShellPalette.canvas.opacity(0.92))
                            )
                    }
                    .buttonStyle(.plain)

                    Button {
                        isPresented = false
                    } label: {
                        Image(systemName: "xmark")
                            .font(.system(size: 11, weight: .bold))
                            .foregroundStyle(ShellPalette.mutedInk)
                            .frame(width: 26, height: 26)
                            .background(
                                RoundedRectangle(cornerRadius: 8, style: .continuous)
                                    .fill(ShellPalette.canvas.opacity(0.92))
                            )
                    }
                    .buttonStyle(.plain)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
                .background(
                    RoundedRectangle(cornerRadius: 18, style: .continuous)
                        .fill(Color.white.opacity(0.9))
                )
                .overlay {
                    RoundedRectangle(cornerRadius: 18, style: .continuous)
                        .stroke(ShellPalette.line.opacity(0.32), lineWidth: 1)
                }
                .shadow(color: Color.black.opacity(0.035), radius: 10, y: 4)

                if voiceController.isListening || primaryIntent != nil {
                    VStack(alignment: .leading, spacing: 12) {
                        if voiceController.isListening {
                            HStack(spacing: 8) {
                                Circle()
                                    .fill(ShellPalette.attention)
                                    .frame(width: 8, height: 8)
                                Text("Listening for shell actions")
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(ShellPalette.mutedInk)
                            }
                        }

                        if let primaryIntent {
                            VStack(alignment: .leading, spacing: 10) {
                                sectionLabel("Best match")
                                Button {
                                    execute(primaryIntent.route)
                                } label: {
                                    ShellCommandRow(
                                        title: primaryIntent.title,
                                        detail: primaryIntent.detail,
                                        accent: primaryIntent.accent
                                    )
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }

                VStack(alignment: .leading, spacing: 10) {
                    sectionLabel("Actions")
                    VStack(spacing: 8) {
                        ForEach(matchingActions) { action in
                            Button {
                                perform(action)
                            } label: {
                                ShellCommandRow(
                                    title: action.title,
                                    detail: action.detail,
                                    accent: ShellPalette.accent
                                )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }

                if !matchingRoutingCandidates.isEmpty && !query.isEmpty {
                    VStack(alignment: .leading, spacing: 10) {
                        sectionLabel("Routing")
                        VStack(spacing: 8) {
                            ForEach(matchingRoutingCandidates) { candidate in
                                Button {
                                    execute(.candidate(candidate))
                                } label: {
                                    ShellCommandRow(
                                        title: "Focus \(candidate.paneID)",
                                        detail: routingDetail(for: candidate),
                                        accent: ShellPalette.accent
                                    )
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }

                if !matchingAttention.isEmpty {
                    VStack(alignment: .leading, spacing: 10) {
                        sectionLabel("Attention")
                        VStack(spacing: 8) {
                            ForEach(matchingAttention) { item in
                                Button {
                                    host.focusAttentionItem(item)
                                    isPresented = false
                                } label: {
                                    ShellCommandRow(
                                        title: shellNormalizedTitle(item.title) ?? item.title,
                                        detail: shellUserFacingSummary(item.summary) ?? "Needs attention",
                                        accent: item.attention == .awaitingUser ? ShellPalette.attention : ShellPalette.ink
                                    )
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
            }
        }
        .padding(16)
        .frame(width: 478, height: 568)
        .background(
            ZStack {
                RoundedRectangle(cornerRadius: 24, style: .continuous)
                    .fill(.ultraThinMaterial)
                RoundedRectangle(cornerRadius: 24, style: .continuous)
                    .fill(ShellPalette.window.opacity(0.9))
            }
        )
        .overlay {
            RoundedRectangle(cornerRadius: 24, style: .continuous)
                .stroke(ShellPalette.line.opacity(0.42), lineWidth: 1)
        }
        .shadow(color: Color.black.opacity(0.12), radius: 26, y: 16)
        .onAppear {
            isQueryFocused = true
        }
        .onDisappear {
            voiceController.stopListening()
        }
        .onExitCommand {
            voiceController.stopListening()
            isPresented = false
        }
    }

    private func perform(_ action: ShellCommandSurfaceAction) {
        switch action {
        case .newSpace:
            _ = host.createTerminalSpace()
        case .newAlanSpace:
            _ = host.createAlanSpace()
        case .openSurface:
            _ = host.openTerminalSurface()
        case .openAlanSurface:
            _ = host.openAlanSurface()
        case .jumpToAttention:
            if let firstAttention = host.attentionItems.first {
                host.focusAttentionItem(firstAttention)
            }
        case .focusBestPane:
            _ = host.focusTopRoutingCandidate()
        case .splitHorizontal:
            _ = host.splitFocusedPane(direction: .horizontal)
        case .splitVertical:
            _ = host.splitFocusedPane(direction: .vertical)
        case .liftPane:
            _ = host.liftSelectedPaneToSurface()
        case .closePane:
            _ = host.closeSelectedPane()
        case .closeSurface:
            _ = host.closeSelectedSurface()
        case .copySnapshot:
            host.copySnapshotJSON()
        }
        isPresented = false
    }

    private func execute(_ route: ShellCommandSurfaceIntent.Route) {
        switch route {
        case let .action(action):
            perform(action)
        case let .attention(item):
            host.focusAttentionItem(item)
            isPresented = false
        case let .candidate(candidate):
            host.focus(paneID: candidate.paneID)
            isPresented = false
        }
    }

    private func executePrimaryIntent() {
        guard let primaryIntent else { return }
        execute(primaryIntent.route)
    }

    private func routingDetail(for candidate: AlanShellRoutingCandidate) -> String {
        let pane = host.shellState.panes.first(where: { $0.paneID == candidate.paneID })
        let title = pane?.viewport?.title ?? pane?.process?.program ?? candidate.paneID
        let reasons = candidate.reasons.prefix(3).joined(separator: " • ")
        let detail = reasons.isEmpty ? "score \(Int(candidate.score * 100))" : reasons
        return "\(title) • \(detail)"
    }

    private func sectionLabel(_ value: String) -> some View {
        Text(value)
            .font(.system(size: 10, weight: .semibold, design: .rounded))
            .textCase(.uppercase)
            .foregroundStyle(ShellPalette.mutedInk)
    }
}

@MainActor
private final class ShellVoiceCommandController: NSObject, ObservableObject, NSSpeechRecognizerDelegate {
    @Published private(set) var isListening = false

    private let recognizer = NSSpeechRecognizer()
    private var recognitionHandler: ((String) -> Void)?

    override init() {
        super.init()
        recognizer?.delegate = self
        recognizer?.listensInForegroundOnly = false
        recognizer?.blocksOtherRecognizers = false
        recognizer?.commands = [
            "new space",
            "new alan space",
            "open tab",
            "open in alan",
            "focus best pane",
            "route to best pane",
            "split horizontal",
            "split vertical",
            "lift pane",
            "close pane",
            "close tab",
            "jump to attention",
            "focus waiting pane",
            "copy snapshot",
        ]
    }

    func toggleListening(handler: @escaping (String) -> Void) {
        isListening ? stopListening() : startListening(handler: handler)
    }

    func startListening(handler: @escaping (String) -> Void) {
        recognitionHandler = handler
        recognizer?.startListening()
        isListening = recognizer != nil
    }

    func stopListening() {
        recognizer?.stopListening()
        isListening = false
        recognitionHandler = nil
    }

    func speechRecognizer(_ sender: NSSpeechRecognizer, didRecognizeCommand command: String) {
        recognitionHandler?(command)
    }
}

private struct ShellCommandRow: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isHovered = false
    let title: String
    let detail: String
    let accent: Color

    var body: some View {
        HStack(alignment: .center, spacing: 12) {
            Circle()
                .fill(accent)
                .frame(width: 7, height: 7)

            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(ShellPalette.ink)
                Text(detail)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(ShellPalette.mutedInk)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .lineLimit(2)
            }

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 13, style: .continuous)
                .fill(Color.white.opacity(0.84))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 13, style: .continuous)
                .stroke(ShellPalette.line.opacity(isHovered ? 0.24 : 0.12), lineWidth: 1)
        }
        .scaleEffect(isHovered ? 1.004 : 1)
        .shadow(color: isHovered ? Color.black.opacity(0.022) : .clear, radius: 6, y: 3)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isHovered)
        .onHover { isHovered = $0 }
    }
}

private struct ShellToolbarButton: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isHovered = false
    let symbol: String
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(ShellPalette.ink)
                .frame(width: 30, height: 30)
                .background(
                    RoundedRectangle(cornerRadius: 9, style: .continuous)
                        .fill(isHovered ? Color.white.opacity(0.92) : ShellPalette.panel)
                )
                .overlay {
                    RoundedRectangle(cornerRadius: 9, style: .continuous)
                        .stroke(ShellPalette.line.opacity(isHovered ? 0.32 : 0.16), lineWidth: 1)
                }
        }
        .buttonStyle(.plain)
        .scaleEffect(isHovered ? 1.03 : 1)
        .shadow(color: isHovered ? Color.black.opacity(0.032) : .clear, radius: 6, y: 3)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.14), value: isHovered)
        .onHover { isHovered = $0 }
        .help(label)
    }
}

private struct ShellToolbarGlyph: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isHovered = false
    let symbol: String

    var body: some View {
        Image(systemName: symbol)
            .font(.system(size: 13, weight: .semibold))
            .foregroundStyle(ShellPalette.ink)
            .frame(width: 30, height: 30)
            .background(
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .fill(isHovered ? Color.white.opacity(0.92) : ShellPalette.panel)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .stroke(ShellPalette.line.opacity(isHovered ? 0.32 : 0.16), lineWidth: 1)
            }
            .scaleEffect(isHovered ? 1.03 : 1)
            .shadow(color: isHovered ? Color.black.opacity(0.032) : .clear, radius: 6, y: 3)
            .animation(reduceMotion ? nil : .easeOut(duration: 0.14), value: isHovered)
            .onHover { isHovered = $0 }
    }
}

private struct ShellToolbarBadgeButton: View {
    let symbol: String
    let count: Int

    var body: some View {
        ZStack(alignment: .topTrailing) {
            ShellToolbarGlyph(symbol: symbol)

            if count > 1 {
                Text("\(min(count, 9))")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 5)
                    .padding(.vertical, 2)
                    .background(
                        Capsule(style: .continuous)
                            .fill(ShellPalette.attention)
                    )
                    .offset(x: 7, y: -6)
            } else {
                Circle()
                    .fill(ShellPalette.attention)
                    .frame(width: 8, height: 8)
                    .overlay {
                        Circle()
                            .stroke(Color.white.opacity(0.95), lineWidth: 1.5)
                    }
                    .offset(x: 6, y: -5)
            }
        }
    }
}

private struct StatusBadge: View {
    let title: String
    let value: String
    let accent: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .textCase(.uppercase)
                .foregroundStyle(ShellPalette.mutedInk)
            Text(value)
                .font(.system(size: 13, weight: .semibold, design: .rounded))
                .foregroundStyle(accent)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.white.opacity(0.54))
        )
    }
}

private struct InspectorCard<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(title)
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(ShellPalette.ink)
            content
        }
        .padding(15)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 20, style: .continuous)
                .fill(Color.white.opacity(0.78))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 20, style: .continuous)
                .stroke(ShellPalette.line.opacity(0.18), lineWidth: 1)
        }
        .shadow(color: Color.black.opacity(0.03), radius: 8, y: 3)
    }
}

private struct KeyValueRow: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.system(size: 10, weight: .semibold))
                .textCase(.uppercase)
                .foregroundStyle(ShellPalette.mutedInk)
            Text(value)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ShellPalette.ink)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

func shellVisibleLabel(_ raw: String?) -> String? {
    guard let raw else { return nil }
    let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty, trimmed != "/", trimmed != "-" else { return nil }
    return trimmed
}

func shellPathLeaf(_ raw: String?) -> String? {
    guard let visible = shellVisibleLabel(raw) else { return nil }
    if visible == "~" {
        return "Home"
    }

    guard visible.contains("/") else { return nil }
    let components = visible.split(separator: "/").map(String.init)
    return components.last.flatMap(shellVisibleLabel)
}

func shellNormalizedTitle(_ raw: String?) -> String? {
    guard var candidate = shellVisibleLabel(raw) else { return nil }

    for suffix in [" - fish", " - zsh", " - bash", " - sh"] {
        if candidate.lowercased().hasSuffix(suffix) {
            candidate.removeLast(suffix.count)
            break
        }
    }

    candidate = candidate.trimmingCharacters(in: .whitespacesAndNewlines)
    guard let visible = shellVisibleLabel(candidate) else { return nil }

    if let leaf = shellPathLeaf(visible) {
        return leaf
    }

    return visible
}

func shellDisplayTitle(
    rawTitle: String?,
    workingDirectoryName: String?,
    cwd: String?,
    program: String?,
    launchTarget: ShellLaunchTarget,
    fallback: String? = nil
) -> String {
    if let workingDirectoryName = shellVisibleLabel(workingDirectoryName) {
        return workingDirectoryName
    }

    if let cwdLeaf = shellPathLeaf(cwd) {
        return cwdLeaf
    }

    if let normalizedTitle = shellNormalizedTitle(rawTitle) {
        return normalizedTitle
    }

    if let fallback = shellVisibleLabel(fallback) {
        return fallback
    }

    if launchTarget == .alan {
        return "Alan"
    }

    if let program = shellVisibleLabel(program) {
        return program
    }

    return "Terminal"
}

func shellSurfaceDisplayTitle(
    surface: ShellSurface?,
    panes: [ShellPane],
    fallback: String
) -> String {
    guard let surface else { return fallback }

    if let title = shellNormalizedTitle(surface.title) {
        return title
    }

    let preferredPane = panes.first(where: { $0.resolvedLaunchTarget == .alan }) ?? panes.first

    return shellDisplayTitle(
        rawTitle: preferredPane?.viewport?.title,
        workingDirectoryName: preferredPane?.context?.workingDirectoryName,
        cwd: preferredPane?.cwd,
        program: preferredPane?.process?.program,
        launchTarget: preferredPane?.resolvedLaunchTarget ?? .shell,
        fallback: fallback
    )
}
#endif
