import SwiftUI

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
    var isStreaming: Bool

    init(id: UUID = UUID(), role: Role, text: String, isStreaming: Bool = false) {
        self.id = id
        self.role = role
        self.text = text
        self.isStreaming = isStreaming
    }
}

@MainActor
final class AlanChatViewModel: ObservableObject {
    @Published var baseURLString = "http://127.0.0.1:8090"
    @Published var healthStatus = "Not checked"
    @Published var sessionID: String?
    @Published var draftMessage = ""
    @Published var messages: [ChatMessage] = []
    @Published var lastError: String?
    @Published var isCheckingHealth = false
    @Published var isCreatingSession = false
    @Published var isSending = false

    private var lastEventID: String?

    var canSendMessage: Bool {
        !isSending && !draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    func checkHealth() {
        Task { await runCheckHealth() }
    }

    func createSession() {
        Task { await runCreateSession() }
    }

    func sendDraftMessage() {
        let text = draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty, !isSending else {
            return
        }
        draftMessage = ""
        Task { await runSendMessage(text) }
    }

    private func runCheckHealth() async {
        isCheckingHealth = true
        lastError = nil
        defer { isCheckingHealth = false }

        do {
            let client = try AlanAPIClient(baseURLString: baseURLString)
            let response = try await client.checkHealth()
            healthStatus = response.status
        } catch {
            lastError = error.localizedDescription
        }
    }

    private func runCreateSession() async {
        isCreatingSession = true
        lastError = nil
        defer { isCreatingSession = false }

        do {
            let client = try AlanAPIClient(baseURLString: baseURLString)
            let response = try await client.createSession()
            sessionID = response.sessionID
            lastEventID = nil
            appendSystemMessage("Session ready: \(response.sessionID)")
        } catch {
            lastError = error.localizedDescription
            appendErrorMessage(error.localizedDescription)
        }
    }

    private func runSendMessage(_ text: String) async {
        isSending = true
        lastError = nil
        appendUserMessage(text)
        let assistantMessageID = appendAssistantPlaceholder()
        defer { isSending = false }

        do {
            let client = try AlanAPIClient(baseURLString: baseURLString)
            let activeSessionID = try await ensureSession(using: client)
            _ = try await client.submitInput(sessionID: activeSessionID, text: text)
            try await collectAssistantResponse(
                using: client,
                sessionID: activeSessionID,
                assistantMessageID: assistantMessageID
            )
        } catch {
            lastError = error.localizedDescription
            appendErrorMessage(error.localizedDescription)
            finishAssistantMessage(id: assistantMessageID, fallbackText: "Request failed.")
        }
    }

    private func ensureSession(using client: AlanAPIClient) async throws -> String {
        if let sessionID {
            return sessionID
        }

        isCreatingSession = true
        defer { isCreatingSession = false }

        let response = try await client.createSession()
        sessionID = response.sessionID
        lastEventID = nil
        appendSystemMessage("Session ready: \(response.sessionID)")
        return response.sessionID
    }

    private func collectAssistantResponse(
        using client: AlanAPIClient,
        sessionID: String,
        assistantMessageID: UUID
    ) async throws {
        let timeoutAt = Date().addingTimeInterval(90)
        var receivedText = false

        while Date() < timeoutAt {
            let page = try await client.readEvents(sessionID: sessionID, afterEventID: lastEventID)
            if page.gap, let oldestEventID = page.oldestEventID {
                lastEventID = oldestEventID
            }

            var turnCompleted = false
            for event in page.events {
                lastEventID = event.eventID

                switch event.type {
                case "text_delta", "message_delta", "message_delta_chunk":
                    if let chunk = event.textChunk, !chunk.isEmpty {
                        receivedText = true
                        appendAssistantChunk(chunk, to: assistantMessageID)
                    }

                case "error":
                    if let message = event.message, !message.isEmpty {
                        appendErrorMessage(message)
                    }
                    turnCompleted = true

                case "stream_lagged":
                    if let replayFromEventID = event.replayFromEventID {
                        lastEventID = replayFromEventID
                    }

                case "turn_completed":
                    turnCompleted = true

                default:
                    break
                }
            }

            if turnCompleted {
                let fallback = receivedText ? nil : "No text was returned."
                finishAssistantMessage(id: assistantMessageID, fallbackText: fallback)
                return
            }

            if page.events.isEmpty {
                try await Task.sleep(nanoseconds: 250_000_000)
            }
        }

        throw AlanAPIError.responseTimeout
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

    private func appendAssistantPlaceholder() -> UUID {
        let id = UUID()
        messages.append(ChatMessage(id: id, role: .assistant, text: "", isStreaming: true))
        return id
    }

    private func appendAssistantChunk(_ chunk: String, to messageID: UUID) {
        guard let index = messages.firstIndex(where: { $0.id == messageID }) else {
            return
        }
        messages[index].text.append(chunk)
    }

    private func finishAssistantMessage(id: UUID, fallbackText: String?) {
        guard let index = messages.firstIndex(where: { $0.id == id }) else {
            return
        }

        if messages[index].text.isEmpty, let fallbackText {
            messages[index].text = fallbackText
        }
        messages[index].isStreaming = false
    }
}

struct ContentView: View {
    @StateObject private var viewModel = AlanChatViewModel()

    var body: some View {
        NavigationStack {
            ZStack {
                LinearGradient(
                    colors: [Color.cyan.opacity(0.28), Color.blue.opacity(0.2), Color.indigo.opacity(0.18)],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
                .ignoresSafeArea()

                VStack(spacing: 0) {
                    connectionPanel
                    Divider().overlay(.white.opacity(0.22))
                    conversationPanel
                }
            }
            .navigationTitle("Alan Chat")
            .safeAreaInset(edge: .bottom) {
                composerPanel
            }
        }
    }

    private var connectionPanel: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Label("Server", systemImage: "network")
                    .font(.headline)
                Spacer()
                Text(viewModel.healthStatus)
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(.regularMaterial, in: Capsule())
            }

            TextField("Base URL", text: $viewModel.baseURLString)
#if os(iOS)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .keyboardType(.URL)
#endif
                .font(.callout.monospaced())
                .padding(.horizontal, 12)
                .padding(.vertical, 10)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12, style: .continuous))

