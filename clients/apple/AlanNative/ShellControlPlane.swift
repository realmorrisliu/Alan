import Foundation
import Darwin

#if os(macOS)
func alanShellControlPlaneRootURL(
    windowID: String,
    fileManager: FileManager = .default
) -> URL {
    fileManager.temporaryDirectory
        .appendingPathComponent("alan-shell-control", isDirectory: true)
        .appendingPathComponent(windowID, isDirectory: true)
}

func alanShellControlPlaneSocketURL(
    windowID: String,
    fileManager: FileManager = .default
) -> URL {
    alanShellControlPlaneRootURL(windowID: windowID, fileManager: fileManager)
        .appendingPathComponent("shell.sock")
}

func alanShellPaneSupportDirectoryURL(
    windowID: String,
    paneID: String,
    fileManager: FileManager = .default
) -> URL {
    alanShellControlPlaneRootURL(windowID: windowID, fileManager: fileManager)
        .appendingPathComponent("panes", isDirectory: true)
        .appendingPathComponent(paneID, isDirectory: true)
}

func alanShellBindingFileURL(
    windowID: String,
    paneID: String,
    fileManager: FileManager = .default
) -> URL {
    alanShellPaneSupportDirectoryURL(windowID: windowID, paneID: paneID, fileManager: fileManager)
        .appendingPathComponent("alan-binding.json")
}

enum AlanShellControlCommandKind: String, Codable {
    case state
    case spaceList = "space.list"
    case spaceCreate = "space.create"
    case spaceOpenAlan = "space.open_alan"
    case tabList = "tab.list"
    case tabOpen = "tab.open"
    case tabClose = "tab.close"
    case paneList = "pane.list"
    case paneSnapshot = "pane.snapshot"
    case paneSplit = "pane.split"
    case paneClose = "pane.close"
    case paneLift = "pane.lift"
    case paneMove = "pane.move"
    case paneFocus = "pane.focus"
    case paneSendText = "pane.send_text"
    case attentionInbox = "attention.inbox"
    case attentionSet = "attention.set"
    case routingCandidates = "routing.candidates"
    case eventsRead = "events.read"
}

struct AlanShellControlCommand: Codable {
    let requestID: String
    let command: AlanShellControlCommandKind
    let spaceID: String?
    let tabID: String?
    let paneID: String?
    let direction: ShellSplitDirection?
    let title: String?
    let cwd: String?
    let text: String?
    let attention: ShellAttentionState?
    let afterEventID: String?
    let limit: Int?

    private enum CodingKeys: String, CodingKey {
        case requestID = "request_id"
        case command
        case spaceID = "space_id"
        case tabID = "tab_id"
        case paneID = "pane_id"
        case direction
        case title
        case cwd
        case text
        case attention
        case afterEventID = "after_event_id"
        case limit
    }
}

struct AlanShellControlResponse: Codable {
    let requestID: String
    let contractVersion: String
    let applied: Bool?
    let state: ShellStateSnapshot?
    let spaces: [ShellSpace]?
    let tabs: [ShellTab]?
    let panes: [ShellPane]?
    let pane: ShellPane?
    let items: [AlanShellAttentionInboxItem]?
    let candidates: [AlanShellRoutingCandidate]?
    let events: [AlanShellEventEnvelope]?
    let focusedPaneID: String?
    let spaceID: String?
    let tabID: String?
    let paneID: String?
    let acceptedBytes: Int?
    let deliveryCode: String?
    let runtimePhase: String?
    let latestEventID: String?
    let errorCode: String?
    let errorMessage: String?

    private enum CodingKeys: String, CodingKey {
        case requestID = "request_id"
        case contractVersion = "contract_version"
        case applied
        case state
        case spaces
        case tabs
        case panes
        case pane
        case items
        case candidates
        case events
        case focusedPaneID = "focused_pane_id"
        case spaceID = "space_id"
        case tabID = "tab_id"
        case paneID = "pane_id"
        case acceptedBytes = "accepted_bytes"
        case deliveryCode = "delivery_code"
        case runtimePhase = "runtime_phase"
        case latestEventID = "latest_event_id"
        case errorCode = "error_code"
        case errorMessage = "error_message"
    }
}

struct AlanShellAttentionInboxItem: Codable, Equatable, Identifiable {
    let itemID: String
    let spaceID: String
    let tabID: String
    let paneID: String
    let attention: ShellAttentionState
    let summary: String

    var id: String { itemID }

    private enum CodingKeys: String, CodingKey {
        case itemID = "item_id"
        case spaceID = "space_id"
        case tabID = "tab_id"
        case paneID = "pane_id"
        case attention
        case summary
    }
}

struct AlanShellRoutingCandidate: Codable, Equatable, Identifiable {
    let paneID: String
    let score: Double
    let reasons: [String]

    var id: String { paneID }

    private enum CodingKeys: String, CodingKey {
        case paneID = "pane_id"
        case score
        case reasons
    }
}

enum AlanShellJSONValue: Codable, Equatable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case array([AlanShellJSONValue])
    case object([String: AlanShellJSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([AlanShellJSONValue].self) {
            self = .array(value)
        } else {
            self = .object(try container.decode([String: AlanShellJSONValue].self))
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case let .string(value):
            try container.encode(value)
        case let .number(value):
            try container.encode(value)
        case let .bool(value):
            try container.encode(value)
        case let .array(value):
            try container.encode(value)
        case let .object(value):
            try container.encode(value)
        case .null:
            try container.encodeNil()
        }
    }
}

struct AlanShellEventEnvelope: Codable, Equatable, Identifiable {
    let eventID: String
    let type: String
    let timestamp: String
    let windowID: String
    let spaceID: String?
    let tabID: String?
    let paneID: String?
    let payload: [String: AlanShellJSONValue]

    var id: String { eventID }

    private enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
        case timestamp
        case windowID = "window_id"
        case spaceID = "space_id"
        case tabID = "tab_id"
        case paneID = "pane_id"
        case payload
    }
}

struct AlanShellBindingProjection: Codable, Equatable {
    let sessionID: String
    let runStatus: String
    let pendingYield: Bool
    let source: String?
    let lastProjectedAt: String?
    let windowID: String?
    let spaceID: String?
    let tabID: String?
    let paneID: String?

    private enum CodingKeys: String, CodingKey {
        case sessionID = "session_id"
        case runStatus = "run_status"
        case pendingYield = "pending_yield"
        case source
        case lastProjectedAt = "last_projected_at"
        case windowID = "window_id"
        case spaceID = "space_id"
        case tabID = "tab_id"
        case paneID = "pane_id"
    }

    var shellBinding: ShellAlanBinding {
        ShellAlanBinding(
            sessionID: sessionID,
            runStatus: runStatus,
            pendingYield: pendingYield,
            source: source ?? "pane_binding_file",
            lastProjectedAt: lastProjectedAt
        )
    }
}

extension AlanShellBindingProjection {
    private enum LegacyCodingKeys: String, CodingKey {
        case surfaceID = "surface_id"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let legacyContainer = try decoder.container(keyedBy: LegacyCodingKeys.self)

        let tabID: String?
        if let decodedTabID = try container.decodeIfPresent(String.self, forKey: .tabID) {
            tabID = decodedTabID
        } else {
            tabID = try legacyContainer.decodeIfPresent(String.self, forKey: .surfaceID)
        }

