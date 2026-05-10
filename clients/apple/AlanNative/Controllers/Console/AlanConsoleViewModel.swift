import Foundation
import SwiftUI

@MainActor
final class AlanConsoleViewModel: ObservableObject {
    enum ComposeMode: String, CaseIterable, Identifiable {
        case turn = "New Turn"
        case steer = "Steer"

        var id: String { rawValue }
    }

    enum SessionConnectionState {
        case idle
        case connecting
        case online
        case degraded

        var label: String {
            switch self {
            case .idle:
                return "Idle"
            case .connecting:
                return "Syncing"
            case .online:
                return "Online"
            case .degraded:
                return "Reconnecting"
            }
        }

        var tint: Color {
            switch self {
            case .idle:
                return .gray
            case .connecting:
                return .yellow
            case .online:
                return .green
            case .degraded:
                return .orange
            }
        }
    }

    @Published var baseURLString = "http://127.0.0.1:8090"
    @Published var healthStatus = "Not checked"
    @Published var lastError: String?

    @Published var governanceProfile: GovernanceProfile = .conservative
    @Published var streamingMode: SessionStreamingMode = .auto
    @Published var partialStreamRecoveryMode: PartialStreamRecoveryMode = .continueOnce

    @Published var sessions: [SessionListItem] = []
    @Published var selectedSessionID: String?

    @Published var composeMode: ComposeMode = .turn
    @Published var draftMessage = ""

    @Published var messages: [ChatMessage] = []
    @Published var timeline: [TimelineEntry] = []

    @Published var pendingYield: PendingYieldState?
    @Published var confirmationModifications = ""
    @Published var structuredAnswerDrafts: [String: String] = [:]
    @Published var customResumePayload = "{\n  \"choice\": \"approve\"\n}"

    @Published var rollbackTurnsText = "1"
    @Published var connectionState: SessionConnectionState = .idle

    @Published var isCheckingHealth = false
    @Published var isRefreshingSessions = false
    @Published var isCreatingSession = false
    @Published var isSending = false
    @Published var isRunningAction = false

    private var lastEventID: String?
    private var eventPumpTask: Task<Void, Never>?
    private var eventReducer = ConsoleEventReducer()
    private var lastPumpErrorMessage: String?

    deinit {
        eventPumpTask?.cancel()
    }

    func bootstrap() async {
        await refreshHealth()
        await refreshSessions(keepSelection: false)
    }

    func shutdown() {
        eventPumpTask?.cancel()
        eventPumpTask = nil
    }

