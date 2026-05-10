import SwiftUI

enum ConsoleTheme {
    static let windowBackground = LinearGradient(
        colors: [
            Color.consoleAdaptive(light: (0.96, 0.97, 0.99), dark: (0.06, 0.07, 0.08)),
            Color.consoleAdaptive(light: (0.92, 0.94, 0.97), dark: (0.09, 0.10, 0.12)),
            Color.consoleAdaptive(light: (0.95, 0.96, 0.98), dark: (0.07, 0.08, 0.10)),
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let sidebarFill = Color.consoleAdaptive(light: (0.95, 0.96, 0.98), dark: (0.13, 0.14, 0.16))
    static let sidebarBorder = Color.consoleAdaptive(light: (0.82, 0.85, 0.90), dark: (0.29, 0.31, 0.36))

    static let panelFill = Color.consoleAdaptive(light: (1.00, 1.00, 1.00), dark: (0.10, 0.11, 0.13))
    static let inlinePanel = Color.consoleAdaptive(light: (0.95, 0.96, 0.98), dark: (0.14, 0.15, 0.18))
    static let panelBorder = Color.consoleAdaptive(light: (0.84, 0.87, 0.92), dark: (0.28, 0.30, 0.34))

    static let textPrimary = Color.consoleAdaptive(light: (0.08, 0.10, 0.14), dark: (0.94, 0.95, 0.97))
    static let textSecondary = Color.consoleAdaptive(light: (0.22, 0.26, 0.33), dark: (0.74, 0.77, 0.83))
    static let textMuted = Color.consoleAdaptive(light: (0.40, 0.45, 0.53), dark: (0.56, 0.60, 0.67))

    static let accent = Color.consoleAdaptive(light: (0.20, 0.44, 0.85), dark: (0.28, 0.58, 0.96))
    static let success = Color.consoleAdaptive(light: (0.17, 0.62, 0.33), dark: (0.39, 0.81, 0.48))
    static let warning = Color.consoleAdaptive(light: (0.85, 0.56, 0.12), dark: (0.98, 0.75, 0.29))
    static let error = Color.consoleAdaptive(light: (0.77, 0.22, 0.24), dark: (0.94, 0.38, 0.40))

    static let selectionFill = Color.consoleAdaptive(light: (0.74, 0.84, 0.99), dark: (0.24, 0.48, 0.85))
        .opacity(0.30)
    static let selectionBorder = Color.consoleAdaptive(light: (0.33, 0.54, 0.90), dark: (0.34, 0.63, 0.99))
        .opacity(0.58)

    static let badgeFill = Color.consoleAdaptive(light: (0.74, 0.84, 0.99), dark: (0.24, 0.48, 0.85))
        .opacity(0.30)
    static let badgeBorder = Color.consoleAdaptive(light: (0.33, 0.54, 0.90), dark: (0.34, 0.63, 0.99))
        .opacity(0.58)

    static let userBubble = Color.consoleAdaptive(light: (0.86, 0.92, 1.00), dark: (0.16, 0.23, 0.34))
    static let assistantBubble = Color.consoleAdaptive(light: (0.95, 0.96, 0.98), dark: (0.14, 0.15, 0.18))
    static let systemBubble = Color.consoleAdaptive(light: (0.93, 0.94, 0.96), dark: (0.17, 0.17, 0.19))
    static let errorBubble = Color.consoleAdaptive(light: (1.00, 0.91, 0.92), dark: (0.26, 0.13, 0.14))
}

struct SidebarActionButtonStyle: ButtonStyle {
    var prominent: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .semibold, design: .rounded))
            .foregroundStyle(prominent ? Color.white : ConsoleTheme.textPrimary)
            .padding(.horizontal, 12)
            .padding(.vertical, 9)
            .background(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(prominent ? ConsoleTheme.accent : ConsoleTheme.inlinePanel)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .strokeBorder(prominent ? Color.clear : ConsoleTheme.panelBorder, lineWidth: 1)
            )
            .opacity(configuration.isPressed ? 0.82 : 1)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

struct InlineActionButtonStyle: ButtonStyle {
    var prominent: Bool = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .semibold, design: .rounded))
            .foregroundStyle(prominent ? Color.white : ConsoleTheme.textPrimary)
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .fill(prominent ? ConsoleTheme.accent : ConsoleTheme.inlinePanel)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .strokeBorder(prominent ? Color.clear : ConsoleTheme.panelBorder, lineWidth: 1)
            )
            .opacity(configuration.isPressed ? 0.84 : 1)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

