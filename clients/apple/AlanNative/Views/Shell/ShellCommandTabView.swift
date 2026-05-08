import SwiftUI

#if os(macOS)
private enum ShellCommandTabAction: String, CaseIterable, Identifiable {
    case newSpace
    case newAlanSpace
    case openTab
    case openAlanTab
    case jumpToAttention
    case focusBestPane
    case splitRight
    case splitDown
    case splitLeft
    case splitUp
    case focusLeft
    case focusRight
    case focusUp
    case focusDown
    case equalizeSplits
    case liftPane
    case closePane
    case closeTab
    case copySnapshot

    var id: String { rawValue }

    var title: String {
        switch self {
        case .newSpace:
            return "Create Space"
        case .newAlanSpace:
            return "Create Space with Alan"
        case .openTab:
            return "Open New Tab"
        case .openAlanTab:
            return "Open In Alan"
        case .jumpToAttention:
            return "Jump To Attention"
        case .focusBestPane:
            return "Focus Best Routing Pane"
        case .splitRight:
            return "Split Pane Right"
        case .splitDown:
            return "Split Pane Down"
        case .splitLeft:
            return "Split Pane Left"
        case .splitUp:
            return "Split Pane Up"
        case .focusLeft:
            return "Focus Pane Left"
        case .focusRight:
            return "Focus Pane Right"
        case .focusUp:
            return "Focus Pane Up"
        case .focusDown:
            return "Focus Pane Down"
        case .equalizeSplits:
            return "Equalize Splits"
        case .liftPane:
            return "Lift Focused Pane To Tab"
        case .closePane:
            return "Close Focused Pane"
        case .closeTab:
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
        case .openTab:
            return "Open another tab inside the current space."
        case .openAlanTab:
            return "Open another tab that boots directly into Alan."
        case .jumpToAttention:
            return "Jump to the strongest pane that currently needs approval or attention."
        case .focusBestPane:
            return "Use shell routing signals to jump to the strongest candidate pane."
        case .splitRight:
            return "Create a side-by-side split to the right of the focused pane."
        case .splitDown:
            return "Create a stacked split beneath the focused pane."
        case .splitLeft:
            return "Create a side-by-side split to the left of the focused pane."
        case .splitUp:
            return "Create a stacked split above the focused pane."
        case .focusLeft:
            return "Move terminal focus to the pane on the left."
        case .focusRight:
            return "Move terminal focus to the pane on the right."
        case .focusUp:
            return "Move terminal focus to the pane above."
        case .focusDown:
            return "Move terminal focus to the pane below."
        case .equalizeSplits:
            return "Return the current tab's split dividers to equal ratios."
        case .liftPane:
            return "Move the focused pane into its own tab without losing shell identity."
        case .closePane:
            return "Close the focused pane and keep the remaining tab layout intact."
        case .closeTab:
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
        case .openTab:
            return ["open tab", "new tab", "open terminal tab"]
        case .openAlanTab:
            return ["open in alan", "alan tab", "new alan tab"]
        case .jumpToAttention:
            return ["jump to attention", "jump attention", "focus waiting pane", "approval", "waiting pane"]
        case .focusBestPane:
            return ["best pane", "route", "routing", "focus best", "jump pane"]
        case .splitRight:
            return ["split right", "split vertical", "side by side"]
        case .splitDown:
            return ["split down", "split horizontal", "split below", "stack split"]
        case .splitLeft:
            return ["split left"]
        case .splitUp:
            return ["split up", "split above"]
        case .focusLeft:
            return ["focus left", "pane left"]
        case .focusRight:
            return ["focus right", "pane right"]
        case .focusUp:
            return ["focus up", "pane up"]
        case .focusDown:
            return ["focus down", "pane down"]
        case .equalizeSplits:
            return ["equalize", "balance splits", "reset split ratios"]
        case .liftPane:
            return ["lift pane", "move pane", "extract pane"]
        case .closePane:
            return ["close pane", "remove pane"]
        case .closeTab:
            return ["close tab", "close tab"]
        case .copySnapshot:
            return ["copy snapshot", "copy json", "debug snapshot"]
        }
    }
}

private struct ShellCommandTabIntent: Identifiable {
    enum Route {
        case action(ShellCommandTabAction)
        case attention(ShellAttentionItem)
        case candidate(AlanShellRoutingCandidate)
    }

    let title: String
    let detail: String
    let accent: Color
    let route: Route

    var id: String { title }
}

struct ShellCommandTabView: View {
    @ObservedObject var host: ShellHostController
    @Binding var isPresented: Bool
    @State private var query = ""
    @FocusState private var isQueryFocused: Bool
    @StateObject private var voiceController = ShellVoiceCommandController()

    private var matchingActions: [ShellCommandTabAction] {
        let allActions = ShellCommandTabAction.allCases.filter { $0.matches(query: query) }

        guard query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return allActions
        }

