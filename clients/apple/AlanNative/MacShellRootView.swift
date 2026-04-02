import SwiftUI

#if os(macOS)
import AppKit

enum ShellPalette {
    static let canvas = Color(red: 0.95, green: 0.94, blue: 0.91)
    static let sidebar = Color(red: 0.16, green: 0.18, blue: 0.20)
    static let sidebarCard = Color(red: 0.21, green: 0.23, blue: 0.26)
    static let workspace = Color(red: 0.90, green: 0.88, blue: 0.84)
    static let terminal = Color(red: 0.10, green: 0.12, blue: 0.15)
    static let terminalSoft = Color(red: 0.14, green: 0.17, blue: 0.20)
    static let accent = Color(red: 0.78, green: 0.46, blue: 0.20)
    static let accentSoft = Color(red: 0.89, green: 0.79, blue: 0.64)
    static let ink = Color(red: 0.18, green: 0.17, blue: 0.15)
    static let mutedInk = Color(red: 0.41, green: 0.39, blue: 0.35)
    static let line = Color(red: 0.79, green: 0.74, blue: 0.67)
}

struct MacShellRootView: View {
    @StateObject private var host: ShellHostController
    @State private var isCommandSurfacePresented = false

    init() {
        _host = StateObject(wrappedValue: ShellHostController.live())
    }

