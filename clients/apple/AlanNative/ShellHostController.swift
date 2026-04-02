import Foundation

#if os(macOS)
import AppKit
import SwiftUI

struct ShellAttentionItem: Identifiable, Equatable {
    let paneID: String
    let spaceID: String
    let surfaceID: String
    let title: String
    let summary: String
    let attention: ShellAttentionState

    var id: String { paneID }
}

private enum ShellSurfaceCloseResult {
    case closed
    case surfaceNotFound
    case lastSurface
}

private enum ShellPaneCloseResult {
    case closed
    case paneNotFound
    case lastSurface
}

private enum ShellPaneLiftResult {
    case lifted
    case paneNotFound
    case lastPane
}

@MainActor
final class ShellHostController: ObservableObject {
    private static let iso8601Formatter = ISO8601DateFormatter()
    private static let persistenceFileName = "shell-state-v0.1.json"

    private let fileManager: FileManager
    private let persistenceURL: URL
    private lazy var controlPlane = AlanShellControlPlane(windowID: shellState.windowID) { [weak self] command in
        self?.handleControlPlaneCommand(command)
            ?? AlanShellControlResponse(
                requestID: command.requestID,
                contractVersion: "0.1",
                applied: false,
                state: nil,
                spaces: nil,
                surfaces: nil,
                panes: nil,
                pane: nil,
                items: nil,
                candidates: nil,
                events: nil,
                focusedPaneID: nil,
                spaceID: command.spaceID,
                surfaceID: command.surfaceID,
                paneID: command.paneID,
                acceptedBytes: nil,
                latestEventID: nil,
                errorCode: "host_unavailable",
                errorMessage: "Alan Shell host is unavailable."
            )
    } stateAdoptionHandler: { [weak self] state in
        self?.adoptStateFromControlPlane(state)
    } bindingProjectionHandler: { [weak self] paneID, binding in
        self?.applyAlanBinding(binding, for: paneID)
    }

    @Published private(set) var shellState: ShellStateSnapshot
    @Published var selectedSpaceID: String?
    @Published var selectedSurfaceID: String?
    @Published private(set) var lastCopiedAt: Date?
    @Published private(set) var terminalRuntime: TerminalHostRuntimeSnapshot = .placeholder

    private var runtimesByPaneID: [String: TerminalHostRuntimeSnapshot] = [:]

    init(
        shellState: ShellStateSnapshot,
        fileManager: FileManager = .default,
        persistenceURL: URL? = nil
    ) {
        self.fileManager = fileManager
        self.persistenceURL = persistenceURL ?? Self.defaultPersistenceURL(fileManager: fileManager)
        self.shellState = shellState
        self.selectedSpaceID = shellState.focusedSpaceID ?? shellState.spaces.first?.spaceID
        self.selectedSurfaceID = shellState.focusedSurfaceID ?? shellState.spaces.first?.surfaces.first?.surfaceID

        if shellState.panes.isEmpty {
            publishControlPlaneState()
        } else {
            shellState.panes.map(\.paneID).forEach(primeBootContext)
        }
        synchronizeSelection()
    }

    static func live(fileManager: FileManager = .default) -> ShellHostController {
        let persistenceURL = defaultPersistenceURL(fileManager: fileManager)
        let shellState =
            restoreShellState(fileManager: fileManager, persistenceURL: persistenceURL)
            ?? .bootstrapDefault()
        return ShellHostController(
            shellState: shellState,
            fileManager: fileManager,
            persistenceURL: persistenceURL
        )
    }

    var spaces: [ShellSpace] {
        shellState.spaces
    }

    var selectedSpace: ShellSpace? {
        shellState.spaces.first { $0.spaceID == selectedSpaceID } ?? shellState.spaces.first
    }

    var selectedSurface: ShellSurface? {
        guard let selectedSurfaceID else {
            return selectedSpace?.surfaces.first
        }
        return selectedSpace?.surfaces.first { $0.surfaceID == selectedSurfaceID } ?? selectedSpace?.surfaces.first
    }

    var selectedSurfacePaneTree: ShellPaneTreeNode? {
        selectedSurface?.paneTree
    }

    var panesForSelectedSurface: [ShellPane] {
        guard let surfaceID = selectedSurface?.surfaceID else { return [] }
        return shellState.panes.filter { $0.surfaceID == surfaceID }
    }

