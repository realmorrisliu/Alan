import SwiftUI

#if os(macOS)
struct TerminalPaneView: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            paneCanvas
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)

            if showsMetadataStrip {
                paneMetadataStrip
            }

        }
        .padding(.top, 8)
        .padding(.trailing, 8)
        .padding(.bottom, 8)
    }

    private var paneMetadataStrip: some View {
        let selectedPane = host.selectedPane
        let selectedPaneTitle = selectedPane.map {
            shellDisplayTitle(
                rawTitle: $0.viewport?.title,
                workingDirectoryName: $0.context?.workingDirectoryName,
                cwd: $0.cwd,
                program: $0.process?.program,
                launchTarget: $0.resolvedLaunchTarget
            )
        }

        return ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                if let selectedPane,
                   let status = shellTerminalStatusSummary(for: selectedPane)
                {
                    compactMetaChip(
                        title: status,
                        icon: statusIcon(for: selectedPane),
                        tint: statusTint(for: selectedPane)
                    )
                }

                if let workingDirectory = shellVisibleLabel(
                    selectedPane?.context?.workingDirectoryName
                        ?? selectedPane?.cwd
                )
                   , workingDirectory != selectedPaneTitle
                {
                    compactMetaChip(title: workingDirectory, icon: "folder")
                }

                if let branch = selectedPane?.context?.gitBranch {
                    compactMetaChip(title: branch, icon: "point.topleft.down.curvedto.point.bottomright.up")
                }

                if let attention = selectedPane?.attention,
                   attention == .awaitingUser || attention == .notable
                {
                    compactMetaChip(
                        title: attention.rawValue.replacingOccurrences(of: "_", with: " "),
                        icon: "bell.badge",
                        tint: attention == .awaitingUser ? ShellPalette.attention : ShellPalette.mutedInk
                    )
                }

                if let binding = selectedPane?.alanBinding {
                    compactMetaChip(
                        title: "Alan \(binding.runStatus)",
                        icon: "sparkles",
                        tint: ShellPalette.accent
                    )
                }
            }
        }
    }

    private var showsMetadataStrip: Bool {
        let selectedPane = host.selectedPane
        let selectedPaneTitle = selectedPane.map {
            shellDisplayTitle(
                rawTitle: $0.viewport?.title,
                workingDirectoryName: $0.context?.workingDirectoryName,
                cwd: $0.cwd,
                program: $0.process?.program,
                launchTarget: $0.resolvedLaunchTarget
            )
        }

        let workingDirectory = shellVisibleLabel(
            selectedPane?.context?.workingDirectoryName
                ?? selectedPane?.cwd
        )

        return selectedPane.flatMap(shellTerminalStatusSummary) != nil
            || (workingDirectory != nil && workingDirectory != selectedPaneTitle)
            || selectedPane?.context?.gitBranch != nil
            || selectedPane?.attention == .awaitingUser
            || selectedPane?.attention == .notable
            || selectedPane?.alanBinding != nil
    }

    private func compactMetaChip(title: String, icon: String, tint: Color? = nil) -> some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
            Text(title)
                .lineLimit(1)
        }
        .font(.system(size: 11, weight: .semibold))
        .foregroundStyle(tint ?? ShellPalette.mutedInk)
        .padding(.horizontal, 9)
        .padding(.vertical, 6)
        .background(
            Capsule(style: .continuous)
                .fill(Color.white.opacity(0.62))
        )
        .overlay {
            Capsule(style: .continuous)
                .stroke(ShellPalette.line.opacity(0.22), lineWidth: 1)
        }
    }

    private func statusIcon(for pane: ShellPane) -> String {
        if pane.context?.processState == "exited"
            || pane.context?.surfaceReadiness == "child_exited"
        {
            return "checkmark.circle"
        }
        if pane.context?.rendererHealth == "failed"
            || pane.context?.rendererPhase == "failed"
            || pane.context?.surfaceReadiness == "renderer_failed"
        {
            return "exclamationmark.triangle"
        }
        if pane.attention == .awaitingUser || pane.attention == .notable {
            return "bell.badge"
        }
        return "info.circle"
    }

    private func statusTint(for pane: ShellPane) -> Color {
        if pane.context?.rendererHealth == "failed"
            || pane.context?.rendererPhase == "failed"
            || pane.context?.surfaceReadiness == "renderer_failed"
            || pane.attention == .awaitingUser
        {
            return ShellPalette.attention
        }
        if pane.attention == .notable {
            return ShellPalette.mutedInk
        }
        return ShellPalette.mutedInk
    }

    private var paneCanvas: some View {
        Group {
            if let paneTree = host.selectedTabPaneTree {
                ShellPaneTreeLayoutView(
                    node: paneTree,
                    host: host
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            } else {
                VStack(alignment: .leading, spacing: 5) {
                    Text("No tab selected")
                        .font(.system(size: 18, weight: .semibold))
                        .foregroundStyle(ShellPalette.ink)
                    Text("Open a tab to start a new shell here.")
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(ShellPalette.mutedInk)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                .padding(28)
            }
        }
        .modifier(ShellTerminalSurfaceFrame())
    }

    private var runtimeCard: some View {
        let runtime = host.selectedPaneRuntime
        return TerminalInfoCard(title: "Host Runtime", accent: ShellPalette.accent) {
            TerminalInfoRow(label: "Stage", value: runtime.stageLabel)
            TerminalInfoRow(label: "Focus", value: runtime.isFocused ? "focused" : "background")
            TerminalInfoRow(label: "Renderer", value: rendererKindLabel(for: runtime.renderer.kind))
            TerminalInfoRow(label: "Phase", value: runtime.renderer.phaseLabel)
            TerminalInfoRow(
                label: "Logical",
                value: sizeLabel(for: runtime.logicalSize)
            )
            TerminalInfoRow(
                label: "Backing",
                value: sizeLabel(for: runtime.backingSize)
            )
            TerminalInfoRow(
                label: "Display",
                value: runtime.displayName ?? "not attached"
            )
            TerminalInfoRow(
                label: "Display ID",
                value: host.selectedPane?.context?.displayID ?? runtime.displayID ?? "pending"
            )
            TerminalInfoRow(
                label: "Window",
                value: host.selectedPane?.context?.windowTitle ?? runtime.attachedWindowTitle ?? "pending"
            )
            TerminalInfoRow(
                label: "Status",
                value: runtime.renderer.summary
            )
            TerminalInfoRow(
                label: "Surface",
                value: surfaceReadinessLabel(runtime.surfaceState.readiness)
            )
            TerminalInfoRow(
                label: "Input",
                value: runtime.surfaceState.inputReady ? "ready" : "not ready"
            )
            TerminalInfoRow(
                label: "Mode",
                value: runtime.surfaceState.terminalMode.rawValue.replacingOccurrences(of: "_", with: " ")
            )
            TerminalInfoRow(
                label: "Renderer Health",
                value: runtime.surfaceState.rendererHealth
            )
            TerminalInfoRow(
                label: "cwd",
                value: runtime.paneMetadata.workingDirectory ?? "pending"
            )
            TerminalInfoRow(
                label: "Title",
                value: runtime.paneMetadata.title ?? "pending"
            )
            TerminalInfoRow(
                label: "Attention",
                value: runtime.paneMetadata.attention.rawValue
            )
            TerminalInfoRow(
                label: "Process",
                value: runtime.paneMetadata.processExited ? "exited" : "running"
            )
            TerminalInfoRow(
                label: "Branch",
                value: host.selectedPane?.context?.gitBranch ?? "pending"
            )
            TerminalInfoRow(
                label: "Process State",
                value: host.selectedPane?.context?.processState ?? "pending"
            )
            TerminalInfoRow(
                label: "Exit",
                value: host.selectedPane?.context?.lastCommandExitCode.map(String.init) ?? "pending"
            )
            TerminalInfoRow(
                label: "Metadata",
                value: host.selectedPane?.context?.lastMetadataAt ?? "pending"
            )

            if let failureReason = runtime.renderer.failureReason,
               !failureReason.isEmpty {
                Divider()
                    .overlay(ShellPalette.line.opacity(0.22))

                TerminalInfoRow(label: "Failure", value: failureReason)
            }

            if !runtime.renderer.recentEvents.isEmpty {
                Divider()
                    .overlay(ShellPalette.line.opacity(0.22))

                VStack(alignment: .leading, spacing: 8) {
                    Text("Recent Events")
                        .font(.system(size: 11, weight: .semibold, design: .rounded))
                        .textCase(.uppercase)
                        .foregroundStyle(ShellPalette.mutedInk)

                    ForEach(Array(runtime.renderer.recentEvents.enumerated()), id: \.offset) { entry in
                        TerminalMonoLine(text: entry.element)
                    }
                }
            }
        }
    }

    private var bootCard: some View {
        TerminalInfoCard(title: "Boot Contract", accent: ShellPalette.ink) {
            TerminalInfoRow(
                label: "Target",
                value: host.selectedPane?.resolvedLaunchTarget.rawValue ?? "pending"
            )
            TerminalInfoRow(
                label: "Strategy",
                value: host.selectedPaneBootProfile?.command.strategy.rawValue ?? "pending"
            )
            TerminalInfoRow(
                label: "Command",
                value: host.selectedPaneBootProfile?.launchCommandString ?? "pending"
            )
            TerminalInfoRow(
                label: "Resolved",
                value: host.selectedPaneBootProfile?.command.detail ?? "PATH lookup"
            )
            TerminalInfoRow(
                label: "cwd",
                value: host.selectedPaneBootProfile?.workingDirectory ?? "pending"
            )
            TerminalInfoRow(
                label: "Control",
                value: host.selectedPaneBootProfile?.environment["ALAN_SHELL_CONTROL_DIR"] ?? "pending"
            )
            TerminalInfoRow(
                label: "Socket",
                value: host.selectedPaneBootProfile?.environment["ALAN_SHELL_SOCKET"] ?? "pending"
            )
            TerminalInfoRow(
                label: "Binding",
                value: host.selectedPaneBootProfile?.environment["ALAN_SHELL_BINDING_FILE"] ?? "pending"
            )
            TerminalInfoRow(
                label: "Integration",
                value: host.selectedPane?.context?.shellIntegrationSource ?? "pending"
            )

            if let bootProfile = host.selectedPaneBootProfile {
                Divider()
                    .overlay(ShellPalette.line.opacity(0.22))

                VStack(alignment: .leading, spacing: 8) {
                    Text("Environment")
                        .font(.system(size: 11, weight: .semibold, design: .rounded))
                        .textCase(.uppercase)
                        .foregroundStyle(ShellPalette.mutedInk)

                    ForEach(Array(bootProfile.environmentPreview.prefix(4)), id: \.key) { entry in
                        TerminalMonoLine(text: "\(entry.key)=\(entry.value)")
                    }
                }

                Divider()
                    .overlay(ShellPalette.line.opacity(0.22))

                VStack(alignment: .leading, spacing: 8) {
                    Text("Command Discovery")
                        .font(.system(size: 11, weight: .semibold, design: .rounded))
                        .textCase(.uppercase)
                        .foregroundStyle(ShellPalette.mutedInk)

                    ForEach(Array(bootProfile.command.candidates.prefix(4))) { candidate in
                        HStack(alignment: .top, spacing: 8) {
                            Circle()
                                .fill(candidate.isPresent ? Color.green.opacity(0.9) : Color.orange.opacity(0.82))
                                .frame(width: 8, height: 8)
                                .padding(.top, 4)

                            VStack(alignment: .leading, spacing: 3) {
                                Text(candidate.label)
                                    .font(.system(size: 12, weight: .semibold, design: .rounded))
                                    .foregroundStyle(ShellPalette.ink)
                                Text(candidate.path)
                                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                                    .foregroundStyle(ShellPalette.mutedInk)
                                    .lineLimit(2)
                            }
                        }
                    }
                }
            }
        }
    }

    private var ghosttyCard: some View {
        TerminalInfoCard(title: "Ghostty", accent: ShellPalette.accent) {
            TerminalInfoRow(
                label: "Status",
                value: host.selectedPaneBootProfile?.ghostty.summary ?? "pending"
            )
            TerminalInfoRow(
                label: "Setup",
                value: host.selectedPaneBootProfile?.ghostty.setupCommand ?? "pending"
            )

            if let candidates = host.selectedPaneBootProfile?.ghostty.candidates {
                Divider()
                    .overlay(ShellPalette.line.opacity(0.22))

                VStack(alignment: .leading, spacing: 8) {
                    Text("Discovery")
                        .font(.system(size: 11, weight: .semibold, design: .rounded))
                        .textCase(.uppercase)
                        .foregroundStyle(ShellPalette.mutedInk)

                    ForEach(Array(candidates.prefix(3))) { candidate in
                        HStack(alignment: .top, spacing: 8) {
                            Circle()
                                .fill(candidate.isPresent ? Color.green.opacity(0.9) : Color.orange.opacity(0.82))
                                .frame(width: 8, height: 8)
                                .padding(.top, 4)

                            VStack(alignment: .leading, spacing: 3) {
                                Text(candidate.label)
                                    .font(.system(size: 12, weight: .semibold, design: .rounded))
                                    .foregroundStyle(ShellPalette.ink)
                                Text(candidate.path)
                                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                                    .foregroundStyle(ShellPalette.mutedInk)
                                    .lineLimit(2)
                            }
                        }
                    }
                }
            }
        }
    }

    private var alanBindingCard: some View {
        TerminalInfoCard(title: "Alan Binding", accent: ShellPalette.ink) {
            if let binding = host.selectedPane?.alanBinding {
                TerminalInfoRow(label: "Session", value: binding.sessionID)
                TerminalInfoRow(label: "Run", value: binding.runStatus)
                TerminalInfoRow(label: "Yield", value: binding.pendingYield ? "pending" : "none")
                TerminalInfoRow(label: "Source", value: binding.source ?? "binding file")
                TerminalInfoRow(label: "Projected", value: binding.lastProjectedAt ?? "pending")
            } else {
                Text("This pane is shell-addressable even when no Alan session is projected onto it.")
                    .font(.system(size: 13, weight: .medium, design: .rounded))
                    .foregroundStyle(ShellPalette.mutedInk)
            }
        }
    }

    private func sizeLabel(for size: CGSize) -> String {
        guard size != .zero else { return "pending" }
        return "\(Int(size.width)) × \(Int(size.height))"
    }

    private func rendererKindLabel(for kind: TerminalRendererKind) -> String {
        kind.rawValue.replacingOccurrences(of: "_", with: " ")
    }

    private func surfaceReadinessLabel(_ readiness: AlanTerminalSurfaceReadiness) -> String {
        switch readiness {
        case .ready:
            return "ready"
        case .unready(let reason):
            return reason.rawValue.replacingOccurrences(of: "_", with: " ")
        }
    }
}

private struct ShellTerminalSurfaceFrame: ViewModifier {
    private let shape = RoundedRectangle(cornerRadius: 12, style: .continuous)

    func body(content: Content) -> some View {
        content
            .background(ShellPalette.terminal)
            .clipShape(shape)
            .overlay {
                shape.stroke(
                    ShellPalette.line.opacity(0.18),
                    lineWidth: 1
                )
            }
            .shadow(
                color: Color.black.opacity(0.10),
                radius: 18,
                y: 8
            )
    }
}

private struct ShellPaneTreeLayoutView: View {
    let node: ShellPaneTreeNode
    @ObservedObject var host: ShellHostController

    var body: some View {
        switch node.kind {
        case .pane:
            if let paneID = node.paneID,
               let pane = host.shellState.panes.first(where: { $0.paneID == paneID }) {
                ShellTerminalLeafView(
                    pane: pane,
                    bootProfile: host.bootProfile(for: pane),
                    isSelected: host.selectedPane?.paneID == pane.paneID,
                    runtimeRegistry: host.terminalRuntimeRegistry,
                    activationDelegate: host,
                    onWorkspaceCommand: { command in
                        host.performShellWorkspaceCommand(command)
                    },
                    onRuntimeUpdate: host.updateTerminalRuntime,
                    onMetadataUpdate: { metadata in
                        host.updateTerminalMetadata(metadata, for: pane.paneID)
                    }
                )
            }
        case .split:
            ShellSplitLayoutView(node: node, host: host)
        }
    }
}

private struct ShellSplitLayoutView: View {
    let node: ShellPaneTreeNode
    @ObservedObject var host: ShellHostController
    @State private var dragStartRatio: Double?
    @State private var dragPreviewRatio: Double?

    private var children: [ShellPaneTreeNode] {
        node.children ?? []
    }

    var body: some View {
        if children.count == 2 {
            GeometryReader { proxy in
                if node.direction == .vertical {
                    HStack(spacing: 0) {
                        ShellPaneTreeLayoutView(node: children[0], host: host)
                            .frame(width: primaryLength(total: proxy.size.width))
                        ShellSplitDividerView(direction: .vertical)
                            .gesture(resizeGesture(totalLength: proxy.size.width))
                        ShellPaneTreeLayoutView(node: children[1], host: host)
                            .frame(width: secondaryLength(total: proxy.size.width))
                    }
                } else {
                    VStack(spacing: 0) {
                        ShellPaneTreeLayoutView(node: children[0], host: host)
                            .frame(height: primaryLength(total: proxy.size.height))
                        ShellSplitDividerView(direction: .horizontal)
                            .gesture(resizeGesture(totalLength: proxy.size.height))
                        ShellPaneTreeLayoutView(node: children[1], host: host)
                            .frame(height: secondaryLength(total: proxy.size.height))
                    }
                }
            }
        } else if node.direction == .vertical {
            HStack(spacing: 0) {
                indexedChildrenWithDividers
            }
        } else {
            VStack(spacing: 0) {
                indexedChildrenWithDividers
            }
        }
    }

    @ViewBuilder
    private var indexedChildrenWithDividers: some View {
        ForEach(Array(children.enumerated()), id: \.element.id) { index, child in
            if index > 0 {
                ShellSplitDividerView(direction: node.direction ?? .vertical)
            }
            ShellPaneTreeLayoutView(node: child, host: host)
        }
    }

    private var dividerThickness: CGFloat { ShellSplitDividerMetrics.thickness }

    private func primaryLength(total: CGFloat) -> CGFloat {
        max((total - dividerThickness) * node.splitRatio, 0)
    }

    private func secondaryLength(total: CGFloat) -> CGFloat {
        max(total - dividerThickness - primaryLength(total: total), 0)
    }

    private func resizeGesture(totalLength: CGFloat) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                if dragStartRatio == nil {
                    dragStartRatio = node.splitRatio
                }
                let delta = node.direction == .vertical
                    ? value.translation.width
                    : value.translation.height
                let usableLength = max(totalLength - dividerThickness, 1)
                let nextRatio = (dragStartRatio ?? node.splitRatio) + Double(delta / usableLength)
                if host.resizeSplit(splitNodeID: node.nodeID, ratio: nextRatio, persist: false) {
                    dragPreviewRatio = nextRatio
                }
            }
            .onEnded { _ in
                if let finalRatio = dragPreviewRatio {
                    _ = host.resizeSplit(splitNodeID: node.nodeID, ratio: finalRatio, persist: true)
                }
                dragStartRatio = nil
                dragPreviewRatio = nil
            }
    }
}

