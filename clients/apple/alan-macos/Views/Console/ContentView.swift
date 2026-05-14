import SwiftUI

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
            Text("alan")
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
