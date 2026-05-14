#if os(macOS)
import AppKit

@MainActor
final class TerminalHostOverlayPresenter {
    let overlayCard = AlanTerminalPassiveOverlayView()

    private let bodyStack = NSStackView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let subtitleLabel = NSTextField(wrappingLabelWithString: "")
    private let commandLabel = NSTextField(wrappingLabelWithString: "")
    private let footerLabel = NSTextField(wrappingLabelWithString: "")
    private let statusBadge = NSTextField(labelWithString: "")

    func install(in hostView: NSView) {
        bodyStack.orientation = .vertical
        bodyStack.alignment = .leading
        bodyStack.spacing = 10
        bodyStack.translatesAutoresizingMaskIntoConstraints = false

        overlayCard.material = .hudWindow
        overlayCard.blendingMode = .withinWindow
        overlayCard.state = .active
        overlayCard.translatesAutoresizingMaskIntoConstraints = false
        overlayCard.wantsLayer = true
        overlayCard.layer?.cornerRadius = ShellRadii.overlay
        overlayCard.layer?.borderWidth = 1
        overlayCard.layer?.borderColor = NSColor.white.withAlphaComponent(0.12).cgColor

        statusBadge.font = .systemFont(ofSize: 11, weight: .semibold)
        statusBadge.textColor = NSColor.white.withAlphaComponent(0.92)
        statusBadge.drawsBackground = true
        statusBadge.backgroundColor = NSColor.white.withAlphaComponent(0.10)
        statusBadge.isBezeled = false
        statusBadge.isEditable = false
        statusBadge.isSelectable = false
        statusBadge.alignment = .center
        statusBadge.lineBreakMode = .byTruncatingTail
        statusBadge.maximumNumberOfLines = 1
        statusBadge.cell?.usesSingleLineMode = true
        statusBadge.cell?.wraps = false
        statusBadge.cell?.backgroundStyle = .raised

        titleLabel.font = .systemFont(ofSize: 18, weight: .semibold)
        titleLabel.textColor = .white

        subtitleLabel.font = .systemFont(ofSize: 12, weight: .medium)
        subtitleLabel.textColor = NSColor.white.withAlphaComponent(0.64)
        subtitleLabel.maximumNumberOfLines = 2

        commandLabel.font = .monospacedSystemFont(ofSize: 12, weight: .medium)
        commandLabel.textColor = NSColor(calibratedRed: 0.81, green: 0.90, blue: 0.98, alpha: 1)
        commandLabel.maximumNumberOfLines = 3

        footerLabel.font = .systemFont(ofSize: 11, weight: .regular)
        footerLabel.textColor = NSColor.white.withAlphaComponent(0.52)
        footerLabel.maximumNumberOfLines = 4

        [statusBadge, titleLabel, subtitleLabel, commandLabel, footerLabel].forEach(
            bodyStack.addArrangedSubview
        )

        hostView.addSubview(overlayCard)
        overlayCard.addSubview(bodyStack)

        NSLayoutConstraint.activate([
            overlayCard.centerXAnchor.constraint(equalTo: hostView.centerXAnchor),
            overlayCard.centerYAnchor.constraint(equalTo: hostView.centerYAnchor),
            overlayCard.widthAnchor.constraint(lessThanOrEqualToConstant: 460),
            overlayCard.leadingAnchor.constraint(
                greaterThanOrEqualTo: hostView.leadingAnchor,
                constant: 24
            ),
            overlayCard.trailingAnchor.constraint(
                lessThanOrEqualTo: hostView.trailingAnchor,
                constant: -24
            ),

            bodyStack.topAnchor.constraint(equalTo: overlayCard.topAnchor, constant: 18),
            bodyStack.leadingAnchor.constraint(equalTo: overlayCard.leadingAnchor, constant: 18),
            bodyStack.trailingAnchor.constraint(equalTo: overlayCard.trailingAnchor, constant: -18),
            bodyStack.bottomAnchor.constraint(equalTo: overlayCard.bottomAnchor, constant: -18),
        ])
    }

    func configure(pane: ShellPane?, bootProfile: AlanShellBootProfile?) {
        titleLabel.stringValue = pane?.viewport?.title ?? pane?.process?.program ?? "Terminal"
        subtitleLabel.stringValue = pane?.viewport?.summary ?? "Preparing the native terminal view."

        if let bootProfile {
            commandLabel.stringValue = "$ \(bootProfile.launchCommandString)"

            let envSummary = bootProfile.environmentPreview
                .prefix(4)
                .map { "\($0.key)=\($0.value)" }
                .joined(separator: "\n")
            footerLabel.stringValue = [
                "launch: \(bootProfile.command.strategy.rawValue)",
                bootProfile.command.detail,
                "cwd: \(bootProfile.workingDirectory)",
                envSummary.isEmpty ? nil : envSummary,
                "setup: \(bootProfile.ghostty.setupCommand)",
            ]
            .compactMap { $0 }
            .joined(separator: "\n")
        } else {
            commandLabel.stringValue = "$ /bin/zsh -l"
            footerLabel.stringValue = "Select a pane to prepare a terminal boot profile."
        }
    }

    func updateSubtitle(_ summary: String?) {
        guard let summary else { return }
        subtitleLabel.stringValue = summary
    }

    func syncStatusBadge(
        bootProfile: AlanShellBootProfile?,
        renderer: TerminalRendererSnapshot
    ) {
        if bootProfile == nil {
            statusBadge.stringValue = "Select a pane"
            return
        }

        if let bootProfile, !bootProfile.ghostty.isReady {
            statusBadge.stringValue = "GhosttyKit pending"
            return
        }

        statusBadge.stringValue = renderer.kind == .ghosttyLive
            ? "Ghostty live · \(renderer.phaseLabel)"
            : "GhosttyKit ready"
    }

    func syncOverlay(
        overlayState: AlanTerminalOverlayState?,
        bootProfile: AlanShellBootProfile?
    ) {
        guard let overlayState else {
            overlayCard.isHidden = true
            return
        }

        statusBadge.stringValue = overlayState.badge
        titleLabel.stringValue = overlayState.title
        subtitleLabel.stringValue = overlayState.message
        if let action = overlayState.action {
            commandLabel.stringValue = action
        }
        footerLabel.stringValue = bootProfile.map {
            "launch: \($0.command.strategy.rawValue)\ncwd: \($0.workingDirectory)"
        }
        ?? "Select a pane to prepare a terminal boot profile."
        overlayCard.isHidden = false
    }
}

final class AlanTerminalPassiveOverlayView: NSVisualEffectView {
    override var mouseDownCanMoveWindow: Bool { false }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}
#endif
