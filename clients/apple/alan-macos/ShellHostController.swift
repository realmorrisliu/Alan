import Foundation

#if os(macOS)
import AppKit
import SwiftUI

struct ShellAttentionItem: Identifiable, Equatable {
    let paneID: String
    let spaceID: String
    let tabID: String
    let title: String
    let summary: String
    let attention: ShellAttentionState

    var id: String { paneID }
}

enum ShellTabCloseResult {
    case closed
    case tabNotFound
    case lastTab
}

enum ShellPaneCloseResult {
    case closed
    case paneNotFound
    case lastTab
}

enum ShellPaneLiftResult {
    case lifted
    case paneNotFound
    case lastPane
}

@MainActor
struct ShellWindowContext {
    let windowID: String
    let persistenceURL: URL
    let terminalRuntimeRegistry: TerminalRuntimeRegistry

    var controlRootURL: URL {
        alanShellControlPlaneRootURL(windowID: windowID)
    }

    var socketURL: URL {
        alanShellControlPlaneSocketURL(windowID: windowID)
    }

    var stateURL: URL {
        controlRootURL.appendingPathComponent("state.json")
    }

    var eventsURL: URL {
        controlRootURL.appendingPathComponent("events.jsonl")
    }

    static func make(
        fileManager: FileManager = .default,
        windowID: String = "window_\(UUID().uuidString.lowercased())",
        terminalRuntimeRegistry: TerminalRuntimeRegistry? = nil
    ) -> ShellWindowContext {
        ShellWindowContext(
            windowID: windowID,
            persistenceURL: ShellStatePersistenceStore.defaultPersistenceURL(
                windowID: windowID,
                fileManager: fileManager
            ),
            terminalRuntimeRegistry: terminalRuntimeRegistry ?? TerminalRuntimeRegistry()
        )
    }
}

@MainActor
final class ShellHostController: ObservableObject, TerminalHostActivationDelegate {
    enum StartupMode {
        case fresh
        case restorePrevious
        case workspaceManifest
    }

    private static let unpinnedTabRetentionTTL: TimeInterval = 12 * 60 * 60
    private static let iso8601Formatter = ISO8601DateFormatter()
    private let fileManager: FileManager
    private let windowContext: ShellWindowContext
    private let persistenceURL: URL
    private let persistenceStore: ShellStatePersistenceStore
    private let workspaceManifestStore: ShellWorkspaceManifestStore?
    private var workspaceManifest: ShellWorkspaceManifest?
    private var terminalActiveTasksByPaneID: [String: ShellTabActiveTaskState] = [:]
    private let paneProjection: ShellPaneProjectionService
    private let clipboardWriter: ShellClipboardWriter
    lazy var controlPlane = AlanShellControlPlane(windowID: windowContext.windowID) { [weak self] command in
        self?.handleControlPlaneCommand(command)
            ?? AlanShellControlResponse(
                requestID: command.requestID,
                contractVersion: "0.1",
                applied: false,
                state: nil,
                spaces: nil,
                tabs: nil,
                panes: nil,
                pane: nil,
                items: nil,
                candidates: nil,
                events: nil,
                focusedPaneID: nil,
                spaceID: command.spaceID,
                tabID: command.tabID,
                paneID: command.paneID,
                acceptedBytes: nil,
                deliveryCode: nil,
                runtimePhase: nil,
                latestEventID: nil,
                errorCode: "host_unavailable",
                errorMessage: "alan terminal workspace host is unavailable."
            )
    } stateAdoptionHandler: { [weak self] state in
        self?.adoptStateFromControlPlane(state)
    } bindingProjectionHandler: { [weak self] paneID, binding in
        self?.applyAlanBinding(binding, for: paneID)
    } diagnosticHandler: { [weak self] message in
        self?.recordControlPlaneDiagnostic(message)
    }

    @Published private(set) var shellState: ShellStateSnapshot
    @Published var selectedSpaceID: String?
    @Published var selectedTabID: String?
    @Published private(set) var lastCopiedAt: Date?
    @Published private(set) var terminalRuntime: TerminalHostRuntimeSnapshot = .placeholder
    @Published private(set) var controlPlaneDiagnostics: [String] = []
    @Published private(set) var commandInputRequestID = 0
    @Published private(set) var activityNotifications: [ShellActivityNotificationRoute] = []

    let terminalRuntimeRegistry: TerminalRuntimeRegistry
    private let appIsActiveProvider: @MainActor () -> Bool
    private var routedActivityNotificationKeys: Set<String> = []

    init(
        shellState: ShellStateSnapshot,
        fileManager: FileManager = .default,
        windowContext: ShellWindowContext? = nil,
        persistenceURL: URL? = nil,
        terminalRuntimeRegistry: TerminalRuntimeRegistry? = nil,
        workspaceManifestStore: ShellWorkspaceManifestStore? = nil,
        workspaceManifest: ShellWorkspaceManifest? = nil,
        appIsActiveProvider: @escaping @MainActor () -> Bool = { NSApp.isActive }
    ) {
        self.fileManager = fileManager
        self.paneProjection = ShellPaneProjectionService(fileManager: fileManager)
        let resolvedContext = windowContext ?? ShellWindowContext.make(fileManager: fileManager)
        self.windowContext = resolvedContext
        self.persistenceURL = persistenceURL ?? resolvedContext.persistenceURL
        self.persistenceStore = ShellStatePersistenceStore(
            fileManager: fileManager,
            persistenceURL: self.persistenceURL
        )
        self.workspaceManifestStore = workspaceManifestStore
        self.workspaceManifest = workspaceManifest
        self.clipboardWriter = ShellClipboardWriter()
        self.appIsActiveProvider = appIsActiveProvider
        self.shellState = shellState
        self.terminalRuntimeRegistry =
            terminalRuntimeRegistry
            ?? resolvedContext.terminalRuntimeRegistry
        self.selectedSpaceID = shellState.focusedSpaceID ?? shellState.spaces.first?.spaceID
        self.selectedTabID = shellState.focusedTabID ?? shellState.spaces.first?.tabs.first?.tabID

        if shellState.panes.isEmpty {
            publishControlPlaneState()
        } else {
            shellState.panes.map(\.paneID).forEach(primeBootContext)
        }
        synchronizeSelection()
    }

    deinit {
        let terminalRuntimeRegistry = terminalRuntimeRegistry
        Task { @MainActor in
            terminalRuntimeRegistry.releaseAllRuntimes()
        }
    }