    var canSendMessage: Bool {
        !isSending && !draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    func refreshHealth() async {
        isCheckingHealth = true
        defer { isCheckingHealth = false }

        do {
            let response = try await client().checkHealth()
            healthStatus = response.status
            lastError = nil
        } catch {
            reportError(error, timelineTitle: "Health Check Failed")
        }
    }

    func refreshSessions(keepSelection: Bool = true) async {
        isRefreshingSessions = true
        defer { isRefreshingSessions = false }

        do {
            let sessionList = try await client().listSessions()
            sessions = sessionList
            if keepSelection {
                if let selectedSessionID,
                   sessionList.contains(where: { $0.sessionID == selectedSessionID })
                {
                    return
                }
                if selectedSessionID != nil {
                    selectedSessionID = nil
                    messages = []
                    pendingYield = nil
                    eventReducer.reset()
                    lastEventID = nil
                    eventPumpTask?.cancel()
                    eventPumpTask = nil
                    connectionState = .idle
                }
            } else {
                selectedSessionID = nil
            }
        } catch {
            reportError(error, timelineTitle: "Session List Failed")
        }
    }

    func createSession() async {
        isCreatingSession = true
        defer { isCreatingSession = false }

        do {
            let response = try await client().createSession(
                governanceProfile: governanceProfile,
                streamingMode: streamingMode,
                partialStreamRecoveryMode: partialStreamRecoveryMode
            )
            appendTimeline(
                "Session Created",
                detail: shortID(response.sessionID),
                level: .action
            )
            await refreshSessions(keepSelection: true)
            try await activateSession(response.sessionID)
            lastError = nil
        } catch {
            reportError(error, timelineTitle: "Create Session Failed")
        }
    }

    func openSession(_ sessionID: String) async {
        do {
            try await activateSession(sessionID)
            lastError = nil
        } catch {
            reportError(error, timelineTitle: "Open Session Failed")
        }
    }

    func forkCurrentSession() async {
        guard let sessionID = selectedSessionID else {
            return
        }
        isRunningAction = true
        defer { isRunningAction = false }

        do {
            let response = try await client().forkSession(sessionID: sessionID)
            appendTimeline(
                "Session Forked",
                detail: "\(shortID(response.forkedFromSessionID)) -> \(shortID(response.sessionID))",
                level: .action
            )
            await refreshSessions(keepSelection: true)
            try await activateSession(response.sessionID)
            lastError = nil
        } catch {
            reportError(error, timelineTitle: "Fork Failed")
        }
    }

    func deleteCurrentSession() async {
        guard let sessionID = selectedSessionID else {
            return
        }
        isRunningAction = true
        defer { isRunningAction = false }

        do {
            try await client().deleteSession(sessionID: sessionID)
            appendTimeline("Session Deleted", detail: shortID(sessionID), level: .action)
            selectedSessionID = nil
            messages = []
            pendingYield = nil
            eventReducer.reset()
            lastEventID = nil
            eventPumpTask?.cancel()
            eventPumpTask = nil
            connectionState = .idle
            await refreshSessions(keepSelection: false)
            lastError = nil
        } catch {
            reportError(error, timelineTitle: "Delete Failed")
        }
    }

    func sendDraftMessage() async {
        let text = draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else {
            return
        }

        isSending = true
        defer { isSending = false }

        do {
            let sessionID = try await ensureActiveSession()

            switch composeMode {
            case .turn:
                appendUserMessage(text)
                _ = try await client().sendTurn(sessionID: sessionID, text: text)
                appendTimeline("Turn Submitted", detail: shortID(sessionID), level: .action)
            case .steer:
                _ = try await client().sendInput(sessionID: sessionID, text: text)
                appendTimeline("Steer Input Submitted", detail: text, level: .action)
            }

            draftMessage = ""
            lastError = nil
        } catch {
            reportError(error, timelineTitle: "Send Failed")
        }
    }

    func interruptCurrentSession() async {
        guard let sessionID = selectedSessionID else {
            return
        }
        await runSessionAction(title: "Interrupt", detail: shortID(sessionID)) {
            _ = try await self.client().interrupt(sessionID: sessionID)
        }
    }

    func compactCurrentSession() async {
        guard let sessionID = selectedSessionID else {
            return
        }
        await runSessionAction(title: "Compaction Requested", detail: shortID(sessionID)) {
            _ = try await self.client().compact(sessionID: sessionID)
        }
    }

    func rollbackCurrentSession() async {
        guard let sessionID = selectedSessionID else {
            return
        }
        let turns = Int(rollbackTurnsText) ?? 0
        guard turns > 0 else {
            lastError = "Rollback turns must be a positive integer"
            return
        }

        await runSessionAction(title: "Rollback Requested", detail: "\(turns) turn(s)") {
            _ = try await self.client().rollback(sessionID: sessionID, turns: turns)
        }
    }

    func approvePendingConfirmation() async {
        await submitConfirmation(choice: "approve", modifications: nil)
    }

    func rejectPendingConfirmation() async {
        await submitConfirmation(choice: "reject", modifications: nil)
    }

    func modifyPendingConfirmation() async {
        let modifications = confirmationModifications
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !modifications.isEmpty else {
            lastError = "Modification note cannot be empty"
            return
        }
        await submitConfirmation(choice: "modify", modifications: modifications)
    }

    func submitStructuredInputAnswers() async {
        guard let pendingYield, pendingYield.kind == "structured_input" else {
            return
        }
        guard let sessionID = selectedSessionID else {
            return
        }

        var answers: [JSONValue] = []
        for question in pendingYield.questions {
            let rawValue = structuredAnswerDrafts[question.id] ?? ""
            let value = rawValue.trimmingCharacters(in: .whitespacesAndNewlines)
            if question.required && value.isEmpty {
                lastError = "Question '\(question.label)' is required"
                return
            }
            if !value.isEmpty {
                answers.append(
                    .object([
                        "question_id": .string(question.id),
                        "value": .string(value),
                    ])
                )
            }
        }

        let payload = JSONValue.object([
            "answers": .array(answers),
        ])

        await runSessionAction(title: "Structured Input Submitted", detail: shortID(pendingYield.requestID)) {
            _ = try await self.client().resume(
                sessionID: sessionID,
                requestID: pendingYield.requestID,
                payload: payload
            )
            self.pendingYield = nil
            self.structuredAnswerDrafts = [:]
        }
    }

    func submitCustomResumePayload() async {
        guard let pendingYield else {
            return
        }
        guard let sessionID = selectedSessionID else {
            return
        }

        let payloadText = customResumePayload.trimmingCharacters(in: .whitespacesAndNewlines)
        let payload: JSONValue
        if payloadText.isEmpty {
            payload = .object([:])
        } else {
            payload = parseResumePayload(from: payloadText)
        }

        await runSessionAction(title: "Yield Resumed", detail: pendingYield.kind) {
            _ = try await self.client().resume(
                sessionID: sessionID,
                requestID: pendingYield.requestID,
                payload: payload
            )
            self.pendingYield = nil
        }
    }

    func clearTimeline() {
        timeline = []
    }

    private func activateSession(_ sessionID: String) async throws {
        selectedSessionID = sessionID
        connectionState = .connecting
        pendingYield = nil
        confirmationModifications = ""
        structuredAnswerDrafts = [:]
        eventReducer.reset()
        lastEventID = nil

        let session = try await client().readSession(sessionID: sessionID)
        messages = session.messages.map { history in
            let role = chatRole(from: history.role)
            let text: String
            if let toolName = history.toolName, !toolName.isEmpty {
                text = "[\(toolName)] \(history.content)"
            } else {
                text = history.content
            }
            return ChatMessage(role: role, text: text, turnID: nil, isStreaming: false)
        }
        timeline = []
        appendTimeline("Session Loaded", detail: shortID(sessionID), level: .info)

        _ = try? await client().resumeSession(sessionID: sessionID)

        startEventPump(sessionID: sessionID)
    }

    private func ensureActiveSession() async throws -> String {
        if let selectedSessionID {
            return selectedSessionID
        }

        let response = try await client().createSession(
            governanceProfile: governanceProfile,
            streamingMode: streamingMode,
            partialStreamRecoveryMode: partialStreamRecoveryMode
        )
        await refreshSessions(keepSelection: true)
        try await activateSession(response.sessionID)
        return response.sessionID
    }

    private func startEventPump(sessionID: String) {
        eventPumpTask?.cancel()
        eventPumpTask = Task { [weak self] in
            await self?.runEventPump(sessionID: sessionID)
        }
    }

    private func runEventPump(sessionID: String) async {
        connectionState = .connecting
        let eventReader: ConsoleEventReader
        do {
            eventReader = try ConsoleEventReader(client: client())
        } catch {
            reportEventPumpError(error)
            return
        }

        while !Task.isCancelled {
            guard selectedSessionID == sessionID else {
                return
            }

            do {
                let page = try await eventReader.readNextPage(
                    sessionID: sessionID,
                    afterEventID: lastEventID
                )

                connectionState = .online
                lastPumpErrorMessage = nil

                if page.gap {
                    appendTimeline(
                        "Event Gap Detected",
                        detail: "Re-sync from available buffer",
                        level: .warning
                    )
                }

                if let oldest = page.oldestEventID, page.gap {
                    lastEventID = oldest
                }

                if page.events.isEmpty {
                    try await Task.sleep(nanoseconds: 250_000_000)
                    continue
                }

                for event in page.events {
                    if Task.isCancelled || selectedSessionID != sessionID {
                        return
                    }
                    lastEventID = event.eventID
                    reduce(event: event)
                }
            } catch is CancellationError {
                return
            } catch {
                reportEventPumpError(error)
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }

    private func reduce(event: SessionEventEnvelope) {
        var projection = ConsoleEventProjection(
            messages: messages,
            timeline: timeline,
            pendingYield: pendingYield,
            confirmationModifications: confirmationModifications,
            structuredAnswerDrafts: structuredAnswerDrafts,
            customResumePayload: customResumePayload
        )
        eventReducer.reduce(event: event, into: &projection)
        messages = projection.messages
        timeline = projection.timeline
        pendingYield = projection.pendingYield
        confirmationModifications = projection.confirmationModifications
        structuredAnswerDrafts = projection.structuredAnswerDrafts
        customResumePayload = projection.customResumePayload
    }

    private func appendUserMessage(_ text: String) {
        messages.append(ChatMessage(role: .user, text: text))
    }

    private func appendTimeline(_ title: String, detail: String?, level: TimelineEntry.Level) {
        timeline.append(TimelineEntry(title: title, detail: detail, level: level))
        if timeline.count > 800 {
            timeline.removeFirst(timeline.count - 800)
        }
    }

    private func submitConfirmation(choice: String, modifications: String?) async {
        guard let pendingYield else {
            return
        }
        guard pendingYield.kind == "confirmation" else {
            return
        }
        guard let sessionID = selectedSessionID else {
            return
        }

        var payload: [String: JSONValue] = [
            "choice": .string(choice),
        ]

        if let modifications, !modifications.isEmpty {
            payload["modifications"] = .string(modifications)
        }

        await runSessionAction(title: "Confirmation Submitted", detail: choice) {
            _ = try await self.client().resume(
                sessionID: sessionID,
                requestID: pendingYield.requestID,
                payload: .object(payload)
            )
            self.pendingYield = nil
            self.confirmationModifications = ""
        }
    }

    private func runSessionAction(
        title: String,
        detail: String,
        operation: @escaping () async throws -> Void
    ) async {
        isRunningAction = true
        defer { isRunningAction = false }

        do {
            try await operation()
            appendTimeline(title, detail: detail, level: .action)
            lastError = nil
        } catch {
            reportError(error, timelineTitle: "\(title) Failed")
        }
    }

    private func parseResumePayload(from text: String) -> JSONValue {
        if let data = text.data(using: .utf8),
           let object = try? JSONSerialization.jsonObject(with: data)
        {
            return JSONValue.from(any: object)
        }
        return .string(text)
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

    private func chatRole(from rawRole: String) -> ChatMessage.Role {
        switch rawRole.lowercased() {
        case "user":
            return .user
        case "assistant":
            return .assistant
        case "error":
            return .error
        default:
            return .system
        }
    }

    private func reportError(_ error: Error, timelineTitle: String) {
        let message = error.localizedDescription
        lastError = message
        appendTimeline(timelineTitle, detail: message, level: .error)
    }

    private func reportEventPumpError(_ error: Error) {
        connectionState = .degraded
        let message = error.localizedDescription
        if message != lastPumpErrorMessage {
            lastPumpErrorMessage = message
            appendTimeline("Event Stream Error", detail: message, level: .warning)
            lastError = message
        }
    }

    private func client() throws -> AlanAPIClient {
        try AlanAPIClient(baseURLString: baseURLString)
    }
}
