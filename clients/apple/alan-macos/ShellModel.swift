import Foundation

struct ShellSidebarTabProjection: Equatable {
    let title: String
    let secondaryLine: String
    let activity: TerminalActivitySnapshot?
    let progress: TerminalActivityProgress?
    let accessibilityActivityLabel: String?
}

enum ShellActivityNotificationVisibility: String, Equatable {
    case focusedVisible
    case visibleUnfocused
    case background
}

enum ShellActivityNotificationKind: String, Equatable {
    case needsInput = "needs_input"
    case failed
    case commandCompleted = "command_completed"
    case processExited = "process_exited"
}

struct ShellActivityNotificationRoute: Equatable, Identifiable {
    let id: String
    let paneID: String
    let tabID: String
    let spaceID: String
    let kind: ShellActivityNotificationKind
    let title: String
    let body: String
    let attention: ShellAttentionState
}

struct ShellPaneTitleBarDetailProjection: Equatable, Identifiable {
    let id: String
    let title: String
    let help: String
}

func shellUserFacingSummary(_ summary: String?) -> String? {
    guard let summary else { return nil }

    let trimmed = summary.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { return nil }

    let internalOnlySummaries = [
        "command finished",
        "command succeeded",
        "title updated",
        "input committed",
        "terminal bell",
        "terminal rendering",
        "window attached",
    ]

    let lowercasedSummary = trimmed.lowercased()
    if internalOnlySummaries.contains(lowercasedSummary)
        || lowercasedSummary.hasPrefix("command failed")
    {
        return nil
    }

    return trimmed
}

func shellTerminalStatusSummary(for pane: ShellPane, now: Date? = nil) -> String? {
    if pane.context?.processState == "exited"
        || pane.context?.surfaceReadiness == "child_exited"
    {
        if let exitCode = pane.context?.lastCommandExitCode {
            return "Exited \(exitCode)"
        }
        return "Exited"
    }

    if pane.context?.rendererHealth == "failed"
        || pane.context?.rendererPhase == "failed"
        || pane.context?.surfaceReadiness == "renderer_failed"
    {
        return "Renderer failed"
    }

    if pane.context?.readonly == true {
        return "Read-only"
    }

    if pane.context?.inputReady == false,
       pane.context?.surfaceReadiness == "input_not_ready"
    {
        return "Starting"
    }

    switch shellEffectiveAttention(for: pane, now: now) {
    case .awaitingUser:
        guard let rawSummary = pane.viewport?.summary else { return "Needs attention" }
        return shellUserFacingSummary(rawSummary)
    case .notable:
        guard let rawSummary = pane.viewport?.summary else { return "Terminal bell" }
        return shellUserFacingSummary(rawSummary)
    case .active, .idle:
        return nil
    }
}

func shellVisibleLabel(_ raw: String?) -> String? {
    guard let raw else { return nil }
    let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty, trimmed != "/", trimmed != "-" else { return nil }
    if trimmed.lowercased() == "alan" {
        return "alan"
    }
    return trimmed
}

func shellPathLeaf(_ raw: String?) -> String? {
    guard let visible = shellVisibleLabel(raw) else { return nil }
    if visible == "~" {
        return "Home"
    }

    guard visible.contains("/") else { return nil }
    let components = visible.split(separator: "/").map(String.init)
    return components.last.flatMap(shellVisibleLabel)
}

func shellNormalizedTitle(_ raw: String?) -> String? {
    guard var candidate = shellVisibleLabel(raw) else { return nil }

    let internalOnlyTitles = [
        "title updated",
        "input committed",
        "terminal rendering",
        "window attached",
    ]

    let lowercasedCandidate = candidate.lowercased()
    if internalOnlyTitles.contains(lowercasedCandidate)
        || lowercasedCandidate.hasPrefix("pane_")
    {
        return nil
    }

    for suffix in [" - fish", " - zsh", " - bash", " - sh"] {
        if candidate.lowercased().hasSuffix(suffix) {
            candidate.removeLast(suffix.count)
            break
        }
    }

    candidate = candidate.trimmingCharacters(in: .whitespacesAndNewlines)
    guard let visible = shellVisibleLabel(candidate) else { return nil }

    if let leaf = shellPathLeaf(visible) {
        return leaf
    }

    return visible
}