            HStack(spacing: 8) {
                Button("Check Health") {
                    viewModel.checkHealth()
                }
                .buttonStyle(.glass)
                .disabled(viewModel.isCheckingHealth || viewModel.isSending)

                Button(viewModel.sessionID == nil ? "Create Session" : "New Session") {
                    viewModel.createSession()
                }
                .buttonStyle(.glassProminent)
                .disabled(viewModel.isCreatingSession || viewModel.isSending)
            }

            Text("Session: \(viewModel.sessionID ?? "Not created")")
                .font(.caption.monospaced())
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
        .padding(16)
        .background(.ultraThinMaterial)
    }

    private var conversationPanel: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 10) {
                    if viewModel.messages.isEmpty {
                        Text("Create a session and send your first message.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                            .padding(.top, 20)
                    }

                    ForEach(viewModel.messages) { message in
                        MessageBubble(message: message)
                            .id(message.id)
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 14)
            }
            .onChange(of: viewModel.messages.count) { _, _ in
                guard let lastID = viewModel.messages.last?.id else {
                    return
                }
                withAnimation(.easeOut(duration: 0.2)) {
                    proxy.scrollTo(lastID, anchor: .bottom)
                }
            }
        }
    }

    private var composerPanel: some View {
        VStack(spacing: 8) {
            if let lastError = viewModel.lastError {
                Text(lastError)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }

            HStack(alignment: .bottom, spacing: 10) {
                TextField("Message Alan…", text: $viewModel.draftMessage, axis: .vertical)
                    .lineLimit(1...6)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 10)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                    .onSubmit {
                        viewModel.sendDraftMessage()
                    }

                Button {
                    viewModel.sendDraftMessage()
                } label: {
                    if viewModel.isSending {
                        ProgressView()
                            .frame(width: 22, height: 22)
                    } else {
                        Image(systemName: "arrow.up")
                            .font(.headline.weight(.bold))
                            .frame(width: 22, height: 22)
                    }
                }
                .buttonStyle(.glassProminent)
                .disabled(!viewModel.canSendMessage || viewModel.isCreatingSession)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(.ultraThinMaterial)
    }
}

private struct MessageBubble: View {
    let message: ChatMessage

    private var alignment: HorizontalAlignment {
        switch message.role {
        case .user:
            return .trailing
        case .assistant, .system, .error:
            return .leading
        }
    }

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

    private var tint: Color {
        switch message.role {
        case .user:
            return .blue
        case .assistant:
            return .teal
        case .system:
            return .gray
        case .error:
            return .red
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
                Spacer(minLength: 36)
            }

            VStack(alignment: alignment, spacing: 6) {
                Text(roleTitle)
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)

                Text(bodyText)
                    .font(.body)
                    .textSelection(.enabled)
            }
            .frame(
                maxWidth: 560,
                alignment: message.role == .user ? .trailing : .leading
            )
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .strokeBorder(tint.opacity(0.35), lineWidth: 1)
            )

            if message.role != .user {
                Spacer(minLength: 36)
            }
        }
    }
}
