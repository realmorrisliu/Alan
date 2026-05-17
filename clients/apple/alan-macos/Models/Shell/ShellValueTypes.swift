import Foundation

enum ShellAttentionState: String, Codable, CaseIterable {
    case idle
    case active
    case awaitingUser = "awaiting_user"
    case notable
}

enum ShellTabKind: String, Codable, CaseIterable {
    case terminal
    case scratch
    case log
}

enum ShellPaneTreeKind: String, Codable {
    case split
    case pane
}

enum ShellSplitDirection: String, Codable {
    case horizontal
    case vertical
}

enum ShellPaneSplitDirection: String, Codable, CaseIterable {
    case left
    case right
    case up
    case down

    var splitDirection: ShellSplitDirection {
        switch self {
        case .left, .right:
            return .vertical
        case .up, .down:
            return .horizontal
        }
    }

    var placesNewPaneBeforeTarget: Bool {
        switch self {
        case .left, .up:
            return true
        case .right, .down:
            return false
        }
    }

    static func defaultPlacement(for splitDirection: ShellSplitDirection) -> ShellPaneSplitDirection {
        switch splitDirection {
        case .horizontal:
            return .down
        case .vertical:
            return .right
        }
    }
}

enum ShellSpatialFocusDirection: String, Codable, CaseIterable {
    case left
    case right
    case up
    case down

    var splitDirection: ShellSplitDirection {
        switch self {
        case .left, .right:
            return .vertical
        case .up, .down:
            return .horizontal
        }
    }

    var movesForward: Bool {
        switch self {
        case .right, .down:
            return true
        case .left, .up:
            return false
        }
    }
}

enum ShellWorkspaceCommand: String, CaseIterable, Identifiable {
    case newTerminalTab
    case newAlanTab
    case splitLeft
    case splitRight
    case splitUp
    case splitDown
    case focusLeft
    case focusRight
    case focusUp
    case focusDown
    case equalizeSplits
    case closePane
    case closeTab

    var id: String { rawValue }
}

enum ShellLaunchTarget: String, Codable, CaseIterable {
    case shell
    case alan
}

enum ShellTabActiveTaskState: String, Codable, Equatable, CaseIterable {
    case inactive
    case foregroundCommand = "foreground_command"
    case alanRunning = "alan_running"
    case alanPendingYield = "alan_pending_yield"
    case alanSession = "alan_session"
    case unknown

    var protectsFromPruning: Bool {
        switch self {
        case .inactive:
            return false
        case .foregroundCommand, .alanRunning, .alanPendingYield, .alanSession, .unknown:
            return true
        }
    }
}

enum TerminalActivitySourceKind: String, Codable, Equatable, CaseIterable {
    case codex
    case claude
    case openCode = "open_code"
    case alan
    case shell
    case progress
    case command
    case process
    case unknown
}

enum TerminalActivityStatus: String, Codable, Equatable, CaseIterable {
    case needsInput = "needs_input"
    case failed
    case paused
    case progress
    case running
    case bell
    case exited
    case idle
    case done
    case stale
}

enum TerminalActivityPriority: String, Codable, Equatable, CaseIterable {
    case passive
    case active
    case notable
    case awaitingUser = "awaiting_user"
}

enum TerminalActivityProgressKind: String, Codable, Equatable, CaseIterable {
    case percent
    case indeterminate
    case paused
    case failed
}

struct TerminalActivitySource: Codable, Equatable {
    let kind: TerminalActivitySourceKind
    let label: String?
}

struct TerminalActivityProgress: Codable, Equatable {
    let kind: TerminalActivityProgressKind
    let percent: Int?

    init(kind: TerminalActivityProgressKind, percent: Int? = nil) {
        self.kind = kind
        self.percent = percent.map { min(max($0, 0), 100) }
    }

    static func percent(_ value: Int) -> TerminalActivityProgress {
        TerminalActivityProgress(kind: .percent, percent: value)
    }

    static let indeterminate = TerminalActivityProgress(kind: .indeterminate)
    static let paused = TerminalActivityProgress(kind: .paused)
    static let failed = TerminalActivityProgress(kind: .failed)
}

struct TerminalActivityCommandOutcome: Codable, Equatable {
    let exitCode: Int?
    let durationMilliseconds: Int?
    let commandText: String?

    private enum CodingKeys: String, CodingKey {
        case exitCode = "exit_code"
        case durationMilliseconds = "duration_milliseconds"
        case commandText = "command_text"
    }
}