        self.init(
            sessionID: try container.decode(String.self, forKey: .sessionID),
            runStatus: try container.decode(String.self, forKey: .runStatus),
            pendingYield: try container.decode(Bool.self, forKey: .pendingYield),
            source: try container.decodeIfPresent(String.self, forKey: .source),
            lastProjectedAt: try container.decodeIfPresent(String.self, forKey: .lastProjectedAt),
            windowID: try container.decodeIfPresent(String.self, forKey: .windowID),
            spaceID: try container.decodeIfPresent(String.self, forKey: .spaceID),
            tabID: tabID,
            paneID: try container.decodeIfPresent(String.self, forKey: .paneID)
        )
    }
}

enum AlanShellLocalCommandSideEffect {
    case sendText(paneID: String, text: String)
}

private struct AlanShellLocalCommandResult {
    let response: AlanShellControlResponse
    let updatedState: ShellStateSnapshot?
    let sideEffect: AlanShellLocalCommandSideEffect?
}

private enum AlanShellLocalCommandExecutor {
    static func execute(
        command: AlanShellControlCommand,
        state: ShellStateSnapshot
    ) -> AlanShellLocalCommandResult? {
        switch command.command {
        case .state:
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: state,
                    applied: true,
                    snapshot: state,
                    spaceID: state.focusedSpaceID,
                    tabID: state.focusedTabID,
                    paneID: state.focusedPaneID
                ),
                updatedState: nil,
                sideEffect: nil
            )

        case .spaceList:
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: state,
                    applied: true,
                    spaces: state.spaces,
                    spaceID: command.spaceID ?? state.focusedSpaceID
                ),
                updatedState: nil,
                sideEffect: nil
            )

        case .spaceCreate, .spaceOpenAlan:
            let launchTarget: ShellLaunchTarget = command.command == .spaceOpenAlan ? .alan : .shell
            let result = state.creatingSpace(
                launchTarget: launchTarget,
                title: command.title,
                workingDirectory: command.cwd
            )
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: result.state,
                    applied: true,
                    spaceID: result.spaceID,
                    tabID: result.tabID,
                    paneID: result.paneID
                ),
                updatedState: result.state,
                sideEffect: nil
            )

        case .tabList:
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: state,
                    applied: true,
                    tabs: state.tabs(in: command.spaceID),
                    spaceID: command.spaceID ?? state.focusedSpaceID,
                    tabID: state.focusedTabID
                ),
                updatedState: nil,
                sideEffect: nil
            )

        case .tabOpen:
            do {
                let result = try state.openingTerminalTab(
                    in: command.spaceID,
                    title: command.title,
                    workingDirectory: command.cwd
                )
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .tabClose:
            guard let tabID = command.tabID else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        tabID: command.tabID,
                        errorCode: "tab_required",
                        errorMessage: "tab_id is required."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }

            do {
                let result = try state.closingTab(tabID)
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .paneList:
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: state,
                    applied: true,
                    panes: state.panes(in: command.tabID),
                    tabID: command.tabID ?? state.focusedTabID
                ),
                updatedState: nil,
                sideEffect: nil
            )

        case .paneSnapshot:
            guard let paneID = command.paneID,
                  let pane = state.pane(paneID: paneID)
            else {
                return AlanShellLocalCommandResult(
                    response: failureResponse(
                        for: .paneNotFound,
                        command: command,
                        state: state
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: state,
                    applied: true,
                    pane: pane,
                    spaceID: pane.spaceID,
                    tabID: pane.tabID,
                    paneID: pane.paneID
                ),
                updatedState: nil,
                sideEffect: nil
            )

        case .paneSplit:
            guard let paneID = command.paneID else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        errorCode: "pane_required",
                        errorMessage: "pane_id is required."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            guard let direction = command.direction else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        paneID: paneID,
                        errorCode: "direction_required",
                        errorMessage: "direction is required for pane.split."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            do {
                let result = try state.splittingPane(paneID, direction: direction)
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .paneClose:
            guard let paneID = command.paneID else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        errorCode: "pane_required",
                        errorMessage: "pane_id is required."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            do {
                let result = try state.closingPane(paneID)
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .paneLift:
            guard let paneID = command.paneID else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        errorCode: "pane_required",
                        errorMessage: "pane_id is required."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            do {
                let result = try state.movingPaneToNewTab(paneID, title: command.title)
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .paneMove:
            guard let paneID = command.paneID,
                  let targetTabID = command.tabID
            else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        tabID: command.tabID,
                        paneID: command.paneID,
                        errorCode: "pane_move_target_required",
                        errorMessage: "pane_id and tab_id are required."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            do {
                let result = try state.movingPane(
                    paneID,
                    toTab: targetTabID,
                    direction: command.direction ?? .vertical
                )
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .paneFocus:
            guard let paneID = command.paneID else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        errorCode: "pane_required",
                        errorMessage: "pane_id is required."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            do {
                let result = try state.focusingPane(paneID)
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .paneSendText:
            return nil

        case .attentionInbox:
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: state,
                    applied: true,
                    items: attentionInboxItems(from: state)
                ),
                updatedState: nil,
                sideEffect: nil
            )

        case .attentionSet:
            guard let paneID = command.paneID,
                  let attention = command.attention
            else {
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: state,
                        applied: false,
                        errorCode: "attention_target_required",
                        errorMessage: "pane_id and attention are required."
                    ),
                    updatedState: nil,
                    sideEffect: nil
                )
            }
            do {
                let result = try state.settingAttention(attention, for: paneID)
                return AlanShellLocalCommandResult(
                    response: response(
                        for: command,
                        state: result.state,
                        applied: true,
                        spaceID: result.spaceID,
                        tabID: result.tabID,
                        paneID: result.paneID
                    ),
                    updatedState: result.state,
                    sideEffect: nil
                )
            } catch let error as ShellStateMutationError {
                return AlanShellLocalCommandResult(
                    response: failureResponse(for: error, command: command, state: state),
                    updatedState: nil,
                    sideEffect: nil
                )
            } catch {
                return nil
            }

        case .routingCandidates:
            return AlanShellLocalCommandResult(
                response: response(
                    for: command,
                    state: state,
                    applied: true,
                    candidates: routingCandidates(from: state, preferredPaneID: command.paneID)
                ),
                updatedState: nil,
                sideEffect: nil
            )

        case .eventsRead:
            return nil
        }
    }

    private static func failureResponse(
        for error: ShellStateMutationError,
        command: AlanShellControlCommand,
        state: ShellStateSnapshot
    ) -> AlanShellControlResponse {
        switch error {
        case .spaceNotFound:
            return response(
                for: command,
                state: state,
                applied: false,
                spaceID: command.spaceID,
                errorCode: error.rawValue,
                errorMessage: "The requested space does not exist."
            )
        case .tabNotFound:
            return response(
                for: command,
                state: state,
                applied: false,
                tabID: command.tabID,
                errorCode: error.rawValue,
                errorMessage: "The requested tab does not exist."
            )
        case .paneNotFound:
            return response(
                for: command,
                state: state,
                applied: false,
                paneID: command.paneID,
                errorCode: error.rawValue,
                errorMessage: "The requested pane does not exist."
            )
        case .lastTab:
            return response(
                for: command,
                state: state,
                applied: false,
                tabID: command.tabID,
                errorCode: error.rawValue,
                errorMessage: "Alan Shell must keep at least one tab open."
            )
        case .lastPane:
            return response(
                for: command,
                state: state,
                applied: false,
                paneID: command.paneID,
                errorCode: error.rawValue,
                errorMessage: "This action requires the pane to have at least one sibling."
            )
        case .invalidMoveTarget:
            return response(
                for: command,
                state: state,
                applied: false,
                tabID: command.tabID,
                paneID: command.paneID,
                errorCode: error.rawValue,
                errorMessage: "The pane cannot be moved onto its current tab."
            )
        }
    }

    private static func response(
        for command: AlanShellControlCommand,
        state: ShellStateSnapshot,
        applied: Bool,
        snapshot: ShellStateSnapshot? = nil,
        spaces: [ShellSpace]? = nil,
        tabs: [ShellTab]? = nil,
        panes: [ShellPane]? = nil,
        pane: ShellPane? = nil,
        items: [AlanShellAttentionInboxItem]? = nil,
        candidates: [AlanShellRoutingCandidate]? = nil,
        events: [AlanShellEventEnvelope]? = nil,
        spaceID: String? = nil,
        tabID: String? = nil,
        paneID: String? = nil,
        acceptedBytes: Int? = nil,
        deliveryCode: String? = nil,
        runtimePhase: String? = nil,
        latestEventID: String? = nil,
        errorCode: String? = nil,
        errorMessage: String? = nil
    ) -> AlanShellControlResponse {
        AlanShellControlResponse(
            requestID: command.requestID,
            contractVersion: state.contractVersion,
            applied: applied,
            state: snapshot,
            spaces: spaces,
            tabs: tabs,
            panes: panes,
            pane: pane,
            items: items,
            candidates: candidates,
            events: events,
            focusedPaneID: state.focusedPaneID,
            spaceID: spaceID,
            tabID: tabID,
            paneID: paneID,
            acceptedBytes: acceptedBytes,
            deliveryCode: deliveryCode,
            runtimePhase: runtimePhase,
            latestEventID: latestEventID,
            errorCode: errorCode,
            errorMessage: errorMessage
        )
    }
}

