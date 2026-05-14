import Foundation

#if os(macOS)
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
#endif