struct TerminalActivityAgentMetadata: Codable, Equatable {
    let kind: TerminalActivitySourceKind
    let safeSessionLabel: String?
    let projectLabel: String?
    let workingDirectory: String?

    private enum CodingKeys: String, CodingKey {
        case kind
        case safeSessionLabel = "safe_session_label"
        case projectLabel = "project_label"
        case workingDirectory = "working_directory"
    }
}

struct TerminalActivityDisplay: Codable, Equatable {
    let sourceLabel: String
    let stateLabel: String
    let detailLabel: String?
    let paneHint: String?

    var sourceFirstLabel: String {
        [paneHint, "\(sourceLabel) · \(stateLabel)"]
            .compactMap { label -> String? in
                guard let label, !label.isEmpty else { return nil }
                return label
            }
            .joined(separator: " · ")
    }

    private enum CodingKeys: String, CodingKey {
        case sourceLabel = "source_label"
        case stateLabel = "state_label"
        case detailLabel = "detail_label"
        case paneHint = "pane_hint"
    }
}

struct TerminalActivityFreshness: Codable, Equatable {
    let updatedAt: String
    let staleAt: String?
    let expiresAt: String?

    private enum CodingKeys: String, CodingKey {
        case updatedAt = "updated_at"
        case staleAt = "stale_at"
        case expiresAt = "expires_at"
    }
}

struct TerminalActivitySnapshot: Codable, Equatable {
    private static let iso8601Formatter = ISO8601DateFormatter()

    let source: TerminalActivitySource
    let status: TerminalActivityStatus
    let priority: TerminalActivityPriority
    let progress: TerminalActivityProgress?
    let command: TerminalActivityCommandOutcome?
    let agent: TerminalActivityAgentMetadata?
    let display: TerminalActivityDisplay
    let freshness: TerminalActivityFreshness

    var isSidebarWorthy: Bool {
        switch status {
        case .needsInput, .failed, .paused, .progress, .running, .bell, .exited:
            return true
        case .idle, .done, .stale:
            return false
        }
    }

    var sidebarPriorityRank: Int {
        switch status {
        case .needsInput:
            return 70
        case .failed:
            return 60
        case .paused:
            return 50
        case .progress:
            return 40
        case .running:
            return 30
        case .bell, .exited:
            return 20
        case .idle, .done, .stale:
            return 0
        }
    }

    func isFresh(at now: Date) -> Bool {
        if let expiresAt = freshness.expiresAt.flatMap(Self.iso8601Formatter.date(from:)),
           now >= expiresAt
        {
            return false
        }

        if let staleAt = freshness.staleAt.flatMap(Self.iso8601Formatter.date(from:)),
           now >= staleAt
        {
            return false
        }

        return true
    }

    static func primarySidebarActivity(
        _ activities: [TerminalActivitySnapshot]
    ) -> TerminalActivitySnapshot? {
        primarySidebarActivity(activities, now: Date())
    }

    static func primarySidebarActivity(
        _ activities: [TerminalActivitySnapshot],
        now: Date?
    ) -> TerminalActivitySnapshot? {
        activities
            .filter { activity in
                activity.isSidebarWorthy && now.map(activity.isFresh(at:)) != false
            }
            .max { lhs, rhs in
                if lhs.sidebarPriorityRank == rhs.sidebarPriorityRank {
                    return lhs.freshness.updatedAt < rhs.freshness.updatedAt
                }
                return lhs.sidebarPriorityRank < rhs.sidebarPriorityRank
            }
    }

    static func progressActivity(percent: Int, now: Date) -> TerminalActivitySnapshot {
        let boundedPercent = min(max(percent, 0), 100)
        return progressActivity(
            progress: .percent(boundedPercent),
            status: .progress,
            priority: .active,
            stateLabel: "\(boundedPercent)%",
            now: now
        )
    }

    static func progressActivity(
        progress: TerminalActivityProgress,
        status: TerminalActivityStatus,
        priority: TerminalActivityPriority,
        stateLabel: String,
        now: Date
    ) -> TerminalActivitySnapshot {
        return TerminalActivitySnapshot(
            source: TerminalActivitySource(kind: .progress, label: "Progress"),
            status: status,
            priority: priority,
            progress: progress,
            command: nil,
            agent: nil,
            display: TerminalActivityDisplay(
                sourceLabel: "Progress",
                stateLabel: stateLabel,
                detailLabel: nil,
                paneHint: nil
            ),
            freshness: TerminalActivityFreshness(
                updatedAt: Self.iso8601Formatter.string(from: now),
                staleAt: Self.iso8601Formatter.string(from: now.addingTimeInterval(15)),
                expiresAt: nil
            )
        )
    }