private enum AlanShellPublishedStateMerger {
    static func merge(
        authoritative: ShellStateSnapshot?,
        incoming: ShellStateSnapshot
    ) -> ShellStateSnapshot {
        guard let authoritative else { return incoming }

        // Preserve richer metadata for panes that still exist, but never
        // resurrect panes or tabs that the incoming snapshot removed.
        let authoritativePanesByID = Dictionary(
            uniqueKeysWithValues: authoritative.panes.map { ($0.paneID, $0) }
        )
        let mergedPanes = incoming.panes.map { pane in
            merge(authoritativePane: authoritativePanesByID[pane.paneID], incomingPane: pane)
        }
        let focusedPaneID = incoming.focusedPaneID ?? authoritative.focusedPaneID
        let focusedPane = focusedPaneID.flatMap { candidate in
            mergedPanes.first(where: { $0.paneID == candidate })
        }
        let mergedSpaces = incoming.spaces.map { space in
            ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: strongestAttention(in: mergedPanes.filter { $0.spaceID == space.spaceID }),
                tabs: space.tabs
            )
        }

        return ShellStateSnapshot(
            contractVersion: incoming.contractVersion,
            windowID: incoming.windowID,
            focusedSpaceID: focusedPane?.spaceID ?? incoming.focusedSpaceID ?? authoritative.focusedSpaceID,
            focusedTabID: focusedPane?.tabID ?? incoming.focusedTabID ?? authoritative.focusedTabID,
            focusedPaneID: focusedPane?.paneID ?? focusedPaneID,
            spaces: mergedSpaces,
            panes: mergedPanes
        )
    }

    private static func merge(
        authoritativePane: ShellPane?,
        incomingPane: ShellPane
    ) -> ShellPane {
        guard let authoritativePane else { return incomingPane }

        return ShellPane(
            paneID: incomingPane.paneID,
            tabID: incomingPane.tabID,
            spaceID: incomingPane.spaceID,
            launchTarget: incomingPane.launchTarget ?? authoritativePane.launchTarget,
            cwd: incomingPane.cwd ?? authoritativePane.cwd,
            process: incomingPane.process ?? authoritativePane.process,
            attention: incomingPane.attention,
            context: merge(authoritativeContext: authoritativePane.context, incomingContext: incomingPane.context),
            viewport: merge(authoritativeViewport: authoritativePane.viewport, incomingViewport: incomingPane.viewport),
            alanBinding: incomingPane.alanBinding ?? authoritativePane.alanBinding
        )
    }

    private static func merge(
        authoritativeContext: ShellContextSnapshot?,
        incomingContext: ShellContextSnapshot?
    ) -> ShellContextSnapshot? {
        guard authoritativeContext != nil || incomingContext != nil else { return nil }
        let workingDirectoryName =
            incomingContext?.workingDirectoryName ?? authoritativeContext?.workingDirectoryName
        let repositoryRoot =
            incomingContext?.repositoryRoot ?? authoritativeContext?.repositoryRoot
        let gitBranch = incomingContext?.gitBranch ?? authoritativeContext?.gitBranch
        let controlPath = incomingContext?.controlPath ?? authoritativeContext?.controlPath
        let socketPath = incomingContext?.socketPath ?? authoritativeContext?.socketPath
        let alanBindingFile =
            incomingContext?.alanBindingFile ?? authoritativeContext?.alanBindingFile
        let launchCommand =
            incomingContext?.launchCommand ?? authoritativeContext?.launchCommand
        let launchStrategy =
            incomingContext?.launchStrategy ?? authoritativeContext?.launchStrategy
        let shellIntegrationSource =
            incomingContext?.shellIntegrationSource ?? authoritativeContext?.shellIntegrationSource
        let processState = incomingContext?.processState ?? authoritativeContext?.processState
        let rendererPhase = incomingContext?.rendererPhase ?? authoritativeContext?.rendererPhase
        let rendererHealth =
            incomingContext?.rendererHealth ?? authoritativeContext?.rendererHealth
        let surfaceReadiness =
            incomingContext?.surfaceReadiness ?? authoritativeContext?.surfaceReadiness
        let inputReady = incomingContext?.inputReady ?? authoritativeContext?.inputReady
        let readonly = incomingContext?.readonly ?? authoritativeContext?.readonly
        let terminalMode = incomingContext?.terminalMode ?? authoritativeContext?.terminalMode
        let displayName = incomingContext?.displayName ?? authoritativeContext?.displayName
        let displayID = incomingContext?.displayID ?? authoritativeContext?.displayID
        let windowTitle = incomingContext?.windowTitle ?? authoritativeContext?.windowTitle
        let lastMetadataAt =
            incomingContext?.lastMetadataAt ?? authoritativeContext?.lastMetadataAt
        let lastCommandExitCode =
            incomingContext?.lastCommandExitCode ?? authoritativeContext?.lastCommandExitCode

        return ShellContextSnapshot(
            workingDirectoryName: workingDirectoryName,
            repositoryRoot: repositoryRoot,
            gitBranch: gitBranch,
            controlPath: controlPath,
            socketPath: socketPath,
            alanBindingFile: alanBindingFile,
            launchCommand: launchCommand,
            launchStrategy: launchStrategy,
            shellIntegrationSource: shellIntegrationSource,
            processState: processState,
            rendererPhase: rendererPhase,
            rendererHealth: rendererHealth,
            surfaceReadiness: surfaceReadiness,
            inputReady: inputReady,
            readonly: readonly,
            terminalMode: terminalMode,
            displayName: displayName,
            displayID: displayID,
            windowTitle: windowTitle,
            lastMetadataAt: lastMetadataAt,
            lastCommandExitCode: lastCommandExitCode
        )
    }

    private static func merge(
        authoritativeViewport: ShellViewportSnapshot?,
        incomingViewport: ShellViewportSnapshot?
    ) -> ShellViewportSnapshot? {
        guard authoritativeViewport != nil || incomingViewport != nil else { return nil }
        return ShellViewportSnapshot(
            title: incomingViewport?.title ?? authoritativeViewport?.title,
            summary: incomingViewport?.summary ?? authoritativeViewport?.summary,
            visibleExcerpt: incomingViewport?.visibleExcerpt ?? authoritativeViewport?.visibleExcerpt,
            lastActivityAt: incomingViewport?.lastActivityAt ?? authoritativeViewport?.lastActivityAt
        )
    }

    private static func strongestAttention(in panes: [ShellPane]) -> ShellAttentionState {
        panes
            .map(\.attention)
            .max(by: { attentionRank(for: $0) < attentionRank(for: $1) })
            ?? .idle
    }

    private static func attentionRank(for attention: ShellAttentionState) -> Int {
        switch attention {
        case .idle:
            return 0
        case .active:
            return 1
        case .notable:
            return 2
        case .awaitingUser:
            return 3
        }
    }
}

