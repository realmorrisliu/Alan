import Foundation

@main
struct ShellActionRegistryTestRunner {
    static func main() throws {
        try ShellActionRegistryTests.run()
    }
}

private enum ShellActionRegistryTests {
    static func run() throws {
        try verifiesActionIDsAreUniqueAndStable()
        try verifiesShortcutConflictsAreRejected()
        try verifiesContextTabTargetDoesNotSelectTabFirst()
        try verifiesUnavailableActionDoesNotExecuteHandler()
        try verifiesCommandInputRemainsOutOfRegistry()
        print("Shell action registry tests passed.")
    }

    private static func verifiesActionIDsAreUniqueAndStable() throws {
        let registry = ShellActionRegistry.standard
        let ids = registry.actions.map(\.id.rawValue)

        expect(Set(ids).count == ids.count, "shell action ids must be unique")
        expect(
            registry.action(for: .tabPin)?.id.rawValue == "shell.tab.pin",
            "pin-tab action id must stay stable"
        )
        expect(
            registry.action(for: .paneSplitRight)?.title == "Split Right",
            "registered actions must expose user-facing labels"
        )
    }

    private static func verifiesShortcutConflictsAreRejected() throws {
        let duplicateShortcut = ShellActionShortcut(
            key: "t",
            modifiers: [.command],
            context: .shell
        )
        let first = ShellActionDescriptor(
            id: .newTerminalTab,
            title: "New Terminal Tab",
            targetKind: .currentSelection,
            defaultShortcut: duplicateShortcut,
            effect: .workspaceCommand(.newTerminalTab)
        )
        let second = ShellActionDescriptor(
            id: .newAlanTab,
            title: "New alan tab",
            targetKind: .currentSelection,
            defaultShortcut: duplicateShortcut,
            effect: .workspaceCommand(.newAlanTab)
        )

        do {
            _ = try ShellActionRegistry(actions: [first, second])
            expect(false, "duplicate shortcuts in the same context must be rejected")
        } catch ShellActionRegistryError.duplicateShortcut(let shortcut, let actionIDs) {
            expect(shortcut == duplicateShortcut, "duplicate shortcut error must include the shortcut")
            expect(
                actionIDs == [.newTerminalTab, .newAlanTab],
                "duplicate shortcut error must name both conflicting actions"
            )
        }
    }

    private static func verifiesContextTabTargetDoesNotSelectTabFirst() throws {
        var state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        state = try state.openingTab(
            launchTarget: .shell,
            in: "space_main",
            title: "Second",
            workingDirectory: "/tmp"
        ).state
        let selectedTabBefore = state.focusedTabID
        guard let contextTab = state.spaces.first?.tabs.first(where: { $0.tabID != selectedTabBefore }) else {
            throw TestFailure("expected a second tab")
        }

        let resolved = ShellActionRegistry.standard.resolve(
            .tabClose,
            target: .contextTab(contextTab.tabID),
            state: state
        )
        var handledEffects: [ShellActionEffect] = []
        let execution = ShellActionRegistry.standard.execute(
            .tabClose,
            target: .contextTab(contextTab.tabID),
            state: state
        ) { effect in
            handledEffects.append(effect)
            return true
        }

        expect(resolved.resolvedTarget == .tab(contextTab.tabID), "context menu must preserve clicked tab")
        expect(state.focusedTabID == selectedTabBefore, "resolving context target must not select the tab first")
        expect(execution == .executed, "context tab close must execute when the tab exists")
        expect(
            handledEffects == [.closeTab(contextTab.tabID)],
            "context tab close must route the clicked tab id to the handler"
        )
    }

    private static func verifiesUnavailableActionDoesNotExecuteHandler() throws {
        let state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        var handledEffects: [ShellActionEffect] = []

        let result = ShellActionRegistry.standard.execute(
            .tabMoveToSpace,
            target: .currentSelection,
            state: state
        ) { effect in
            handledEffects.append(effect)
            return true
        }

        expect(
            result == .unavailable(reason: "Move Tab to Space is not available yet"),
            "placeholder actions must be disabled"
        )
        expect(handledEffects.isEmpty, "unavailable actions must not execute handlers")
    }

    private static func verifiesCommandInputRemainsOutOfRegistry() throws {
        let actionIDs = Set(ShellActionRegistry.standard.actions.map(\.id))

        expect(!actionIDs.contains(.commandInputOpen), "Command-P command input must stay out of registry")
        expect(
            ShellActionRegistry.standard.action(for: .tabMoveToSpace)?.availability(
                state: ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp"),
                target: .currentSelection
            ) == .unavailable(reason: "Move Tab to Space is not available yet"),
            "future tab organization actions must be registered as disabled placeholders"
        )
    }
}

private func expect(_ condition: @autoclosure () -> Bool, _ message: String) {
    if !condition() {
        fatalError(message)
    }
}

private struct TestFailure: Error, CustomStringConvertible {
    let description: String

    init(_ description: String) {
        self.description = description
    }
}