    static func commandCompletion(exitCode: Int, now: Date) -> TerminalActivitySnapshot {
        let succeeded = exitCode == 0
        let status: TerminalActivityStatus = succeeded ? .done : .failed
        let priority: TerminalActivityPriority = succeeded ? .passive : .notable
        let stateLabel = succeeded ? "Command succeeded" : "Command failed \(exitCode)"
        return TerminalActivitySnapshot(
            source: TerminalActivitySource(kind: .command, label: "Shell"),
            status: status,
            priority: priority,
            progress: nil,
            command: TerminalActivityCommandOutcome(
                exitCode: exitCode,
                durationMilliseconds: nil,
                commandText: nil
            ),
            agent: nil,
            display: TerminalActivityDisplay(
                sourceLabel: "Shell",
                stateLabel: stateLabel,
                detailLabel: nil,
                paneHint: nil
            ),
            freshness: TerminalActivityFreshness(
                updatedAt: Self.iso8601Formatter.string(from: now),
                staleAt: succeeded ? nil : Self.iso8601Formatter.string(from: now.addingTimeInterval(30)),
                expiresAt: nil
            )
        )
    }
}

struct ShellProcessBinding: Codable, Equatable {
    let program: String
    let argvPreview: [String]?

    private enum CodingKeys: String, CodingKey {
        case program
        case argvPreview = "argv_preview"
    }
}

struct ShellContextSnapshot: Codable, Equatable {
    let workingDirectoryName: String?
    let repositoryRoot: String?
    let gitBranch: String?
    let controlPath: String?
    let socketPath: String?
    let alanBindingFile: String?
    let launchCommand: String?
    let launchStrategy: String?
    let shellIntegrationSource: String?
    let processState: String?
    let rendererPhase: String?
    let rendererHealth: String?
    let surfaceReadiness: String?
    let inputReady: Bool?
    let readonly: Bool?
    let terminalMode: String?
    let displayName: String?
    let displayID: String?
    let windowTitle: String?
    let lastMetadataAt: String?
    let lastCommandExitCode: Int?

    init(
        workingDirectoryName: String?,
        repositoryRoot: String?,
        gitBranch: String?,
        controlPath: String?,
        socketPath: String? = nil,
        alanBindingFile: String?,
        launchCommand: String? = nil,
        launchStrategy: String?,
        shellIntegrationSource: String?,
        processState: String?,
        rendererPhase: String? = nil,
        rendererHealth: String? = nil,
        surfaceReadiness: String? = nil,
        inputReady: Bool? = nil,
        readonly: Bool? = nil,
        terminalMode: String? = nil,
        displayName: String? = nil,
        displayID: String? = nil,
        windowTitle: String? = nil,
        lastMetadataAt: String?,
        lastCommandExitCode: Int?
    ) {
        self.workingDirectoryName = workingDirectoryName
        self.repositoryRoot = repositoryRoot
        self.gitBranch = gitBranch
        self.controlPath = controlPath
        self.socketPath = socketPath
        self.alanBindingFile = alanBindingFile
        self.launchCommand = launchCommand
        self.launchStrategy = launchStrategy
        self.shellIntegrationSource = shellIntegrationSource
        self.processState = processState
        self.rendererPhase = rendererPhase
        self.rendererHealth = rendererHealth
        self.surfaceReadiness = surfaceReadiness
        self.inputReady = inputReady
        self.readonly = readonly
        self.terminalMode = terminalMode
        self.displayName = displayName
        self.displayID = displayID
        self.windowTitle = windowTitle
        self.lastMetadataAt = lastMetadataAt
        self.lastCommandExitCode = lastCommandExitCode
    }

    private enum CodingKeys: String, CodingKey {
        case workingDirectoryName = "working_directory_name"
        case repositoryRoot = "repository_root"
        case gitBranch = "git_branch"
        case controlPath = "control_path"
        case socketPath = "socket_path"
        case alanBindingFile = "alan_binding_file"
        case launchCommand = "launch_command"
        case launchStrategy = "launch_strategy"
        case shellIntegrationSource = "shell_integration_source"
        case processState = "process_state"
        case rendererPhase = "renderer_phase"
        case rendererHealth = "renderer_health"
        case surfaceReadiness = "surface_readiness"
        case inputReady = "input_ready"
        case readonly
        case terminalMode = "terminal_mode"
        case displayName = "display_name"
        case displayID = "display_id"
        case windowTitle = "window_title"
        case lastMetadataAt = "last_metadata_at"
        case lastCommandExitCode = "last_command_exit_code"
    }
}
