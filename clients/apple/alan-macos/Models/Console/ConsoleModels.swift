import Foundation

struct ChatMessage: Identifiable, Equatable {
    enum Role {
        case user
        case assistant
        case system
        case error
    }

    let id: UUID
    let role: Role
    var text: String
    var turnID: String?
    var isStreaming: Bool
    var timestamp: Date

    init(
        id: UUID = UUID(),
        role: Role,
        text: String,
        turnID: String? = nil,
        isStreaming: Bool = false,
        timestamp: Date = .now
    ) {
        self.id = id
        self.role = role
        self.text = text
        self.turnID = turnID
        self.isStreaming = isStreaming
        self.timestamp = timestamp
    }
}

struct TimelineEntry: Identifiable {
    enum Level {
        case info
        case action
        case warning
        case error
    }

    let id: UUID
    let title: String
    let detail: String?
    let level: Level
    let timestamp: Date

    init(id: UUID = UUID(), title: String, detail: String? = nil, level: Level, timestamp: Date = .now) {
        self.id = id
        self.title = title
        self.detail = detail
        self.level = level
        self.timestamp = timestamp
    }
}

struct StructuredQuestionOption: Identifiable, Equatable {
    let value: String
    let label: String
    let description: String?

    var id: String { value }
}

struct StructuredQuestionItem: Identifiable, Equatable {
    let id: String
    let label: String
    let prompt: String
    let required: Bool
    let options: [StructuredQuestionOption]
}

struct PendingYieldState: Equatable {
    let requestID: String
    let kind: String
    let payload: JSONValue?
    let summary: String?
    let options: [String]
    let title: String?
    let prompt: String?
    let questions: [StructuredQuestionItem]

    static func from(event: SessionEventEnvelope) -> PendingYieldState {
        let payload = event.payload
        let kind = event.normalizedYieldKind ?? "custom"
        let object = payload?.objectValue ?? [:]

        let summary = object["summary"]?.stringValue
        let options = object["options"]?.arrayValue?
            .compactMap { $0.stringValue }
            ?? []
        let title = object["title"]?.stringValue
        let prompt = object["prompt"]?.stringValue

        let questions = object["questions"]?.arrayValue?
            .compactMap { questionValue -> StructuredQuestionItem? in
                guard let question = questionValue.objectValue else {
                    return nil
                }
                guard
                    let id = question["id"]?.stringValue,
                    let label = question["label"]?.stringValue,
                    let prompt = question["prompt"]?.stringValue
                else {
                    return nil
                }

                let required = question["required"]?.boolValue ?? false
                let options = question["options"]?.arrayValue?
                    .compactMap { optionValue -> StructuredQuestionOption? in
                        guard let option = optionValue.objectValue else {
                            return nil
                        }
                        guard
                            let value = option["value"]?.stringValue,
                            let optionLabel = option["label"]?.stringValue
                        else {
                            return nil
                        }
                        return StructuredQuestionOption(
                            value: value,
                            label: optionLabel,
                            description: option["description"]?.stringValue
                        )
                    } ?? []

                return StructuredQuestionItem(
                    id: id,
                    label: label,
                    prompt: prompt,
                    required: required,
                    options: options
                )
            } ?? []

        return PendingYieldState(
            requestID: event.requestID ?? event.toolCallID ?? UUID().uuidString,
            kind: kind,
            payload: payload,
            summary: summary,
            options: options,
            title: title,
            prompt: prompt,
            questions: questions
        )
    }
}