func shellDisplayTitle(
    rawTitle: String?,
    workingDirectoryName: String?,
    cwd: String?,
    program: String?,
    launchTarget: ShellLaunchTarget,
    fallback: String? = nil
) -> String {
    if let workingDirectoryName = shellVisibleLabel(workingDirectoryName) {
        return workingDirectoryName
    }

    if let cwdLeaf = shellPathLeaf(cwd) {
        return cwdLeaf
    }

    if let normalizedTitle = shellNormalizedTitle(rawTitle) {
        return normalizedTitle
    }

    if let fallback = shellVisibleLabel(fallback) {
        return fallback
    }

    if launchTarget == .alan {
        return "alan"
    }

    if let program = shellVisibleLabel(program) {
        return program
    }

    return "Terminal"
}

func shellPaneTitleBarTitle(for pane: ShellPane) -> String {
    if let normalizedTitle = shellNormalizedTitle(pane.viewport?.title) {
        return normalizedTitle
    }

    if let cwdLeaf = shellPathLeaf(pane.cwd) {
        return cwdLeaf
    }

    if let workingDirectory = shellVisibleLabel(pane.context?.workingDirectoryName) {
        return workingDirectory
    }

    if pane.resolvedLaunchTarget == .alan {
        return "alan"
    }

    if let program = shellVisibleLabel(pane.process?.program) {
        return program
    }

    return "Terminal"
}

func shellPaneActivityAccessoryLabel(for pane: ShellPane, now: Date? = nil) -> String? {
    guard let activity = pane.activity else { return nil }
    if let now, !activity.isFresh(at: now) {
        return nil
    }

    switch activity.status {
    case .idle, .stale:
        return nil
    case .done:
        return activity.source.kind == .command ? nil : activity.display.sourceFirstLabel
    case .needsInput, .failed, .paused, .progress, .running, .bell, .exited:
        return activity.display.sourceFirstLabel
    }
}

func shellPaneStatusAccessoryLabel(for pane: ShellPane, now: Date? = nil) -> String? {
    guard let status = shellTerminalStatusSummary(for: pane, now: now) else { return nil }
    if shellEffectiveAttention(for: pane, now: now) == .notable,
       status == "Terminal bell"
    {
        return nil
    }
    return status
}

func shellPaneTitleBarDetailProjection(
    for pane: ShellPane,
    title: String,
    now: Date? = nil
) -> [ShellPaneTitleBarDetailProjection] {
    var items: [ShellPaneTitleBarDetailProjection] = []

    if let activityLabel = shellPaneActivityAccessoryLabel(for: pane, now: now) {
        items.append(
            ShellPaneTitleBarDetailProjection(
                id: "activity",
                title: activityLabel,
                help: activityLabel
            )
        )
    }

    if let status = shellPaneStatusAccessoryLabel(for: pane, now: now) {
        items.append(
            ShellPaneTitleBarDetailProjection(
                id: "status",
                title: status,
                help: status
            )
        )
    }

    if let context = shellPaneContextAccessoryProjection(for: pane, title: title) {
        items.append(context)
    }

    if let branch = shellPaneBranchAccessoryProjection(for: pane, title: title) {
        items.append(branch)
    }

    if let process = shellPaneProcessAccessoryProjection(for: pane, title: title) {
        items.append(process)
    }

    if let alan = shellPaneAlanAccessoryProjection(for: pane) {
        items.append(alan)
    }

    return items
}

