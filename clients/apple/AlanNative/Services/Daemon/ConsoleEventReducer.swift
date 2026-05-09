import Foundation

struct ConsoleEventReader {
    let client: AlanAPIClient
    let pageLimit: Int

    init(client: AlanAPIClient, pageLimit: Int = 200) {
        self.client = client
        self.pageLimit = pageLimit
    }

    func readNextPage(sessionID: String, afterEventID: String?) async throws -> ReadEventsResponse {
        try await client.readEvents(
            sessionID: sessionID,
            afterEventID: afterEventID,
            limit: pageLimit
        )
    }
}

struct ConsoleEventProjection {
    var messages: [ChatMessage]
    var timeline: [TimelineEntry]
    var pendingYield: PendingYieldState?
    var confirmationModifications: String
    var structuredAnswerDrafts: [String: String]
    var customResumePayload: String

    mutating func appendTimeline(_ title: String, detail: String?, level: TimelineEntry.Level) {
        timeline.append(TimelineEntry(title: title, detail: detail, level: level))
        if timeline.count > 800 {
            timeline.removeFirst(timeline.count - 800)
        }
    }

    mutating func appendSystemMessage(_ text: String) {
        messages.append(ChatMessage(role: .system, text: text))
    }

    mutating func appendErrorMessage(_ text: String) {
        messages.append(ChatMessage(role: .error, text: text))
    }
}

struct ConsoleEventReducer {
    private var assistantMessageByTurnID: [String: UUID] = [:]

    mutating func reset() {
        assistantMessageByTurnID.removeAll()
    }

    mutating func reduce(event: SessionEventEnvelope, into projection: inout ConsoleEventProjection) {
        switch event.type {
        case "turn_started":
            projection.appendTimeline("Turn Started", detail: shortID(event.turnID), level: .info)

        case "text_delta", "message_delta", "message_delta_chunk":
            if let chunk = event.textChunk, !chunk.isEmpty {
                appendAssistantChunk(
                    chunk,
                    turnID: event.turnID ?? "turn_unknown",
                    isFinal: event.isFinal == true,
                    messages: &projection.messages
                )
            }

        case "thinking_delta":
            if let chunk = event.textChunk, !chunk.isEmpty {
                projection.appendTimeline(
                    "Thinking",
                    detail: summarize(chunk, maxLength: 140),
                    level: .info
                )
            }

        case "tool_call_started":
            let title = "Tool Started"
            let detail = "\(event.name ?? "tool") • \(shortID(event.toolCallID))"
            projection.appendTimeline(title, detail: detail, level: .action)

        case "tool_call_completed":
            let preview = event.resultPreview ?? "Completed"
            projection.appendTimeline("Tool Completed", detail: preview, level: .action)

        case "yield":
            let pending = PendingYieldState.from(event: event)
            projection.pendingYield = pending
            projection.confirmationModifications = ""
            projection.structuredAnswerDrafts = Dictionary(
                uniqueKeysWithValues: pending.questions.map { ($0.id, "") }
            )
            projection.customResumePayload = "{\n  \"choice\": \"approve\"\n}"
            projection.appendTimeline("Action Required", detail: pending.kind, level: .warning)

        case "warning":
            if let message = event.message {
                projection.appendSystemMessage("Warning: \(message)")
                projection.appendTimeline("Warning", detail: message, level: .warning)
            }

        case "error":
            if let message = event.message {
                if event.recoverable == true {
                    projection.appendSystemMessage("Recoverable: \(message)")
                    projection.appendTimeline("Recoverable Error", detail: message, level: .warning)
                } else {
                    projection.appendErrorMessage(message)
                    projection.appendTimeline("Error", detail: message, level: .error)
                }
            }

        case "turn_completed":
            finishAssistantMessage(for: event.turnID, messages: &projection.messages)
            projection.appendTimeline(
                "Turn Completed",
                detail: event.summary,
                level: .info
            )

        default:
            projection.appendTimeline(
                event.type.replacingOccurrences(of: "_", with: " ").capitalized,
                detail: event.message ?? event.summary,
                level: .info
            )
        }
    }

    private mutating func appendAssistantChunk(
        _ chunk: String,
        turnID: String,
        isFinal: Bool,
        messages: inout [ChatMessage]
    ) {
        let messageID: UUID
        if let existing = assistantMessageByTurnID[turnID] {
            messageID = existing
        } else {
            let id = UUID()
            assistantMessageByTurnID[turnID] = id
            messages.append(ChatMessage(id: id, role: .assistant, text: "", turnID: turnID, isStreaming: true))
            messageID = id
        }

        guard let index = messages.firstIndex(where: { $0.id == messageID }) else {
            return
        }

        messages[index].text.append(chunk)
        messages[index].isStreaming = !isFinal

        if isFinal {
            assistantMessageByTurnID.removeValue(forKey: turnID)
        }
    }

    private mutating func finishAssistantMessage(for turnID: String?, messages: inout [ChatMessage]) {
        guard let turnID else {
            return
        }
        guard let messageID = assistantMessageByTurnID[turnID] else {
            return
        }
        guard let index = messages.firstIndex(where: { $0.id == messageID }) else {
            assistantMessageByTurnID.removeValue(forKey: turnID)
            return
        }

        messages[index].isStreaming = false
        if messages[index].text.isEmpty {
            messages[index].text = "No response text returned."
        }
        assistantMessageByTurnID.removeValue(forKey: turnID)
    }

    private func summarize(_ text: String, maxLength: Int) -> String {
        let compact = text.replacingOccurrences(of: "\n", with: " ")
        if compact.count <= maxLength {
            return compact
        }
        let end = compact.index(compact.startIndex, offsetBy: maxLength)
        return "\(compact[..<end])..."
    }

    private func shortID(_ value: String?) -> String {
        guard let value, !value.isEmpty else {
            return "-"
        }
        if value.count <= 12 {
            return value
        }
        let prefix = value.prefix(8)
        return "\(prefix)…"
    }
}