    var selectedPane: ShellPane? {
        if let focusedPane, focusedPane.surfaceID == selectedSurface?.surfaceID {
            return focusedPane
        }
        return panesForSelectedSurface.first
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
        shellState.panes
            .compactMap { pane in
                guard pane.attention != .idle else { return nil }
                return ShellAttentionItem(
                    paneID: pane.paneID,
                    spaceID: pane.spaceID,
                    surfaceID: pane.surfaceID,
                    title: pane.viewport?.title ?? pane.process?.program ?? "Pane",
                    summary: pane.viewport?.summary ?? "Activity detected",
                    attention: pane.attention
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

    var moveDestinationSurfaces: [ShellSurface] {
        guard let selectedPane else { return [] }
        return shellState.spaces
            .flatMap(\.surfaces)
            .filter { $0.surfaceID != selectedPane.surfaceID }
            .sorted {
                if $0.surfaceID == $1.surfaceID {
                    return ($0.title ?? "") < ($1.title ?? "")
                }
                return $0.surfaceID < $1.surfaceID
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
        guard let paneID else { return .placeholder }
        return runtimesByPaneID[paneID] ?? .placeholder
    }

    func select(spaceID: String) {
        guard let space = shellState.spaces.first(where: { $0.spaceID == spaceID }) else { return }
        selectedSpaceID = spaceID
        selectedSurfaceID = space.surfaces.first?.surfaceID
        terminalRuntime = runtime(for: selectedPane?.paneID)
    }

    func select(surfaceID: String) {
        guard selectedSpace?.surfaces.contains(where: { $0.surfaceID == surfaceID }) == true else { return }
        selectedSurfaceID = surfaceID
        terminalRuntime = runtime(for: selectedPane?.paneID)
    }

    func focusAttentionItem(_ item: ShellAttentionItem) {
        select(spaceID: item.spaceID)
        select(surfaceID: item.surfaceID)
        focus(paneID: item.paneID)
    }

    func focus(paneID: String) {
        guard let result = try? shellState.focusingPane(paneID) else { return }
        applyMutationResult(result)
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
            workingDirectory: workingDirectory
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
    func openSurface(
        launchTarget: ShellLaunchTarget = .shell,
        in spaceID: String? = nil,
        title: String? = nil,
        workingDirectory: String? = nil
    ) -> String? {
        let result: ShellStateMutationResult
        do {
            result = try shellState.openingSurface(
                launchTarget: launchTarget,
                in: spaceID,
                title: title,
                workingDirectory: workingDirectory
            )
        } catch {
            return nil
        }
        applyMutationResult(result)
        return result.surfaceID
    }

    @discardableResult
    func openTerminalSurface(
        in spaceID: String? = nil,
        title: String? = nil,
        workingDirectory: String? = nil
    ) -> String? {
        openSurface(
            launchTarget: .shell,
            in: spaceID,
            title: title,
            workingDirectory: workingDirectory
        )
    }

    @discardableResult
    func openAlanSurface(
        in spaceID: String? = nil,
        title: String? = nil,
        workingDirectory: String? = nil
    ) -> String? {
        openSurface(
            launchTarget: .alan,
            in: spaceID,
            title: title,
            workingDirectory: workingDirectory
        )
    }

    @discardableResult
    func splitFocusedPane(direction: ShellSplitDirection) -> String? {
        guard let focusedPaneID = shellState.focusedPaneID else { return nil }
        return splitPane(paneID: focusedPaneID, direction: direction)
    }

    @discardableResult
    func splitPane(paneID: String, direction: ShellSplitDirection) -> String? {
        let result: ShellStateMutationResult
        do {
            result = try shellState.splittingPane(
                paneID,
                direction: direction
            )
        } catch {
            return nil
        }
        applyMutationResult(result)
        return result.paneID
    }

    @discardableResult
    func closeSelectedSurface() -> Bool {
        guard let selectedSurfaceID else { return false }
        return closeSurface(surfaceID: selectedSurfaceID) == .closed
    }

    @discardableResult
    func closeSelectedPane() -> Bool {
        guard let paneID = selectedPane?.paneID else { return false }
        return closePane(paneID: paneID) == .closed
    }

    @discardableResult
    func liftSelectedPaneToSurface(title: String? = nil) -> Bool {
        guard let paneID = selectedPane?.paneID else { return false }
        return liftPaneToSurface(paneID: paneID, title: title) == .lifted
    }

    @discardableResult
    func moveSelectedPane(
        toSurface surfaceID: String,
        direction: ShellSplitDirection = .vertical
    ) -> Bool {
        guard let paneID = selectedPane?.paneID else { return false }
        return movePane(paneID: paneID, toSurface: surfaceID, direction: direction)
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
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(snapshotJSON, forType: .string)
        lastCopiedAt = .now
    }

    func updateTerminalRuntime(_ runtime: TerminalHostRuntimeSnapshot) {
        if let paneID = runtime.paneID {
            runtimesByPaneID[paneID] = runtime
        }

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
            let projectedContext = projectedContext(
                for: pane,
                bootProfile: bootProfile,
                workingDirectory: runtime.surfaceMetadata.workingDirectory ?? pane.cwd,
                processExited: runtime.surfaceMetadata.processExited,
                lastCommandExitCode: runtime.surfaceMetadata.lastCommandExitCode,
                lastMetadataAt: runtime.surfaceMetadata.lastUpdatedAt,
                existing: pane.context,
                runtime: runtime
            )

            updatePaneState(paneID: paneID) { current in
                let projectedBinding = projectedAlanBinding(
                    for: current,
                    binding: current.alanBinding,
                    processExited: runtime.surfaceMetadata.processExited
                )
                return ShellPane(
                    paneID: current.paneID,
                    surfaceID: current.surfaceID,
                    spaceID: current.spaceID,
                    launchTarget: current.launchTarget,
                    cwd: current.cwd ?? bootProfile.workingDirectory,
                    process: current.process,
                    attention: current.attention,
                    context: projectedContext,
                    viewport: current.viewport,
                    alanBinding: projectedBinding
                )
            }
        }
    }

    func updateTerminalMetadata(_ metadata: TerminalSurfaceMetadataSnapshot, for paneID: String) {
        guard let pane = pane(paneID: paneID) else { return }
        let bootProfile = AlanShellBootProfile.forPane(pane, shellState: shellState)
        let projectedContext = projectedContext(
            for: pane,
            bootProfile: bootProfile,
            workingDirectory: metadata.workingDirectory ?? pane.cwd,
            processExited: metadata.processExited,
            lastCommandExitCode: metadata.lastCommandExitCode,
            lastMetadataAt: metadata.lastUpdatedAt,
            existing: pane.context,
            runtime: runtime(for: pane.paneID)
        )

        updatePaneState(
            paneID: pane.paneID,
            surfaceTitleOverride: metadata.title
        ) { current in
            let projectedBinding = projectedAlanBinding(
                for: current,
                binding: current.alanBinding,
                processExited: metadata.processExited
            )
            let viewport = ShellViewportSnapshot(
                title: metadata.title ?? current.viewport?.title,
                summary: metadata.summary ?? current.viewport?.summary,
                visibleExcerpt: current.viewport?.visibleExcerpt,
                lastActivityAt: metadata.lastUpdatedAt.map(Self.iso8601Formatter.string)
                    ?? current.viewport?.lastActivityAt
            )

            return ShellPane(
                paneID: current.paneID,
                surfaceID: current.surfaceID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: metadata.workingDirectory ?? current.cwd ?? bootProfile.workingDirectory,
                process: current.process,
                attention: projectedAttention(
                    metadataAttention: metadata.attention,
                    processExited: metadata.processExited,
                    binding: projectedBinding
                ),
                context: projectedContext,
                viewport: viewport,
                alanBinding: projectedBinding
            )
        }
    }

    private func applyAlanBinding(_ binding: ShellAlanBinding?, for paneID: String) {
        guard let pane = pane(paneID: paneID) else { return }
        let bootProfile = AlanShellBootProfile.forPane(pane, shellState: shellState)
        let projectedBinding = projectedAlanBinding(
            for: pane,
            binding: binding,
            processExited: runtime(for: pane.paneID).surfaceMetadata.processExited
        )
        let projectedContext = projectedContext(
            for: pane,
            bootProfile: bootProfile,
            workingDirectory: pane.cwd,
            processExited: nil,
            lastCommandExitCode: pane.context?.lastCommandExitCode,
            lastMetadataAt: nil,
            existing: pane.context,
            runtime: runtime(for: pane.paneID)
        )

        updatePaneState(paneID: paneID) { current in
            let bindingSummary: String?
            if let projectedBinding {
                bindingSummary = projectedBinding.pendingYield
                    ? "Alan is waiting for user input"
                    : "Alan run status: \(projectedBinding.runStatus)"
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
                surfaceID: current.surfaceID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: current.cwd ?? bootProfile.workingDirectory,
                process: current.process,
                attention: projectedBinding?.pendingYield == true ? .awaitingUser : current.attention,
                context: projectedContext,
                viewport: viewport,
                alanBinding: projectedBinding
            )
        }
    }

    private func primeBootContext(for paneID: String) {
        guard let pane = pane(paneID: paneID) else { return }
        let bootProfile = AlanShellBootProfile.forPane(pane, shellState: shellState)
        let projectedContext = projectedContext(
            for: pane,
            bootProfile: bootProfile,
            workingDirectory: pane.cwd ?? bootProfile.workingDirectory,
            processExited: nil,
            lastCommandExitCode: pane.context?.lastCommandExitCode,
            lastMetadataAt: nil,
            existing: pane.context,
            runtime: runtime(for: pane.paneID)
        )

        updatePaneState(paneID: paneID) { current in
            let projectedBinding = projectedAlanBinding(
                for: current,
                binding: current.alanBinding,
                processExited: false
            )
            return ShellPane(
                paneID: current.paneID,
                surfaceID: current.surfaceID,
                spaceID: current.spaceID,
                launchTarget: current.launchTarget,
                cwd: current.cwd ?? bootProfile.workingDirectory,
                process: current.process,
                attention: current.attention,
                context: projectedContext,
                viewport: current.viewport,
                alanBinding: projectedBinding
            )
        }
    }

    private func updatePaneState(
        paneID: String,
        surfaceTitleOverride: String? = nil,
        transform: (ShellPane) -> ShellPane
    ) {
        guard let existingPane = shellState.panes.first(where: { $0.paneID == paneID }) else { return }
        let transformedPane = transform(existingPane)
        let currentSurfaceTitle = shellState.surface(surfaceID: existingPane.surfaceID)?.title
        let requestedSurfaceTitle = surfaceTitleOverride ?? currentSurfaceTitle

        guard transformedPane != existingPane || requestedSurfaceTitle != currentSurfaceTitle else {
            return
        }

        let updatedPanes = shellState.panes.map { pane in
            pane.paneID == paneID ? transformedPane : pane
        }
        let updatedSpaces = rebuildSpaces(
            using: updatedPanes,
            surfaceTitleOverride: surfaceTitleOverride,
            paneID: paneID
        )

        shellState = ShellStateSnapshot(
            contractVersion: shellState.contractVersion,
            windowID: shellState.windowID,
            focusedSpaceID: shellState.focusedSpaceID,
            focusedSurfaceID: shellState.focusedSurfaceID,
            focusedPaneID: shellState.focusedPaneID,
            spaces: updatedSpaces,
            panes: updatedPanes
        )
        synchronizeSelection()
        publishControlPlaneState()
    }

    private func rebuildSpaces(
        using panes: [ShellPane],
        surfaceTitleOverride: String?,
        paneID: String
    ) -> [ShellSpace] {
        let surfaceID = shellState.panes.first(where: { $0.paneID == paneID })?.surfaceID

        return shellState.spaces.map { space in
            let surfaces = space.surfaces.map { surface in
                let nextTitle: String?
                if surface.surfaceID == surfaceID, let surfaceTitleOverride {
                    nextTitle = surfaceTitleOverride
                } else {
                    nextTitle = surface.title
                }

                return ShellSurface(
                    surfaceID: surface.surfaceID,
                    kind: surface.kind,
                    title: nextTitle,
                    paneTree: surface.paneTree
                )
            }

            return ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: strongestAttention(in: panes.filter { $0.spaceID == space.spaceID }),
                surfaces: surfaces
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
            focusedSurfaceID: focusedPane?.surfaceID ?? spaces.first?.surfaces.first?.surfaceID,
            focusedPaneID: resolvedFocusedPaneID,
            spaces: spaces,
            panes: panes
        )
        synchronizeSelection()
        publishControlPlaneState()
    }

    private func applyMutationResult(_ result: ShellStateMutationResult) {
        adoptStateFromControlPlane(result.state)
    }

    private func adoptStateFromControlPlane(_ state: ShellStateSnapshot) {
        let paneIDs = Set(state.panes.map(\.paneID))
        runtimesByPaneID = runtimesByPaneID.filter { paneIDs.contains($0.key) }

        let hydratedPanes = state.panes.map { pane in
            guard paneNeedsBootContextProjection(pane) else { return pane }
            let bootProfile = AlanShellBootProfile.forPane(pane, shellState: state)
            let projectedContext = projectedContext(
                for: pane,
                bootProfile: bootProfile,
                workingDirectory: pane.cwd ?? bootProfile.workingDirectory,
                processExited: nil,
                lastCommandExitCode: pane.context?.lastCommandExitCode,
                lastMetadataAt: nil,
                existing: pane.context,
                runtime: self.runtime(for: pane.paneID)
            )
            return ShellPane(
                paneID: pane.paneID,
                surfaceID: pane.surfaceID,
                spaceID: pane.spaceID,
                launchTarget: pane.launchTarget,
                cwd: pane.cwd ?? bootProfile.workingDirectory,
                process: pane.process,
                attention: pane.attention,
                context: projectedContext,
                viewport: pane.viewport,
                alanBinding: pane.alanBinding
            )
        }

        let hydratedSpaces = state.spaces.map { space in
            ShellSpace(
                spaceID: space.spaceID,
                title: space.title,
                attention: strongestAttention(in: hydratedPanes.filter { $0.spaceID == space.spaceID }),
                surfaces: space.surfaces
            )
        }

        shellState = ShellStateSnapshot(
            contractVersion: state.contractVersion,
            windowID: state.windowID,
            focusedSpaceID: state.focusedSpaceID,
            focusedSurfaceID: state.focusedSurfaceID,
            focusedPaneID: state.focusedPaneID,
            spaces: hydratedSpaces,
            panes: hydratedPanes
        )
        synchronizeSelection()
        publishControlPlaneState()
    }

    private func synchronizeSelection() {
        if let focusedPane = focusedPane {
            selectedSpaceID = focusedPane.spaceID
            selectedSurfaceID = focusedPane.surfaceID
            terminalRuntime = runtime(for: focusedPane.paneID)
            return
        }

        selectedSpaceID = selectedSpaceID ?? shellState.spaces.first?.spaceID
        selectedSurfaceID = selectedSurfaceID ?? selectedSpace?.surfaces.first?.surfaceID
        terminalRuntime = runtime(for: selectedPane?.paneID)
    }

    private func persistShellState() {
        let parentURL = persistenceURL.deletingLastPathComponent()
        try? fileManager.createDirectory(at: parentURL, withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(shellState) else { return }
        try? data.write(to: persistenceURL, options: .atomic)
    }

    private func pane(paneID: String) -> ShellPane? {
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

    private func closeSurface(surfaceID: String) -> ShellSurfaceCloseResult {
        do {
            let result = try shellState.closingSurface(surfaceID)
            applyMutationResult(result)
            return .closed
        } catch ShellStateMutationError.lastSurface {
            return .lastSurface
        } catch ShellStateMutationError.surfaceNotFound {
            return .surfaceNotFound
        } catch {
            return .surfaceNotFound
        }
    }

    private func closePane(paneID: String) -> ShellPaneCloseResult {
        do {
            let result = try shellState.closingPane(paneID)
            applyMutationResult(result)
            return .closed
        } catch ShellStateMutationError.lastSurface {
            return .lastSurface
        } catch ShellStateMutationError.paneNotFound {
            return .paneNotFound
        } catch {
            return .paneNotFound
        }
    }

    private func movePane(
        paneID: String,
        toSurface targetSurfaceID: String,
        direction: ShellSplitDirection
    ) -> Bool {
        do {
            let result = try shellState.movingPane(
                paneID,
                toSurface: targetSurfaceID,
                direction: direction
            )
            applyMutationResult(result)
            return true
        } catch {
            return false
        }
    }

    private func liftPaneToSurface(paneID: String, title: String? = nil) -> ShellPaneLiftResult {
        do {
            let result = try shellState.movingPaneToNewSurface(paneID, title: title)
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

    private var totalSurfaceCount: Int {
        shellState.spaces.reduce(into: 0) { partialResult, space in
            partialResult += space.surfaces.count
        }
    }

    private func attentionInboxRows() -> [AlanShellAttentionInboxItem] {
        attentionItems.map { item in
            AlanShellAttentionInboxItem(
                itemID: "attn_\(item.paneID)",
                spaceID: item.spaceID,
                surfaceID: item.surfaceID,
                paneID: item.paneID,
                attention: item.attention,
                summary: item.summary
            )
        }
    }

    private func routingCandidates(preferredPaneID: String?) -> [AlanShellRoutingCandidate] {
        let preferredPane = preferredPaneID.flatMap { pane(paneID: $0) }
        let focusedPane = self.focusedPane

        return shellState.panes.map { candidate in
            var score = 0.0
            var reasons: [String] = []

            if candidate.paneID == preferredPaneID {
                score += 0.4
                reasons.append("requested")
            }
            if candidate.paneID == shellState.focusedPaneID {
                score += 0.3
                reasons.append("focused")
            }
            if candidate.attention == .awaitingUser {
                score += 0.25
                reasons.append("attention:awaiting_user")
            } else if candidate.attention == .notable {
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
            if let preferredPane, candidate.surfaceID == preferredPane.surfaceID {
                score += 0.1
                reasons.append("same_surface")
            } else if let focusedPane, candidate.surfaceID == focusedPane.surfaceID {
                score += 0.08
                reasons.append("same_surface")
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

    private func paneList(surfaceID: String?) -> [ShellPane] {
        guard let surfaceID else {
            return shellState.panes
        }
        return shellState.panes.filter { $0.surfaceID == surfaceID }
    }

    private func surfaceList(spaceID: String?) -> [ShellSurface] {
        if let spaceID {
            return shellState.spaces.first(where: { $0.spaceID == spaceID })?.surfaces ?? []
        }
        return shellState.spaces.flatMap(\.surfaces)
    }

    private func response(
        requestID: String,
        applied: Bool,
        state: ShellStateSnapshot? = nil,
        spaces: [ShellSpace]? = nil,
        surfaces: [ShellSurface]? = nil,
        panes: [ShellPane]? = nil,
        pane: ShellPane? = nil,
        items: [AlanShellAttentionInboxItem]? = nil,
        candidates: [AlanShellRoutingCandidate]? = nil,
        events: [AlanShellEventEnvelope]? = nil,
        spaceID: String? = nil,
        surfaceID: String? = nil,
        paneID: String? = nil,
        acceptedBytes: Int? = nil,
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
            surfaces: surfaces,
            panes: panes,
            pane: pane,
            items: items,
            candidates: candidates,
            events: events,
            focusedPaneID: shellState.focusedPaneID,
            spaceID: spaceID,
            surfaceID: surfaceID,
            paneID: paneID,
            acceptedBytes: acceptedBytes,
            latestEventID: latestEventID,
            errorCode: errorCode,
            errorMessage: errorMessage
        )
    }

    private func paneNeedsBootContextProjection(_ pane: ShellPane) -> Bool {
        guard let context = pane.context else { return true }
        return context.controlPath == nil
            || context.alanBindingFile == nil
            || context.launchStrategy == nil
    }

    private func projectedAttention(
        metadataAttention: ShellAttentionState,
        processExited: Bool,
        binding: ShellAlanBinding?
    ) -> ShellAttentionState {
        if binding?.pendingYield == true || processExited {
            return .awaitingUser
        }

        return metadataAttention
    }

    private func projectedContext(
        for pane: ShellPane,
        bootProfile: AlanShellBootProfile,
        workingDirectory: String?,
        processExited: Bool?,
        lastCommandExitCode: Int?,
        lastMetadataAt: Date?,
        existing: ShellContextSnapshot?,
        runtime: TerminalHostRuntimeSnapshot? = nil
    ) -> ShellContextSnapshot {
        let resolvedWorkingDirectory = workingDirectory ?? pane.cwd ?? bootProfile.workingDirectory
        let repositoryContext = repositoryContext(for: resolvedWorkingDirectory)

        return ShellContextSnapshot(
            workingDirectoryName: workingDirectoryName(for: resolvedWorkingDirectory)
                ?? existing?.workingDirectoryName,
            repositoryRoot: repositoryContext.repositoryRoot ?? existing?.repositoryRoot,
            gitBranch: repositoryContext.gitBranch ?? existing?.gitBranch,
            controlPath: bootProfile.environment["ALAN_SHELL_CONTROL_DIR"] ?? existing?.controlPath,
            socketPath: bootProfile.environment["ALAN_SHELL_SOCKET"] ?? existing?.socketPath,
            alanBindingFile: bootProfile.environment["ALAN_SHELL_BINDING_FILE"]
                ?? existing?.alanBindingFile,
            launchCommand: bootProfile.launchCommandString,
            launchStrategy: bootProfile.command.strategy.rawValue,
            shellIntegrationSource: "ghostty_shell_integration",
            processState: processExited.map { $0 ? "exited" : "running" } ?? existing?.processState,
            rendererPhase: runtime?.renderer.phase.rawValue ?? existing?.rendererPhase,
            displayName: runtime?.displayName ?? existing?.displayName,
            displayID: runtime?.displayID ?? existing?.displayID,
            windowTitle: runtime?.attachedWindowTitle ?? existing?.windowTitle,
            lastMetadataAt: lastMetadataAt.map(Self.iso8601Formatter.string) ?? existing?.lastMetadataAt,
            lastCommandExitCode: lastCommandExitCode ?? existing?.lastCommandExitCode
        )
    }

    private func projectedAlanBinding(
        for pane: ShellPane,
        binding: ShellAlanBinding?,
        processExited: Bool
    ) -> ShellAlanBinding? {
        if let binding {
            return binding
        }

        if let existing = pane.alanBinding {
            return existing
        }

        guard pane.resolvedLaunchTarget == .alan, !processExited else {
            return nil
        }

        return ShellAlanBinding(
            sessionID: "pending:\(pane.paneID)",
            runStatus: "booting",
            pendingYield: false,
            source: "alan_shell_boot_projection",
            lastProjectedAt: Self.iso8601Formatter.string(from: .now)
        )
    }

    private func workingDirectoryName(for path: String?) -> String? {
        guard let path, !path.isEmpty else { return nil }
        let lastComponent = URL(fileURLWithPath: path).lastPathComponent
        return lastComponent.isEmpty ? path : lastComponent
    }

    private func repositoryContext(for workingDirectory: String?) -> (repositoryRoot: String?, gitBranch: String?) {
        guard let workingDirectory, !workingDirectory.isEmpty else {
            return (nil, nil)
        }

        var currentURL = URL(fileURLWithPath: workingDirectory, isDirectory: true).standardizedFileURL
        var isDirectory: ObjCBool = false

        if !fileManager.fileExists(atPath: currentURL.path, isDirectory: &isDirectory) {
            return (nil, nil)
        }

        if !isDirectory.boolValue {
            currentURL.deleteLastPathComponent()
        }

        while true {
            let gitEntryURL = currentURL.appendingPathComponent(".git")
            if fileManager.fileExists(atPath: gitEntryURL.path) {
                let gitDirectoryURL = resolveGitDirectory(for: gitEntryURL, repositoryRoot: currentURL)
                let gitBranch = gitDirectoryURL.flatMap(readGitBranch(from:))
                return (currentURL.path, gitBranch)
            }

            let parentURL = currentURL.deletingLastPathComponent()
            if parentURL.path == currentURL.path {
                return (nil, nil)
            }
            currentURL = parentURL
        }
    }

    private func resolveGitDirectory(for gitEntryURL: URL, repositoryRoot: URL) -> URL? {
        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: gitEntryURL.path, isDirectory: &isDirectory) else {
            return nil
        }

        if isDirectory.boolValue {
            return gitEntryURL
        }

        guard let content = try? String(contentsOf: gitEntryURL, encoding: .utf8) else {
            return nil
        }

        let prefix = "gitdir:"
        guard content.hasPrefix(prefix) else { return nil }
        let rawPath = content.dropFirst(prefix.count).trimmingCharacters(in: .whitespacesAndNewlines)
        guard !rawPath.isEmpty else { return nil }

        let pathURL = URL(fileURLWithPath: rawPath, relativeTo: repositoryRoot)
        return pathURL.standardizedFileURL
    }

    private func readGitBranch(from gitDirectoryURL: URL) -> String? {
        let headURL = gitDirectoryURL.appendingPathComponent("HEAD")
        guard let head = try? String(contentsOf: headURL, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines),
            !head.isEmpty
        else {
            return nil
        }

        let refPrefix = "ref: "
        if head.hasPrefix(refPrefix) {
            let reference = String(head.dropFirst(refPrefix.count))
            return reference.split(separator: "/").last.map(String.init)
        }

        return "detached:\(String(head.prefix(12)))"
    }

    private func strongestAttention(in panes: [ShellPane]) -> ShellAttentionState {
        panes
            .map(\.attention)
            .max(by: { Self.attentionRank(for: $0) < Self.attentionRank(for: $1) })
            ?? .idle
    }

    private func publishControlPlaneState() {
        persistShellState()
        controlPlane.publish(state: shellState)
    }

    private func handleControlPlaneCommand(_ command: AlanShellControlCommand) -> AlanShellControlResponse {
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
                ? "Failed to create a new Alan space."
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

        case .surfaceList:
            return response(
                requestID: command.requestID,
                applied: true,
                surfaces: surfaceList(spaceID: command.spaceID),
                spaceID: command.spaceID
            )

        case .surfaceOpen:
            guard let surfaceID = openTerminalSurface(
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
                surfaceID: surfaceID,
                paneID: shellState.focusedPaneID
            )

        case .surfaceClose:
            guard let surfaceID = command.surfaceID else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    errorCode: "surface_required",
                    errorMessage: "surface_id is required."
                )
            }

            switch closeSurface(surfaceID: surfaceID) {
            case .closed:
                return response(
                    requestID: command.requestID,
                    applied: true,
                    surfaceID: surfaceID,
                    paneID: shellState.focusedPaneID
                )
            case .surfaceNotFound:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    surfaceID: surfaceID,
                    errorCode: "surface_not_found",
                    errorMessage: "The requested surface does not exist."
                )
            case .lastSurface:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    surfaceID: surfaceID,
                    errorCode: "last_surface",
                    errorMessage: "Alan Shell must keep at least one surface open."
                )
            }

        case .paneList:
            return response(
                requestID: command.requestID,
                applied: true,
                panes: paneList(surfaceID: command.surfaceID),
                surfaceID: command.surfaceID
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
                surfaceID: pane.surfaceID,
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
                surfaceID: shellState.focusedSurfaceID,
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
                    surfaceID: shellState.focusedSurfaceID,
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
            case .lastSurface:
                return response(
                    requestID: command.requestID,
                    applied: false,
                    paneID: paneID,
                    errorCode: "last_surface",
                    errorMessage: "Alan Shell must keep at least one pane open."
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

            switch liftPaneToSurface(paneID: paneID, title: command.title) {
            case .lifted:
                return response(
                    requestID: command.requestID,
                    applied: true,
                    spaceID: shellState.focusedSpaceID,
                    surfaceID: shellState.focusedSurfaceID,
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
                  let targetSurfaceID = command.surfaceID
            else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    surfaceID: command.surfaceID,
                    paneID: command.paneID,
                    errorCode: "pane_move_target_required",
                    errorMessage: "pane_id and surface_id are required."
                )
            }

            let direction = command.direction ?? .vertical
            guard movePane(paneID: paneID, toSurface: targetSurfaceID, direction: direction) else {
                return response(
                    requestID: command.requestID,
                    applied: false,
                    surfaceID: targetSurfaceID,
                    paneID: paneID,
                    errorCode: "invalid_move_target",
                    errorMessage: "The requested pane could not be moved to the target surface."
                )
            }

            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: shellState.focusedSpaceID,
                surfaceID: shellState.focusedSurfaceID,
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
                surfaceID: shellState.focusedSurfaceID,
                paneID: paneID
            )

        case .paneSendText:
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

            let text = command.text ?? ""
            NotificationCenter.default.post(
                name: .alanShellSendText,
                object: nil,
                userInfo: [
                    AlanShellNotificationKey.paneID: paneID,
                    AlanShellNotificationKey.text: text,
                ]
            )

            return response(
                requestID: command.requestID,
                applied: true,
                spaceID: shellState.focusedSpaceID,
                surfaceID: shellState.focusedSurfaceID,
                paneID: paneID,
                acceptedBytes: text.lengthOfBytes(using: .utf8)
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
                spaceID: shellState.focusedSpaceID,
                surfaceID: shellState.focusedSurfaceID,
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

    private static func defaultPersistenceURL(fileManager: FileManager) -> URL {
        let appSupportURL =
            fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.temporaryDirectory
        return appSupportURL
            .appendingPathComponent("AlanNative", isDirectory: true)
            .appendingPathComponent(persistenceFileName)
    }

    private static func restoreShellState(
        fileManager: FileManager,
        persistenceURL: URL
    ) -> ShellStateSnapshot? {
        guard fileManager.fileExists(atPath: persistenceURL.path),
              let data = try? Data(contentsOf: persistenceURL),
              let state = try? JSONDecoder().decode(ShellStateSnapshot.self, from: data),
              !state.spaces.isEmpty,
              !state.panes.isEmpty
        else {
            return nil
        }
        return state.migratingLegacyAlanBootstrapIfNeeded()
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

extension ShellHostController {
    static let spikePreview = ShellHostController(shellState: .spikePreview)
}
#endif