private func shellPaneContextAccessoryProjection(
    for pane: ShellPane,
    title: String
) -> ShellPaneTitleBarDetailProjection? {
    let repositoryLabel = shellPathLeaf(pane.context?.repositoryRoot)
    let cwdLabel = shellPathLeaf(pane.cwd)
        ?? shellVisibleLabel(pane.context?.workingDirectoryName)
    guard let label = repositoryLabel ?? cwdLabel,
          !shellLabelsMatch(label, title)
    else {
        return nil
    }

    return ShellPaneTitleBarDetailProjection(
        id: repositoryLabel == nil ? "cwd" : "worktree",
        title: label,
        help: repositoryLabel == nil ? "Directory \(label)" : "Worktree \(label)"
    )
}

private func shellPaneBranchAccessoryProjection(
    for pane: ShellPane,
    title: String
) -> ShellPaneTitleBarDetailProjection? {
    guard let branch = shellVisibleLabel(pane.context?.gitBranch),
          !shellLabelsMatch(branch, title)
    else {
        return nil
    }

    return ShellPaneTitleBarDetailProjection(
        id: "branch",
        title: branch,
        help: "Git branch \(branch)"
    )
}

private func shellPaneProcessAccessoryProjection(
    for pane: ShellPane,
    title: String
) -> ShellPaneTitleBarDetailProjection? {
    guard let program = shellVisibleLabel(pane.process?.program),
          !shellLabelsMatch(program, title),
          !shellProcessDuplicatesAgentOrAlan(program, pane: pane)
    else {
        return nil
    }

    return ShellPaneTitleBarDetailProjection(
        id: "process",
        title: program,
        help: "Process \(program)"
    )
}

private func shellPaneAlanAccessoryProjection(for pane: ShellPane) -> ShellPaneTitleBarDetailProjection? {
    guard let binding = pane.alanBinding,
          pane.activity?.source.kind != .alan
    else {
        return nil
    }

    let title = binding.pendingYield ? "Input" : shellVisibleLabel(binding.runStatus)
    guard let title else { return nil }
    return ShellPaneTitleBarDetailProjection(
        id: "alan",
        title: title,
        help: "alan \(binding.runStatus)"
    )
}

private func shellProcessDuplicatesAgentOrAlan(_ program: String, pane: ShellPane) -> Bool {
    let lowercasedProgram = program.lowercased()
    if pane.alanBinding != nil || pane.resolvedLaunchTarget == .alan {
        return lowercasedProgram.contains("alan")
    }

    guard let activity = pane.activity else { return false }
    switch activity.source.kind {
    case .codex:
        return lowercasedProgram.contains("codex")
    case .claude:
        return lowercasedProgram.contains("claude")
    case .openCode:
        return lowercasedProgram.contains("opencode") || lowercasedProgram.contains("open-code")
    case .alan:
        return lowercasedProgram.contains("alan")
    case .shell, .progress, .command, .process, .unknown:
        return false
    }
}

private func shellLabelsMatch(_ lhs: String, _ rhs: String) -> Bool {
    lhs.trimmingCharacters(in: .whitespacesAndNewlines)
        .caseInsensitiveCompare(rhs.trimmingCharacters(in: .whitespacesAndNewlines)) == .orderedSame
}

func shellActivityNotificationKey(
    for activity: TerminalActivitySnapshot,
    paneID: String
) -> String {
    [
        paneID,
        activity.source.kind.rawValue,
        activity.status.rawValue,
        activity.freshness.updatedAt,
        shellActivityNotificationPayloadDiscriminator(for: activity),
    ].joined(separator: ":")
}

private func shellActivityNotificationPayloadDiscriminator(
    for activity: TerminalActivitySnapshot
) -> String {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    if let data = try? encoder.encode(activity) {
        return data.base64EncodedString()
    }

    return [
        activity.source.label ?? "",
        activity.priority.rawValue,
        activity.display.sourceFirstLabel,
        activity.freshness.staleAt ?? "",
        activity.freshness.expiresAt ?? "",
    ].joined(separator: "|")
}