private struct ShellSplitDividerView: View {
    @State private var isHovered = false
    let direction: ShellSplitDirection

    var body: some View {
        seam
            .frame(
                width: direction == .vertical ? ShellSplitDividerMetrics.thickness : nil,
                height: direction == .horizontal ? ShellSplitDividerMetrics.thickness : nil
            )
            .contentShape(Rectangle())
            .onHover { isHovered = $0 }
            .help("Resize split")
    }

    @ViewBuilder
    private var seam: some View {
        if direction == .vertical {
            HStack(spacing: 0) {
                Rectangle().fill(ShellSplitDividerTint.shadow(isHovered: isHovered))
                Rectangle().fill(ShellSplitDividerTint.highlight(isHovered: isHovered))
            }
        } else {
            VStack(spacing: 0) {
                Rectangle().fill(ShellSplitDividerTint.shadow(isHovered: isHovered))
                Rectangle().fill(ShellSplitDividerTint.highlight(isHovered: isHovered))
            }
        }
    }
}

private enum ShellSplitDividerMetrics {
    static let thickness: CGFloat = 2
}

private enum ShellSplitDividerTint {
    static func shadow(isHovered: Bool) -> Color {
        Color.black.opacity(isHovered ? 0.22 : 0.14)
    }

    static func highlight(isHovered: Bool) -> Color {
        ShellPalette.terminalSoft.opacity(isHovered ? 0.48 : 0.34)
    }
}

