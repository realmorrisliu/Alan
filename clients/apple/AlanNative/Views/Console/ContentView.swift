import SwiftUI
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

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
    private var assistantMessageByTurnID: [String: UUID] = [:]
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
                    assistantMessageByTurnID.removeAll()
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
            assistantMessageByTurnID.removeAll()
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
        assistantMessageByTurnID.removeAll()
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

        while !Task.isCancelled {
            guard selectedSessionID == sessionID else {
                return
            }

            do {
                let page = try await client().readEvents(
                    sessionID: sessionID,
                    afterEventID: lastEventID,
                    limit: 200
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
                    handle(event: event)
                }
            } catch is CancellationError {
                return
            } catch {
                connectionState = .degraded
                let message = error.localizedDescription
                if message != lastPumpErrorMessage {
                    lastPumpErrorMessage = message
                    appendTimeline("Event Stream Error", detail: message, level: .warning)
                    lastError = message
                }
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
    }

    private func handle(event: SessionEventEnvelope) {
        switch event.type {
        case "turn_started":
            appendTimeline("Turn Started", detail: shortID(event.turnID), level: .info)

        case "text_delta", "message_delta", "message_delta_chunk":
            if let chunk = event.textChunk, !chunk.isEmpty {
                appendAssistantChunk(
                    chunk,
                    turnID: event.turnID ?? "turn_unknown",
                    isFinal: event.isFinal == true
                )
            }

        case "thinking_delta":
            if let chunk = event.textChunk, !chunk.isEmpty {
                appendTimeline(
                    "Thinking",
                    detail: summarize(chunk, maxLength: 140),
                    level: .info
                )
            }

        case "tool_call_started":
            let title = "Tool Started"
            let detail = "\(event.name ?? "tool") • \(shortID(event.toolCallID))"
            appendTimeline(title, detail: detail, level: .action)

        case "tool_call_completed":
            let preview = event.resultPreview ?? "Completed"
            appendTimeline("Tool Completed", detail: preview, level: .action)

        case "yield":
            let pending = PendingYieldState.from(event: event)
            pendingYield = pending
            confirmationModifications = ""
            structuredAnswerDrafts = Dictionary(uniqueKeysWithValues: pending.questions.map { ($0.id, "") })
            customResumePayload = "{\n  \"choice\": \"approve\"\n}"
            appendTimeline("Action Required", detail: pending.kind, level: .warning)

        case "warning":
            if let message = event.message {
                appendSystemMessage("Warning: \(message)")
                appendTimeline("Warning", detail: message, level: .warning)
            }

        case "error":
            if let message = event.message {
                if event.recoverable == true {
                    appendSystemMessage("Recoverable: \(message)")
                    appendTimeline("Recoverable Error", detail: message, level: .warning)
                } else {
                    appendErrorMessage(message)
                    appendTimeline("Error", detail: message, level: .error)
                }
            }

        case "turn_completed":
            finishAssistantMessage(for: event.turnID)
            appendTimeline(
                "Turn Completed",
                detail: event.summary,
                level: .info
            )

        default:
            appendTimeline(
                event.type.replacingOccurrences(of: "_", with: " ").capitalized,
                detail: event.message ?? event.summary,
                level: .info
            )
        }
    }

    private func appendUserMessage(_ text: String) {
        messages.append(ChatMessage(role: .user, text: text))
    }

    private func appendSystemMessage(_ text: String) {
        messages.append(ChatMessage(role: .system, text: text))
    }

    private func appendErrorMessage(_ text: String) {
        messages.append(ChatMessage(role: .error, text: text))
    }

    private func appendAssistantChunk(_ chunk: String, turnID: String, isFinal: Bool) {
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

    private func finishAssistantMessage(for turnID: String?) {
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

    private func client() throws -> AlanAPIClient {
        try AlanAPIClient(baseURLString: baseURLString)
    }
}

struct ContentView: View {
    @StateObject private var viewModel = AlanConsoleViewModel()
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var showsConnectionControls = false
    @State private var showsSessionDefaults = false

    #if !os(macOS)
    @State private var mobilePane: MobilePane = .chat
    #endif

    #if os(macOS)
    @State private var showsInspector = true
    #endif

    enum MobilePane: String, CaseIterable, Identifiable {
        case chat = "Chat"
        case timeline = "Activity"

        var id: String { rawValue }
    }

    private var selectedSession: SessionListItem? {
        guard let selectedID = viewModel.selectedSessionID else {
            return nil
        }
        return viewModel.sessions.first(where: { $0.sessionID == selectedID })
    }

    var body: some View {
        ZStack {
            ConsoleTheme.windowBackground
                .ignoresSafeArea()

            NavigationSplitView {
                sidebar
            } detail: {
                detail
            }
            .navigationSplitViewStyle(.balanced)
        }
        .task {
            await viewModel.bootstrap()
        }
        .onDisappear {
            viewModel.shutdown()
        }
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            sidebarWindowBar

            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    quickActionSection
                    connectionSection
                    sessionDefaultsSection
                    threadsSection
                }
                .padding(14)
            }

            sidebarFooter
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(ConsoleTheme.sidebarFill, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .strokeBorder(ConsoleTheme.sidebarBorder, lineWidth: 1)
        )
        .padding(.vertical, 8)
        .padding(.horizontal, 8)
    }

    private var sidebarWindowBar: some View {
        HStack(spacing: 8) {
            HStack(spacing: 6) {
                Circle().fill(Color(red: 0.97, green: 0.40, blue: 0.37)).frame(width: 10, height: 10)
                Circle().fill(Color(red: 1.00, green: 0.74, blue: 0.29)).frame(width: 10, height: 10)
                Circle().fill(Color(red: 0.38, green: 0.80, blue: 0.42)).frame(width: 10, height: 10)
            }

            Spacer()

            Text("Update")
                .font(.system(size: 11, weight: .semibold, design: .rounded))
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
                .background(ConsoleTheme.badgeFill, in: Capsule())
                .overlay(
                    Capsule()
                        .strokeBorder(ConsoleTheme.badgeBorder, lineWidth: 1)
                )
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(ConsoleTheme.sidebarFill.opacity(0.95))
    }

    private var quickActionSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Alan")
                .font(.system(size: 20, weight: .semibold, design: .rounded))
                .foregroundStyle(ConsoleTheme.textPrimary)

            Button {
                Task { await viewModel.createSession() }
            } label: {
                Label("New thread", systemImage: "square.and.pencil")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(SidebarActionButtonStyle(prominent: true))
            .disabled(viewModel.isCreatingSession)

            Button {
                Task { await viewModel.refreshSessions() }
            } label: {
                Label("Refresh threads", systemImage: "arrow.clockwise")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(SidebarActionButtonStyle(prominent: false))
            .disabled(viewModel.isRefreshingSessions)
        }
    }

    private var connectionSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button {
                withAnimation(reduceMotion ? nil : .easeOut(duration: 0.2)) {
                    showsConnectionControls.toggle()
                }
            } label: {
                HStack {
                    Text("Connection")
                        .font(.system(size: 12, weight: .semibold, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textSecondary)
                    Spacer()
                    Image(systemName: showsConnectionControls ? "chevron.up" : "chevron.down")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(ConsoleTheme.textMuted)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 6) {
                    Circle()
                        .fill(viewModel.connectionState.tint)
                        .frame(width: 8, height: 8)
                    Text(viewModel.connectionState.label)
                        .font(.system(size: 12, weight: .medium, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textPrimary)
                    Spacer()
                    Text(viewModel.healthStatus)
                        .font(.system(size: 11, weight: .medium, design: .monospaced))
                        .foregroundStyle(ConsoleTheme.textMuted)
                }

                if showsConnectionControls {
                    TextField("http://127.0.0.1:8090", text: $viewModel.baseURLString)
                        .textFieldStyle(CompactDarkFieldStyle())
                    #if os(iOS)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .keyboardType(.URL)
                    #endif

                    HStack(spacing: 8) {
                        Button("Health") {
                            Task { await viewModel.refreshHealth() }
                        }
                        .buttonStyle(InlineActionButtonStyle())
                        .disabled(viewModel.isCheckingHealth)

                        Button("Resume") {
                            if let sessionID = viewModel.selectedSessionID {
                                Task { await viewModel.openSession(sessionID) }
                            }
                        }
                        .buttonStyle(InlineActionButtonStyle())
                        .disabled(viewModel.selectedSessionID == nil)
                    }
                }
            }
            .padding(10)
            .background(ConsoleTheme.inlinePanel, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
            )
        }
    }

    private var sessionDefaultsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button {
                withAnimation(reduceMotion ? nil : .easeOut(duration: 0.2)) {
                    showsSessionDefaults.toggle()
                }
            } label: {
                HStack {
                    Text("Session defaults")
                        .font(.system(size: 12, weight: .semibold, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textSecondary)
                    Spacer()
                    Image(systemName: showsSessionDefaults ? "chevron.up" : "chevron.down")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(ConsoleTheme.textMuted)
                }
            }
            .buttonStyle(.plain)

            if showsSessionDefaults {
                VStack(alignment: .leading, spacing: 8) {
                    configPicker(title: "Governance", selection: $viewModel.governanceProfile)
                    configPicker(title: "Streaming", selection: $viewModel.streamingMode)
                    configPicker(title: "Recovery", selection: $viewModel.partialStreamRecoveryMode)
                }
            }
        }
    }

    private func configPicker<Value: Hashable & Identifiable & CaseIterable & RawRepresentable>(
        title: String,
        selection: Binding<Value>
    ) -> some View where Value.RawValue == String {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.system(size: 11, weight: .medium, design: .rounded))
                .foregroundStyle(ConsoleTheme.textMuted)

            Picker(title, selection: selection) {
                ForEach(Array(Value.allCases), id: \.id) { value in
                    Text(value.rawValue).tag(value)
                }
            }
            .pickerStyle(.menu)
            .tint(ConsoleTheme.textPrimary)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(ConsoleTheme.inlinePanel, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
            )
        }
    }

    private var threadsSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Threads")
                    .font(.system(size: 12, weight: .semibold, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textSecondary)

                Spacer()

                Text("\(viewModel.sessions.count)")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textMuted)
            }

            VStack(spacing: 6) {
                if viewModel.sessions.isEmpty {
                    Text("No active sessions")
                        .font(.system(size: 12, weight: .regular, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textMuted)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.vertical, 8)
                }

                ForEach(viewModel.sessions) { session in
                    sessionRow(session)
                }
            }
        }
    }

    private func sessionRow(_ session: SessionListItem) -> some View {
        let selected = session.sessionID == viewModel.selectedSessionID

        return Button {
            Task { await viewModel.openSession(session.sessionID) }
        } label: {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 6) {
                    Circle()
                        .fill(session.active ? ConsoleTheme.success : ConsoleTheme.textMuted)
                        .frame(width: 7, height: 7)

                    Text(shortID(session.sessionID))
                        .font(.system(size: 12, weight: .medium, design: .monospaced))
                        .foregroundStyle(ConsoleTheme.textPrimary)

                    Spacer(minLength: 8)

                    if session.active {
                        Text("live")
                            .font(.system(size: 10, weight: .semibold, design: .rounded))
                            .foregroundStyle(ConsoleTheme.success)
                    }
                }

                Text("\(session.governance.profile.rawValue)  •  \(session.streamingMode.rawValue)")
                    .font(.system(size: 11, weight: .regular, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textMuted)
                    .lineLimit(1)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 9)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(selected ? ConsoleTheme.selectionFill : Color.clear)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .strokeBorder(selected ? ConsoleTheme.selectionBorder : Color.clear, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }

    private var sidebarFooter: some View {
        HStack {
            Label("Settings", systemImage: "gearshape")
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundStyle(ConsoleTheme.textMuted)
            Spacer()
            Text("v0.1")
                .font(.system(size: 11, weight: .regular, design: .monospaced))
                .foregroundStyle(ConsoleTheme.textMuted)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .background(ConsoleTheme.sidebarFill.opacity(0.95))
    }

    @ViewBuilder
    private var detail: some View {
        if viewModel.selectedSessionID == nil {
            emptyDetail
        } else {
            sessionDetail
        }
    }

    private var emptyDetail: some View {
        VStack(spacing: 12) {
            Image(systemName: "rectangle.stack.badge.play")
                .font(.system(size: 34, weight: .regular))
                .foregroundStyle(ConsoleTheme.textMuted)

            Text("Choose a thread or create a new one")
                .font(.system(size: 22, weight: .semibold, design: .rounded))
                .foregroundStyle(ConsoleTheme.textPrimary)

            Text("macOS provides full control. iPhone focuses on remote steering and approvals.")
                .font(.system(size: 14, weight: .regular, design: .rounded))
                .foregroundStyle(ConsoleTheme.textMuted)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var sessionDetail: some View {
        VStack(spacing: 12) {
            detailHeader

            if let error = viewModel.lastError {
                Text(error)
                    .font(.system(size: 12, weight: .medium, design: .rounded))
                    .foregroundStyle(ConsoleTheme.error)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(ConsoleTheme.error.opacity(0.08), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .strokeBorder(ConsoleTheme.error.opacity(0.35), lineWidth: 1)
                    )
            }

            if let pending = viewModel.pendingYield {
                pendingYieldCard(pending)
            }

            #if os(macOS)
            HStack(spacing: 12) {
                conversationPanel

                if showsInspector {
                    timelinePanel
                        .frame(width: 320)
                        .transition(.move(edge: .trailing).combined(with: .opacity))
                }
            }
            #else
            VStack(spacing: 8) {
                Picker("Pane", selection: $mobilePane) {
                    ForEach(MobilePane.allCases) { pane in
                        Text(pane.rawValue).tag(pane)
                    }
                }
                .pickerStyle(.segmented)
                .tint(ConsoleTheme.accent)

                if mobilePane == .chat {
                    conversationPanel
                } else {
                    timelinePanel
                }
            }
            #endif
        }
        .padding(14)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: viewModel.pendingYield?.requestID)
    }

    private var detailHeader: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                Text(shortID(viewModel.selectedSessionID))
                    .font(.system(size: 15, weight: .semibold, design: .monospaced))
                    .foregroundStyle(ConsoleTheme.textPrimary)
                Text(selectedSession?.workspaceID ?? "Remote Agent Control")
                    .font(.system(size: 12, weight: .regular, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textMuted)
            }

            Spacer(minLength: 16)

            HStack(spacing: 8) {
                Label(viewModel.connectionState.label, systemImage: "circle.fill")
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundStyle(viewModel.connectionState.tint)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 5)
                    .background(ConsoleTheme.inlinePanel, in: Capsule())

                HStack(spacing: 6) {
                    Text("n")
                        .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        .foregroundStyle(ConsoleTheme.textMuted)
                    TextField("1", text: $viewModel.rollbackTurnsText)
                        .textFieldStyle(CompactDarkFieldStyle())
                        .frame(width: 42)
                    Button("Rollback") {
                        Task { await viewModel.rollbackCurrentSession() }
                    }
                    .buttonStyle(InlineActionButtonStyle())
                    .disabled(viewModel.isRunningAction)
                }

                Button("Interrupt") {
                    Task { await viewModel.interruptCurrentSession() }
                }
                .buttonStyle(InlineActionButtonStyle())
                .disabled(viewModel.isRunningAction)

                Menu {
                    Button("Fork session") {
                        Task { await viewModel.forkCurrentSession() }
                    }
                    Button("Compact context") {
                        Task { await viewModel.compactCurrentSession() }
                    }
                    Divider()
                    Button("Delete session", role: .destructive) {
                        Task { await viewModel.deleteCurrentSession() }
                    }
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "chevron.down.circle")
                        Text("Open")
                    }
                    .font(.system(size: 12, weight: .semibold, design: .rounded))
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(ConsoleTheme.accent, in: Capsule())
                    .foregroundStyle(.white)
                }

                #if os(macOS)
                Button {
                    withAnimation(reduceMotion ? nil : .easeOut(duration: 0.2)) {
                        showsInspector.toggle()
                    }
                } label: {
                    Image(systemName: showsInspector ? "sidebar.right" : "sidebar.right.fill")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(ConsoleTheme.textPrimary)
                        .padding(8)
                        .background(ConsoleTheme.inlinePanel, in: Circle())
                }
                .buttonStyle(.plain)
                #endif
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(ConsoleTheme.panelFill, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
        )
    }

    private var conversationPanel: some View {
        VStack(spacing: 0) {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 12) {
                        if viewModel.messages.isEmpty {
                            Text("Ask for follow-up changes")
                                .font(.system(size: 13, weight: .regular, design: .rounded))
                                .foregroundStyle(ConsoleTheme.textMuted)
                                .padding(.top, 20)
                        }

                        ForEach(viewModel.messages) { message in
                            MessageBubble(message: message)
                                .id(message.id)
                        }
                    }
                    .padding(.horizontal, 14)
                    .padding(.vertical, 16)
                }
                .onChange(of: viewModel.messages.count) { _, _ in
                    guard let lastID = viewModel.messages.last?.id else {
                        return
                    }
                    if reduceMotion {
                        proxy.scrollTo(lastID, anchor: .bottom)
                    } else {
                        withAnimation(.easeOut(duration: 0.2)) {
                            proxy.scrollTo(lastID, anchor: .bottom)
                        }
                    }
                }
            }

            Divider()
                .overlay(ConsoleTheme.panelBorder)

            composerPanel
                .padding(12)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(ConsoleTheme.panelFill, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
        )
    }

    private var composerPanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            ZStack(alignment: .topLeading) {
                if viewModel.draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    Text("Ask for follow-up changes")
                        .font(.system(size: 14, weight: .regular, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textMuted)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 10)
                }

                TextEditor(text: $viewModel.draftMessage)
                    .font(.system(size: 14, weight: .regular, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textPrimary)
                    .scrollContentBackground(.hidden)
                    .frame(minHeight: 76, maxHeight: 120)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 4)
            }
            .background(ConsoleTheme.inlinePanel, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
            )

            HStack(spacing: 8) {
                Picker("Compose", selection: $viewModel.composeMode) {
                    ForEach(AlanConsoleViewModel.ComposeMode.allCases) { mode in
                        Text(mode.rawValue).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .tint(ConsoleTheme.accent)

                Button {
                    Task { await viewModel.sendDraftMessage() }
                } label: {
                    Group {
                        if viewModel.isSending {
                            ProgressView()
                                .controlSize(.small)
                                .tint(.white)
                        } else {
                            Image(systemName: "arrow.up")
                                .font(.system(size: 14, weight: .bold))
                        }
                    }
                    .foregroundStyle(.white)
                    .frame(width: 34, height: 34)
                    .background(ConsoleTheme.accent, in: Circle())
                }
                .buttonStyle(.plain)
                .disabled(!viewModel.canSendMessage)
                .keyboardShortcut(.return, modifiers: [.command])
            }
        }
    }

    private var timelinePanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Activity")
                    .font(.system(size: 14, weight: .semibold, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textPrimary)
                Spacer()
                Button("Clear") {
                    viewModel.clearTimeline()
                }
                .buttonStyle(InlineActionButtonStyle())
            }

            ScrollView {
                LazyVStack(spacing: 8) {
                    if viewModel.timeline.isEmpty {
                        Text("No events yet")
                            .font(.system(size: 12, weight: .regular, design: .rounded))
                            .foregroundStyle(ConsoleTheme.textMuted)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.vertical, 8)
                    }

                    ForEach(viewModel.timeline) { entry in
                        TimelineRow(entry: entry)
                    }
                }
            }
        }
        .padding(12)
        .frame(maxHeight: .infinity)
        .background(ConsoleTheme.panelFill, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
        )
    }

    @ViewBuilder
    private func pendingYieldCard(_ pending: PendingYieldState) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label("Action required", systemImage: "exclamationmark.triangle.fill")
                    .font(.system(size: 13, weight: .semibold, design: .rounded))
                    .foregroundStyle(ConsoleTheme.warning)
                Spacer()
                Text(shortID(pending.requestID))
                    .font(.system(size: 11, weight: .regular, design: .monospaced))
                    .foregroundStyle(ConsoleTheme.textMuted)
            }

            Text("kind: \(pending.kind)")
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(ConsoleTheme.textMuted)

            if pending.kind == "confirmation" {
                if let summary = pending.summary {
                    Text(summary)
                        .font(.system(size: 13, weight: .regular, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textPrimary)
                }

                if !pending.options.isEmpty {
                    Text("options: \(pending.options.joined(separator: ", "))")
                        .font(.system(size: 11, weight: .regular, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textMuted)
                }

                HStack(spacing: 8) {
                    Button("Approve") {
                        Task { await viewModel.approvePendingConfirmation() }
                    }
                    .buttonStyle(InlineActionButtonStyle(prominent: true))

                    Button("Reject") {
                        Task { await viewModel.rejectPendingConfirmation() }
                    }
                    .buttonStyle(InlineActionButtonStyle())
                }

                HStack(spacing: 8) {
                    TextField("Modification notes", text: $viewModel.confirmationModifications)
                        .textFieldStyle(CompactDarkFieldStyle())

                    Button("Modify") {
                        Task { await viewModel.modifyPendingConfirmation() }
                    }
                    .buttonStyle(InlineActionButtonStyle())
                }
            } else if pending.kind == "structured_input" {
                if let title = pending.title {
                    Text(title)
                        .font(.system(size: 13, weight: .semibold, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textPrimary)
                }
                if let prompt = pending.prompt {
                    Text(prompt)
                        .font(.system(size: 12, weight: .regular, design: .rounded))
                        .foregroundStyle(ConsoleTheme.textMuted)
                }

                ForEach(pending.questions) { question in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 4) {
                            Text(question.label)
                                .font(.system(size: 12, weight: .semibold, design: .rounded))
                                .foregroundStyle(ConsoleTheme.textPrimary)

                            if question.required {
                                Text("*")
                                    .font(.system(size: 12, weight: .bold))
                                    .foregroundStyle(ConsoleTheme.error)
                            }
                        }

                        Text(question.prompt)
                            .font(.system(size: 11, weight: .regular, design: .rounded))
                            .foregroundStyle(ConsoleTheme.textMuted)

                        TextField(
                            question.options.isEmpty
                                ? "Answer"
                                : "Options: \(question.options.map { $0.value }.joined(separator: ", "))",
                            text: answerBinding(questionID: question.id)
                        )
                        .textFieldStyle(CompactDarkFieldStyle())
                    }
                    .padding(8)
                    .background(ConsoleTheme.inlinePanel, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
                    )
                }

                Button("Submit answers") {
                    Task { await viewModel.submitStructuredInputAnswers() }
                }
                .buttonStyle(InlineActionButtonStyle(prominent: true))
            } else {
                Text("Provide raw resume payload JSON")
                    .font(.system(size: 11, weight: .regular, design: .rounded))
                    .foregroundStyle(ConsoleTheme.textMuted)

                TextEditor(text: $viewModel.customResumePayload)
                    .font(.system(size: 12, weight: .regular, design: .monospaced))
                    .scrollContentBackground(.hidden)
                    .frame(minHeight: 92)
                    .padding(4)
                    .background(ConsoleTheme.inlinePanel, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .strokeBorder(ConsoleTheme.panelBorder, lineWidth: 1)
                    )

                Button("Resume") {
                    Task { await viewModel.submitCustomResumePayload() }
                }
                .buttonStyle(InlineActionButtonStyle(prominent: true))
            }
        }
        .padding(12)
        .background(ConsoleTheme.panelFill, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .strokeBorder(ConsoleTheme.warning.opacity(0.35), lineWidth: 1)
        )
    }

    private func shortID(_ value: String?) -> String {
        guard let value, !value.isEmpty else {
            return "-"
        }
        if value.count <= 16 {
            return value
        }
        return "\(value.prefix(8))…\(value.suffix(4))"
    }

    private func answerBinding(questionID: String) -> Binding<String> {
        Binding(
            get: {
                viewModel.structuredAnswerDrafts[questionID] ?? ""
            },
            set: { newValue in
                viewModel.structuredAnswerDrafts[questionID] = newValue
            }
        )
    }
}