private func attentionInboxItems(from state: ShellStateSnapshot) -> [AlanShellAttentionInboxItem] {
    state.panes
        .filter { $0.attention != .idle }
        .sorted {
            attentionRank(for: $0.attention) == attentionRank(for: $1.attention)
                ? $0.paneID < $1.paneID
                : attentionRank(for: $0.attention) > attentionRank(for: $1.attention)
        }
        .map { pane in
            AlanShellAttentionInboxItem(
                itemID: "attn_\(pane.paneID)",
                spaceID: pane.spaceID,
                tabID: pane.tabID,
                paneID: pane.paneID,
                attention: pane.attention,
                summary: pane.viewport?.summary
                    ?? pane.alanBinding.map { $0.pendingYield ? "Alan is waiting for user input" : "Alan run status: \($0.runStatus)" }
                    ?? pane.process?.program
                    ?? "Activity detected"
            )
        }
}

private func routingCandidates(
    from state: ShellStateSnapshot,
    preferredPaneID: String?
) -> [AlanShellRoutingCandidate] {
    let preferredPane = preferredPaneID.flatMap(state.pane(paneID:))
    let focusedPane = state.focusedPaneID.flatMap(state.pane(paneID:))

    return state.panes.map { pane in
        var score = 0.0
        var reasons: [String] = []

        if pane.paneID == preferredPaneID {
            score += 0.4
            reasons.append("requested")
        }
        if pane.paneID == state.focusedPaneID {
            score += 0.3
            reasons.append("focused")
        }
        if pane.attention == .awaitingUser {
            score += 0.25
            reasons.append("attention:awaiting_user")
        } else if pane.attention == .notable {
            score += 0.12
            reasons.append("attention:notable")
        }
        if pane.alanBinding?.pendingYield == true {
            score += 0.2
            reasons.append("alan_binding:yielded")
        } else if let runStatus = pane.alanBinding?.runStatus {
            score += 0.08
            reasons.append("alan_binding:\(runStatus)")
        }
        if let preferredPane, pane.tabID == preferredPane.tabID {
            score += 0.1
            reasons.append("same_tab")
        } else if let focusedPane, pane.tabID == focusedPane.tabID {
            score += 0.08
            reasons.append("same_tab")
        }
        if let preferredPane, pane.spaceID == preferredPane.spaceID {
            score += 0.05
            reasons.append("same_space")
        } else if let focusedPane, pane.spaceID == focusedPane.spaceID {
            score += 0.04
            reasons.append("same_space")
        }
        if let process = pane.process?.program {
            reasons.append("process:\(process)")
        }

        return AlanShellRoutingCandidate(
            paneID: pane.paneID,
            score: min(score, 1.0),
            reasons: Array(Set(reasons)).sorted()
        )
    }
    .sorted {
        $0.score == $1.score ? $0.paneID < $1.paneID : $0.score > $1.score
    }
}

private func attentionRank(for attention: ShellAttentionState) -> Int {
    switch attention {
    case .idle:
        return 0
    case .active:
        return 1
    case .notable:
        return 2
    case .awaitingUser:
        return 3
    }
}

final class AlanShellSocketServer {
    private static let maxRequestBytes = 1_048_576
    private static let readTimeoutSeconds = 5
    private static let commandResponseTimeoutSeconds: TimeInterval = 5
    private static let maxConcurrentClients = 4

    private let socketURL: URL
    private let queue = DispatchQueue(label: "dev.alan.shell.control.socket", qos: .userInitiated)
    private let clientQueue = DispatchQueue(
        label: "dev.alan.shell.control.socket.clients",
        qos: .userInitiated,
        attributes: .concurrent
    )
    private let clientSemaphore = DispatchSemaphore(value: AlanShellSocketServer.maxConcurrentClients)
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let commandHandler: (AlanShellControlCommand) -> AlanShellControlResponse
    private let stateAdoptionHandler: (ShellStateSnapshot) -> Void
    private let sideEffectHandler: (AlanShellLocalCommandSideEffect) -> Void
    private let debugEnabled = ProcessInfo.processInfo.environment["ALAN_SHELL_DEBUG_SOCKET"] == "1"
    private let stateLock = NSLock()
    private var listeningFileDescriptor: Int32 = -1
    private var isRunning = false
    private var lastCachedState: ShellStateSnapshot?
    private var lastPublishedState: ShellStateSnapshot?

    init(
        socketURL: URL,
        commandHandler: @escaping (AlanShellControlCommand) -> AlanShellControlResponse,
        stateAdoptionHandler: @escaping (ShellStateSnapshot) -> Void,
        sideEffectHandler: @escaping (AlanShellLocalCommandSideEffect) -> Void
    ) {
        self.socketURL = socketURL
        self.commandHandler = commandHandler
        self.stateAdoptionHandler = stateAdoptionHandler
        self.sideEffectHandler = sideEffectHandler
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    }

    deinit {
        stop()
    }

    @discardableResult
    func mergePublishedState(
        _ state: ShellStateSnapshot
    ) -> (previous: ShellStateSnapshot?, merged: ShellStateSnapshot) {
        stateLock.lock()
        let previousState = lastPublishedState
        let mergedState = AlanShellPublishedStateMerger.merge(
            authoritative: lastCachedState ?? lastPublishedState,
            incoming: state
        )
        lastCachedState = mergedState
        lastPublishedState = mergedState
        stateLock.unlock()
        return (previousState, mergedState)
    }

