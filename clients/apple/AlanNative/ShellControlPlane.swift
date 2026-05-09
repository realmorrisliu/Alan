import Foundation

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

enum AlanShellPublishedStateMerger {
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
