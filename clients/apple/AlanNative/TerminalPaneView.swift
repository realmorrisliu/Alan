import SwiftUI

#if os(macOS)
struct TerminalPaneView: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        HStack(alignment: .top, spacing: 18) {
            VStack(alignment: .leading, spacing: 16) {
                header
                paneCanvas
                paneSelectorStrip
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            VStack(spacing: 14) {
                runtimeCard
                bootCard
                ghosttyCard
                alanBindingCard
            }
            .frame(width: 308)
        }
        .padding(22)
        .background(
            RoundedRectangle(cornerRadius: 34, style: .continuous)
                .fill(Color.white.opacity(0.28))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 34, style: .continuous)
                .stroke(ShellPalette.line.opacity(0.3), lineWidth: 1)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 5) {
                    Text(host.selectedSurface?.title ?? "Terminal Surface")
                        .font(.system(size: 24, weight: .semibold, design: .rounded))
                        .foregroundStyle(ShellPalette.ink)
                    Text(host.selectedPane?.viewport?.summary ?? "Terminal surface with optional agent overlays.")
                        .font(.system(size: 14, weight: .medium, design: .rounded))
                        .foregroundStyle(ShellPalette.mutedInk)
                }

                Spacer(minLength: 16)

                HStack(spacing: 10) {
                    TerminalPaneChip(
                        icon: "rectangle.split.2x1",
                        title: host.selectedPane?.paneID ?? "no pane",
                        foreground: ShellPalette.ink,
                        background: Color.white.opacity(0.8)
                    )
                    TerminalPaneChip(
                        icon: "wave.3.right",
                        title: host.selectedPaneRuntime.stageLabel,
                        foreground: ShellPalette.accent,
                        background: ShellPalette.accentSoft.opacity(0.5)
                    )
                }
            }

            HStack(spacing: 10) {
                TerminalActionButton(
                    icon: "square.split.2x1",
                    title: "Split Horizontal",
                    action: { _ = host.splitFocusedPane(direction: .horizontal) }
                )
                TerminalActionButton(
                    icon: "rectangle.split.2x1",
                    title: "Split Vertical",
                    action: { _ = host.splitFocusedPane(direction: .vertical) }
                )
                TerminalActionButton(
                    icon: "plus",
                    title: "New Surface",
                    action: { _ = host.openTerminalSurface() }
                )
                TerminalActionButton(
                    icon: "sparkles.rectangle.stack",
                    title: "Open Alan",
                    action: { _ = host.openAlanSurface() }
                )
                if !host.moveDestinationSurfaces.isEmpty {
                    Menu {
                        ForEach(host.moveDestinationSurfaces) { surface in
                            Button(surface.title ?? surface.surfaceID) {
                                _ = host.moveSelectedPane(toSurface: surface.surfaceID)
                            }
                        }
                    } label: {
                        TerminalActionLabel(
                            icon: "arrowshape.turn.up.right",
                            title: "Move Pane"
                        )
                    }
                    .menuStyle(.borderlessButton)
                    .fixedSize()
                }
                TerminalActionButton(
                    icon: "arrow.up.right.square",
                    title: "Lift Pane",
                    action: { _ = host.liftSelectedPaneToSurface() }
                )
                TerminalActionButton(
                    icon: "xmark.square",
                    title: "Close Pane",
                    isDestructive: true,
                    action: { _ = host.closeSelectedPane() }
                )
                TerminalActionButton(
                    icon: "xmark",
                    title: "Close Surface",
                    isDestructive: true,
                    action: { _ = host.closeSelectedSurface() }
                )
            }
        }
    }

    private var paneCanvas: some View {
        Group {
            if let paneTree = host.selectedSurfacePaneTree {
                ShellPaneTreeLayoutView(
                    node: paneTree,
                    host: host
                )
                .frame(maxWidth: .infinity, minHeight: 470, maxHeight: .infinity, alignment: .topLeading)
            } else {
                VStack(alignment: .leading, spacing: 10) {
                    Text("No surface selected")
                        .font(.system(size: 18, weight: .semibold, design: .rounded))
                        .foregroundStyle(ShellPalette.ink)
                    Text("Open a terminal surface to materialize a pane tree and boot a terminal host.")
                        .font(.system(size: 14, weight: .medium, design: .rounded))
                        .foregroundStyle(ShellPalette.mutedInk)
                }
                .frame(maxWidth: .infinity, minHeight: 470, alignment: .leading)
                .padding(28)
                .background(
                    RoundedRectangle(cornerRadius: 28, style: .continuous)
                        .fill(Color.white.opacity(0.5))
                )
            }
        }
    }

    private var paneSelectorStrip: some View {
        HStack(spacing: 12) {
            ForEach(host.panesForSelectedSurface) { pane in
                TerminalPaneSelectorButton(
                    pane: pane,
                    isFocused: host.selectedPane?.paneID == pane.paneID,
                    onSelect: { host.focus(paneID: pane.paneID) }
                )
            }
        }
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
                label: "cwd",
                value: runtime.surfaceMetadata.workingDirectory ?? "pending"
            )
            TerminalInfoRow(
                label: "Title",
                value: runtime.surfaceMetadata.title ?? "pending"
            )
            TerminalInfoRow(
                label: "Attention",
                value: runtime.surfaceMetadata.attention.rawValue
            )
            TerminalInfoRow(
                label: "Process",
                value: runtime.surfaceMetadata.processExited ? "exited" : "running"
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
                    runtime: host.runtime(for: pane.paneID),
                    bootProfile: host.bootProfile(for: pane),
                    isFocused: host.shellState.focusedPaneID == pane.paneID,
                    onSelect: { host.focus(paneID: pane.paneID) },
                    onRuntimeUpdate: host.updateTerminalRuntime,
                    onMetadataUpdate: { metadata in
                        host.updateTerminalMetadata(metadata, for: pane.paneID)
                    }
                )
            }
        case .split:
            if node.direction == .vertical {
                HStack(spacing: 14) {
                    splitChildren
                }
            } else {
                VStack(spacing: 14) {
                    splitChildren
                }
            }
        }
    }

    @ViewBuilder
    private var splitChildren: some View {
        ForEach(node.children ?? []) { child in
            ShellPaneTreeLayoutView(node: child, host: host)
        }
    }
}