func shellActivityAttention(for activity: TerminalActivitySnapshot) -> ShellAttentionState? {
    switch activity.status {
    case .needsInput, .exited:
        return .awaitingUser
    case .failed:
        return .notable
    case .paused, .progress, .running, .bell, .idle, .done, .stale:
        return nil
    }
}

func shellEffectiveAttention(for pane: ShellPane, now: Date? = nil) -> ShellAttentionState {
    let storedAttention = pane.attention
    guard let activity = pane.activity else { return storedAttention }

    if let now,
       !activity.isFresh(at: now),
       let activityAttention = shellActivityAttention(for: activity),
       activityAttention == storedAttention
    {
        return shellPersistentAttention(for: pane)
    }

    if let now, !activity.isFresh(at: now) {
        return storedAttention
    }

    guard let activityAttention = shellActivityAttention(for: activity),
          shellAttentionRank(for: activityAttention) > shellAttentionRank(for: storedAttention)
    else {
        return storedAttention
    }

    return activityAttention
}

private func shellPersistentAttention(for pane: ShellPane) -> ShellAttentionState {
    if pane.alanBinding?.pendingYield == true {
        return .awaitingUser
    }

    if pane.context?.processState == "exited"
        || pane.context?.surfaceReadiness == "child_exited"
    {
        return .awaitingUser
    }

    if pane.context?.rendererHealth == "failed"
        || pane.context?.rendererPhase == "failed"
        || pane.context?.surfaceReadiness == "renderer_failed"
    {
        return .notable
    }

    return .idle
}

