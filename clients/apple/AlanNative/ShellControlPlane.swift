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
    private let diagnosticHandler: @MainActor (String) -> Void
    private let socketServer: AlanShellSocketServer
    private var filePoller: AlanShellControlFilePoller?
    private var trackedPaneIDs: Set<String> = []
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
        self.filePoller = nil
        self.filePoller = AlanShellControlFilePoller(
            windowID: windowID,
            fileManager: fileManager,
            commandsURL: commandsURL,
            resultsURL: resultsURL,
            encoder: encoder,
            decoder: decoder,
            commandHandler: { [weak self, commandHandler] command in
                guard let self else { return commandHandler(command) }
                return self.responseForPolledCommand(command)
            },
            bindingProjectionHandler: bindingProjectionHandler,
            diagnosticHandler: diagnosticHandler
        )

        ensureDirectories()
        socketServer.start()
        filePoller?.start()
    }

    deinit {
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

    private func responseForPolledCommand(
        _ command: AlanShellControlCommand
    ) -> AlanShellControlResponse {
        specialCommandResponse(for: command)
            ?? socketServer.handleLocally(command)
            ?? commandHandler(command)
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
        filePoller?.updateTrackedPaneIDs(paneIDs)

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
}
#endif
