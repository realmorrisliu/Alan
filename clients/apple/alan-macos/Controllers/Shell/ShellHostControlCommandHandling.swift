import Foundation

#if os(macOS)
@MainActor
extension ShellHostController {
    func attentionInboxRows() -> [AlanShellAttentionInboxItem] {
        attentionItems.map { item in
            AlanShellAttentionInboxItem(
                itemID: "attn_\(item.paneID)",
                spaceID: item.spaceID,
                tabID: item.tabID,
                paneID: item.paneID,
                attention: item.attention,
                summary: item.summary
            )
        }
    }

    func routingCandidates(preferredPaneID: String?) -> [AlanShellRoutingCandidate] {
        let preferredPane = preferredPaneID.flatMap { pane(paneID: $0) }
        let focusedPane = self.focusedPane
        let now = Date()

        return shellState.panes.map { candidate in
            var score = 0.0
            var reasons: [String] = []
            let attention = shellEffectiveAttention(for: candidate, now: now)

            if candidate.paneID == preferredPaneID {
                score += 0.4
                reasons.append("requested")
            }
            if candidate.paneID == shellState.focusedPaneID {
                score += 0.3
                reasons.append("focused")
            }
            if attention == .awaitingUser {
                score += 0.25
                reasons.append("attention:awaiting_user")
            } else if attention == .notable {
                score += 0.12
                reasons.append("attention:notable")
            }
            if candidate.alanBinding?.pendingYield == true {
                score += 0.2
                reasons.append("alan_binding:yielded")
            } else if let runStatus = candidate.alanBinding?.runStatus {
                score += 0.08
                reasons.append("alan_binding:\(runStatus)")
            }
            if let preferredPane, candidate.tabID == preferredPane.tabID {
                score += 0.1
                reasons.append("same_tab")
            } else if let focusedPane, candidate.tabID == focusedPane.tabID {
                score += 0.08
                reasons.append("same_tab")
            }
            if let preferredPane, candidate.spaceID == preferredPane.spaceID {
                score += 0.05
                reasons.append("same_space")
            } else if let focusedPane, candidate.spaceID == focusedPane.spaceID {
                score += 0.04
                reasons.append("same_space")
            }
            if let process = candidate.process?.program {
                reasons.append("process:\(process)")
            }

            return AlanShellRoutingCandidate(
                paneID: candidate.paneID,
                score: min(score, 1.0),
                reasons: Array(Set(reasons)).sorted()
            )
        }
        .sorted {
            $0.score == $1.score ? $0.paneID < $1.paneID : $0.score > $1.score
        }
    }

    func paneList(tabID: String?) -> [ShellPane] {
        guard let tabID else {
            return shellState.panes
        }
        return shellState.panes.filter { $0.tabID == tabID }
    }

    func tabList(spaceID: String?) -> [ShellTab] {
        if let spaceID {
            return shellState.spaces.first(where: { $0.spaceID == spaceID })?.tabs ?? []
        }
        return shellState.spaces.flatMap(\.tabs)
    }

    func response(
        requestID: String,
        applied: Bool,
        state: ShellStateSnapshot? = nil,
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
            requestID: requestID,
            contractVersion: shellState.contractVersion,
            applied: applied,
            state: state,
            spaces: spaces,
            tabs: tabs,
            panes: panes,
            pane: pane,
            items: items,
            candidates: candidates,
            events: events,
            focusedPaneID: shellState.focusedPaneID,
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

    func handleControlPlaneCommand(_ command: AlanShellControlCommand) -> AlanShellControlResponse {
        switch command.command {
        case .state:
            return response(
                requestID: command.requestID,
                applied: true,
                state: shellState
            )

        case .spaceList:
            return response(
                requestID: command.requestID,
                applied: true,
                spaces: shellState.spaces
            )

        case .spaceCreate, .spaceOpenAlan:
            let launchTarget: ShellLaunchTarget = command.command == .spaceOpenAlan ? .alan : .shell
            let failureMessage = launchTarget == .alan
                ? "Failed to create a new alan space."
                : "Failed to create a new shell space."
            guard let spaceID = createSpace(
                launchTarget: launchTarget,
                title: command.title,
                workingDirectory: command.cwd
            ) else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "space_create_failed",
                    errorMessage: failureMessage
                )
            }
            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: spaceID,
                paneID: shellState.focusedPaneID
            )

        case .tabList:
            return response(
                requestID: command.requestID,
                applied: true,
                tabs: tabList(spaceID: command.spaceID),
                spaceID: command.spaceID
            )