private struct ShellTerminalLeafView: View {
    @AppStorage("alanShellDimsInactiveSplitPanes") private var dimsInactiveSplitPanes = true

    let pane: ShellPane
    let bootProfile: AlanShellBootProfile?
    let isSelected: Bool
    let runtimeRegistry: TerminalRuntimeRegistry
    let activationDelegate: TerminalHostActivationDelegate?
    let onWorkspaceCommand: (ShellWorkspaceCommand) -> Void
    let onRuntimeUpdate: (TerminalHostRuntimeSnapshot) -> Void
    let onMetadataUpdate: (TerminalPaneMetadataSnapshot) -> Void

    var body: some View {
        TerminalHostView(
            pane: pane,
            bootProfile: bootProfile,
            isSelected: isSelected,
            runtimeRegistry: runtimeRegistry,
            activationDelegate: activationDelegate,
            onWorkspaceCommand: onWorkspaceCommand,
            onRuntimeUpdate: onRuntimeUpdate,
            onMetadataUpdate: onMetadataUpdate
        )
        .id(pane.paneID)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(ShellPalette.terminal)
        .overlay {
            ShellInactivePaneDim(
                isSelected: isSelected,
                isEnabled: dimsInactiveSplitPanes
            )
        }
    }
}

private struct ShellInactivePaneDim: View {
    let isSelected: Bool
    let isEnabled: Bool