    static func live(
        fileManager: FileManager = .default,
        windowContext: ShellWindowContext? = nil,
        startupMode: StartupMode = .fresh,
        workspaceManifestURL: URL? = nil,
        defaultWorkingDirectory: String? = nil,
        now: Date = .now
    ) -> ShellHostController {
        let usesStableWindowContext = startupMode == .restorePrevious || startupMode == .workspaceManifest
        let resolvedWindowContext =
            windowContext
            ?? ShellStatePersistenceStore.restoredWindowContext(
                fileManager: fileManager,
                restorePrevious: startupMode == .restorePrevious
            )
            ?? ShellStatePersistenceStore.defaultWindowContext(
                fileManager: fileManager,
                restorePrevious: usesStableWindowContext
            )
        let persistenceURL = resolvedWindowContext.persistenceURL
        let shellState: ShellStateSnapshot
        let manifestStore: ShellWorkspaceManifestStore?
        let manifest: ShellWorkspaceManifest?
        let manifestRecovery: ShellWorkspaceManifestRecovery?
        let retiredTabCount: Int
        switch startupMode {
        case .fresh:
            shellState = .bootstrapDefault(windowID: resolvedWindowContext.windowID)
            manifestStore = nil
            manifest = nil
            manifestRecovery = nil
            retiredTabCount = 0
        case .restorePrevious:
            shellState =
                ShellStatePersistenceStore.restoreShellState(
                    fileManager: fileManager,
                    persistenceURL: persistenceURL
                )
                ?? .bootstrapDefault(windowID: resolvedWindowContext.windowID)
            manifestStore = nil
            manifest = nil
            manifestRecovery = nil
            retiredTabCount = 0
        case .workspaceManifest:
            let workingDirectory = defaultWorkingDirectory
                ?? fileManager.homeDirectoryForCurrentUser.path
            let store = ShellWorkspaceManifestStore(
                fileManager: fileManager,
                manifestURL: workspaceManifestURL
                    ?? ShellWorkspaceManifestStore.defaultManifestURL(
                        windowID: resolvedWindowContext.windowID,
                        fileManager: fileManager
                    )
            )
            let loadResult = try? store.loadOrCreateDefault(
                windowID: resolvedWindowContext.windowID,
                defaultWorkingDirectory: workingDirectory,
                now: now
            )
            let loadedManifest = loadResult?.manifest
                ?? ShellWorkspaceManifest.defaultManifest(
                    windowID: resolvedWindowContext.windowID,
                    defaultWorkingDirectory: workingDirectory,
                    now: now
                )
            let retainedManifest = loadedManifest.pruningExpiredTabs(
                now: now,
                ttl: Self.unpinnedTabRetentionTTL
            )
            retiredTabCount = max(
                loadedManifest.spaces.reduce(0) { $0 + $1.tabs.count }
                    - retainedManifest.spaces.reduce(0) { $0 + $1.tabs.count },
                0
            )
            if retainedManifest != loadedManifest {
                try? store.save(retainedManifest)
            }
            shellState = ShellWorkspaceMaterializer.materialize(
                manifest: retainedManifest,
                defaultWorkingDirectory: workingDirectory,
                now: now
            )
            manifestStore = store
            manifest = retainedManifest
            manifestRecovery = loadResult?.recovery
        }

        let controller = ShellHostController(
            shellState: shellState,
            fileManager: fileManager,
            windowContext: resolvedWindowContext,
            persistenceURL: persistenceURL,
            workspaceManifestStore: manifestStore,
            workspaceManifest: manifest
        )
        if startupMode == .fresh {
            controller.persistShellState()
        }
        if let manifestRecovery {
            controller.recordWorkspaceManifestRecovery(manifestRecovery)
        }
        if retiredTabCount > 0 {
            controller.recordControlPlaneDiagnostic(
                "workspace manifest retired \(retiredTabCount) inactive unpinned tab(s)"
            )
        }
        return controller
    }

    var spaces: [ShellSpace] {
        shellState.spaces
    }

    var selectedSpace: ShellSpace? {
        shellState.spaces.first { $0.spaceID == selectedSpaceID } ?? shellState.spaces.first
    }

    var selectedTab: ShellTab? {
        guard let selectedTabID else {
            return selectedSpace?.tabs.first
        }
        return selectedSpace?.tabs.first { $0.tabID == selectedTabID } ?? selectedSpace?.tabs.first
    }

    var selectedTabPaneTree: ShellPaneTreeNode? {
        selectedTab?.paneTree
    }

    var panesForSelectedTab: [ShellPane] {
        guard let tabID = selectedTab?.tabID else { return [] }
        return shellState.panes.filter { $0.tabID == tabID }
    }

    var selectedPane: ShellPane? {
        if let focusedPane, focusedPane.tabID == selectedTab?.tabID {
            return focusedPane
        }
        return panesForSelectedTab.first
    }

    var focusedPane: ShellPane? {
        guard let focusedPaneID = shellState.focusedPaneID else { return nil }
        return pane(paneID: focusedPaneID)
    }

    var selectedPaneBootProfile: AlanShellBootProfile? {
        bootProfile(for: selectedPane)
    }

    var selectedPaneRuntime: TerminalHostRuntimeSnapshot {
        runtime(for: selectedPane?.paneID)
    }

    var attentionItems: [ShellAttentionItem] {
        let now = Date()
        return shellState.panes
            .compactMap { pane in
                let attention = shellEffectiveAttention(for: pane, now: now)
                guard attention != .idle else { return nil }
                return ShellAttentionItem(
                    paneID: pane.paneID,
                    spaceID: pane.spaceID,
                    tabID: pane.tabID,
                    title: pane.viewport?.title ?? pane.process?.program ?? "Pane",
                    summary: pane.viewport?.summary ?? "Activity detected",
                    attention: attention
                )
            }
            .sorted {
                Self.attentionRank(for: $0.attention) == Self.attentionRank(for: $1.attention)
                    ? $0.paneID < $1.paneID
                    : Self.attentionRank(for: $0.attention) > Self.attentionRank(for: $1.attention)
            }
    }

    var routingCandidates: [AlanShellRoutingCandidate] {
        routingCandidates(preferredPaneID: selectedPane?.paneID)
    }

    var moveDestinationTabs: [ShellTab] {
        guard let selectedPane else { return [] }
        return shellState.spaces
            .flatMap(\.tabs)
            .filter { $0.tabID != selectedPane.tabID }
            .sorted {
                if $0.tabID == $1.tabID {
                    return ($0.title ?? "") < ($1.title ?? "")
                }
                return $0.tabID < $1.tabID
            }
    }

    var awaitingAttentionCount: Int {
        attentionItems.filter { $0.attention == .awaitingUser }.count
    }

    var snapshotJSON: String {
        shellState.prettyPrintedJSON
    }

    func bootProfile(for pane: ShellPane?) -> AlanShellBootProfile? {
        guard let pane else { return nil }
        return AlanShellBootProfile.forPane(pane, shellState: shellState)
    }

    func runtime(for paneID: String?) -> TerminalHostRuntimeSnapshot {
        terminalRuntimeRegistry.snapshot(for: paneID)
    }

    func select(spaceID: String) {
        guard let paneID = targetPaneID(forSpaceID: spaceID) else {
            guard shellState.space(spaceID: spaceID) != nil else { return }
            shellState = ShellStateSnapshot(
                contractVersion: shellState.contractVersion,
                windowID: shellState.windowID,
                focusedSpaceID: spaceID,
                focusedTabID: nil,
                focusedPaneID: nil,
                spaces: shellState.spaces,
                panes: shellState.panes
            )
            synchronizeSelection()
            publishControlPlaneState()
            return
        }
        focus(paneID: paneID, requestTerminalFocus: true)
    }

    func select(tabID: String) {
        guard let paneID = targetPaneID(forTabID: tabID, in: selectedSpace) else { return }
        focus(paneID: paneID, requestTerminalFocus: true)
    }

    @discardableResult
    func selectSpace(at index: Int) -> Bool {
        guard spaces.indices.contains(index) else { return false }
        select(spaceID: spaces[index].spaceID)
        return true
    }

    @discardableResult
    func selectAdjacentSpace(offset: Int) -> Bool {
        guard spaces.count > 1 else { return false }
        guard let selectedSpaceID,
              let currentIndex = spaces.firstIndex(where: { $0.spaceID == selectedSpaceID })
        else {
            select(spaceID: spaces[0].spaceID)
            return true
        }

        let nextIndex = (currentIndex + offset + spaces.count) % spaces.count
        select(spaceID: spaces[nextIndex].spaceID)
        return true
    }