private struct ShellTerminalLeafView: View {
    let pane: ShellPane
    let runtime: TerminalHostRuntimeSnapshot
    let bootProfile: AlanShellBootProfile?
    let isFocused: Bool
    let onSelect: () -> Void
    let onRuntimeUpdate: (TerminalHostRuntimeSnapshot) -> Void
    let onMetadataUpdate: (TerminalSurfaceMetadataSnapshot) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(pane.viewport?.title ?? pane.paneID)
                        .font(.system(size: 14, weight: .semibold, design: .rounded))
                        .foregroundStyle(ShellPalette.ink)
                    Text(pane.viewport?.summary ?? "Terminal pane")
                        .font(.system(size: 12, weight: .medium, design: .rounded))
                        .foregroundStyle(ShellPalette.mutedInk)
                        .lineLimit(1)
                }

                Spacer(minLength: 8)

                TerminalPaneChip(
                    icon: isFocused ? "scope" : "circle",
                    title: runtime.stageLabel,
                    foreground: isFocused ? ShellPalette.accent : ShellPalette.mutedInk,
                    background: isFocused ? ShellPalette.accentSoft.opacity(0.45) : Color.white.opacity(0.65)
                )

                Button(action: onSelect) {
                    Text(isFocused ? "Focused" : "Focus")
                        .font(.system(size: 12, weight: .semibold, design: .rounded))
                        .foregroundStyle(isFocused ? ShellPalette.accent : ShellPalette.ink)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 8)
                        .background(
                            Capsule(style: .continuous)
                                .fill(Color.white.opacity(isFocused ? 0.9 : 0.72))
                        )
                }
                .buttonStyle(.plain)
            }

            TerminalHostView(
                pane: pane,
                bootProfile: bootProfile,
                onRuntimeUpdate: onRuntimeUpdate,
                onMetadataUpdate: onMetadataUpdate
            )
            .frame(minHeight: 260)
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 28, style: .continuous)
                .fill(Color.white.opacity(isFocused ? 0.46 : 0.26))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 28, style: .continuous)
                .stroke(
                    isFocused ? ShellPalette.accent.opacity(0.55) : ShellPalette.line.opacity(0.2),
                    lineWidth: isFocused ? 1.5 : 1
                )
        }
    }
}

private struct TerminalPaneSelectorButton: View {
    let pane: ShellPane
    let isFocused: Bool
    let onSelect: () -> Void

    private var summaryText: String {
        pane.viewport?.summary ?? "No summary"
    }

    var body: some View {
        Button(action: onSelect) {
            VStack(alignment: .leading, spacing: 4) {
                Text(pane.paneID)
                    .font(.system(size: 12, weight: .semibold, design: .monospaced))
                Text(summaryText)
                    .font(.system(size: 12, weight: .medium, design: .rounded))
                    .lineLimit(2)
            }
            .foregroundStyle(isFocused ? ShellPalette.ink : ShellPalette.mutedInk)
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .fill(isFocused ? Color.white.opacity(0.82) : Color.white.opacity(0.42))
            )
            .overlay {
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .stroke(ShellPalette.line.opacity(isFocused ? 0.42 : 0.18), lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
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
        .font(.system(size: 12, weight: .semibold, design: .rounded))
        .foregroundStyle(foreground)
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(
            Capsule(style: .continuous)
                .fill(Color.white.opacity(0.72))
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
        .font(.system(size: 12, weight: .semibold, design: .rounded))
        .foregroundStyle(foreground)
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
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
