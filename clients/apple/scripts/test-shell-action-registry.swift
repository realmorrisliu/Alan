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
        try verifiesStandardShortcutDefaults()
        try verifiesKeyboardActionLookup()
        try verifiesShortcutConflictsAreRejected()
        try verifiesDynamicSpaceShortcutConflictsAreRejected()
        try verifiesContextTabTargetDoesNotSelectTabFirst()
        try verifiesUnavailableActionDoesNotExecuteHandler()
        try verifiesUnavailableShortcutActionDoesNotExecuteHandler()
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

    private static func verifiesStandardShortcutDefaults() throws {
        let registry = ShellActionRegistry.standard

        let expectedShortcuts: [(ShellActionID, ShellActionShortcut)] = [
            (.newTerminalTab, ShellActionShortcut(key: "t", modifiers: [.command], context: .shell)),
            (.newAlanTab, ShellActionShortcut(key: "t", modifiers: [.command, .option], context: .shell)),
            (.tabClose, ShellActionShortcut(key: "w", modifiers: [.command], context: .shell)),
            (.tabSelectPrevious, ShellActionShortcut(key: "[", modifiers: [.command, .shift], context: .shell)),
            (.tabSelectNext, ShellActionShortcut(key: "]", modifiers: [.command, .shift], context: .shell)),
            (
                .tabMoveLeft,
                ShellActionShortcut(
                    key: "leftArrow",
                    modifiers: [.command, .option, .shift],
                    context: .shell
                )
            ),
            (
                .tabMoveRight,
                ShellActionShortcut(
                    key: "rightArrow",
                    modifiers: [.command, .option, .shift],
                    context: .shell
                )
            ),
            (.paneSplitRight, ShellActionShortcut(key: "d", modifiers: [.command], context: .shell)),
            (.paneSplitDown, ShellActionShortcut(key: "d", modifiers: [.command, .shift], context: .shell)),
            (.paneSplitLeft, ShellActionShortcut(key: "d", modifiers: [.command, .option], context: .shell)),
            (
                .paneSplitUp,
                ShellActionShortcut(key: "d", modifiers: [.command, .option, .shift], context: .shell)
            ),
            (.paneEqualizeSplits, ShellActionShortcut(key: "=", modifiers: [.command, .option], context: .shell)),
            (
                .paneFocusRight,
                ShellActionShortcut(key: "rightArrow", modifiers: [.command, .control], context: .shell)
            ),
            (.paneClose, ShellActionShortcut(key: "w", modifiers: [.command, .shift], context: .shell)),
            (.findOpen, ShellActionShortcut(key: "f", modifiers: [.command], context: .shell)),
            (
                .spaceSelectPrevious,
                ShellActionShortcut(key: "leftArrow", modifiers: [.command, .option], context: .shell)
            ),
            (
                .spaceSelectNext,
                ShellActionShortcut(key: "rightArrow", modifiers: [.command, .option], context: .shell)
            ),
        ]

        for (actionID, shortcut) in expectedShortcuts {
            expect(
                registry.defaultShortcut(for: actionID) == shortcut,
                "\(actionID.rawValue) must keep its expected default shortcut"
            )
        }

        expect(
            registry.defaultShortcut(for: .spaceSelectByIndex, target: .spaceIndex(1))
                == ShellActionShortcut(key: "2", modifiers: [.command, .option], context: .shell),
            "space numeric shortcuts must be derived dynamically from the target index"
        )
        expect(
            registry.defaultShortcut(for: .tabPin) == nil,
            "pin tab must not receive a shortcut before tab organization owns that action"
        )
        expect(
            registry.defaultShortcut(for: .tabMoveToSpace) == nil,
            "move tab to space must stay action-only in this phase"
        )
    }

    private static func verifiesKeyboardActionLookup() throws {
        let registry = ShellActionRegistry.standard

        expect(
            registry.keyboardAction(
                for: ShellActionShortcut(key: "t", modifiers: [.command], context: .shell)
            ) == ShellKeyboardAction(id: .newTerminalTab, target: .currentSelection),
            "command-t must resolve to new-terminal-tab through the registry"
        )
        expect(
            registry.keyboardAction(
                for: ShellActionShortcut(key: "]", modifiers: [.command, .shift], context: .shell)
            ) == ShellKeyboardAction(id: .tabSelectNext, target: .currentSelection),
            "command-shift-] must resolve to next-tab through the registry"
        )
        expect(
            registry.keyboardAction(
                for: ShellActionShortcut(key: "2", modifiers: [.command, .option], context: .shell)
            ) == ShellKeyboardAction(id: .spaceSelectByIndex, target: .spaceIndex(1)),
            "command-option-2 must resolve to dynamic second-space selection"
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

    private static func verifiesDynamicSpaceShortcutConflictsAreRejected() throws {
        let conflictingSpaceShortcut = ShellActionShortcut(
            key: "1",
            modifiers: [.command, .option],
            context: .shell
        )
        let custom = ShellActionDescriptor(
            id: .newTerminalTab,
            title: "Conflicting Dynamic Shortcut",
            targetKind: .currentSelection,
            defaultShortcut: conflictingSpaceShortcut,
            effect: .workspaceCommand(.newTerminalTab)
        )
        let dynamicSpaceSelection = ShellActionDescriptor(
            id: .spaceSelectByIndex,
            title: "Select Space",
            targetKind: .space,
            effect: .selectSpaceAt(0)
        )

        do {
            _ = try ShellActionRegistry(actions: [custom, dynamicSpaceSelection])
            expect(false, "dynamic numeric space shortcuts must participate in conflict detection")
        } catch ShellActionRegistryError.duplicateShortcut(let shortcut, let actionIDs) {
            expect(shortcut == conflictingSpaceShortcut, "duplicate shortcut error must include the dynamic shortcut")
            expect(
                Set(actionIDs) == Set([.newTerminalTab, .spaceSelectByIndex]),
                "dynamic space shortcut conflicts must name both action ids"
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

    private static func verifiesUnavailableShortcutActionDoesNotExecuteHandler() throws {
        let state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        var handledEffects: [ShellActionEffect] = []

        expect(
            ShellActionRegistry.standard.defaultShortcut(for: .tabMoveLeft) != nil,
            "disabled move-tab actions must still expose menu shortcut hints"
        )
        let result = ShellActionRegistry.standard.execute(
            .tabMoveLeft,
            target: .currentSelection,
            state: state
        ) { effect in
            handledEffects.append(effect)
            return true
        }

        expect(
            result == .unavailable(reason: "Move Tab Left is not available yet"),
            "disabled move-tab shortcuts must report a stable unavailable reason"
        )
        expect(handledEffects.isEmpty, "disabled move-tab shortcuts must not mutate state")
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