        let defaultActions: [ShellCommandTabAction] = [
            .openTab,
            .openAlanTab,
            .splitRight,
            .splitDown,
            .equalizeSplits,
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

    private var primaryIntent: ShellCommandTabIntent? {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return nil }

        if normalized.contains("route")
            || normalized.contains("best pane")
            || normalized.contains("focus best")
        {
            if let candidate = matchingRoutingCandidates.first {
                return ShellCommandTabIntent(
                    title: "Focus \(routingTitle(for: candidate))",
                    detail: routingDetail(for: candidate),
                    accent: ShellPalette.accent,
                    route: .candidate(candidate)
                )
            }
        }

        if normalized.contains("attention") || normalized.contains("waiting") || normalized.contains("jump") {
            if let firstAttention = matchingAttention.first {
                return ShellCommandTabIntent(
                    title: "Jump To \(firstAttention.title)",
                    detail: "Focus the pane that currently needs attention first.",
                    accent: firstAttention.attention == .awaitingUser ? ShellPalette.accent : ShellPalette.ink,
                    route: .attention(firstAttention)
                )
            }
        }

        if let action = matchingActions.first {
            return ShellCommandTabIntent(
                title: title(for: action),
                detail: detail(for: action),
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
                        prompt: Text("Go to or Command...")
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
                            RoundedRectangle(cornerRadius: ShellRadii.control, style: .continuous)
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
                                RoundedRectangle(cornerRadius: ShellRadii.control, style: .continuous)
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
                                RoundedRectangle(cornerRadius: ShellRadii.control, style: .continuous)
                                    .fill(ShellPalette.canvas.opacity(0.92))
                            )
                    }
                    .buttonStyle(.plain)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
                .background(
                    RoundedRectangle(cornerRadius: ShellRadii.surface, style: .continuous)
                        .fill(Color.white.opacity(0.9))
                )
                .overlay {
                    RoundedRectangle(cornerRadius: ShellRadii.surface, style: .continuous)
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
                                    title: title(for: action),
                                    detail: detail(for: action),
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
                                        title: "Focus \(routingTitle(for: candidate))",
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
                RoundedRectangle(cornerRadius: ShellRadii.overlay, style: .continuous)
                    .fill(.ultraThinMaterial)
                RoundedRectangle(cornerRadius: ShellRadii.overlay, style: .continuous)
                    .fill(ShellPalette.window.opacity(0.9))
            }
        )
        .overlay {
            RoundedRectangle(cornerRadius: ShellRadii.overlay, style: .continuous)
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

    private func perform(_ action: ShellCommandTabAction) {
        switch action {
        case .newSpace:
            _ = host.createTerminalSpace()
        case .newAlanSpace:
            _ = host.createAlanSpace()
        case .openTab:
            host.performShellWorkspaceCommand(.newTerminalTab)
        case .openAlanTab:
            host.performShellWorkspaceCommand(.newAlanTab)
        case .jumpToAttention:
            if let firstAttention = host.attentionItems.first {
                host.focusAttentionItem(firstAttention)
            }
        case .focusBestPane:
            _ = host.focusTopRoutingCandidate()
        case .splitRight:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.splitRight)
        case .splitDown:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.splitDown)
        case .splitLeft:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.splitLeft)
        case .splitUp:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.splitUp)
        case .focusLeft:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.focusLeft)
        case .focusRight:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.focusRight)
        case .focusUp:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.focusUp)
        case .focusDown:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.focusDown)
        case .equalizeSplits:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.equalizeSplits)
        case .liftPane:
            _ = host.liftSelectedPaneToTab()
        case .closePane:
            host.performShellWorkspaceCommand(.closePane)
        case .closeTab:
            host.performShellWorkspaceCommand(.closeTab)
        case .copySnapshot:
            host.copySnapshotJSON()
        }
        isPresented = false
    }

    private func execute(_ route: ShellCommandTabIntent.Route) {
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
        let title = routingTitle(for: candidate)
        let reasons = candidate.reasons.prefix(3).joined(separator: " • ")
        let detail = reasons.isEmpty ? "score \(Int(candidate.score * 100))" : reasons
        return "\(title) • \(detail)"
    }

    private func routingTitle(for candidate: AlanShellRoutingCandidate) -> String {
        guard let pane = host.shellState.panes.first(where: { $0.paneID == candidate.paneID }) else {
            return "Terminal Pane"
        }

        return shellDisplayTitle(
            rawTitle: pane.viewport?.title,
            workingDirectoryName: pane.context?.workingDirectoryName,
            cwd: pane.cwd,
            program: pane.process?.program,
            launchTarget: pane.resolvedLaunchTarget,
            fallback: pane.viewport?.summary
        )
    }

    private func title(for action: ShellCommandTabAction) -> String {
        action.title
    }

    private func detail(for action: ShellCommandTabAction) -> String {
        action.detail
    }

    private func sectionLabel(_ value: String) -> some View {
        Text(value)
            .font(.system(size: 10, weight: .semibold, design: .rounded))
            .textCase(.uppercase)
            .foregroundStyle(ShellPalette.mutedInk)
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
            RoundedRectangle(cornerRadius: ShellRadii.row, style: .continuous)
                .fill(Color.white.opacity(0.84))
        )
        .overlay {
            RoundedRectangle(cornerRadius: ShellRadii.row, style: .continuous)
                .stroke(ShellPalette.line.opacity(isHovered ? 0.24 : 0.12), lineWidth: 1)
        }
        .scaleEffect(isHovered ? 1.004 : 1)
        .shadow(color: isHovered ? Color.black.opacity(0.022) : .clear, radius: 6, y: 3)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: isHovered)
        .onHover { isHovered = $0 }
    }
}
#endif