    func start() {
        stop()
        try? FileManager.default.createDirectory(
            at: socketURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        unlink(socketURL.path)

        let socketFD = socket(AF_UNIX, SOCK_STREAM, 0)
        guard socketFD >= 0 else { return }
        guard bindSocket(fileDescriptor: socketFD) else {
            close(socketFD)
            unlink(socketURL.path)
            return
        }
        guard listen(socketFD, 16) == 0 else {
            close(socketFD)
            unlink(socketURL.path)
            return
        }

        debugLog("socket start path=\(socketURL.path) fd=\(socketFD)")
        listeningFileDescriptor = socketFD
        isRunning = true
        queue.async { [weak self] in
            self?.acceptLoop(fileDescriptor: socketFD)
        }
    }

    func stop() {
        isRunning = false
        if listeningFileDescriptor >= 0 {
            debugLog("socket stop fd=\(listeningFileDescriptor)")
            close(listeningFileDescriptor)
            unlink(socketURL.path)
        }
        listeningFileDescriptor = -1
    }

    private func bindSocket(fileDescriptor: Int32) -> Bool {
        var address = sockaddr_un()
        address.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
        address.sun_family = sa_family_t(AF_UNIX)

        let pathBytes = socketURL.path.utf8CString
        let maxPathLength = MemoryLayout.size(ofValue: address.sun_path)
        guard pathBytes.count <= maxPathLength else { return false }

        withUnsafeMutablePointer(to: &address.sun_path.0) { pointer in
            pointer.initialize(repeating: 0, count: maxPathLength)
            pathBytes.withUnsafeBufferPointer { bytes in
                guard let source = bytes.baseAddress else { return }
                pointer.update(from: source, count: bytes.count)
            }
        }

        var bindResult: Int32 = -1
        withUnsafePointer(to: &address) { addressPointer in
            addressPointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { socketAddress in
                bindResult = Darwin.bind(
                    fileDescriptor,
                    socketAddress,
                    socklen_t(MemoryLayout<sockaddr_un>.size)
                )
            }
        }

        return bindResult == 0
    }

    private func acceptLoop(fileDescriptor: Int32) {
        while isRunning {
            let clientFD = accept(fileDescriptor, nil, nil)
            if clientFD == -1 {
                if errno == EINTR {
                    continue
                }
                if !isRunning || errno == EBADF || errno == EINVAL {
                    debugLog("socket accept loop exit errno=\(errno)")
                    return
                }
                debugLog("socket accept retry errno=\(errno)")
                continue
            }

            debugLog("socket accepted client fd=\(clientFD)")
            configureClient(fileDescriptor: clientFD)
            clientQueue.async { [weak self] in
                guard let self else {
                    AlanShellSocketServer.closeClient(clientFD)
                    return
                }
                guard self.clientSemaphore.wait(timeout: .now() + 5) == .success else {
                    self.debugLog("socket client rejected by concurrency limit fd=\(clientFD)")
                    AlanShellSocketServer.closeClient(clientFD)
                    return
                }
                defer { self.clientSemaphore.signal() }
                self.handleClient(fileDescriptor: clientFD)
            }
        }
    }

    private func configureClient(fileDescriptor: Int32) {
        var noSigPipe: Int32 = 1
        setsockopt(
            fileDescriptor,
            SOL_SOCKET,
            SO_NOSIGPIPE,
            &noSigPipe,
            socklen_t(MemoryLayout<Int32>.size)
        )

        var timeout = timeval(tv_sec: Self.readTimeoutSeconds, tv_usec: 0)
        setsockopt(
            fileDescriptor,
            SOL_SOCKET,
            SO_RCVTIMEO,
            &timeout,
            socklen_t(MemoryLayout<timeval>.size)
        )
        setsockopt(
            fileDescriptor,
            SOL_SOCKET,
            SO_SNDTIMEO,
            &timeout,
            socklen_t(MemoryLayout<timeval>.size)
        )
    }

    private func handleClient(fileDescriptor: Int32) {
        guard let requestData = readRequest(from: fileDescriptor),
              let command = try? decoder.decode(AlanShellControlCommand.self, from: requestData)
        else {
            debugLog("socket decode failed fd=\(fileDescriptor)")
            AlanShellSocketServer.closeClient(fileDescriptor)
            return
        }

        debugLog("socket command=\(command.command) fd=\(fileDescriptor)")
        if let localResponse = handleLocally(command) {
            let responseData = (try? encoder.encode(localResponse)) ?? Data()
            debugLog("socket cached response bytes=\(responseData.count) fd=\(fileDescriptor)")
            AlanShellSocketServer.write(responseData, to: fileDescriptor)
            AlanShellSocketServer.write(Data([0x0A]), to: fileDescriptor)
            AlanShellSocketServer.closeClient(fileDescriptor)
            return
        }

        let semaphore = DispatchSemaphore(value: 0)
        var response: AlanShellControlResponse?
        DispatchQueue.main.async { [weak self] in
            guard let self else {
                semaphore.signal()
                return
            }
            response = self.commandHandler(command)
            semaphore.signal()
        }
        if semaphore.wait(timeout: .now() + Self.commandResponseTimeoutSeconds) != .success {
            debugLog("socket response timeout fd=\(fileDescriptor)")
            let timeoutResponse = AlanShellControlResponse(
                requestID: command.requestID,
                contractVersion: "0.1",
                applied: false,
                state: nil,
                spaces: nil,
                tabs: nil,
                panes: nil,
                pane: nil,
                items: nil,
                candidates: nil,
                events: nil,
                focusedPaneID: nil,
                spaceID: command.spaceID,
                tabID: command.tabID,
                paneID: command.paneID,
                acceptedBytes: nil,
                deliveryCode: TerminalRuntimeDeliveryCode.timeout.rawValue,
                runtimePhase: nil,
                latestEventID: nil,
                errorCode: "command_timeout",
                errorMessage: "Alan Shell control command timed out."
            )
            let responseData = (try? encoder.encode(timeoutResponse)) ?? Data()
            AlanShellSocketServer.write(responseData, to: fileDescriptor)
            AlanShellSocketServer.write(Data([0x0A]), to: fileDescriptor)
            AlanShellSocketServer.closeClient(fileDescriptor)
            return
        }

        let resolvedResponse =
            response
            ?? AlanShellControlResponse(
                requestID: command.requestID,
                contractVersion: "0.1",
                applied: false,
                state: nil,
                spaces: nil,
                tabs: nil,
                panes: nil,
                pane: nil,
                items: nil,
                candidates: nil,
                events: nil,
                focusedPaneID: nil,
                spaceID: command.spaceID,
                tabID: command.tabID,
                paneID: command.paneID,
                acceptedBytes: nil,
                deliveryCode: nil,
                runtimePhase: nil,
                latestEventID: nil,
                errorCode: "host_unavailable",
                errorMessage: "Alan Shell host is unavailable."
            )
        let responseData = (try? encoder.encode(resolvedResponse)) ?? Data()
        debugLog("socket response bytes=\(responseData.count) fd=\(fileDescriptor)")
        AlanShellSocketServer.write(responseData, to: fileDescriptor)
        AlanShellSocketServer.write(Data([0x0A]), to: fileDescriptor)
        AlanShellSocketServer.closeClient(fileDescriptor)
    }

    private func readRequest(from fileDescriptor: Int32) -> Data? {
        var data = Data()
        var buffer = [UInt8](repeating: 0, count: 4096)

        while true {
            let bytesRead = read(fileDescriptor, &buffer, buffer.count)
            if bytesRead > 0 {
                data.append(buffer, count: bytesRead)
                guard data.count <= Self.maxRequestBytes else {
                    debugLog("socket request too large fd=\(fileDescriptor) bytes=\(data.count)")
                    return nil
                }
                if data.contains(0x0A) {
                    debugLog("socket read newline fd=\(fileDescriptor) bytes=\(data.count)")
                    break
                }
                continue
            }

            if bytesRead == 0 {
                debugLog("socket read eof fd=\(fileDescriptor) bytes=\(data.count)")
                break
            }

            if errno == EINTR {
                continue
            }

            debugLog("socket read error fd=\(fileDescriptor) errno=\(errno)")
            return nil
        }

        if let newlineIndex = data.firstIndex(of: 0x0A) {
            data = data.prefix(upTo: newlineIndex)
        }

        return data.isEmpty ? nil : data
    }

    private static func write(_ data: Data, to fileDescriptor: Int32) {
        data.withUnsafeBytes { bytes in
            guard let baseAddress = bytes.baseAddress else { return }
            var offset = 0
            while offset < bytes.count {
                let written = Darwin.write(
                    fileDescriptor,
                    baseAddress.advanced(by: offset),
                    bytes.count - offset
                )
                if written > 0 {
                    offset += written
                    continue
                }
                if written == -1 && errno == EINTR {
                    continue
                }
                break
            }
        }
    }

    private static func closeClient(_ fileDescriptor: Int32) {
        shutdown(fileDescriptor, SHUT_RDWR)
        close(fileDescriptor)
    }

    func handleLocally(_ command: AlanShellControlCommand) -> AlanShellControlResponse? {
        let localResult: AlanShellLocalCommandResult? = {
            stateLock.lock()
            defer { stateLock.unlock() }
            guard let state = lastCachedState ?? lastPublishedState else { return nil }
            let result = AlanShellLocalCommandExecutor.execute(command: command, state: state)
            if let updatedState = result?.updatedState {
                lastCachedState = updatedState
            }
            return result
        }()

        guard let localResult else { return nil }
        if let updatedState = localResult.updatedState {
            if Thread.isMainThread {
                stateAdoptionHandler(updatedState)
            } else {
                DispatchQueue.main.sync {
                    stateAdoptionHandler(updatedState)
                }
            }
        }
        if let sideEffect = localResult.sideEffect {
            sideEffectHandler(sideEffect)
        }
        return localResult.response
    }

    private func debugLog(_ message: String) {
        guard debugEnabled else { return }
        let logURL = FileManager.default.temporaryDirectory.appendingPathComponent("alan-shell-socket-debug.log")
        let line = "[\(ISO8601DateFormatter().string(from: .now))] \(message)\n"
        if FileManager.default.fileExists(atPath: logURL.path) {
            if let handle = try? FileHandle(forWritingTo: logURL) {
                _ = try? handle.seekToEnd()
                try? handle.write(contentsOf: Data(line.utf8))
                try? handle.close()
            }
        } else {
            try? Data(line.utf8).write(to: logURL, options: .atomic)
        }
    }
}

@MainActor
final class AlanShellControlPlane {
    private let windowID: String
    private let fileManager: FileManager
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder
    private let rootURL: URL
    private let socketURL: URL
    private let panesURL: URL
    private let commandsURL: URL
    private let resultsURL: URL
    private let stateFileURL: URL
    private let eventsFileURL: URL
    private let commandHandler: (AlanShellControlCommand) -> AlanShellControlResponse
    private let stateAdoptionHandler: @MainActor (ShellStateSnapshot) -> Void
    private let bindingProjectionHandler: @MainActor (String, ShellAlanBinding?) -> Void
    private let diagnosticHandler: @MainActor (String) -> Void
    private let socketServer: AlanShellSocketServer
    private var pollSource: DispatchSourceTimer?
    private var trackedPaneIDs: Set<String> = []
    private var lastBindingPayloadByPaneID: [String: Data] = [:]
    private var events: [AlanShellEventEnvelope] = []
    private var nextEventOrdinal = 1