    @discardableResult
    func selectAdjacentTab(offset: Int) -> Bool {
        guard let selectedSpace,
              !selectedSpace.tabs.isEmpty
        else {
            return false
        }
        guard selectedSpace.tabs.count > 1 else { return false }
        let currentTabID = selectedTab?.tabID ?? selectedSpace.tabs.first?.tabID
        guard let currentTabID,
              let currentIndex = selectedSpace.tabs.firstIndex(where: { $0.tabID == currentTabID })
        else {
            return false
        }

        let nextIndex = (currentIndex + offset + selectedSpace.tabs.count) % selectedSpace.tabs.count
        select(tabID: selectedSpace.tabs[nextIndex].tabID)
        return true
    }

    func focusAttentionItem(_ item: ShellAttentionItem) {
        focus(paneID: item.paneID, requestTerminalFocus: true)
    }

    func focus(paneID: String) {
        focus(paneID: paneID, requestTerminalFocus: false)
    }

    private func focus(paneID: String, requestTerminalFocus: Bool) {
        guard let result = try? shellState.focusingPane(paneID) else { return }
        applyMutationResult(result)
        if requestTerminalFocus {
            terminalRuntimeRegistry.requestFocus(for: paneID)
        }
    }

    private func targetPaneID(forSpaceID spaceID: String) -> String? {
        guard let space = shellState.spaces.first(where: { $0.spaceID == spaceID }) else {
            return nil
        }
        let targetTab =
            space.tabs.first { tab in
                guard let focusedPaneID = shellState.focusedPaneID else { return false }
                return tab.contains(paneID: focusedPaneID)
            }
            ?? space.tabs.first
        return targetTab.flatMap(targetPaneID)
    }

    private func targetPaneID(
        forTabID tabID: String,
        in space: ShellSpace?
    ) -> String? {
        guard let tab = space?.tabs.first(where: { $0.tabID == tabID }) else {
            return nil
        }
        return targetPaneID(for: tab)
    }

    private func targetPaneID(for tab: ShellTab) -> String? {
        if let focusedPaneID = shellState.focusedPaneID,
           tab.contains(paneID: focusedPaneID)
        {
            return focusedPaneID
        }
        return tab.paneTree.paneIDs.first { paneID in
            pane(paneID: paneID)?.tabID == tab.tabID
        }
    }

    func requestCommandInput() {
        commandInputRequestID += 1
    }

    func refocusSelectedTerminalPane() {
        guard let paneID = selectedPane?.paneID else { return }
        terminalRuntimeRegistry.requestFocus(for: paneID)
    }

    func terminalHostDidRequestActivation(paneID: String) {
        focus(paneID: paneID)
    }

    @discardableResult
    func createSpace(
        launchTarget: ShellLaunchTarget = .shell,
        title: String? = nil,
        workingDirectory: String? = nil
    ) -> String? {
        let result = shellState.creatingSpace(
            launchTarget: launchTarget,
            title: title,
            workingDirectory: workingDirectory,
            reservedPaneIDs: terminalRuntimeRegistry.registeredPaneIDs
        )
        applyMutationResult(result)
        return result.spaceID
    }

    @discardableResult
    func createTerminalSpace(title: String? = nil, workingDirectory: String? = nil) -> String? {
        createSpace(launchTarget: .shell, title: title, workingDirectory: workingDirectory)
    }

    @discardableResult
    func createAlanSpace(title: String? = nil, workingDirectory: String? = nil) -> String? {
        createSpace(launchTarget: .alan, title: title, workingDirectory: workingDirectory)
    }

    @discardableResult
    func deleteSpace(spaceID: String) -> Bool {
        let result: ShellStateMutationResult
        do {
            result = try shellState.deletingSpace(spaceID)
        } catch {
            return false
        }
        applyMutationResult(result)
        return true
    }

    func isTabPinned(tabID: String) -> Bool {
        if let tab = shellState.tab(tabID: tabID) {
            return tab.isPinned
        }
        return workspaceManifest?
            .spaces
            .flatMap(\.tabs)
            .first { $0.tabID == tabID }?
            .isPinned == true
    }

    @discardableResult
    func pinTab(tabID: String? = nil) -> Bool {
        guard let targetTabID = tabID ?? selectedTabID else { return false }
        if isTabPinned(tabID: targetTabID) {
            return updatePinnedTabSnapshot(tabID: targetTabID)
        }

        let result: ShellStateMutationResult
        do {
            result = try shellState.pinningTab(targetTabID)
        } catch {
            return false
        }
        applyMutationResult(result, pinSnapshotTabIDs: [targetTabID])
        recordControlPlaneDiagnostic("workspace manifest pinned tab: \(targetTabID)")
        return true
    }

    @discardableResult
    func unpinTab(tabID: String? = nil) -> Bool {
        guard let targetTabID = tabID ?? selectedTabID else { return false }
        let result: ShellStateMutationResult
        do {
            result = try shellState.unpinningTab(targetTabID)
        } catch {
            return false
        }
        applyMutationResult(result)
        recordControlPlaneDiagnostic("workspace manifest unpinned tab: \(targetTabID)")
        return true
    }

    @discardableResult
    func updatePinnedTabSnapshot(tabID: String? = nil) -> Bool {
        guard let targetTabID = tabID ?? selectedTabID else { return false }
        guard isTabPinned(tabID: targetTabID) else { return false }
        return updateWorkspaceManifestTab(tabID: targetTabID) { tab, snapshot in
            tab.pinSnapshot = snapshot
            tab.liveSnapshot = snapshot
        } diagnostic: {
            "workspace manifest updated pinned tab: \($0)"
        }
    }

    @discardableResult
    func reorderTab(
        tabID: String,
        targetSpaceID: String? = nil,
        section: ShellTabOrganizationSection,
        index: Int
    ) -> Bool {
        let wasPinned = isTabPinned(tabID: tabID)
        let result: ShellStateMutationResult
        do {
            result = try shellState.organizingTab(
                tabID: tabID,
                targetSpaceID: targetSpaceID,
                section: section,
                index: index
            )
        } catch {
            return false
        }
        let needsPinSnapshot = !wasPinned && section == .pinned
        applyMutationResult(result, pinSnapshotTabIDs: needsPinSnapshot ? [tabID] : [])
        return true
    }

    @discardableResult
    func moveTab(tabID: String? = nil, offset: Int) -> Bool {
        guard let targetTabID = tabID ?? selectedTabID else { return false }
        let result: ShellStateMutationResult
        do {
            result = try shellState.movingTab(targetTabID, sectionOffset: offset)
        } catch {
            return false
        }
        applyMutationResult(result)
        return true
    }

    @discardableResult
    func moveTabToSpace(tabID: String, targetSpaceID: String) -> Bool {
        let result: ShellStateMutationResult
        do {
            result = try shellState.movingTabToSpace(
                tabID: tabID,
                targetSpaceID: targetSpaceID
            )
        } catch {
            return false
        }
        applyMutationResult(result)
        return true
    }

    @discardableResult
    func openTab(
        launchTarget: ShellLaunchTarget = .shell,
        in spaceID: String? = nil,
        title: String? = nil,
        workingDirectory: String? = nil
    ) -> String? {
        let result: ShellStateMutationResult
        do {
            result = try shellState.openingTab(
                launchTarget: launchTarget,
                in: spaceID,
                title: title,
                workingDirectory: workingDirectory,
                reservedPaneIDs: terminalRuntimeRegistry.registeredPaneIDs
            )
        } catch {
            return nil
        }
        applyMutationResult(result)
        return result.tabID
    }

    @discardableResult
    func openTerminalTab(
        in spaceID: String? = nil,
        title: String? = nil,
        workingDirectory: String? = nil
    ) -> String? {
        let resolvedWorkingDirectory = workingDirectory
            ?? focusedPaneWorkingDirectory()
        return openTab(
            launchTarget: .shell,
            in: spaceID,
            title: title,
            workingDirectory: resolvedWorkingDirectory
        )
    }

