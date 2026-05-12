import SwiftUI

#if os(macOS)
private enum ShellCommandInputAction: CaseIterable {
    case newTerminalTab
    case newAlanTab
    case splitRight
    case splitDown
    case splitLeft
    case splitUp
    case focusLeft
    case focusRight
    case focusUp
    case focusDown
    case equalizeSplits
    case closePane
    case closeTab
    case jumpToAttention

    var aliases: Set<String> {
        switch self {
        case .newTerminalTab:
            return ["new tab", "new terminal tab", "open tab", "open terminal tab"]
        case .newAlanTab:
            return ["new alan tab", "open alan tab", "open in alan", "alan tab"]
        case .splitRight:
            return ["split right", "split pane right"]
        case .splitDown:
            return ["split down", "split below", "split pane down"]
        case .splitLeft:
            return ["split left", "split pane left"]
        case .splitUp:
            return ["split up", "split above", "split pane up"]
        case .focusLeft:
            return ["focus left", "focus pane left", "pane left"]
        case .focusRight:
            return ["focus right", "focus pane right", "pane right"]
        case .focusUp:
            return ["focus up", "focus pane up", "pane up"]
        case .focusDown:
            return ["focus down", "focus pane down", "pane down"]
        case .equalizeSplits:
            return ["equalize splits", "balance splits", "reset splits"]
        case .closePane:
            return ["close pane", "close focused pane"]
        case .closeTab:
            return ["close tab", "close current tab"]
        case .jumpToAttention:
            return ["jump to attention", "focus attention", "attention"]
        }
    }

    static func resolve(_ rawValue: String) -> ShellCommandInputAction? {
        let normalized = normalizedCommand(rawValue)
        guard !normalized.isEmpty else { return nil }
        return allCases.first { $0.aliases.contains(normalized) }
    }

    private static func normalizedCommand(_ rawValue: String) -> String {
        rawValue
            .lowercased()
            .replacingOccurrences(of: "-", with: " ")
            .replacingOccurrences(of: "_", with: " ")
            .split(whereSeparator: \.isWhitespace)
            .joined(separator: " ")
    }
}

struct ShellCommandTabView: View {
    @ObservedObject var host: ShellHostController
    @Binding var isPresented: Bool
    @State private var query = ""
    @State private var unresolvedMessage: String?
    @State private var unresolvedAttemptID = 0
    @FocusState private var isQueryFocused: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .center, spacing: 12) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(ShellPalette.accent)

                TextField(
                    "",
                    text: $query,
                    prompt: Text("Go to or Command...")
                        .foregroundStyle(ShellPalette.mutedInk.opacity(0.9))
                )
                .textFieldStyle(.plain)
                .font(.system(size: 17, weight: .medium))
                .foregroundStyle(ShellPalette.ink)
                .focused($isQueryFocused)
                .onChange(of: query) { _, _ in
                    unresolvedMessage = nil
                }
                .onSubmit {
                    submit()
                }
                .onKeyPress(.return) {
                    submit()
                    return .handled
                }

                Text("⌘P")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(ShellPalette.mutedInk.opacity(0.85))
                    .padding(.horizontal, 7)
                    .padding(.vertical, 4)
                    .background(
                        ShellMaterialShape(
                            role: .controlGlass,
                            shape: RoundedRectangle(
                                cornerRadius: ShellRadii.control,
                                style: .continuous
                            )
                        )
                    )

                Button {
                    dismissAndRestoreFocus()
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundStyle(ShellPalette.mutedInk)
                        .frame(width: 26, height: 26)
                        .background(
                            ShellMaterialShape(
                                role: .controlGlass,
                                shape: RoundedRectangle(
                                    cornerRadius: ShellRadii.control,
                                    style: .continuous
                                )
                            )
                        )
                }
                .buttonStyle(.plain)
                .help("Close command input")
            }

            if let unresolvedMessage {
                Text(unresolvedMessage)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(ShellPalette.mutedInk)
                    .padding(.leading, 27)
                    .transition(.opacity)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .frame(width: 520, alignment: .leading)
        .background {
            ShellMaterialBackgroundView(.floatingOverlay)
                .clipShape(RoundedRectangle(cornerRadius: ShellRadii.overlay, style: .continuous))
        }
        .overlay {
            RoundedRectangle(cornerRadius: ShellRadii.overlay, style: .continuous)
                .stroke(ShellMaterialRole.floatingOverlay.stroke, lineWidth: 1)
        }
        .shadow(color: Color.black.opacity(0.12), radius: 24, y: 14)
        .offset(x: unresolvedAttemptID.isMultiple(of: 2) ? 0 : 1.5)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.12), value: unresolvedAttemptID)
        .onAppear {
            isQueryFocused = true
        }
        .onExitCommand {
            dismissAndRestoreFocus()
        }
    }

    private func submit() {
        guard let action = ShellCommandInputAction.resolve(query) else {
            unresolvedMessage = query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? "Type a command name."
                : "No matching command."
            unresolvedAttemptID += 1
            isQueryFocused = true
            return
        }

        perform(action)
    }

    private func perform(_ action: ShellCommandInputAction) {
        switch action {
        case .newTerminalTab:
            host.performShellWorkspaceCommand(.newTerminalTab)
        case .newAlanTab:
            host.performShellWorkspaceCommand(.newAlanTab)
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
        case .closePane:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.closePane)
        case .closeTab:
            host.performShellWorkspaceCommand(ShellWorkspaceCommand.closeTab)
        case .jumpToAttention:
            if let firstAttention = host.attentionItems.first {
                host.focusAttentionItem(firstAttention)
            }
        }
        dismissAndRestoreFocus()
    }

    private func dismissAndRestoreFocus() {
        isPresented = false
        DispatchQueue.main.async {
            host.refocusSelectedTerminalPane()
        }
    }
}
#endif