    var body: some View {
        ZStack {
            HStack(spacing: 0) {
                ShellSidebarView(host: host)
                    .frame(width: 276)

                VStack(spacing: 0) {
                    ShellTopBarView(
                        host: host,
                        isCommandSurfacePresented: $isCommandSurfacePresented
                    )
                    Divider()
                        .overlay(ShellPalette.line.opacity(0.55))
                    ShellWorkspaceView(host: host)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(ShellPalette.workspace)

                ShellInspectorView(host: host)
                    .frame(width: 332)
            }
            .background(ShellPalette.canvas)
            .frame(minWidth: 1340, minHeight: 820)

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
        window.title = "Alan Shell"
        window.minSize = NSSize(width: 1180, height: 760)
        window.tabbingMode = .disallowed

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

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 10) {
                Text("Alan Shell")
                    .font(.system(size: 28, weight: .semibold, design: .rounded))
                    .foregroundStyle(.white)
                Text("A real terminal app for humans first, with agents as peers.")
                    .font(.system(size: 13, weight: .medium, design: .rounded))
                    .foregroundStyle(.white.opacity(0.64))
            }

            VStack(spacing: 10) {
                Button(action: { _ = host.createTerminalSpace() }) {
                    HStack(spacing: 10) {
                        Image(systemName: "plus.circle.fill")
                        Text("New Space")
                    }
                    .font(.system(size: 14, weight: .semibold, design: .rounded))
                    .foregroundStyle(ShellPalette.sidebar)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 12)
                    .background(
                        RoundedRectangle(cornerRadius: 16, style: .continuous)
                            .fill(ShellPalette.accentSoft)
                    )
                }
                .buttonStyle(.plain)

                Button(action: { _ = host.createAlanSpace() }) {
                    HStack(spacing: 10) {
                        Image(systemName: "sparkles")
                        Text("New Alan Space")
                    }
                    .font(.system(size: 13, weight: .semibold, design: .rounded))
                    .foregroundStyle(.white.opacity(0.84))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(
                        RoundedRectangle(cornerRadius: 16, style: .continuous)
                            .fill(Color.white.opacity(0.08))
                    )
                }
                .buttonStyle(.plain)
            }

            ShellSidebarSection(title: "Attention") {
                VStack(spacing: 8) {
                    if host.attentionItems.isEmpty {
                        ShellEmptyStateRow(
                            title: "Nothing waiting",
                            detail: "Approvals, notifications, and notable panes will collect here."
                        )
                    } else {
                        ForEach(host.attentionItems) { item in
                            Button {
                                host.select(spaceID: item.spaceID)
                                host.select(surfaceID: item.surfaceID)
                                host.focus(paneID: item.paneID)
                            } label: {
                                ShellAttentionRow(item: item)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }

            ShellSidebarSection(title: "Spaces") {
                VStack(spacing: 10) {
                    ForEach(host.spaces) { space in
                        Button {
                            host.select(spaceID: space.spaceID)
                        } label: {
                            ShellSpaceRow(
                                space: space,
                                isSelected: host.selectedSpace?.spaceID == space.spaceID
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }

            Spacer(minLength: 0)

            VStack(alignment: .leading, spacing: 6) {
                Text("Design direction")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .textCase(.uppercase)
                    .foregroundStyle(.white.opacity(0.42))
                Text("Arc-grade organization with a terminal-native center.")
                    .font(.system(size: 13, weight: .medium, design: .rounded))
                    .foregroundStyle(.white.opacity(0.7))
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .fill(ShellPalette.sidebarCard.opacity(0.92))
            )
        }
        .padding(20)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(ShellPalette.sidebar)
    }
}

private struct ShellWorkspaceView: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(spacing: 18) {
            ShellSurfaceRailView(host: host)

            HSplitView {
                TerminalPaneView(host: host)
                    .frame(minWidth: 720)

                ShellSnapshotPanel(host: host)
                    .frame(minWidth: 300, idealWidth: 360)
            }
        }
        .padding(22)
    }
}

private struct ShellTopBarView: View {
    @ObservedObject var host: ShellHostController
    @Binding var isCommandSurfacePresented: Bool

    var body: some View {
        HStack(spacing: 14) {
            VStack(alignment: .leading, spacing: 3) {
                Text(host.selectedSpace?.title ?? "No Space")
                    .font(.system(size: 26, weight: .semibold, design: .rounded))
                    .foregroundStyle(ShellPalette.ink)
                Text("Focused pane: \(host.shellState.focusedPaneID ?? "none")")
                    .font(.system(size: 13, weight: .medium, design: .rounded))
                    .foregroundStyle(ShellPalette.mutedInk)
            }

            Spacer(minLength: 16)

            Button {
                isCommandSurfacePresented = true
            } label: {
                HStack(spacing: 10) {
                    Image(systemName: "command")
                        .foregroundStyle(ShellPalette.accent)
                    Text("Open command surface")
                        .foregroundStyle(ShellPalette.mutedInk)
                    Spacer(minLength: 0)
                    Text("⌘K")
                        .font(.system(size: 12, weight: .semibold, design: .monospaced))
                        .foregroundStyle(ShellPalette.accent)
                }
                .font(.system(size: 14, weight: .medium, design: .rounded))
                .padding(.horizontal, 16)
                .padding(.vertical, 12)
                .frame(maxWidth: 380)
                .background(
                    Capsule(style: .continuous)
                        .fill(Color.white.opacity(0.68))
                )
            }
            .buttonStyle(.plain)
            .keyboardShortcut("k", modifiers: [.command])

            StatusBadge(title: "Host", value: "macOS", accent: ShellPalette.accent)
            StatusBadge(title: "Mode", value: "Live", accent: ShellPalette.ink)
            StatusBadge(
                title: "Attention",
                value: host.awaitingAttentionCount > 0 ? "\(host.awaitingAttentionCount) waiting" : "quiet",
                accent: host.awaitingAttentionCount > 0 ? ShellPalette.accent : ShellPalette.mutedInk
            )
        }
        .padding(.horizontal, 22)
        .padding(.vertical, 18)
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

private struct ShellSnapshotPanel: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Snapshot")
                    .font(.system(size: 19, weight: .semibold, design: .rounded))
                Spacer(minLength: 0)
                Button("Copy JSON") {
                    host.copySnapshotJSON()
                }
                .buttonStyle(.borderless)
                .foregroundStyle(ShellPalette.accent)
            }

            Text("The canonical shell snapshot is already materialized locally. This becomes the basis for `alan-shell state`.")
                .font(.system(size: 13, weight: .medium, design: .rounded))
                .foregroundStyle(ShellPalette.mutedInk)

            if let lastCopiedAt = host.lastCopiedAt {
                Text("Copied \(lastCopiedAt.formatted(date: .omitted, time: .standard))")
                    .font(.system(size: 12, weight: .medium, design: .rounded))
                    .foregroundStyle(ShellPalette.accent)
            }

            ScrollView {
                Text(host.snapshotJSON)
                    .font(.system(size: 12, weight: .regular, design: .monospaced))
                    .foregroundStyle(ShellPalette.ink)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .textSelection(.enabled)
            }
            .padding(14)
            .background(
                RoundedRectangle(cornerRadius: 22, style: .continuous)
                    .fill(Color.white.opacity(0.64))
            )
        }
        .padding(22)
        .background(
            RoundedRectangle(cornerRadius: 34, style: .continuous)
                .fill(Color.white.opacity(0.38))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 34, style: .continuous)
                .stroke(ShellPalette.line.opacity(0.32), lineWidth: 1)
        }
    }
}

private struct ShellInspectorView: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Inspector")
                .font(.system(size: 26, weight: .semibold, design: .rounded))
                .foregroundStyle(ShellPalette.ink)

            InspectorCard(title: "Focused Pane") {
                VStack(alignment: .leading, spacing: 8) {
                    KeyValueRow(label: "Pane", value: host.focusedPane?.paneID ?? "none")
                    KeyValueRow(label: "Program", value: host.focusedPane?.process?.program ?? "unknown")
                    KeyValueRow(label: "CWD", value: host.focusedPane?.cwd ?? "unknown")
                    KeyValueRow(label: "Attention", value: host.focusedPane?.attention.rawValue ?? "none")
                }
            }

            InspectorCard(title: "Alan Binding") {
                if let binding = host.focusedPane?.alanBinding {
                    VStack(alignment: .leading, spacing: 8) {
                        KeyValueRow(label: "Session", value: binding.sessionID)
                        KeyValueRow(label: "Run", value: binding.runStatus)
                        KeyValueRow(label: "Pending Yield", value: binding.pendingYield ? "true" : "false")
                    }
                } else {
                    Text("No Alan binding projected on the focused pane.")
                        .font(.system(size: 13, weight: .medium, design: .rounded))
                        .foregroundStyle(ShellPalette.mutedInk)
                }
            }

            InspectorCard(title: "Attention Feed") {
                VStack(spacing: 10) {
                    if host.attentionItems.isEmpty {
                        ShellEmptyStateRow(
                            title: "Quiet shell",
                            detail: "When panes need attention they show up here with typed shell identity."
                        )
                    } else {
                        ForEach(host.attentionItems) { item in
                            ShellAttentionRow(item: item)
                        }
                    }
                }
            }

            Spacer(minLength: 0)
        }
        .padding(22)
        .frame(maxHeight: .infinity, alignment: .top)
        .background(Color.white.opacity(0.56))
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(ShellPalette.line.opacity(0.44))
                .frame(width: 1)
        }
    }
}