struct CompactDarkFieldStyle: TextFieldStyle {
    func _body(configuration: TextField<Self._Label>) -> some View {
        configuration
            .font(.system(size: 12, weight: .regular, design: .rounded))
            .foregroundStyle(ConsoleTheme.textPrimary)
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(ConsoleTheme.inlinePanel, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
            )
    }
}

struct MessageBubble: View {
    let message: ChatMessage

    private var roleTitle: String {
        switch message.role {
        case .user:
            return "You"
        case .assistant:
            return "Alan"
        case .system:
            return "System"
        case .error:
            return "Error"
        }
    }

    private var indicatorColor: Color {
        switch message.role {
        case .user:
            return ConsoleTheme.accent
        case .assistant:
            return ConsoleTheme.success
        case .system:
            return ConsoleTheme.textMuted
        case .error:
            return ConsoleTheme.error
        }
    }

    private var bubbleFill: Color {
        switch message.role {
        case .user:
            return ConsoleTheme.userBubble
        case .assistant:
            return ConsoleTheme.assistantBubble
        case .system:
            return ConsoleTheme.systemBubble
        case .error:
            return ConsoleTheme.errorBubble
        }
    }

    private var bodyText: String {
        if message.text.isEmpty && message.isStreaming {
            return "…"
        }
        return message.text
    }

    var body: some View {
        HStack {
            if message.role == .user {
                Spacer(minLength: 72)
            }

            VStack(alignment: .leading, spacing: 7) {
                HStack(spacing: 6) {
                    Circle()
                        .fill(indicatorColor)
                        .frame(width: 7, height: 7)

                    Text(roleTitle)
                        .font(.system(size: 11, weight: .semibold, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textMuted)

                    if message.isStreaming {
                        ProgressView()
                            .controlSize(.mini)
                            .tint(indicatorColor)
                    }
                }

                Text(bodyText)
                    .font(.system(size: 14, weight: .regular, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textPrimary)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .frame(maxWidth: 860, alignment: .leading)
            .background(bubbleFill, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .strokeBorder(indicatorColor.opacity(0.36), lineWidth: 1)
            )

            if message.role != .user {
                Spacer(minLength: 72)
            }
        }
    }
}

struct TimelineRow: View {
    let entry: TimelineEntry

    private var icon: String {
        switch entry.level {
        case .info:
            return "info.circle"
        case .action:
            return "bolt.circle"
        case .warning:
            return "exclamationmark.triangle"
        case .error:
            return "xmark.octagon"
        }
    }

    private var tint: Color {
        switch entry.level {
        case .info:
            return ConsoleTheme.accent
        case .action:
            return ConsoleTheme.success
        case .warning:
            return ConsoleTheme.warning
        case .error:
            return ConsoleTheme.error
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(spacing: 7) {
                Image(systemName: icon)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(tint)

                Text(entry.title)
                    .font(.system(size: 12, weight: .semibold, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textPrimary)

                Spacer(minLength: 6)

                Text(entry.timestamp, style: .time)
                    .font(.system(size: 10, weight: .regular, design: .monospaced))
                    .foregroundStyle(ConsoleTheme.textMuted)
            }

            if let detail = entry.detail, !detail.isEmpty {
                Text(detail)
                    .font(.system(size: 11, weight: .regular, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textMuted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(ConsoleTheme.inlinePanel, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
        )
    }
}