    @discardableResult
    func openAlanTab(
        in spaceID: String? = nil,
        title: String? = nil,
        workingDirectory: String? = nil
    ) -> String? {
        openTab(
            launchTarget: .alan,
            in: spaceID,
            title: title,
            workingDirectory: workingDirectory
        )
    }

    @discardableResult
    func splitFocusedPane(direction: ShellSplitDirection) -> String? {
        splitFocusedPane(placement: .defaultPlacement(for: direction))
    }

    @discardableResult
    func splitFocusedPane(placement: ShellPaneSplitDirection) -> String? {
        guard let focusedPaneID = shellState.focusedPaneID else { return nil }
        return splitPane(paneID: focusedPaneID, placement: placement)
    }

    @discardableResult
    func splitPane(paneID: String, direction: ShellSplitDirection) -> String? {
        splitPane(paneID: paneID, placement: .defaultPlacement(for: direction))
    }

    @discardableResult
    func splitPane(paneID: String, placement: ShellPaneSplitDirection) -> String? {
        let result: ShellStateMutationResult
        do {
            result = try shellState.splittingPane(
                paneID,
                placement: placement,
                reservedPaneIDs: terminalRuntimeRegistry.registeredPaneIDs
            )
        } catch {
            return nil
        }
        applyMutationResult(result)
        return result.paneID
    }

    @discardableResult
    func focusAdjacentPane(direction: ShellSpatialFocusDirection) -> Bool {
        let result: ShellStateMutationResult
        do {
            result = try shellState.focusingAdjacentPane(direction)
        } catch {
            return false
        }
        applyMutationResult(result)
        return true
    }

    @discardableResult
    func performShellWorkspaceCommand(_ command: ShellWorkspaceCommand) -> Bool {
        switch command {
        case .newTerminalTab:
            return openTerminalTab() != nil
        case .newAlanTab:
            return openAlanTab() != nil
        case .splitLeft:
            return splitFocusedPane(placement: .left) != nil
        case .splitRight:
            return splitFocusedPane(placement: .right) != nil
        case .splitUp:
            return splitFocusedPane(placement: .up) != nil
        case .splitDown:
            return splitFocusedPane(placement: .down) != nil
        case .focusLeft:
            return focusAdjacentPane(direction: .left)
        case .focusRight:
            return focusAdjacentPane(direction: .right)
        case .focusUp:
            return focusAdjacentPane(direction: .up)
        case .focusDown:
            return focusAdjacentPane(direction: .down)
        case .equalizeSplits:
            return equalizeSelectedTabSplits()
        case .closePane:
            return closeSelectedPane()
        case .closeTab:
            return closeSelectedTab()
        }
    }

    func shellActionTitle(_ id: ShellActionID) -> String {
        ShellActionRegistry.standard.action(for: id)?.title ?? "Unavailable"
    }

    func shellActionAvailability(
        _ id: ShellActionID,
        target: ShellActionTarget = .currentSelection
    ) -> ShellActionAvailability {
        ShellActionRegistry.standard.resolve(id, target: target, state: shellState).availability
    }

    func shellActionShortcut(
        _ id: ShellActionID,
        target: ShellActionTarget = .currentSelection
    ) -> ShellActionShortcut? {
        ShellActionRegistry.standard.defaultShortcut(for: id, target: target)
    }

    @discardableResult
    func performShellAction(
        _ id: ShellActionID,
        target: ShellActionTarget = .currentSelection
    ) -> ShellActionExecutionResult {
        ShellActionRegistry.standard.execute(
            id,
            target: target,
            state: shellState
        ) { [weak self] effect in
            self?.performShellActionEffect(effect) ?? false
        }
    }

    private func performShellActionEffect(_ effect: ShellActionEffect) -> Bool {
        switch effect {
        case .workspaceCommand(let command):
            return performShellWorkspaceCommand(command)
        case .openTab(let launchTarget, let spaceID):
            switch launchTarget {
            case .shell:
                return openTerminalTab(in: spaceID) != nil
            case .alan:
                return openAlanTab(in: spaceID) != nil
            }
        case .closeTab(let tabID):
            guard let tabID else { return closeSelectedTab() }
            return closeTab(tabID: tabID) == .closed
        case .closePane(let paneID):
            guard let paneID else { return closeSelectedPane() }
            return closePane(paneID: paneID) == .closed
        case .selectAdjacentTab(let offset):
            return selectAdjacentTab(offset: offset)
        case .selectAdjacentSpace(let offset):
            return selectAdjacentSpace(offset: offset)
        case .selectSpaceAt(let index):
            return selectSpace(at: index)
        case .pinTab(let tabID):
            return pinTab(tabID: tabID)
        case .unpinTab(let tabID):
            return unpinTab(tabID: tabID)
        case .updatePinnedTab(let tabID):
            return updatePinnedTabSnapshot(tabID: tabID)
        case .moveTab(let tabID, let offset):
            return moveTab(tabID: tabID, offset: offset)
        case .moveTabToSpace(let tabID, let spaceID):
            guard let tabID, let spaceID else { return false }
            return moveTabToSpace(tabID: tabID, targetSpaceID: spaceID)
        case .disabledPlaceholder:
            return false
        }
    }

    @discardableResult
    func resizeSplit(splitNodeID: String, ratio: Double, persist: Bool = true) -> Bool {
        let result: ShellStateMutationResult
        do {
            result = try shellState.resizingSplit(splitNodeID, ratio: ratio)
        } catch {
            return false
        }
        applyMutationResult(result, publish: persist)
        return true
    }

    @discardableResult
    func equalizeSelectedTabSplits() -> Bool {
        let result: ShellStateMutationResult
        do {
            result = try shellState.equalizingSplits(in: selectedTabID)
        } catch {
            return false
        }
        applyMutationResult(result)
        return true
    }

    @discardableResult
    func closeSelectedTab() -> Bool {
        guard let selectedTabID else { return false }
        return closeTab(tabID: selectedTabID) == .closed
    }

    @discardableResult
    func closeSelectedPane() -> Bool {
        guard let paneID = selectedPane?.paneID else { return false }
        return closePane(paneID: paneID) == .closed
    }

    @discardableResult
    func closePaneByID(_ paneID: String) -> Bool {
        closePane(paneID: paneID) == .closed
    }

    @discardableResult
    func liftSelectedPaneToTab(title: String? = nil) -> Bool {
        guard let paneID = selectedPane?.paneID else { return false }
        return liftPaneToTab(paneID: paneID, title: title) == .lifted
    }

    @discardableResult
    func moveSelectedPane(
        toTab tabID: String,
        direction: ShellSplitDirection = .vertical
    ) -> Bool {
        guard let paneID = selectedPane?.paneID else { return false }
        return movePane(paneID: paneID, toTab: tabID, direction: direction)
    }

    @discardableResult
    func focusTopRoutingCandidate(preferredPaneID: String? = nil) -> String? {
        guard let candidate = routingCandidates(preferredPaneID: preferredPaneID).first else {
            return nil
        }
        focus(paneID: candidate.paneID)
        return candidate.paneID
    }

    @discardableResult
    func setAttention(_ attention: ShellAttentionState, for paneID: String) -> Bool {
        let result: ShellStateMutationResult
        do {
            result = try shellState.settingAttention(attention, for: paneID)
        } catch {
            return false
        }
        applyMutationResult(result)
        return true
    }

    func copySnapshotJSON() {
        clipboardWriter.writeString(snapshotJSON)
        lastCopiedAt = .now
    }