private struct ShellSidebarSection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.system(size: 11, weight: .semibold, design: .rounded))
                .textCase(.uppercase)
                .foregroundStyle(.white.opacity(0.46))

            content
        }
    }
}

private struct ShellSpaceRow: View {
    let space: ShellSpace
    let isSelected: Bool

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(color(for: space.attention))
                .frame(width: 10, height: 42)

            VStack(alignment: .leading, spacing: 4) {
                Text(space.title)
                    .font(.system(size: 14, weight: .semibold, design: .rounded))
                    .foregroundStyle(.white)
                Text("\(space.surfaces.count) surface\(space.surfaces.count == 1 ? "" : "s")")
                    .font(.system(size: 12, weight: .medium, design: .rounded))
                    .foregroundStyle(.white.opacity(0.6))
            }

            Spacer(minLength: 0)
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .fill(isSelected ? ShellPalette.sidebarCard : Color.white.opacity(0.04))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .stroke(isSelected ? Color.white.opacity(0.16) : Color.clear, lineWidth: 1)
        }
    }

    private func color(for attention: ShellAttentionState) -> Color {
        switch attention {
        case .idle:
            return Color.white.opacity(0.22)
        case .active:
            return Color(red: 0.44, green: 0.62, blue: 0.89)
        case .awaitingUser:
            return ShellPalette.accent
        case .notable:
            return Color(red: 0.89, green: 0.74, blue: 0.42)
        }
    }
}

private struct ShellAttentionRow: View {
    let item: ShellAttentionItem

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(item.title)
                    .font(.system(size: 13, weight: .semibold, design: .rounded))
                Spacer(minLength: 8)
                Text(item.attention.rawValue.replacingOccurrences(of: "_", with: " "))
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .textCase(.uppercase)
                    .foregroundStyle(ShellPalette.accent)
            }
            Text(item.summary)
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.white.opacity(0.86))
        )
    }
}

