import Foundation

#if os(macOS)
enum AlanShellLocalCommandSideEffect {
    case sendText(paneID: String, text: String)
}

struct AlanShellLocalCommandResult {
    let response: AlanShellControlResponse
    let updatedState: ShellStateSnapshot?
    let sideEffect: AlanShellLocalCommandSideEffect?
}

enum AlanShellLocalCommandExecutor {
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
        case .splitNotFound:
            return response(
                for: command,
                state: state,
                applied: false,
                tabID: command.tabID,
                paneID: command.paneID,
                errorCode: error.rawValue,
                errorMessage: "The requested split does not exist."
            )
        case .spatialFocusTargetNotFound:
            return response(
                for: command,
                state: state,
                applied: false,
                tabID: command.tabID,
                paneID: command.paneID,
                errorCode: error.rawValue,
                errorMessage: "There is no pane in that direction."
            )
        case .lastTab:
            return response(
                for: command,
                state: state,
                applied: false,
                tabID: command.tabID,
                errorCode: error.rawValue,
                errorMessage: "alan terminal workspace must keep at least one tab open."
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
                    ?? pane.alanBinding.map { $0.pendingYield ? "alan is waiting for user input" : "alan run status: \($0.runStatus)" }
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

#endif