    func updateTerminalRuntime(_ runtime: TerminalHostRuntimeSnapshot) {
        terminalRuntimeRegistry.updateSnapshot(runtime)

        if let paneID = runtime.paneID,
           runtime.isFocused,
           shellState.focusedPaneID != paneID
        {
            focus(paneID: paneID)
            return
        }

        if runtime.paneID == selectedPane?.paneID || runtime.paneID == shellState.focusedPaneID {
            terminalRuntime = runtime
        }

        if let paneID = runtime.paneID,
           let pane = pane(paneID: paneID)
        {
            let bootProfile = AlanShellBootProfile.forPane(pane, shellState: shellState)
            let runtimeProcessExited = paneProjection.projectedProcessExited(
                metadataProcessExited: runtime.paneMetadata.processExited,
                surfaceState: runtime.surfaceState
            ) ?? runtime.paneMetadata.processExited
            let activeTaskChanged = recordTerminalActiveTask(
                runtime.paneMetadata.activeTaskState,
                processExited: runtimeProcessExited,
                for: paneID
            )
            let projectedActivity = runtime.paneMetadata.clearsActivity
                ? nil
                : (runtime.paneMetadata.activity ?? pane.activity)
            if runtimeProcessExited {
                routeActivityNotificationIfNeeded(from: pane, nextActivity: projectedActivity)
            }
            if closePaneAfterChildExitIfNeeded(paneID: paneID, processExited: runtimeProcessExited) {
                return
            }
            let projectedContext = paneProjection.projectedContext(
                for: pane,
                bootProfile: bootProfile,
                workingDirectory: runtime.paneMetadata.workingDirectory ?? pane.cwd,
                processExited: runtime.paneMetadata.processExited,
                lastCommandExitCode: runtime.paneMetadata.lastCommandExitCode,
                lastMetadataAt: runtime.paneMetadata.lastUpdatedAt,
                activeTaskState: runtime.paneMetadata.activeTaskState,
                existing: pane.context,
                runtime: runtime
            )

            let didPublishPaneUpdate = updatePaneState(paneID: paneID) { current in
                let projectedBinding = paneProjection.projectedAlanBinding(
                    for: current,
                    binding: current.alanBinding,
                    processExited: runtimeProcessExited
                )
                let viewport = paneProjection.projectedViewport(
                    current: current,
                    metadata: runtime.paneMetadata,
                    runtime: runtime
                )
                return ShellPane(
                    paneID: current.paneID,
                    tabID: current.tabID,
                    spaceID: current.spaceID,
                    launchTarget: current.launchTarget,
                    cwd: current.cwd ?? bootProfile.workingDirectory,
                    process: current.process,
                    attention: paneProjection.projectedAttention(
                        metadataAttention: runtime.paneMetadata.attention,
                        processExited: runtimeProcessExited,
                        binding: projectedBinding,
                        surfaceState: runtime.surfaceState
                    ),
                    context: projectedContext,
                    viewport: viewport,
                    activity: projectedActivity,
                    alanBinding: projectedBinding
                )
            }
            if activeTaskChanged && !didPublishPaneUpdate {
                syncWorkspaceManifestFromShellState()
            }
        }
    }

    func updateTerminalMetadata(_ metadata: TerminalPaneMetadataSnapshot, for paneID: String) {
        guard let pane = pane(paneID: paneID) else { return }
        let bootProfile = AlanShellBootProfile.forPane(pane, shellState: shellState)
        let runtime = runtime(for: pane.paneID)
        let metadataProcessExited = paneProjection.projectedProcessExited(
            metadataProcessExited: metadata.processExited,
            surfaceState: runtime.surfaceState
        ) ?? metadata.processExited
        let activeTaskChanged = recordTerminalActiveTask(
            metadata.activeTaskState,
            processExited: metadataProcessExited,
            for: paneID
        )
        let projectedActivity = metadata.clearsActivity ? nil : (metadata.activity ?? pane.activity)
        if metadataProcessExited {
            routeActivityNotificationIfNeeded(from: pane, nextActivity: projectedActivity)
        }
        if closePaneAfterChildExitIfNeeded(paneID: paneID, processExited: metadataProcessExited) {
            return
        }
        let projectedContext = paneProjection.projectedContext(
            for: pane,
            bootProfile: bootProfile,
            workingDirectory: metadata.workingDirectory ?? pane.cwd,
            processExited: metadata.processExited,
            lastCommandExitCode: metadata.lastCommandExitCode,
            lastMetadataAt: metadata.lastUpdatedAt,
            activeTaskState: metadata.activeTaskState,
            existing: pane.context,
            runtime: runtime
        )

        let didPublishPaneUpdate = updatePaneState(
            paneID: pane.paneID,
            tabTitleOverride: metadata.title
        ) { current in
            let projectedBinding = paneProjection.projectedAlanBinding(
                for: current,
                binding: current.alanBinding,
                processExited: metadataProcessExited
            )
            let viewport = paneProjection.projectedViewport(
                current: current,
                metadata: metadata,
                runtime: runtime
            )

            return ShellPane(
                paneID: current.paneID,
                tabID: current.tabID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: metadata.workingDirectory ?? current.cwd ?? bootProfile.workingDirectory,
                process: current.process,
                attention: paneProjection.projectedAttention(
                    metadataAttention: metadata.attention,
                    processExited: metadataProcessExited,
                    binding: projectedBinding,
                    surfaceState: runtime.surfaceState
                ),
                context: projectedContext,
                viewport: viewport,
                activity: projectedActivity,
                alanBinding: projectedBinding
            )
        }
        if activeTaskChanged && !didPublishPaneUpdate {
            syncWorkspaceManifestFromShellState()
        }
    }

    private func applyAlanBinding(_ binding: ShellAlanBinding?, for paneID: String) {
        guard let pane = pane(paneID: paneID) else { return }
        let bootProfile = AlanShellBootProfile.forPane(pane, shellState: shellState)
        let runtime = runtime(for: pane.paneID)
        let runtimeProcessExited = paneProjection.projectedProcessExited(
            metadataProcessExited: runtime.paneMetadata.processExited,
            surfaceState: runtime.surfaceState
        ) ?? runtime.paneMetadata.processExited
        let projectedBinding = paneProjection.projectedAlanBinding(
            for: pane,
            binding: binding,
            processExited: runtimeProcessExited
        )
        let projectedContext = paneProjection.projectedContext(
            for: pane,
            bootProfile: bootProfile,
            workingDirectory: pane.cwd,
            processExited: nil,
            lastCommandExitCode: pane.context?.lastCommandExitCode,
            lastMetadataAt: nil,
            activeTaskState: runtime.paneMetadata.activeTaskState,
            existing: pane.context,
            runtime: runtime
        )

        updatePaneState(paneID: paneID) { current in
            let bindingSummary: String?
            if let projectedBinding {
                bindingSummary = projectedBinding.pendingYield
                    ? "alan is waiting for user input"
                    : "alan run status: \(projectedBinding.runStatus)"
            } else {
                bindingSummary = nil
            }

            let viewport = ShellViewportSnapshot(
                title: current.viewport?.title,
                summary: bindingSummary ?? current.viewport?.summary,
                visibleExcerpt: current.viewport?.visibleExcerpt,
                lastActivityAt: binding?.lastProjectedAt ?? current.viewport?.lastActivityAt
            )

            return ShellPane(
                paneID: current.paneID,
                tabID: current.tabID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: current.cwd ?? bootProfile.workingDirectory,
                process: current.process,
                attention: projectedBinding?.pendingYield == true ? .awaitingUser : current.attention,
                context: projectedContext,
                viewport: viewport,
                activity: current.activity,
                alanBinding: projectedBinding
            )
        }
    }