private func shellAttentionRank(for attention: ShellAttentionState) -> Int {
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

func shellActivityNotificationRoute(
    for activity: TerminalActivitySnapshot,
    pane: ShellPane,
    tab: ShellTab?,
    visibility: ShellActivityNotificationVisibility,
    now: Date? = nil,
    longCommandThresholdMilliseconds: Int = 60_000
) -> ShellActivityNotificationRoute? {
    if let now, !activity.isFresh(at: now) {
        return nil
    }

    guard visibility != .focusedVisible else {
        return nil
    }

    let kind: ShellActivityNotificationKind
    let attention: ShellAttentionState
    switch activity.status {
    case .needsInput where shellIsCodingAgentActivity(activity):
        kind = .needsInput
        attention = .awaitingUser
    case .failed where shellIsCodingAgentActivity(activity):
        kind = .failed
        attention = .notable
    case .done
        where activity.source.kind == .command
            && (activity.command?.durationMilliseconds ?? 0) >= longCommandThresholdMilliseconds,
        .failed
        where activity.source.kind == .command
            && (activity.command?.durationMilliseconds ?? 0) >= longCommandThresholdMilliseconds:
        kind = .commandCompleted
        attention = .notable
    case .exited:
        kind = .processExited
        attention = .awaitingUser
    default:
        return nil
    }

    let subject = shellDisplayTitle(
        rawTitle: tab?.title ?? pane.viewport?.title,
        workingDirectoryName: pane.context?.workingDirectoryName,
        cwd: pane.cwd,
        program: pane.process?.program,
        launchTarget: pane.resolvedLaunchTarget,
        fallback: shellFallbackTitle(for: tab?.kind ?? .terminal)
    )
    return ShellActivityNotificationRoute(
        id: shellActivityNotificationKey(for: activity, paneID: pane.paneID),
        paneID: pane.paneID,
        tabID: pane.tabID,
        spaceID: pane.spaceID,
        kind: kind,
        title: activity.display.sourceFirstLabel,
        body: subject,
        attention: attention
    )
}

private func shellIsCodingAgentActivity(_ activity: TerminalActivitySnapshot) -> Bool {
    switch activity.source.kind {
    case .codex, .claude, .openCode, .alan:
        return true
    case .shell, .progress, .command, .process, .unknown:
        return false
    }
}

func shellSidebarTabProjection(
    for tab: ShellTab,
    panes allPanes: [ShellPane],
    focusedPaneID: String?,
    focusedTabID: String?,
    now: Date? = nil
) -> ShellSidebarTabProjection {
    let panes = shellOrderedPanes(for: tab, panes: allPanes)
    let primaryPane = shellPrimaryPane(in: panes, focusedPaneID: focusedPaneID)
    let title = shellSidebarTabTitle(for: tab, primaryPane: primaryPane)
    let isOwningTabFocused = focusedTabID == tab.tabID

    let activityCandidates = panes.enumerated().compactMap { index, pane -> TerminalActivitySnapshot? in
        guard let activity = pane.activity,
              activity.isSidebarWorthy(at: now, owningTabFocused: isOwningTabFocused)
        else { return nil }

        let hint: String?
        if panes.count > 1,
           pane.paneID != primaryPane?.paneID
        {
            hint = "Pane \(index + 1)"
        } else {
            hint = nil
        }
        return activity.withPaneHint(hint)
    }

    if let activity = TerminalActivitySnapshot.primarySidebarActivity(activityCandidates, now: now) {
        return ShellSidebarTabProjection(
            title: title,
            secondaryLine: activity.display.sourceFirstLabel,
            activity: activity,
            progress: activity.progress,
            accessibilityActivityLabel: activity.display.sourceFirstLabel
        )
    }

    let fallback = primaryPane.flatMap { shellTerminalStatusSummary(for: $0, now: now) }
        ?? primaryPane.flatMap { shellSidebarContextLine(for: $0, title: title) }
        ?? shellFallbackTitle(for: tab.kind)
    return ShellSidebarTabProjection(
        title: title,
        secondaryLine: fallback,
        activity: nil,
        progress: nil,
        accessibilityActivityLabel: nil
    )
}

func shellOrderedPanes(for tab: ShellTab, panes allPanes: [ShellPane]) -> [ShellPane] {
    let byID = Dictionary(uniqueKeysWithValues: allPanes.map { ($0.paneID, $0) })
    let ordered = tab.paneTree.paneIDs.compactMap { byID[$0] }
    if !ordered.isEmpty {
        return ordered
    }
    return allPanes.filter { $0.tabID == tab.tabID }
}

private func shellPrimaryPane(in panes: [ShellPane], focusedPaneID: String?) -> ShellPane? {
    if let focusedPaneID,
       let focused = panes.first(where: { $0.paneID == focusedPaneID })
    {
        return focused
    }
    return panes.first
}

private func shellSidebarTabTitle(for tab: ShellTab, primaryPane: ShellPane?) -> String {
    shellDisplayTitle(
        rawTitle: tab.title ?? primaryPane?.viewport?.title,
        workingDirectoryName: primaryPane?.context?.workingDirectoryName,
        cwd: primaryPane?.cwd,
        program: primaryPane?.process?.program,
        launchTarget: primaryPane?.resolvedLaunchTarget ?? .shell,
        fallback: shellFallbackTitle(for: tab.kind)
    )
}

private func shellSidebarContextLine(for pane: ShellPane, title: String) -> String? {
    let contextLabel = shellPathLeaf(pane.context?.repositoryRoot)
        ?? shellVisibleLabel(pane.context?.workingDirectoryName)
        ?? shellPathLeaf(pane.cwd)

    if let branch = shellVisibleLabel(pane.context?.gitBranch) {
        if let contextLabel, contextLabel != title {
            return "\(contextLabel) · \(branch)"
        }
        return branch
    }

    if let contextLabel {
        if contextLabel == title,
           let program = shellVisibleLabel(pane.process?.program)
        {
            return program
        }
        return contextLabel
    }

    return shellVisibleLabel(pane.process?.program)
}

private func shellFallbackTitle(for kind: ShellTabKind) -> String {
    switch kind {
    case .terminal:
        return "Terminal"
    case .scratch:
        return "Scratch"
    case .log:
        return "Logs"
    }
}