        case .tabOpen:
            guard let tabID = openTerminalTab(
                in: command.spaceID,
                title: command.title,
                workingDirectory: command.cwd
            ) else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    spaceID: command.spaceID,
                    errorCode: "space_not_found",
                    errorMessage: "The requested space does not exist."
                )
            }
            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: shellState.focusedSpaceID,
                tabID: tabID,
                paneID: shellState.focusedPaneID
            )

        case .tabClose:
            guard let tabID = command.tabID else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "tab_required",
                    errorMessage: "tab_id is required."
                )
            }

            switch closeTab(tabID: tabID) {
            case .closed:
                return response(
                    requestID: command.requestID,
                    applied: true,
                    tabID: tabID,
                    paneID: shellState.focusedPaneID
                )
            case .tabNotFound:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    tabID: tabID,
                    errorCode: "tab_not_found",
                    errorMessage: "The requested tab does not exist."
                )
            case .lastTab:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    tabID: tabID,
                    errorCode: "last_tab",
                    errorMessage: "alan terminal workspace must keep at least one tab open."
                )
            }

        case .paneList:
            return response(
                requestID: command.requestID,
                applied: true,
                panes: paneList(tabID: command.tabID),
                tabID: command.tabID
            )

        case .paneSnapshot:
            guard let paneID = command.paneID,
                  let pane = pane(paneID: paneID)
            else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: command.paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            }

            return response(
                requestID: command.requestID,
                applied: true,
                pane: pane,
                spaceID: pane.spaceID,
                tabID: pane.tabID,
                paneID: pane.paneID
            )

        case .paneSplit:
            guard let paneID = command.paneID else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "pane_required",
                    errorMessage: "pane_id is required."
                )
            }
            guard let direction = command.direction else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "direction_required",
                    errorMessage: "direction is required for pane.split."
                )
            }
            guard let newPaneID = splitPane(paneID: paneID, direction: direction) else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            }
            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: shellState.focusedSpaceID,
                tabID: shellState.focusedTabID,
                paneID: newPaneID
            )

        case .paneClose:
            guard let paneID = command.paneID else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "pane_required",
                    errorMessage: "pane_id is required."
                )
            }

            switch closePane(paneID: paneID) {
            case .closed:
                return response(
                    requestID: command.requestID,
                    applied: true,
                    spaceID: shellState.focusedSpaceID,
                    tabID: shellState.focusedTabID,
                    paneID: shellState.focusedPaneID
                )
            case .paneNotFound:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            case .lastTab:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "last_tab",
                    errorMessage: "alan terminal workspace must keep at least one pane open."
                )
            }

        case .paneLift:
            guard let paneID = command.paneID else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "pane_required",
                    errorMessage: "pane_id is required."
                )
            }

            switch liftPaneToTab(paneID: paneID, title: command.title) {
            case .lifted:
                return response(
                    requestID: command.requestID,
                    applied: true,
                    spaceID: shellState.focusedSpaceID,
                    tabID: shellState.focusedTabID,
                    paneID: shellState.focusedPaneID
                )
            case .paneNotFound:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            case .lastPane:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "last_pane",
                    errorMessage: "The pane needs at least one sibling before it can be lifted."
                )
            }

        case .paneMove:
            guard let paneID = command.paneID,
                  let targetTabID = command.tabID
            else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    tabID: command.tabID,
                    paneID: command.paneID,
                    errorCode: "pane_move_target_required",
                    errorMessage: "pane_id and tab_id are required."
                )
            }

            let direction = command.direction ?? .vertical
            guard movePane(paneID: paneID, toTab: targetTabID, direction: direction) else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    tabID: targetTabID,
                    paneID: paneID,
                    errorCode: "invalid_move_target",
                    errorMessage: "The requested pane could not be moved to the target tab."
                )
            }

            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: shellState.focusedSpaceID,
                tabID: shellState.focusedTabID,
                paneID: shellState.focusedPaneID
            )

        case .paneFocus:
            guard let paneID = command.paneID,
                  shellState.panes.contains(where: { $0.paneID == paneID })
            else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: command.paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            }

            focus(paneID: paneID)
            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: shellState.focusedSpaceID,
                tabID: shellState.focusedTabID,
                paneID: paneID
            )

        case .paneSendText:
            guard let paneID = command.paneID,
                  let targetPane = pane(paneID: paneID)
            else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: command.paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            }

            let text = command.text ?? ""
            let delivery = terminalRuntimeRegistry.sendText(to: paneID, text: text)
            controlPlane.recordTextDelivery(
                requestID: command.requestID,
                spaceID: targetPane.spaceID,
                tabID: targetPane.tabID,
                paneID: paneID,
                delivery: delivery
            )

            return response(
                requestID: command.requestID,
                applied: delivery.applied,
                spaceID: targetPane.spaceID,
                tabID: targetPane.tabID,
                paneID: paneID,
                acceptedBytes: delivery.acceptedBytes,
                deliveryCode: delivery.code.rawValue,
                runtimePhase: delivery.runtimePhase,
                errorCode: delivery.errorCode,
                errorMessage: delivery.errorMessage
            )

        case .attentionInbox:
            return response(
                requestID: command.requestID,
                applied: true,
                items: attentionInboxRows()
            )

        case .attentionSet:
            guard let paneID = command.paneID,
                  let attention = command.attention
            else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "attention_target_required",
                    errorMessage: "pane_id and attention are required."
                )
            }
            guard let targetPane = pane(paneID: paneID) else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            }
            guard setAttention(attention, for: paneID) else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "pane_not_found",
                    errorMessage: "The requested pane does not exist."
                )
            }
            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: targetPane.spaceID,
                tabID: targetPane.tabID,
                paneID: paneID
            )

        case .routingCandidates:
            return response(
                requestID: command.requestID,
                applied: true,
                candidates: routingCandidates(preferredPaneID: command.paneID)
            )

        case .eventsRead:
            return controlPlane.specialCommandResponse(for: command)
                ?? response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "events_unavailable",
                    errorMessage: "events.read is handled by the shell control plane."
                )
        }
    }

}
#endif