private extension Color {
    static func adaptive(
        light: (Double, Double, Double),
        dark: (Double, Double, Double),
        alpha: Double = 1
    ) -> Color {
        #if os(iOS)
        return Color(
            UIColor { traitCollection in
                let rgb = traitCollection.userInterfaceStyle == .dark ? dark : light
                return UIColor(red: rgb.0, green: rgb.1, blue: rgb.2, alpha: alpha)
            }
        )
        #elseif os(macOS)
        return Color(
            NSColor(name: nil) { appearance in
                let isDark = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                let rgb = isDark ? dark : light
                return NSColor(red: rgb.0, green: rgb.1, blue: rgb.2, alpha: alpha)
            }
        )
        #else
        return Color(red: dark.0, green: dark.1, blue: dark.2, opacity: alpha)
        #endif
    }
}

private enum ConsoleTheme {
    static let windowBackground = LinearGradient(
        colors: [
            Color.adaptive(light: (0.96, 0.97, 0.99), dark: (0.06, 0.07, 0.08)),
            Color.adaptive(light: (0.92, 0.94, 0.97), dark: (0.09, 0.10, 0.12)),
            Color.adaptive(light: (0.95, 0.96, 0.98), dark: (0.07, 0.08, 0.10)),
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let sidebarFill = Color.adaptive(light: (0.95, 0.96, 0.98), dark: (0.13, 0.14, 0.16))
    static let sidebarBorder = Color.adaptive(light: (0.82, 0.85, 0.90), dark: (0.29, 0.31, 0.36))

    static let panelFill = Color.adaptive(light: (1.00, 1.00, 1.00), dark: (0.10, 0.11, 0.13))
    static let inlinePanel = Color.adaptive(light: (0.95, 0.96, 0.98), dark: (0.14, 0.15, 0.18))
    static let panelBorder = Color.adaptive(light: (0.84, 0.87, 0.92), dark: (0.28, 0.30, 0.34))

    static let textPrimary = Color.adaptive(light: (0.08, 0.10, 0.14), dark: (0.94, 0.95, 0.97))
    static let textSecondary = Color.adaptive(light: (0.22, 0.26, 0.33), dark: (0.74, 0.77, 0.83))
    static let textMuted = Color.adaptive(light: (0.40, 0.45, 0.53), dark: (0.56, 0.60, 0.67))

    static let accent = Color.adaptive(light: (0.20, 0.44, 0.85), dark: (0.28, 0.58, 0.96))
    static let success = Color.adaptive(light: (0.17, 0.62, 0.33), dark: (0.39, 0.81, 0.48))
    static let warning = Color.adaptive(light: (0.85, 0.56, 0.12), dark: (0.98, 0.75, 0.29))
    static let error = Color.adaptive(light: (0.77, 0.22, 0.24), dark: (0.94, 0.38, 0.40))

    static let selectionFill = Color.adaptive(light: (0.74, 0.84, 0.99), dark: (0.24, 0.48, 0.85))
        .opacity(0.30)
    static let selectionBorder = Color.adaptive(light: (0.33, 0.54, 0.90), dark: (0.34, 0.63, 0.99))
        .opacity(0.58)

    static let badgeFill = Color.adaptive(light: (0.74, 0.84, 0.99), dark: (0.24, 0.48, 0.85))
        .opacity(0.30)
    static let badgeBorder = Color.adaptive(light: (0.33, 0.54, 0.90), dark: (0.34, 0.63, 0.99))
        .opacity(0.58)

    static let userBubble = Color.adaptive(light: (0.86, 0.92, 1.00), dark: (0.16, 0.23, 0.34))
    static let assistantBubble = Color.adaptive(light: (0.95, 0.96, 0.98), dark: (0.14, 0.15, 0.18))
    static let systemBubble = Color.adaptive(light: (0.93, 0.94, 0.96), dark: (0.17, 0.17, 0.19))
    static let errorBubble = Color.adaptive(light: (1.00, 0.91, 0.92), dark: (0.26, 0.13, 0.14))
}

private struct SidebarActionButtonStyle: ButtonStyle {
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

private struct InlineActionButtonStyle: ButtonStyle {
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

private struct CompactDarkFieldStyle: TextFieldStyle {
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

private struct MessageBubble: View {
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

private struct TimelineRow: View {
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
