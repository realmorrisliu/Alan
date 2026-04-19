import SwiftUI

#if os(macOS)
struct TerminalPaneView: View {
    @ObservedObject var host: ShellHostController

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if showsMetadataStrip {
                paneMetadataStrip
            }
            paneCanvas

            if host.panesForSelectedTab.count > 1 {
                paneSelectorStrip
            }
        }
        .padding(.horizontal, 4)
        .padding(.top, 4)
        .padding(.bottom, 2)
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

        return (workingDirectory != nil && workingDirectory != selectedPaneTitle)
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
    }

    private var paneSelectorStrip: some View {
        HStack(spacing: 0) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 10) {
                    ForEach(Array(host.panesForSelectedTab.enumerated()), id: \.element.paneID) { _, pane in
                        TerminalPaneSelectorButton(
                            pane: pane,
                            isFocused: host.selectedPane?.paneID == pane.paneID,
                            onSelect: { host.focus(paneID: pane.paneID) }
                        )
                    }
                }
            }
        }
        .padding(.top, 2)
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
                HStack(spacing: 10) {
                    splitChildren
                }
            } else {
                VStack(spacing: 10) {
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
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let pane: ShellPane
    let runtime: TerminalHostRuntimeSnapshot
    let bootProfile: AlanShellBootProfile?
    let isFocused: Bool
    let onSelect: () -> Void
    let onRuntimeUpdate: (TerminalHostRuntimeSnapshot) -> Void
    let onMetadataUpdate: (TerminalPaneMetadataSnapshot) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 7) {
                Circle()
                    .fill(isFocused ? ShellPalette.accent : Color.white.opacity(0.28))
                    .frame(width: 5, height: 5)

                VStack(alignment: .leading, spacing: 3) {
                    Text(paneTitle)
                        .font(.system(size: 11.5, weight: .semibold))
                        .tracking(-0.1)
                        .foregroundStyle(.white.opacity(0.96))
                    if let summary = paneSubtitle {
                        Text(summary)
                            .font(.system(size: 9.5, weight: .medium))
                            .foregroundStyle(.white.opacity(0.52))
                            .lineLimit(1)
                    }
                }

                Spacer(minLength: 8)

                if pane.resolvedLaunchTarget == .alan {
                    Image(systemName: "sparkles")
                        .font(.system(size: 8.5, weight: .bold))
                        .foregroundStyle(ShellPalette.accentSoft)
                }

                if pane.attention == .awaitingUser || pane.attention == .notable {
                    HStack(spacing: 4) {
                        Circle()
                            .fill(pane.attention == .awaitingUser ? ShellPalette.attention : Color.white.opacity(0.68))
                            .frame(width: 4.5, height: 4.5)
                        Text(pane.attention == .awaitingUser ? "Waiting" : "Notice")
                            .font(.system(size: 9.5, weight: .semibold))
                            .foregroundStyle(pane.attention == .awaitingUser ? ShellPalette.attention : .white.opacity(0.68))
                    }
                }
            }
            .padding(.horizontal, 12)
            .padding(.top, 8)
            .padding(.bottom, 6)
            .contentShape(Rectangle())
            .onTapGesture(perform: onSelect)

            TerminalHostView(
                pane: pane,
                bootProfile: bootProfile,
                onRuntimeUpdate: onRuntimeUpdate,
                onMetadataUpdate: onMetadataUpdate
            )
            .frame(minHeight: 260, maxHeight: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(ShellPalette.terminal)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .stroke(
                    isFocused ? ShellPalette.accent.opacity(0.55) : ShellPalette.line.opacity(0.18),
                    lineWidth: isFocused ? 1.25 : 1
                )
        }
        .shadow(
            color: isFocused ? ShellPalette.accent.opacity(0.12) : Color.black.opacity(0.035),
            radius: isFocused ? 14 : 8,
            y: isFocused ? 8 : 4
        )
        .scaleEffect(isFocused ? 1 : 0.998)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isFocused)
    }

    private var paneTitle: String {
        shellDisplayTitle(
            rawTitle: pane.viewport?.title,
            workingDirectoryName: pane.context?.workingDirectoryName,
            cwd: pane.cwd,
            program: pane.process?.program,
            launchTarget: pane.resolvedLaunchTarget,
            fallback: pane.resolvedLaunchTarget == .alan ? "Alan" : "Shell"
        )
    }

    private var paneSubtitle: String? {
        if let branch = pane.context?.gitBranch {
            return branch
        }

        if let folder = shellVisibleLabel(pane.context?.workingDirectoryName) ?? shellPathLeaf(pane.cwd),
           folder != paneTitle
        {
            return folder
        }

        if let summary = shellUserFacingSummary(pane.viewport?.summary),
           summary != paneTitle
        {
            return summary
        }

        if let program = shellVisibleLabel(pane.process?.program),
           program.localizedCaseInsensitiveCompare(paneTitle) != .orderedSame
        {
            return program
        }

        return nil
    }
}