    init(
        windowID: String,
        fileManager: FileManager = .default,
        commandHandler: @escaping (AlanShellControlCommand) -> AlanShellControlResponse,
        stateAdoptionHandler: @escaping @MainActor (ShellStateSnapshot) -> Void,
        bindingProjectionHandler: @escaping @MainActor (String, ShellAlanBinding?) -> Void,
        diagnosticHandler: @escaping @MainActor (String) -> Void = { _ in }
    ) {
        self.windowID = windowID
        self.fileManager = fileManager
        self.encoder = JSONEncoder()
        self.encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        self.decoder = JSONDecoder()
        self.rootURL = alanShellControlPlaneRootURL(windowID: windowID, fileManager: fileManager)
        self.socketURL = alanShellControlPlaneSocketURL(windowID: windowID, fileManager: fileManager)
        self.panesURL = rootURL.appendingPathComponent("panes", isDirectory: true)
        self.commandsURL = rootURL.appendingPathComponent("commands", isDirectory: true)
        self.resultsURL = rootURL.appendingPathComponent("results", isDirectory: true)
        self.stateFileURL = rootURL.appendingPathComponent("state.json")
        self.eventsFileURL = rootURL.appendingPathComponent("events.jsonl")
        self.commandHandler = commandHandler
        self.stateAdoptionHandler = stateAdoptionHandler
        self.bindingProjectionHandler = bindingProjectionHandler
        self.diagnosticHandler = diagnosticHandler
        self.socketServer = AlanShellSocketServer(
            socketURL: self.socketURL,
            commandHandler: commandHandler,
            stateAdoptionHandler: { state in
                Task { @MainActor in
                    stateAdoptionHandler(state)
                }
            },
            sideEffectHandler: { _ in }
        )

        ensureDirectories()
        socketServer.start()
        startPolling()
    }

    deinit {
        pollSource?.cancel()
        socketServer.stop()
    }

    var rootPath: String {
        rootURL.path
    }

    var stateFilePath: String {
        stateFileURL.path
    }

    var commandsPath: String {
        commandsURL.path
    }

    var resultsPath: String {
        resultsURL.path
    }

    var socketPath: String {
        socketURL.path
    }

    func publish(state: ShellStateSnapshot) {
        ensureDirectories()
        let mergeResult = socketServer.mergePublishedState(state)
        let mergedState = mergeResult.merged
        synchronizePaneSupportDirectories(for: mergedState)
        recordEvents(from: mergeResult.previous, to: mergedState)
        do {
            let data = try encoder.encode(mergedState)
            try data.write(to: stateFileURL, options: .atomic)
        } catch {
            recordDiagnostic("Failed to persist shell state: \(error.localizedDescription)")
        }
    }

    private func startPolling() {
        pollSource?.cancel()
        let source = DispatchSource.makeTimerSource(queue: DispatchQueue(label: "dev.alan.shell.control.poll"))
        source.schedule(deadline: .now() + .milliseconds(250), repeating: .milliseconds(250), leeway: .milliseconds(100))
        source.setEventHandler { [weak self] in
            Task { @MainActor [weak self] in
                self?.pollCommands()
                self?.pollBindings()
            }
        }
        source.resume()
        pollSource = source
    }

    private func ensureDirectories() {
        [rootURL, panesURL, commandsURL, resultsURL].forEach { url in
            do {
                try fileManager.createDirectory(at: url, withIntermediateDirectories: true)
            } catch {
                recordDiagnostic("Failed to create shell control directory \(url.path): \(error.localizedDescription)")
            }
        }
    }