    private func primeBootContext(for paneID: String) {
        guard let pane = pane(paneID: paneID) else { return }
        let bootProfile = AlanShellBootProfile.forPane(pane, shellState: shellState)
        let runtime = runtime(for: pane.paneID)
        let runtimeProcessExited = paneProjection.projectedProcessExited(
            metadataProcessExited: nil,
            surfaceState: runtime.surfaceState
        ) ?? false
        let projectedContext = paneProjection.projectedContext(
            for: pane,
            bootProfile: bootProfile,
            workingDirectory: pane.cwd ?? bootProfile.workingDirectory,
            processExited: nil,
            lastCommandExitCode: pane.context?.lastCommandExitCode,
            lastMetadataAt: nil,
            activeTaskState: runtime.paneMetadata.activeTaskState,
            existing: pane.context,
            runtime: runtime
        )

        updatePaneState(paneID: paneID) { current in
            let projectedBinding = paneProjection.projectedAlanBinding(
                for: current,
                binding: current.alanBinding,
                processExited: runtimeProcessExited
            )
            return ShellPane(
                paneID: current.paneID,
                tabID: current.tabID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: current.cwd ?? bootProfile.workingDirectory,
                process: current.process,
                attention: current.attention,
                context: projectedContext,
                viewport: current.viewport,
                activity: current.activity,
                alanBinding: projectedBinding
            )
        }
    }

    @discardableResult
    private func updatePaneState(
        paneID: String,
        tabTitleOverride: String? = nil,
        transform: (ShellPane) -> ShellPane
    ) -> Bool {
        guard let existingPane = shellState.panes.first(where: { $0.paneID == paneID }) else {
            return false
        }
        let transformedPane = transform(existingPane)
        let currentTabTitle = shellState.tab(tabID: existingPane.tabID)?.title
        let requestedTabTitle = tabTitleOverride ?? currentTabTitle

        guard transformedPane != existingPane || requestedTabTitle != currentTabTitle else {
            return false
        }

        let updatedPanes = shellState.panes.map { pane in
            pane.paneID == paneID ? transformedPane : pane
        }
        let updatedSpaces = rebuildSpaces(
            using: updatedPanes,
            tabTitleOverride: tabTitleOverride,
            paneID: paneID
        )

        shellState = ShellStateSnapshot(
            contractVersion: shellState.contractVersion,
            windowID: shellState.windowID,
            focusedSpaceID: shellState.focusedSpaceID,
            focusedTabID: shellState.focusedTabID,
            focusedPaneID: shellState.focusedPaneID,
            spaces: updatedSpaces,
            panes: updatedPanes
        )
        synchronizeSelection()
        routeActivityNotificationIfNeeded(from: existingPane, to: transformedPane)
        publishControlPlaneState()
        return true
    }

    private func routeActivityNotificationIfNeeded(
        from existingPane: ShellPane,
        nextActivity: TerminalActivitySnapshot?
    ) {
        guard existingPane.activity != nextActivity,
              let activity = nextActivity,
              let tab = shellState.tab(tabID: existingPane.tabID)
        else {
            return
        }

        routeActivityNotificationIfNeeded(
            activity: activity,
            pane: existingPane,
            tab: tab
        )
    }

    private func routeActivityNotificationIfNeeded(
        from existingPane: ShellPane,
        to updatedPane: ShellPane
    ) {
        guard existingPane.activity != updatedPane.activity,
              let activity = updatedPane.activity,
              let tab = shellState.tab(tabID: updatedPane.tabID)
        else {
            return
        }

        routeActivityNotificationIfNeeded(
            activity: activity,
            pane: updatedPane,
            tab: tab
        )
    }

    private func routeActivityNotificationIfNeeded(
        activity: TerminalActivitySnapshot,
        pane: ShellPane,
        tab: ShellTab
    ) {
        let key = shellActivityNotificationKey(for: activity, paneID: pane.paneID)
        guard !routedActivityNotificationKeys.contains(key),
              let route = shellActivityNotificationRoute(
                  for: activity,
                  pane: pane,
                  tab: tab,
                  visibility: activityNotificationVisibility(for: pane),
                  now: .now
              )
        else {
            return
        }

        routedActivityNotificationKeys.insert(key)
        activityNotifications.append(route)
        if activityNotifications.count > 50 {
            activityNotifications.removeFirst(activityNotifications.count - 50)
        }
    }

    private func activityNotificationVisibility(
        for pane: ShellPane
    ) -> ShellActivityNotificationVisibility {
        let isSelectedSpace = pane.spaceID == selectedSpace?.spaceID
        let isSelectedTab = pane.tabID == selectedTab?.tabID
        guard appIsActiveProvider() else {
            return .background
        }
        if isSelectedSpace,
           isSelectedTab,
           pane.paneID == shellState.focusedPaneID
        {
            return .focusedVisible
        }

        if isSelectedSpace, isSelectedTab {
            return .visibleUnfocused
        }

        return .background
    }