private struct TerminalPaneSelectorButton: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isHovered = false
    let pane: ShellPane
    let isFocused: Bool
    let onSelect: () -> Void

    private var titleText: String {
        shellDisplayTitle(
            rawTitle: pane.viewport?.title,
            workingDirectoryName: pane.context?.workingDirectoryName,
            cwd: pane.cwd,
            program: pane.process?.program,
            launchTarget: pane.resolvedLaunchTarget,
            fallback: pane.resolvedLaunchTarget == .alan ? "Alan" : "Shell"
        )
    }

    private var summaryText: String? {
        if let branch = pane.context?.gitBranch {
            return branch
        }

        if let folder = shellVisibleLabel(pane.context?.workingDirectoryName) ?? shellPathLeaf(pane.cwd),
           folder != titleText
        {
            return folder
        }

        if let summary = shellUserFacingSummary(pane.viewport?.summary),
           summary != titleText
        {
            return summary
        }

        if let program = shellVisibleLabel(pane.process?.program),
           program.localizedCaseInsensitiveCompare(titleText) != .orderedSame
        {
            return program
        }

        return nil
    }

    var body: some View {
        Button(action: onSelect) {
            HStack(spacing: 8) {
                Circle()
                    .fill(isFocused ? ShellPalette.accent : ShellPalette.line.opacity(0.9))
                    .frame(width: 6, height: 6)

                VStack(alignment: .leading, spacing: 3) {
                    Text(titleText)
                        .font(.system(size: 12, weight: .semibold))
                    if let summaryText {
                        Text(summaryText)
                            .font(.system(size: 11, weight: .medium))
                            .lineLimit(1)
                    }
                }
            }
            .foregroundStyle(isFocused ? ShellPalette.ink : ShellPalette.mutedInk)
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .frame(minWidth: 108, alignment: .leading)
            .background(
                Capsule(style: .continuous)
                    .fill(isFocused ? Color.white.opacity(0.74) : (isHovered ? Color.white.opacity(0.36) : Color.white.opacity(0.24)))
            )
            .overlay {
                Capsule(style: .continuous)
                    .stroke(ShellPalette.line.opacity(isFocused ? 0.34 : (isHovered ? 0.2 : 0.12)), lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
        .scaleEffect(isHovered && !isFocused ? 1.01 : 1)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.14), value: isHovered)
        .onHover { isHovered = $0 }
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

func shellUserFacingSummary(_ summary: String?) -> String? {
    guard let summary else { return nil }

    let trimmed = summary.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { return nil }

    let internalOnlySummaries = [
        "title updated",
        "input committed",
        "terminal rendering",
        "window attached",
    ]

    if internalOnlySummaries.contains(trimmed.lowercased()) {
        return nil
    }

    return trimmed
}
#endif