    private func recordDiagnostic(_ message: String) {
        diagnosticHandler(message)
    }

    func specialCommandResponse(for command: AlanShellControlCommand) -> AlanShellControlResponse? {
        guard command.command == .eventsRead else { return nil }
        let rows = readEvents(afterEventID: command.afterEventID, limit: command.limit)
        return AlanShellControlResponse(
            requestID: command.requestID,
            contractVersion: "0.1",
            applied: true,
            state: nil,
            spaces: nil,
            tabs: nil,
            panes: nil,
            pane: nil,
            items: nil,
            candidates: nil,
            events: rows,
            focusedPaneID: nil,
            spaceID: nil,
            tabID: nil,
            paneID: nil,
            acceptedBytes: nil,
            deliveryCode: nil,
            runtimePhase: nil,
            latestEventID: events.last?.eventID,
            errorCode: nil,
            errorMessage: nil
        )
    }

    func recordTextDelivery(
        requestID: String,
        spaceID: String?,
        tabID: String?,
        paneID: String,
        delivery: TerminalRuntimeDeliveryResult
    ) {
        var payload: [String: AlanShellJSONValue] = [
            "request_id": .string(requestID),
            "delivery_code": .string(delivery.code.rawValue),
            "accepted_bytes": .number(Double(delivery.acceptedBytes))
        ]
        if let errorCode = delivery.errorCode {
            payload["error_code"] = .string(errorCode)
        }
        if let errorMessage = delivery.errorMessage {
            payload["error_message"] = .string(errorMessage)
        }
        if let runtimePhase = delivery.runtimePhase {
            payload["runtime_phase"] = .string(runtimePhase)
        }

        appendEvent(
            type: "pane.text_delivery",
            spaceID: spaceID,
            tabID: tabID,
            paneID: paneID,
            payload: payload
        )
    }

    private func synchronizePaneSupportDirectories(for state: ShellStateSnapshot) {
        let paneIDs = Set(state.panes.map(\.paneID))
        let previousPaneIDs = trackedPaneIDs
        trackedPaneIDs = paneIDs

        for paneID in paneIDs {
            let paneURL = alanShellPaneSupportDirectoryURL(
                windowID: windowID,
                paneID: paneID,
                fileManager: fileManager
            )
            do {
                try fileManager.createDirectory(at: paneURL, withIntermediateDirectories: true)
            } catch {
                recordDiagnostic("Failed to create pane support directory \(paneURL.path): \(error.localizedDescription)")
            }
        }

        let stalePaneIDs = Set(lastBindingPayloadByPaneID.keys).subtracting(paneIDs)
        for paneID in stalePaneIDs {
            lastBindingPayloadByPaneID.removeValue(forKey: paneID)
        }

        for paneID in previousPaneIDs.subtracting(paneIDs) {
            let paneURL = alanShellPaneSupportDirectoryURL(
                windowID: windowID,
                paneID: paneID,
                fileManager: fileManager
            )
            do {
                try fileManager.removeItem(at: paneURL)
            } catch {
                recordDiagnostic("Failed to remove stale pane support directory \(paneURL.path): \(error.localizedDescription)")
            }
        }
    }

    private func recordEvents(from previousState: ShellStateSnapshot?, to currentState: ShellStateSnapshot) {
        guard let previousState else { return }

        let previousPanesByID = Dictionary(uniqueKeysWithValues: previousState.panes.map { ($0.paneID, $0) })
        let currentPanesByID = Dictionary(uniqueKeysWithValues: currentState.panes.map { ($0.paneID, $0) })

        if previousState.focusedPaneID != currentState.focusedPaneID {
            appendEvent(
                type: "focus.changed",
                spaceID: currentState.focusedSpaceID,
                tabID: currentState.focusedTabID,
                paneID: currentState.focusedPaneID,
                payload: [
                    "previous_pane_id": .string(previousState.focusedPaneID ?? ""),
                    "current_pane_id": .string(currentState.focusedPaneID ?? "")
                ]
            )
        }

        let previousTabs = Set(previousState.spaces.flatMap(\.tabs).map(\.tabID))
        let currentTabs = Set(currentState.spaces.flatMap(\.tabs).map(\.tabID))
        for createdTabID in currentTabs.subtracting(previousTabs).sorted() {
            if let tab = currentState.tab(tabID: createdTabID),
               let paneID = tab.paneTree.paneIDs.first,
               let pane = currentPanesByID[paneID] {
                appendEvent(
                    type: "tab.created",
                    spaceID: pane.spaceID,
                    tabID: tab.tabID,
                    paneID: paneID,
                    payload: [
                        "tab_id": .string(tab.tabID),
                        "kind": .string(tab.kind.rawValue)
                    ]
                )
            }
        }
        for closedTabID in previousTabs.subtracting(currentTabs).sorted() {
            let pane = previousState.panes.first { $0.tabID == closedTabID }
            appendEvent(
                type: "tab.closed",
                spaceID: pane?.spaceID,
                tabID: closedTabID,
                paneID: pane?.paneID,
                payload: ["tab_id": .string(closedTabID)]
            )
        }

        let allPaneIDs = Set(previousPanesByID.keys).union(currentPanesByID.keys)
        for paneID in allPaneIDs.sorted() {
            let previousPane = previousPanesByID[paneID]
            let currentPane = currentPanesByID[paneID]

            if let previousPane, let currentPane {
                if previousPane.tabID != currentPane.tabID || previousPane.spaceID != currentPane.spaceID {
                    appendEvent(
                        type: "pane.moved",
                        spaceID: currentPane.spaceID,
                        tabID: currentPane.tabID,
                        paneID: currentPane.paneID,
                        payload: [
                            "previous_space_id": .string(previousPane.spaceID),
                            "current_space_id": .string(currentPane.spaceID),
                            "previous_tab_id": .string(previousPane.tabID),
                            "current_tab_id": .string(currentPane.tabID)
                        ]
                    )
                }

                var changedFields: [String] = []
                if previousPane.cwd != currentPane.cwd {
                    changedFields.append("cwd")
                }
                if previousPane.viewport?.title != currentPane.viewport?.title {
                    changedFields.append("viewport.title")
                }
                if previousPane.viewport?.summary != currentPane.viewport?.summary {
                    changedFields.append("viewport.summary")
                }
                if previousPane.context?.gitBranch != currentPane.context?.gitBranch {
                    changedFields.append("context.git_branch")
                }
                if previousPane.context?.lastCommandExitCode != currentPane.context?.lastCommandExitCode {
                    changedFields.append("context.last_command_exit_code")
                }
                if previousPane.context?.rendererPhase != currentPane.context?.rendererPhase {
                    changedFields.append("context.renderer_phase")
                }
                if previousPane.context?.displayName != currentPane.context?.displayName {
                    changedFields.append("context.display_name")
                }
                if previousPane.context?.displayID != currentPane.context?.displayID {
                    changedFields.append("context.display_id")
                }
                if previousPane.context?.windowTitle != currentPane.context?.windowTitle {
                    changedFields.append("context.window_title")
                }
                if previousPane.context?.socketPath != currentPane.context?.socketPath {
                    changedFields.append("context.socket_path")
                }
                if previousPane.context?.launchCommand != currentPane.context?.launchCommand {
                    changedFields.append("context.launch_command")
                }
                if !changedFields.isEmpty {
                    appendEvent(
                        type: "pane.metadata_changed",
                        spaceID: currentPane.spaceID,
                        tabID: currentPane.tabID,
                        paneID: currentPane.paneID,
                        payload: [
                            "changed_fields": .array(changedFields.map(AlanShellJSONValue.string))
                        ]
                    )
                }

                if previousPane.attention != currentPane.attention {
                    appendEvent(
                        type: "attention.changed",
                        spaceID: currentPane.spaceID,
                        tabID: currentPane.tabID,
                        paneID: currentPane.paneID,
                        payload: [
                            "previous": .string(previousPane.attention.rawValue),
                            "current": .string(currentPane.attention.rawValue)
                        ]
                    )
                }

                if previousPane.alanBinding != currentPane.alanBinding {
                    appendEvent(
                        type: "AlanBinding.changed",
                        spaceID: currentPane.spaceID,
                        tabID: currentPane.tabID,
                        paneID: currentPane.paneID,
                        payload: [
                            "session_id": .string(currentPane.alanBinding?.sessionID ?? ""),
                            "run_status": .string(currentPane.alanBinding?.runStatus ?? ""),
                            "pending_yield": .bool(currentPane.alanBinding?.pendingYield ?? false)
                        ]
                    )
                }
            } else if let currentPane {
                appendEvent(
                    type: "pane.created",
                    spaceID: currentPane.spaceID,
                    tabID: currentPane.tabID,
                    paneID: currentPane.paneID,
                    payload: [
                        "pane_id": .string(currentPane.paneID),
                        "tab_id": .string(currentPane.tabID)
                    ]
                )
            } else if let previousPane {
                appendEvent(
                    type: "pane.closed",
                    spaceID: previousPane.spaceID,
                    tabID: previousPane.tabID,
                    paneID: previousPane.paneID,
                    payload: [
                        "pane_id": .string(previousPane.paneID)
                    ]
                )
            }
        }
    }