    var body: some View {
        Rectangle()
            .fill(Color.black.opacity(isSelected || !isEnabled ? 0 : 0.14))
            .allowsHitTesting(false)
    }
}

private struct TerminalActionButton: View {
    let icon: String
    let title: String
    var isDestructive = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            TerminalActionLabel(
                icon: icon,
                title: title,
                foreground: isDestructive ? Color.red.opacity(0.8) : ShellPalette.ink
            )
        }
        .buttonStyle(.plain)
    }
}

private struct TerminalActionLabel: View {
    let icon: String
    let title: String
    var foreground: Color = ShellPalette.ink

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
            Text(title)
        }
        .font(.system(size: 12, weight: .semibold))
        .foregroundStyle(foreground)
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(
            Capsule(style: .continuous)
                .fill(Color.white.opacity(0.8))
        )
    }
}

private struct TerminalPaneChip: View {
    let icon: String
    let title: String
    let foreground: Color
    let background: Color

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
            Text(title)
        }
        .font(.system(size: 11, weight: .semibold))
        .foregroundStyle(foreground)
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(
            Capsule(style: .continuous)
                .fill(background)
        )
    }
}

private struct TerminalInfoCard<Content: View>: View {
    let title: String
    let accent: Color
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title)
                .font(.system(size: 14, weight: .semibold, design: .rounded))
                .foregroundStyle(ShellPalette.ink)

            content
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 22, style: .continuous)
                .fill(Color.white.opacity(0.76))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 22, style: .continuous)
                .stroke(accent.opacity(0.16), lineWidth: 1)
        }
    }
}

private struct TerminalInfoRow: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.system(size: 11, weight: .semibold, design: .rounded))
                .textCase(.uppercase)
                .foregroundStyle(ShellPalette.mutedInk)
            Text(value)
                .font(.system(size: 13, weight: .medium, design: .rounded))
                .foregroundStyle(ShellPalette.ink)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

private struct TerminalMonoLine: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.system(size: 11, weight: .medium, design: .monospaced))
            .foregroundStyle(ShellPalette.mutedInk)
            .lineLimit(2)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(ShellPalette.canvas.opacity(0.78))
            )
    }
}

#endif