    private func rebuildSpaces(
        using panes: [ShellPane],
        tabTitleOverride: String?,
        paneID: String
    ) -> [ShellSpace] {
        let tabID = shellState.panes.first(where: { $0.paneID == paneID })?.tabID

        return shellState.spaces.map { space in
            let tabs = space.tabs.map { tab in
                let nextTitle: String?
                if tab.tabID == tabID, let tabTitleOverride {
                    nextTitle = tabTitleOverride
                } else {
                    nextTitle = tab.title
                }

                return ShellTab(
                    tabID: tab.tabID,
                    kind: tab.kind,
                    title: nextTitle,
                    paneTree: tab.paneTree,
                    isPinned: tab.isPinned
                )
            }

            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: strongestAttention(in: panes.filter { $0.spaceID == space.spaceID }),
                tabs: tabs
            )
        }
    }

    private func replaceShellState(
        spaces: [ShellSpace],
        panes: [ShellPane],
        focusedPaneID: String?
    ) {
        let resolvedFocusedPaneID =
            focusedPaneID.flatMap { candidate in
                panes.contains(where: { $0.paneID == candidate }) ? candidate : nil
            } ?? panes.first?.paneID
        let focusedPane = resolvedFocusedPaneID.flatMap { candidate in
            panes.first(where: { $0.paneID == candidate })
        }

        shellState = ShellStateSnapshot(
            contractVersion: shellState.contractVersion,
            windowID: shellState.windowID,
            focusedSpaceID: focusedPane?.spaceID ?? spaces.first?.spaceID,
            focusedTabID: focusedPane?.tabID ?? spaces.first?.tabs.first?.tabID,
            focusedPaneID: resolvedFocusedPaneID,
            spaces: spaces,
            panes: panes
        )
        synchronizeSelection()
        publishControlPlaneState()
    }

    private func applyMutationResult(
        _ result: ShellStateMutationResult,
        publish: Bool = true,
        pinSnapshotTabIDs: Set<String> = []
    ) {
        adoptStateFromControlPlane(result.state, publish: publish && pinSnapshotTabIDs.isEmpty)
        if publish && !pinSnapshotTabIDs.isEmpty {
            publishControlPlaneState(pinSnapshotTabIDs: pinSnapshotTabIDs)
        }
    }

    private func adoptStateFromControlPlane(
        _ state: ShellStateSnapshot,
        publish: Bool = true
    ) {
        let paneIDs = Set(state.panes.map(\.paneID))
        terminalRuntimeRegistry.releaseRuntimes(excluding: paneIDs)

        let hydratedPanes = state.panes.map { pane in
            guard paneProjection.needsBootContextProjection(pane) else { return pane }
            let bootProfile = AlanShellBootProfile.forPane(pane, shellState: state)
            let projectedContext = paneProjection.projectedContext(
                for: pane,
                bootProfile: bootProfile,
                workingDirectory: pane.cwd ?? bootProfile.workingDirectory,
                processExited: nil,
                lastCommandExitCode: pane.context?.lastCommandExitCode,
                lastMetadataAt: nil,
                activeTaskState: self.runtime(for: pane.paneID).paneMetadata.activeTaskState,
                existing: pane.context,
                runtime: self.runtime(for: pane.paneID)
            )
            return ShellPane(
                paneID: pane.paneID,
                tabID: pane.tabID,
                spaceID: pane.spaceID,
                launchTarget: pane.launchTarget,
                cwd: pane.cwd ?? bootProfile.workingDirectory,
                process: pane.process,
                attention: pane.attention,
                context: projectedContext,
                viewport: pane.viewport,
                activity: pane.activity,
                alanBinding: pane.alanBinding
            )
        }

        let hydratedSpaces = state.spaces.map { space in
            ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: strongestAttention(in: hydratedPanes.filter { $0.spaceID == space.spaceID }),
                tabs: space.tabs
            )
        }

        shellState = ShellStateSnapshot(
            contractVersion: state.contractVersion,
            windowID: state.windowID,
            focusedSpaceID: state.focusedSpaceID,
            focusedTabID: state.focusedTabID,
            focusedPaneID: state.focusedPaneID,
            spaces: hydratedSpaces,
            panes: hydratedPanes
        )
        synchronizeSelection()
        if publish {
            publishControlPlaneState()
        }
    }

    private func recordControlPlaneDiagnostic(_ message: String) {
        let line = "\(Self.iso8601Formatter.string(from: .now)) \(message)"
        guard controlPlaneDiagnostics.last != line else { return }
        controlPlaneDiagnostics.append(line)
        if controlPlaneDiagnostics.count > 12 {
            controlPlaneDiagnostics.removeFirst(controlPlaneDiagnostics.count - 12)
        }
    }

    private func synchronizeSelection() {
        if let focusedPane = focusedPane {
            selectedSpaceID = focusedPane.spaceID
            selectedTabID = focusedPane.tabID
            terminalRuntime = runtime(for: focusedPane.paneID)
            return
        }

        selectedSpaceID = shellState.focusedSpaceID ?? selectedSpaceID ?? shellState.spaces.first?.spaceID
        selectedTabID = shellState.focusedTabID ?? selectedSpace?.tabs.first?.tabID
        terminalRuntime = runtime(for: selectedPane?.paneID)
    }

    private func persistShellState() {
        persistenceStore.save(shellState)
    }

    private func syncWorkspaceManifestFromShellState(
        now: Date = .now,
        pinSnapshotTabIDs: Set<String> = []
    ) {
        guard let workspaceManifestStore else { return }

        let nextManifest = makeWorkspaceManifestFromShellState(now: now)
        var manifestToSave = nextManifest
        if !pinSnapshotTabIDs.isEmpty {
            applyPinSnapshotOverrides(to: &manifestToSave, tabIDs: pinSnapshotTabIDs)
        }
        do {
            try workspaceManifestStore.save(manifestToSave)
            workspaceManifest = manifestToSave
        } catch {
            recordControlPlaneDiagnostic("workspace manifest save failed: \(error)")
        }
    }

    private func applyPinSnapshotOverrides(
        to manifest: inout ShellWorkspaceManifest,
        tabIDs: Set<String>
    ) {
        for spaceIndex in manifest.spaces.indices {
            for tabIndex in manifest.spaces[spaceIndex].tabs.indices {
                let tabID = manifest.spaces[spaceIndex].tabs[tabIndex].tabID
                guard tabIDs.contains(tabID),
                      let tab = shellState.tab(tabID: tabID)
                else { continue }

                let snapshot = makeRestoreSnapshot(for: tab, panes: shellState.panes(in: tabID))
                manifest.spaces[spaceIndex].tabs[tabIndex].isPinned = true
                manifest.spaces[spaceIndex].tabs[tabIndex].pinSnapshot = snapshot
                manifest.spaces[spaceIndex].tabs[tabIndex].liveSnapshot = snapshot
            }
        }
    }

    private func updateWorkspaceManifestTab(
        tabID: String,
        mutate: (inout ShellWorkspaceTabRecord, ShellTabRestoreSnapshot) -> Void,
        diagnostic: (String) -> String
    ) -> Bool {
        guard let tab = shellState.tab(tabID: tabID),
              let workspaceManifestStore
        else {
            return false
        }

        let panes = shellState.panes(in: tabID)
        let snapshot = makeRestoreSnapshot(for: tab, panes: panes)
        var manifest = makeWorkspaceManifestFromShellState(now: .now)
        var didUpdate = false

        for spaceIndex in manifest.spaces.indices {
            guard let tabIndex = manifest.spaces[spaceIndex].tabs.firstIndex(where: { $0.tabID == tabID }) else {
                continue
            }
            mutate(&manifest.spaces[spaceIndex].tabs[tabIndex], snapshot)
            didUpdate = true
            break
        }

        guard didUpdate else { return false }

        do {
            try workspaceManifestStore.save(manifest)
            workspaceManifest = manifest
            objectWillChange.send()
            recordControlPlaneDiagnostic(diagnostic(tabID))
            return true
        } catch {
            recordControlPlaneDiagnostic("workspace manifest save failed: \(error)")
            return false
        }
    }

    private func makeWorkspaceManifestFromShellState(now: Date) -> ShellWorkspaceManifest {
        let existingSpaces = Dictionary(
            uniqueKeysWithValues: (workspaceManifest?.spaces ?? []).map { ($0.spaceID, $0) }
        )
        let existingTabs = Dictionary(
            uniqueKeysWithValues: (workspaceManifest?.spaces ?? [])
                .flatMap(\.tabs)
                .map { ($0.tabID, $0) }
        )

        let spaces = shellState.spaces.enumerated().map { index, space -> ShellWorkspaceSpaceRecord in
            let existingSpace = existingSpaces[space.spaceID]
            let tabRecords = space.tabs.map { tab -> ShellWorkspaceTabRecord in
                let existingTab = existingTabs[tab.tabID]
                let panes = shellState.panes(in: tab.tabID)
                let snapshot = makeRestoreSnapshot(for: tab, panes: panes)
                let paneActivityAt = panes.compactMap { paneActivityDate($0) }.max()
                let lastActivatedAt = tab.tabID == shellState.focusedTabID
                    ? now
                    : (existingTab?.lastActivatedAt ?? now)
                let lastActivityAt = max(
                    existingTab?.lastActivityAt ?? now,
                    paneActivityAt ?? existingTab?.lastActivityAt ?? now
                )

                return ShellWorkspaceTabRecord(
                    tabID: tab.tabID,
                    title: tab.title,
                    kind: tab.kind,
                    createdAt: existingTab?.createdAt ?? now,
                    lastActivatedAt: lastActivatedAt,
                    lastActivityAt: lastActivityAt,
                    isPinned: tab.isPinned,
                    pinSnapshot: tab.isPinned ? existingTab?.pinSnapshot : nil,
                    liveSnapshot: snapshot,
                    activeTask: projectedActiveTask(for: tab, panes: panes)
                )
            }

            return ShellWorkspaceSpaceRecord(
                spaceID: space.spaceID,
                title: space.title,
                order: existingSpace?.order ?? index,
                createdAt: existingSpace?.createdAt ?? now,
                updatedAt: now,
                tabs: tabRecords
            )
        }

        var manifest = ShellWorkspaceManifest(
            schemaVersion: ShellWorkspaceManifest.currentSchemaVersion,
            windowID: shellState.windowID,
            selectedSpaceID: shellState.focusedSpaceID ?? selectedSpaceID,
            selectedTabID: shellState.focusedTabID,
            spaces: spaces
        )
        manifest.repairSelection()
        return manifest
    }

    private func makeRestoreSnapshot(
        for tab: ShellTab,
        panes: [ShellPane]
    ) -> ShellTabRestoreSnapshot {
        let paneByID = Dictionary(uniqueKeysWithValues: panes.map { ($0.paneID, $0) })
        let restorePanes = tab.paneTree.paneIDs.compactMap { paneID -> ShellPaneRestoreRecord? in
            guard let pane = paneByID[paneID] else { return nil }
            return ShellPaneRestoreRecord(
                paneID: pane.paneID,
                launchTarget: pane.resolvedLaunchTarget,
                cwd: pane.cwd,
                title: pane.viewport?.title ?? tab.title
            )
        }
        return ShellTabRestoreSnapshot(paneTree: tab.paneTree, panes: restorePanes)
    }

    private func paneActivityDate(_ pane: ShellPane) -> Date? {
        if let lastActivityAt = pane.viewport?.lastActivityAt,
           let date = Self.iso8601Formatter.date(from: lastActivityAt)
        {
            return date
        }

        if let lastMetadataAt = pane.context?.lastMetadataAt,
           let date = Self.iso8601Formatter.date(from: lastMetadataAt)
        {
            return date
        }

        return nil
    }

    private func projectedActiveTask(
        for tab: ShellTab,
        panes: [ShellPane]
    ) -> ShellTabActiveTaskState {
        if let terminalActiveTask = strongestTerminalActiveTask(
            in: panes.filter { tab.contains(paneID: $0.paneID) }
        ),
           terminalActiveTask.protectsFromPruning
        {
            return terminalActiveTask
        }

        for pane in panes where tab.contains(paneID: pane.paneID) {
            if pane.alanBinding?.pendingYield == true {
                return .alanPendingYield
            }

            if let runStatus = pane.alanBinding?.runStatus,
               !Self.inactiveAlanRunStatuses.contains(runStatus.lowercased())
            {
                return .alanRunning
            }

            if pane.context?.processState == "foreground_command" {
                return .foregroundCommand
            }
        }

        return .inactive
    }

    @discardableResult
    private func recordTerminalActiveTask(
        _ activeTaskState: ShellTabActiveTaskState?,
        processExited: Bool,
        for paneID: String
    ) -> Bool {
        let nextState: ShellTabActiveTaskState?
        if processExited {
            nextState = .inactive
        } else {
            nextState = activeTaskState
        }

        guard let nextState else { return false }
        guard terminalActiveTasksByPaneID[paneID] != nextState else { return false }
        terminalActiveTasksByPaneID[paneID] = nextState
        return true
    }

    private func strongestTerminalActiveTask(in panes: [ShellPane]) -> ShellTabActiveTaskState? {
        panes
            .compactMap { terminalActiveTasksByPaneID[$0.paneID] }
            .max { activeTaskRank($0) < activeTaskRank($1) }
    }

    private func activeTaskRank(_ state: ShellTabActiveTaskState) -> Int {
        switch state {
        case .inactive:
            return 0
        case .unknown:
            return 1
        case .foregroundCommand:
            return 2
        case .alanRunning:
            return 3
        case .alanSession:
            return 4
        case .alanPendingYield:
            return 5
        }
    }

    private static let inactiveAlanRunStatuses: Set<String> = [
        "completed",
        "failed",
        "cancelled",
        "canceled",
        "exited",
        "idle",
    ]

    private func recordWorkspaceManifestRecovery(_ recovery: ShellWorkspaceManifestRecovery) {
        switch recovery {
        case .loadedExisting:
            return
        case .createdDefault:
            recordControlPlaneDiagnostic("workspace manifest created default")
        case .quarantinedCorruptFile(let url):
            recordControlPlaneDiagnostic("workspace manifest corrupt file quarantined: \(url.path)")
        }
    }

    func pane(paneID: String) -> ShellPane? {
        shellState.panes.first { $0.paneID == paneID }
    }

    private func nextID(prefix: String, existing: [String]) -> String {
        let nextOrdinal = existing
            .compactMap { identifier -> Int? in
                let components = identifier.split(separator: "_")
                guard let last = components.last else { return nil }
                return Int(last)
            }
            .max()
            .map { $0 + 1 }
            ?? (existing.isEmpty ? 1 : existing.count + 1)

        return "\(prefix)_\(nextOrdinal)"
    }

    func closeTab(tabID: String) -> ShellTabCloseResult {
        do {
            let result = try shellState.closingTab(tabID)
            applyMutationResult(result)
            return .closed
        } catch ShellStateMutationError.lastTab {
            return .lastTab
        } catch ShellStateMutationError.tabNotFound {
            return .tabNotFound
        } catch {
            return .tabNotFound
        }
    }

    func closePane(paneID: String) -> ShellPaneCloseResult {
        do {
            let result = try shellState.closingPane(paneID)
            applyMutationResult(result)
            return .closed
        } catch ShellStateMutationError.lastTab {
            return .lastTab
        } catch ShellStateMutationError.paneNotFound {
            return .paneNotFound
        } catch {
            return .paneNotFound
        }
    }

    private func focusedPaneWorkingDirectory() -> String? {
        guard let pane = focusedPane ?? selectedPane else { return nil }
        let runtimeCwd = runtime(for: pane.paneID).paneMetadata.workingDirectory
        return nonEmptyWorkingDirectory(runtimeCwd)
            ?? nonEmptyWorkingDirectory(pane.cwd)
    }

    private func nonEmptyWorkingDirectory(_ path: String?) -> String? {
        guard let path else { return nil }
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    @discardableResult
    private func closePaneAfterChildExitIfNeeded(
        paneID: String,
        processExited: Bool
    ) -> Bool {
        guard processExited else { return false }
        guard pane(paneID: paneID) != nil else { return false }
        return closePane(paneID: paneID) == .closed
    }

    func movePane(
        paneID: String,
        toTab targetTabID: String,
        direction: ShellSplitDirection
    ) -> Bool {
        do {
            let result = try shellState.movingPane(
                paneID,
                toTab: targetTabID,
                direction: direction
            )
            applyMutationResult(result)
            return true
        } catch {
            return false
        }
    }

    func liftPaneToTab(paneID: String, title: String? = nil) -> ShellPaneLiftResult {
        do {
            let result = try shellState.movingPaneToNewTab(paneID, title: title)
            applyMutationResult(result)
            return .lifted
        } catch ShellStateMutationError.lastPane {
            return .lastPane
        } catch ShellStateMutationError.paneNotFound {
            return .paneNotFound
        } catch {
            return .paneNotFound
        }
    }

    private var totalTabCount: Int {
        shellState.spaces.reduce(into: 0) { partialResult, space in
            partialResult += space.tabs.count
        }
    }

    private func strongestAttention(in panes: [ShellPane]) -> ShellAttentionState {
        let now = Date()
        return panes
            .map { shellEffectiveAttention(for: $0, now: now) }
            .max(by: { Self.attentionRank(for: $0) < Self.attentionRank(for: $1) })
            ?? .idle
    }

    private func publishControlPlaneState(pinSnapshotTabIDs: Set<String> = []) {
        syncWorkspaceManifestFromShellState(pinSnapshotTabIDs: pinSnapshotTabIDs)
        persistShellState()
        controlPlane.publish(state: shellState)
    }

    static func attentionRank(for attention: ShellAttentionState) -> Int {
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

extension ShellHostController {
    static let spikePreview = ShellHostController(shellState: .spikePreview)
}
#endif