    private func appendEvent(
        type: String,
        spaceID: String?,
        tabID: String?,
        paneID: String?,
        payload: [String: AlanShellJSONValue]
    ) {
        let event = AlanShellEventEnvelope(
            eventID: "ev_\(nextEventOrdinal)",
            type: type,
            timestamp: ISO8601DateFormatter().string(from: .now),
            windowID: windowID,
            spaceID: spaceID,
            tabID: tabID,
            paneID: paneID,
            payload: payload
        )
        nextEventOrdinal += 1
        events.append(event)
        if events.count > 200 {
            events.removeFirst(events.count - 200)
        }
        if let data = try? encoder.encode(event),
           let line = String(data: data, encoding: .utf8) {
            do {
                if fileManager.fileExists(atPath: eventsFileURL.path) {
                    let handle = try FileHandle(forWritingTo: eventsFileURL)
                    defer { try? handle.close() }
                    _ = try handle.seekToEnd()
                    try handle.write(contentsOf: Data("\(line)\n".utf8))
                } else {
                    try Data("\(line)\n".utf8).write(to: eventsFileURL, options: .atomic)
                }
            } catch {
                recordDiagnostic("Failed to persist shell event log: \(error.localizedDescription)")
            }
        }
    }

    private func readEvents(afterEventID: String?, limit: Int?) -> [AlanShellEventEnvelope] {
        let startIndex: Int
        if let afterEventID,
           let index = events.firstIndex(where: { $0.eventID == afterEventID }) {
            startIndex = events.index(after: index)
        } else {
            startIndex = 0
        }

        let slice = events.dropFirst(startIndex)
        let capped = limit.map { max(0, $0) } ?? 50
        return Array(slice.prefix(capped))
    }

    private func pollCommands() {
        ensureDirectories()

        let commandFiles: [URL]
        do {
            commandFiles = try fileManager.contentsOfDirectory(
                at: commandsURL,
                includingPropertiesForKeys: [.creationDateKey, .contentModificationDateKey],
                options: [.skipsHiddenFiles]
            )
            .filter { $0.pathExtension == "json" }
            .sorted(by: compareCommandFiles)
        } catch {
            recordDiagnostic("Failed to read shell command directory: \(error.localizedDescription)")
            return
        }

        for fileURL in commandFiles {
            handleCommandFile(at: fileURL)
        }
    }

    private func handleCommandFile(at fileURL: URL) {
        guard let data = try? Data(contentsOf: fileURL),
              let command = try? decoder.decode(AlanShellControlCommand.self, from: data)
        else {
            recordDiagnostic("Ignored unreadable shell command file \(fileURL.lastPathComponent).")
            do {
                try fileManager.removeItem(at: fileURL)
            } catch {
                recordDiagnostic("Failed to remove unreadable shell command file \(fileURL.lastPathComponent): \(error.localizedDescription)")
            }
            return
        }

        let response =
            specialCommandResponse(for: command)
            ?? socketServer.handleLocally(command)
            ?? commandHandler(command)
        let responseURL = resultsURL.appendingPathComponent("\(command.requestID).json")

        do {
            let responseData = try encoder.encode(response)
            try responseData.write(to: responseURL, options: .atomic)
        } catch {
            recordDiagnostic("Failed to write shell command result \(responseURL.lastPathComponent): \(error.localizedDescription)")
        }

        do {
            try fileManager.removeItem(at: fileURL)
        } catch {
            recordDiagnostic("Failed to remove processed shell command file \(fileURL.lastPathComponent): \(error.localizedDescription)")
        }
    }

    private func compareCommandFiles(_ lhs: URL, _ rhs: URL) -> Bool {
        let lhsValues = try? lhs.resourceValues(forKeys: [.creationDateKey, .contentModificationDateKey])
        let rhsValues = try? rhs.resourceValues(forKeys: [.creationDateKey, .contentModificationDateKey])
        let lhsDate = lhsValues?.creationDate ?? lhsValues?.contentModificationDate ?? .distantPast
        let rhsDate = rhsValues?.creationDate ?? rhsValues?.contentModificationDate ?? .distantPast

        if lhsDate != rhsDate {
            return lhsDate < rhsDate
        }

        return lhs.lastPathComponent < rhs.lastPathComponent
    }

    private func pollBindings() {
        for paneID in trackedPaneIDs.sorted() {
            let bindingURL = alanShellBindingFileURL(
                windowID: windowID,
                paneID: paneID,
                fileManager: fileManager
            )

            guard fileManager.fileExists(atPath: bindingURL.path) else {
                if lastBindingPayloadByPaneID.removeValue(forKey: paneID) != nil {
                    bindingProjectionHandler(paneID, nil)
                }
                continue
            }

            guard let data = try? Data(contentsOf: bindingURL) else {
                recordDiagnostic("Failed to read Alan binding file for \(paneID).")
                continue
            }

            if lastBindingPayloadByPaneID[paneID] == data {
                continue
            }

            guard let projection = try? decoder.decode(AlanShellBindingProjection.self, from: data) else {
                lastBindingPayloadByPaneID[paneID] = data
                recordDiagnostic("Ignored invalid Alan binding file for \(paneID).")
                continue
            }

            lastBindingPayloadByPaneID[paneID] = data
            bindingProjectionHandler(paneID, projection.shellBinding)
        }
    }
}
#endif