private struct ShellEmptyStateRow: View {
    let title: String
    let detail: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.system(size: 13, weight: .semibold, design: .rounded))
            Text(detail)
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundStyle(.secondary)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.white.opacity(0.72))
        )
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
            return "Create Terminal Space"
        case .newAlanSpace:
            return "Create Alan Space"
        case .openSurface:
            return "Open Terminal Surface"
        case .openAlanSurface:
            return "Open Alan Surface"
        case .jumpToAttention:
            return "Jump To Attention"
        case .focusBestPane:
            return "Focus Best Routing Pane"
        case .splitHorizontal:
            return "Split Focused Pane Horizontally"
        case .splitVertical:
            return "Split Focused Pane Vertically"
        case .liftPane:
            return "Lift Focused Pane To Surface"
        case .closePane:
            return "Close Focused Pane"
        case .closeSurface:
            return "Close Current Surface"
        case .copySnapshot:
            return "Copy Shell Snapshot"
        }
    }

    var detail: String {
        switch self {
        case .newSpace:
            return "Start a fresh terminal workspace with a plain login shell."
        case .newAlanSpace:
            return "Start a fresh workspace that boots directly into Alan."
        case .openSurface:
            return "Open another terminal surface inside the current space."
        case .openAlanSurface:
            return "Open another surface that boots directly into Alan."
        case .jumpToAttention:
            return "Jump to the strongest pane that currently needs approval or attention."
        case .focusBestPane:
            return "Use shell routing signals to jump to the strongest candidate pane."
        case .splitHorizontal:
            return "Create a stacked split beneath the focused pane."
        case .splitVertical:
            return "Create a side-by-side split next to the focused pane."
        case .liftPane:
            return "Move the focused pane into its own surface without losing shell identity."
        case .closePane:
            return "Close the focused pane and keep the remaining surface topology intact."
        case .closeSurface:
            return "Close the current surface while preserving the rest of the space."
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
            return ["new alan space", "alan space", "agent space"]
        case .openSurface:
            return ["open surface", "new surface", "new tab"]
        case .openAlanSurface:
            return ["open alan surface", "new alan surface", "alan tab"]
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
        ShellCommandSurfaceAction.allCases.filter { $0.matches(query: query) }
    }

    private var matchingAttention: [ShellAttentionItem] {
        guard !query.isEmpty else { return host.attentionItems }
        return host.attentionItems.filter {
            $0.title.localizedCaseInsensitiveContains(query)
                || $0.summary.localizedCaseInsensitiveContains(query)
        }
    }

    private var matchingRoutingCandidates: [AlanShellRoutingCandidate] {
        let candidates = host.routingCandidates
        guard !query.isEmpty else { return Array(candidates.prefix(4)) }

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
        VStack(alignment: .leading, spacing: 18) {
            HStack(alignment: .top, spacing: 12) {
                VStack(alignment: .leading, spacing: 5) {
                    Text("Command Surface")
                        .font(.system(size: 26, weight: .semibold, design: .rounded))
                        .foregroundStyle(ShellPalette.ink)
                    Text("Type an intent, route it to shell structure, then execute without leaving the terminal.")
                        .font(.system(size: 14, weight: .medium, design: .rounded))
                        .foregroundStyle(ShellPalette.mutedInk)
                }

                Spacer(minLength: 0)

                Button("Close") {
                    isPresented = false
                }
                .buttonStyle(.plain)
                .font(.system(size: 13, weight: .semibold, design: .rounded))
                .foregroundStyle(ShellPalette.accent)
            }

            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(ShellPalette.accent)
                TextField("Try “split vertical”, “lift pane”, or “jump to waiting pane”", text: $query)
                    .textFieldStyle(.plain)
                    .font(.system(size: 15, weight: .medium, design: .rounded))
                    .foregroundStyle(ShellPalette.ink)
                    .focused($isQueryFocused)
                    .onSubmit {
                        executePrimaryIntent()
                    }
                Spacer(minLength: 0)
                Button {
                    voiceController.toggleListening { recognizedCommand in
                        query = recognizedCommand
                        executePrimaryIntent()
                    }
                } label: {
                    Image(systemName: voiceController.isListening ? "mic.fill" : "mic")
                        .foregroundStyle(voiceController.isListening ? ShellPalette.accent : ShellPalette.mutedInk)
                }
                .buttonStyle(.plain)
                Text("Esc")
                    .font(.system(size: 11, weight: .semibold, design: .monospaced))
                    .foregroundStyle(ShellPalette.mutedInk)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .fill(Color.white.opacity(0.86))
            )
            .overlay {
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .stroke(ShellPalette.line.opacity(0.3), lineWidth: 1)
            }

            if voiceController.isListening || primaryIntent != nil {
                VStack(alignment: .leading, spacing: 12) {
                    if voiceController.isListening {
                        HStack(spacing: 8) {
                            Circle()
                                .fill(ShellPalette.accent)
                                .frame(width: 8, height: 8)
                            Text("Listening for shell intents like “new space”, “split vertical”, or “jump to attention”.")
                                .font(.system(size: 13, weight: .medium, design: .rounded))
                                .foregroundStyle(ShellPalette.mutedInk)
                        }
                    }

                    if let primaryIntent {
                        VStack(alignment: .leading, spacing: 10) {
                            sectionLabel("Intent")
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

            VStack(alignment: .leading, spacing: 12) {
                sectionLabel("Actions")
                VStack(spacing: 10) {
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

            if !matchingRoutingCandidates.isEmpty {
                VStack(alignment: .leading, spacing: 12) {
                    sectionLabel("Routing")
                    VStack(spacing: 10) {
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

            VStack(alignment: .leading, spacing: 12) {
                sectionLabel("Attention")
                VStack(spacing: 10) {
                    if matchingAttention.isEmpty {
                        ShellEmptyStateRow(
                            title: "Nothing urgent",
                            detail: "When a pane needs approvals or becomes notable, it shows up here."
                        )
                    } else {
                        ForEach(matchingAttention) { item in
                            Button {
                                host.focusAttentionItem(item)
                                isPresented = false
                            } label: {
                                ShellCommandRow(
                                    title: item.title,
                                    detail: item.summary,
                                    accent: item.attention == .awaitingUser ? ShellPalette.accent : ShellPalette.ink
                                )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
        }
        .padding(24)
        .background(
            RoundedRectangle(cornerRadius: 30, style: .continuous)
                .fill(Color(red: 0.97, green: 0.95, blue: 0.91))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 30, style: .continuous)
                .stroke(ShellPalette.line.opacity(0.4), lineWidth: 1)
        }
        .shadow(color: Color.black.opacity(0.16), radius: 28, y: 18)
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
            .font(.system(size: 11, weight: .semibold, design: .rounded))
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
            "open surface",
            "open alan surface",
            "focus best pane",
            "route to best pane",
            "split horizontal",
            "split vertical",
            "lift pane",
            "close pane",
            "close surface",
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
    let title: String
    let detail: String
    let accent: Color

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(accent.opacity(0.16))
                .frame(width: 10, height: 44)

            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.system(size: 14, weight: .semibold, design: .rounded))
                    .foregroundStyle(ShellPalette.ink)
                Text(detail)
                    .font(.system(size: 12, weight: .medium, design: .rounded))
                    .foregroundStyle(ShellPalette.mutedInk)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }

            Spacer(minLength: 0)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 20, style: .continuous)
                .fill(Color.white.opacity(0.84))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 20, style: .continuous)
                .stroke(ShellPalette.line.opacity(0.24), lineWidth: 1)
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
        VStack(alignment: .leading, spacing: 12) {
            Text(title)
                .font(.system(size: 16, weight: .semibold, design: .rounded))
                .foregroundStyle(ShellPalette.ink)
            content
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 22, style: .continuous)
                .fill(Color.white.opacity(0.78))
        )
    }
}

private struct KeyValueRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(label)
                .font(.system(size: 12, weight: .semibold, design: .rounded))
                .foregroundStyle(ShellPalette.mutedInk)
            Spacer(minLength: 10)
            Text(value)
                .font(.system(size: 12, weight: .medium, design: .monospaced))
                .foregroundStyle(ShellPalette.ink)
                .multilineTextAlignment(.trailing)
        }
    }
}
#endif
