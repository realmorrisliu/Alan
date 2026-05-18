import Darwin
import Foundation
import AppKit

#if os(macOS)
@main
struct TerminalSurfaceControllerTestRunner {
    static func main() async {
        await MainActor.run {
            TerminalSurfaceControllerTests.run()
        }
    }
}

@MainActor
private enum TerminalSurfaceControllerTests {
    static func run() {
        verifiesScrollbackMetricsAndTerminalModes()
        verifiesModeTrackerPreservesTerminalScrollRouting()
        verifiesNativeScrollViewForwardsWheelEvents()
        verifiesNativeScrollViewForwardsMouseEvents()
        verifiesInputCommandRouting()
        verifiesTUIKeyboardRoutingKeepsTerminalOwnedKeysInTerminal()
        verifiesKeyboardPipelineKeepsPhysicalKeysOnGhosttyKeyPath()
        verifiesGhosttyKeyEquivalentRedispatchContract()
        verifiesFocusOnlyClickSuppressionPolicy()
        verifiesTerminalInputRouterOwnsFocusOnlyPointerSequence()
        verifiesTextInputInterpretationPolicyPreservesIMEMarkedText()
        verifiesPointerRoutingFollowsTerminalMouseModes()
        verifiesPointerButtonMappingMatchesGhostty()
        verifiesPaneScopedSearchState()
        verifiesSearchActionsReachSurfaceEngine()
        verifiesScrollbackActionsReachSurfaceEngine()
        verifiesBindClearsStaleScrollbackState()
        verifiesSurfaceCloseRequestIsForwarded()
        verifiesSurfaceSnapshotEqualityIgnoresTimestamp()
        verifiesClipboardDeliveryStates()
        verifiesSelectionCopyAndPasteUseController()
        verifiesMetadataOverlayProjection()
        print("Terminal surface controller tests passed.")
    }

    private static func verifiesScrollbackMetricsAndTerminalModes() {
        let adapter = AlanTerminalScrollbackAdapter()
        let normal = adapter.updateMetrics(
            AlanTerminalScrollbackMetrics(
                totalRows: 140,
                visibleRows: 40,
                firstVisibleRow: 90,
                mode: .normalBuffer
            )
        )

        expect(normal.nativeScrollbarVisible, "normal-buffer scrollback must expose native scrollbar")
        expect(normal.thumbRange.lowerBound == 90, "scrollbar must track first visible row")
        expect(normal.thumbRange.upperBound == 130, "scrollbar must track visible row range")

        let alternate = adapter.updateMetrics(
            AlanTerminalScrollbackMetrics(
                totalRows: 140,
                visibleRows: 40,
                firstVisibleRow: 90,
                mode: .alternateScreen
            )
        )

        expect(
            alternate.nativeScrollbarVisible == false,
            "alternate-screen scrollback must not expose stale normal-buffer scrollbar"
        )
    }

    private static func verifiesModeTrackerPreservesTerminalScrollRouting() {
        let tracker = AlanTerminalModeTracker()

        let initialMode = tracker.resolveMode(
            totalRows: 40,
            visibleRows: 40,
            mouseCaptured: false
        )
        expect(initialMode == .normalBuffer, "initial unscrollable terminal state must stay normal")

        let scrollbackMode = tracker.resolveMode(
            totalRows: 220,
            visibleRows: 40,
            mouseCaptured: false
        )
        expect(scrollbackMode == .normalBuffer, "scrollable primary screen must use normal mode")

        let alternateMode = tracker.resolveMode(
            totalRows: 40,
            visibleRows: 40,
            mouseCaptured: false
        )
        expect(
            alternateMode == .alternateScreen,
            "collapsed scrollbar after scrollback must preserve alternate-screen terminal routing"
        )

        let mouseMode = tracker.resolveMode(
            totalRows: 220,
            visibleRows: 40,
            mouseCaptured: true
        )
        expect(mouseMode == .mouseReporting, "mouse-captured surfaces must route scroll to terminal")

        tracker.reset()
        let resetMode = tracker.resolveMode(
            totalRows: 40,
            visibleRows: 40,
            mouseCaptured: false
        )
        expect(resetMode == .normalBuffer, "new surfaces must not inherit prior alternate-screen state")
    }

    private static func verifiesNativeScrollViewForwardsWheelEvents() {
        let adapter = AlanTerminalNativeScrollViewAdapter()
        var routedDeltaY: Double?
        adapter.onScrollWheel = { event in
            routedDeltaY = event.scrollingDeltaY
            return true
        }

        let event = RecordingTerminalScrollWheelEvent(deltaX: 0, deltaY: -7)
        adapter.scrollView.scrollWheel(with: event)
        expect(routedDeltaY == -7, "native scroll view must forward wheel events to terminal routing")
    }

    private static func verifiesNativeScrollViewForwardsMouseEvents() {
        let adapter = AlanTerminalNativeScrollViewAdapter()
        var routedEvents: [AlanTerminalRoutedMouseEvent] = []
        adapter.onMouseEvent = { routedEvent, _ in
            routedEvents.append(routedEvent)
            return true
        }

        let event = RecordingTerminalMouseEvent()
        adapter.scrollView.mouseDown(with: event)
        adapter.scrollView.mouseDragged(with: event)
        adapter.scrollView.mouseUp(with: event)
        expect(
            routedEvents == [.mouseDown, .mouseDragged, .mouseUp],
            "native scroll view must forward mouse events to terminal routing"
        )
    }

    private static func verifiesInputCommandRouting() {
        let router = AlanTerminalInputRouter()
        let command = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "q",
                keyCode: 12,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(command == .nativeCommand("quit"), "command-q must route to the native app command")

        let findCommand = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "f",
                keyCode: 3,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(findCommand == .nativeCommand("find"), "command-f must route to pane search")

        let printable = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "a",
                keyCode: 0,
                modifiers: [],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            printable == .interpretTextInput,
            "printable key input must enter AppKit text interpretation so IME composition can start"
        )

        let splitDown = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "d",
                keyCode: 2,
                modifiers: [.command, .shift],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            splitDown == .workspaceCommand(.splitDown),
            "command-shift-d must route to shell split down before terminal bindings"
        )

        let splitRight = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "d",
                keyCode: 2,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            splitRight == .workspaceCommand(.splitRight),
            "command-d must route to shell split right before terminal bindings"
        )

        let focusRight = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: nil,
                keyCode: 0x7C,
                modifiers: [.command, .control],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            focusRight == .workspaceCommand(.focusRight),
            "command-control-right must route to shell focus right"
        )
    }

    private static func verifiesTUIKeyboardRoutingKeepsTerminalOwnedKeysInTerminal() {
        let router = AlanTerminalInputRouter()

        let controlWWorkspaceCommand = router.routeWorkspaceCommand(
            AlanTerminalKeyInput(
                characters: "w",
                keyCode: 13,
                modifiers: [.control],
                phase: .down,
                isRepeat: false
            )
        )
        expect(controlWWorkspaceCommand == nil, "control-w must not be consumed as a workspace command")

        let controlW = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "\u{17}",
                keyCode: 13,
                modifiers: [.control],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(controlW == .terminalKey, "control-w must stay a terminal key for Vim split navigation")

        let escape = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "\u{1B}",
                keyCode: 53,
                modifiers: [],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(escape == .terminalKey, "escape must stay a terminal key for TUI command mode")

        let tab = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "\t",
                keyCode: 48,
                modifiers: [],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(tab == .terminalKey, "tab must stay a terminal key for TUI focus and completion")

        let optionF = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "f",
                keyCode: 3,
                modifiers: [.option],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(optionF == .terminalKey, "option-modified keys must preserve modifier-aware terminal delivery")

        let commandT = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "t",
                keyCode: 17,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            commandT == .workspaceCommand(.newTerminalTab),
            "command-t must remain a native workspace shortcut"
        )
    }

    private static func verifiesKeyboardPipelineKeepsPhysicalKeysOnGhosttyKeyPath() {
        let router = AlanTerminalInputRouter()

        let printable = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "a",
                keyCode: 0,
                modifiers: [],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            printable == .interpretTextInput,
            "printable physical keys must enter AppKit text interpretation before Ghostty key delivery"
        )

        let colon = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: ":",
                keyCode: 41,
                modifiers: [.shift],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            colon == .interpretTextInput,
            "shift-printable physical keys must enter AppKit text interpretation before Ghostty key delivery"
        )

        let escape = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "\u{1B}",
                keyCode: 53,
                modifiers: [],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            escape == .terminalKey,
            "Escape must be delivered through Ghostty key events for Vim command mode"
        )

        let controlBracket = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "\u{1B}",
                keyCode: 33,
                modifiers: [.control],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(controlBracket == .terminalKey, "Control-[ must remain terminal-owned")

        let controlW = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "\u{17}",
                keyCode: 13,
                modifiers: [.control],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            controlW == .terminalKey,
            "Control-W must remain terminal-owned for Vim split navigation"
        )

        let commandT = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "t",
                keyCode: 17,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: false
        )
        expect(
            commandT == .workspaceCommand(.newTerminalTab),
            "Command-T must remain an app workspace command"
        )

        let composingBackspace = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "\u{7F}",
                keyCode: 51,
                modifiers: [],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: true
        )
        expect(
            composingBackspace == .interpretTextInput,
            "IME marked text must keep control keys in AppKit text interpretation first"
        )
    }

    private static func verifiesGhosttyKeyEquivalentRedispatchContract() {
        let adapter = AlanTerminalKeyEquivalentAdapter()

        let terminalBinding = AlanTerminalKeyEquivalentInput(
            characters: "\u{1B}",
            charactersIgnoringModifiers: "\u{1B}",
            modifiers: [],
            keyCode: 53,
            timestamp: 10,
            isRepeat: false
        )
        expect(
            adapter.routeKeyEquivalent(
                terminalBinding,
                isFocused: true,
                isTerminalBinding: true
            ) == .sendOriginal,
            "Ghostty terminal bindings must be sent immediately through keyDown"
        )

        let controlSlash = AlanTerminalKeyEquivalentInput(
            characters: "\u{1F}",
            charactersIgnoringModifiers: "/",
            modifiers: [.control],
            keyCode: 44,
            timestamp: 11,
            isRepeat: false
        )
        expect(
            adapter.routeKeyEquivalent(
                controlSlash,
                isFocused: true,
                isTerminalBinding: false
            ) == .sendEquivalent("_"),
            "control-slash must be normalized to control-underscore like Ghostty"
        )

        let commandPeriod = AlanTerminalKeyEquivalentInput(
            characters: ".",
            charactersIgnoringModifiers: ".",
            modifiers: [.command],
            keyCode: 47,
            timestamp: 12,
            isRepeat: false
        )
        expect(
            adapter.routeKeyEquivalent(
                commandPeriod,
                isFocused: true,
                isTerminalBinding: false
            ) == .deferToResponder,
            "unhandled command/control equivalents must first defer to AppKit"
        )
        expect(
            adapter.shouldRedispatchDoCommand(currentEventTimestamp: 12),
            "doCommand must redispatch the same key-equivalent event"
        )
        expect(
            adapter.routeKeyEquivalent(
                commandPeriod,
                isFocused: true,
                isTerminalBinding: false
            ) == .sendEquivalent("."),
            "redispatched command/control equivalents must be delivered once to the terminal"
        )
        expect(
            !adapter.shouldRedispatchDoCommand(currentEventTimestamp: 12),
            "redispatch state must clear after the equivalent is delivered"
        )

        let syntheticCommand = AlanTerminalKeyEquivalentInput(
            characters: ".",
            charactersIgnoringModifiers: ".",
            modifiers: [.command],
            keyCode: 47,
            timestamp: 0,
            isRepeat: false
        )
        expect(
            adapter.routeKeyEquivalent(
                syntheticCommand,
                isFocused: true,
                isTerminalBinding: false
            ) == .deferToResponder,
            "synthetic zero-timestamp AppKit events must not enter the terminal redispatch loop"
        )
        expect(
            !adapter.shouldRedispatchDoCommand(currentEventTimestamp: 0),
            "zero-timestamp events must not arm doCommand redispatch"
        )
    }

    private static func verifiesFocusOnlyClickSuppressionPolicy() {
        let router = AlanTerminalInputRouter()

        expect(
            router.routeLeftMouseDown(
                hitOwnsTerminal: true,
                commandSurfaceVisible: false,
                isFirstResponder: false,
                appIsActive: true,
                windowIsKey: true
            ) == .focusOnly,
            "active-window clicks into an unfocused terminal split must only transfer focus"
        )

        let suppressedDown = router.routePointer(
            primaryPointerInput(phase: .buttonDown),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            suppressedDown == .consumed,
            "focus-only mouse down must suppress the matching terminal button press"
        )

        let suppressedUp = router.routePointer(
            primaryPointerInput(phase: .buttonUp),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            suppressedUp == .consumed,
            "focus-only mouse down must suppress the matching left mouse up"
        )

        _ = router.routeLeftMouseDown(
            hitOwnsTerminal: true,
            commandSurfaceVisible: false,
            isFirstResponder: false,
            appIsActive: true,
            windowIsKey: true
        )
        let suppressedDrag = router.routePointer(
            primaryPointerInput(phase: .drag),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            suppressedDrag == .consumed,
            "focus-only mouse down must suppress matching left mouse drags"
        )
        let dragSuppressionClosingUp = router.routePointer(
            primaryPointerInput(phase: .buttonUp),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            dragSuppressionClosingUp == .consumed,
            "focus-only drag suppression must keep suppressing until mouse up"
        )
        let postSuppressionDown = router.routePointer(
            primaryPointerInput(phase: .buttonDown),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            postSuppressionDown == .terminalSelection(
                .button(state: .press, button: .primary, x: 24, y: 48, modifiers: [])
            ),
            "left mouse-drag suppression must clear after mouse up"
        )

        expect(
            AlanTerminalInputRouter().routeLeftMouseDown(
                hitOwnsTerminal: true,
                commandSurfaceVisible: false,
                isFirstResponder: true,
                appIsActive: true,
                windowIsKey: true
            ) == .deliverToTerminal,
            "clicks in the focused terminal must still reach Vim mouse mode"
        )

        expect(
            AlanTerminalInputRouter().routeLeftMouseDown(
                hitOwnsTerminal: true,
                commandSurfaceVisible: false,
                isFirstResponder: false,
                appIsActive: false,
                windowIsKey: false
            ) == .focusAndDeliver,
            "inactive-window activation clicks must continue through AppKit"
        )
    }

    private static func verifiesTerminalInputRouterOwnsFocusOnlyPointerSequence() {
        let router = AlanTerminalInputRouter()

        expect(
            router.routeLeftMouseDown(
                hitOwnsTerminal: true,
                commandSurfaceVisible: false,
                isFirstResponder: false,
                appIsActive: true,
                windowIsKey: true
            ) == .focusOnly,
            "active-window focus-transfer clicks must be identified by the terminal input router"
        )

        let suppressedDown = router.routePointer(
            primaryPointerInput(phase: .buttonDown),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            suppressedDown == .consumed,
            "focus-transfer primary button-down must be consumed even if AppKit still dispatches it"
        )

        let suppressedDrag = router.routePointer(
            primaryPointerInput(phase: .drag),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            suppressedDrag == .consumed,
            "focus-transfer primary drags must be consumed by the same input router sequence"
        )

        let suppressedUp = router.routePointer(
            primaryPointerInput(phase: .buttonUp),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            suppressedUp == .consumed,
            "focus-transfer primary mouse-up must close the router-owned suppression sequence"
        )

        let nextPrimaryDown = router.routePointer(
            primaryPointerInput(phase: .buttonDown),
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            nextPrimaryDown == .terminalSelection(
                .button(state: .press, button: .primary, x: 24, y: 48, modifiers: [])
            ),
            "router suppression must end before the next primary button sequence"
        )
    }

    private static func primaryPointerInput(
        phase: AlanTerminalPointerPhase
    ) -> AlanTerminalPointerInput {
        AlanTerminalPointerInput(
            phase: phase,
            button: .primary,
            buttonNumber: 0,
            x: 24,
            y: 48,
            modifiers: [],
            pressureStage: nil,
            pressure: nil
        )
    }

    private static func verifiesTextInputInterpretationPolicyPreservesIMEMarkedText() {
        let router = AlanTerminalInputRouter()

        let backspace = AlanTerminalKeyInput(
            characters: "\u{7F}",
            keyCode: 51,
            modifiers: [],
            phase: .down,
            isRepeat: false
        )
        expect(
            router.routeKeyboard(backspace, hasMarkedText: false) == .terminalKey,
            "backspace must remain a terminal-owned key outside IME composition"
        )
        expect(
            router.routeKeyboard(backspace, hasMarkedText: true) == .interpretTextInput,
            "backspace must go through NSTextInput while IME marked text exists"
        )

        let printable = AlanTerminalKeyInput(
            characters: "a",
            keyCode: 0,
            modifiers: [],
            phase: .down,
            isRepeat: false
        )
        expect(
            router.routeKeyboard(printable, hasMarkedText: false) == .interpretTextInput,
            "printable physical input must enter AppKit text interpretation outside active preedit"
        )
        expect(
            router.routeKeyboard(printable, hasMarkedText: true) == .interpretTextInput,
            "printable input must still go through NSTextInput while IME marked text exists"
        )

        let findCommand = router.routeKeyboard(
            AlanTerminalKeyInput(
                characters: "f",
                keyCode: 3,
                modifiers: [.command],
                phase: .down,
                isRepeat: false
            ),
            hasMarkedText: true
        )
        expect(
            findCommand == .nativeCommand("find"),
            "native command shortcuts must not be reinterpreted as IME text input"
        )

        expect(
            AlanTerminalTextCompositionPolicy.shouldSuppressComposingControlInput(
                "\u{08}",
                composing: true
            ),
            "raw C0 control input must not leak to the terminal during composition"
        )
        expect(
            AlanTerminalTextCompositionPolicy.shouldSuppressComposingControlInput(
                "\u{7F}",
                composing: true
            ),
            "backspace must not leak to the terminal during composition"
        )
        expect(
            !AlanTerminalTextCompositionPolicy.shouldSuppressComposingControlInput(
                "\u{08}",
                composing: false
            ),
            "raw C0 control input remains terminal-owned outside composition"
        )
    }

    private static func verifiesPointerRoutingFollowsTerminalMouseModes() {
        let adapter = AlanTerminalPointerAdapter()
        let drag = AlanTerminalPointerInput(
            phase: .drag,
            button: .primary,
            buttonNumber: 0,
            x: 24,
            y: 48,
            modifiers: [.shift],
            pressureStage: nil,
            pressure: nil
        )

        let selectionRoute = adapter.routePointer(
            drag,
            terminalMode: .normalBuffer,
            surfaceReady: true
        )
        expect(
            selectionRoute == .terminalSelection(
                .position(x: 24, y: 48, modifiers: [.shift])
            ),
            "normal-buffer primary drags must be treated as terminal text selection"
        )

        let mouseAppRoute = adapter.routePointer(
            drag,
            terminalMode: .mouseReporting,
            surfaceReady: true
        )
        expect(
            mouseAppRoute == .terminalMouse(
                .position(x: 24, y: 48, modifiers: [.shift])
            ),
            "mouse-reporting drags must be delivered to the terminal application"
        )

        let exited = AlanTerminalPointerInput(
            phase: .exited,
            button: nil,
            buttonNumber: nil,
            x: 24,
            y: 48,
            modifiers: [],
            pressureStage: nil,
            pressure: nil
        )
        expect(
            adapter.routePointer(exited, terminalMode: .normalBuffer, surfaceReady: true)
                == .terminalHover(.position(x: -1, y: -1, modifiers: [])),
            "pointer exit must normalize to a terminal hover-out position"
        )

        let pressure = AlanTerminalPointerInput(
            phase: .pressure,
            button: nil,
            buttonNumber: nil,
            x: 0,
            y: 0,
            modifiers: [],
            pressureStage: 2,
            pressure: 0.5
        )
        expect(
            adapter.routePointer(pressure, terminalMode: .normalBuffer, surfaceReady: true)
                == .terminalMouse(.pressure(stage: 2, pressure: 0.5)),
            "pressure changes must be normalized before delivery to the terminal"
        )
        expect(
            adapter.routePointer(drag, terminalMode: .mouseReporting, surfaceReady: false) == .ignored,
            "unready terminal surfaces must not receive normalized pointer events"
        )
    }

    private static func verifiesPointerButtonMappingMatchesGhostty() {
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(0) == .primary, "button 0 must be primary")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(1) == .secondary, "button 1 must be secondary")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(2) == .middle, "button 2 must be middle")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(3) == .eight, "button 3 must match Ghostty back button mapping")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(4) == .nine, "button 4 must match Ghostty forward button mapping")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(5) == .six, "button 5 must match Ghostty button 6 mapping")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(6) == .seven, "button 6 must match Ghostty button 7 mapping")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(7) == .four, "button 7 must match Ghostty button 4 mapping")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(8) == .five, "button 8 must match Ghostty button 5 mapping")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(9) == .ten, "button 9 must match Ghostty button 10 mapping")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(10) == .eleven, "button 10 must map through Ghostty's high button range")
        expect(AlanTerminalPointerButton.fromAppKitButtonNumber(11) == .unknown, "unsupported button numbers must not pretend to be middle")

        let press = AlanTerminalPointerInput(
            phase: .buttonDown,
            button: nil,
            buttonNumber: 3,
            x: 10,
            y: 11,
            modifiers: [.command],
            pressureStage: nil,
            pressure: nil
        )
        expect(
            AlanTerminalPointerAdapter().routePointer(
                press,
                terminalMode: .mouseReporting,
                surfaceReady: true
            ) == .terminalMouse(
                .button(
                    state: .press,
                    button: .eight,
                    x: 10,
                    y: 11,
                    modifiers: [.command]
                )
            ),
            "other-button presses must preserve their normalized button family"
        )
    }

    private static func verifiesPaneScopedSearchState() {
        let search = AlanTerminalSearchAdapter(paneID: "pane_1")
        search.updateQuery("runtime")
        search.updateMatches(total: 3, selectedIndex: 0)
        search.next()
        expect(search.state.selectedIndex == 1, "next search result must advance inside the pane")
        search.previous()
        expect(search.state.selectedIndex == 0, "previous search result must move backward inside the pane")
        search.dismiss()
        expect(search.state.isActive == false, "dismissed search must be inactive")
        expect(search.state.paneID == "pane_1", "search state must remain pane scoped")
    }

    private static func verifiesSearchActionsReachSurfaceEngine() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        let controller = AlanTerminalSurfaceController()
        var searchStateChangeCount = 0
        controller.onSearchStateChange = {
            searchStateChangeCount += 1
        }
        controller.bind(surfaceHandle: handle, paneID: "pane_1")

        expect(controller.beginSearch(), "controller must start search through the surface engine")
        expect(handle.searchActions == ["start_search"], "begin search must invoke Ghostty search action")
        expect(controller.searchAdapter?.state.isActive == true, "started search must become active")
        let firstFocusRequestID = controller.searchAdapter?.state.focusRequestID

        expect(controller.beginSearch(), "active search must accept a renewed focus request")
        expect(handle.searchActions == ["start_search"], "active search focus must not restart the search engine")
        expect(
            controller.searchAdapter?.state.focusRequestID == firstFocusRequestID.map { $0 + 1 },
            "active search focus must refresh the Find bar focus token"
        )

        expect(
            controller.updateSearchQuery("alan"),
            "query changes must be accepted by the surface search engine"
        )
        expect(
            handle.searchActions.suffix(1) == ["search:alan"],
            "query changes must reach the terminal search action"
        )

        let countBeforeEngineUpdates = searchStateChangeCount
        handle.emitSearchUpdate(.matches(total: 3))
        handle.emitSearchUpdate(.selected(index: 1))
        expect(controller.searchAdapter?.state.totalMatches == 3, "search totals must update from engine callbacks")
        expect(controller.searchAdapter?.state.selectedIndex == 1, "selected match must update from engine callbacks")
        expect(
            searchStateChangeCount == countBeforeEngineUpdates + 2,
            "engine search callbacks must notify host UI refresh"
        )

        controller.nextSearchMatch()
        controller.previousSearchMatch()
        expect(
            handle.searchActions.suffix(2) == ["navigate_search:next", "navigate_search:previous"],
            "search navigation must be delegated to the terminal search engine"
        )

        controller.dismissSearch()
        expect(handle.searchActions.last == "end_search", "dismissed search must end the terminal search")
        expect(controller.searchAdapter?.state.isActive == false, "dismissed search must deactivate local state")

        handle.searchActionsShouldSucceed = false
        expect(
            controller.beginSearch() == false,
            "failed engine start must not pretend search is available"
        )
    }

    private static func verifiesScrollbackActionsReachSurfaceEngine() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        let controller = AlanTerminalSurfaceController()
        var stateChangeCount = 0
        controller.onSurfaceStateChange = {
            stateChangeCount += 1
        }
        controller.bind(surfaceHandle: handle, paneID: "pane_1")

        handle.emitScrollbackUpdate(
            AlanTerminalScrollbackMetrics(
                totalRows: 220,
                visibleRows: 40,
                firstVisibleRow: 120,
                mode: .normalBuffer
            )
        )

        expect(controller.scrollbackAdapter.state.nativeScrollbarVisible, "Ghostty scrollbar updates must expose native scrollbar state")
        expect(
            controller.scrollbackAdapter.state.thumbRange == 120..<160,
            "Ghostty scrollbar updates must set the visible terminal row range"
        )
        expect(stateChangeCount == 1, "scrollbar updates must notify host UI/runtime refresh")

        let normalRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: false)
        )
        expect(
            normalRoute == .nativeScroll(row: 128),
            "normal-buffer non-precise scroll input must become a native row scroll"
        )
        expect(
            handle.scrollActions == ["scroll_to_row:128"],
            "native row scroll must reach the terminal surface"
        )

        controller.syncNativeScrollView(viewportSize: CGSize(width: 800, height: 640))
        let firstPreciseRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: true)
        )
        expect(
            firstPreciseRoute == .ignored,
            "sub-row precise scroll input must be consumed while accumulating native row movement"
        )
        let secondPreciseRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: true)
        )
        expect(
            secondPreciseRoute == .nativeScroll(row: 129),
            "precise scroll input must scale point deltas by row height before native row scroll"
        )
        expect(
            handle.scrollActions.suffix(1) == ["scroll_to_row:129"],
            "accumulated precise scroll must move by one row at a 16-point row height"
        )

        handle.emitScrollbackUpdate(
            AlanTerminalScrollbackMetrics(
                totalRows: 220,
                visibleRows: 40,
                firstVisibleRow: 120,
                mode: .alternateScreen
            )
        )
        let alternateRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: true)
        )
        expect(
            alternateRoute == .terminalScroll,
            "alternate-screen scroll input must stay routed to the terminal app"
        )

        handle.emitScrollbackUpdate(
            AlanTerminalScrollbackMetrics(
                totalRows: 220,
                visibleRows: 40,
                firstVisibleRow: 120,
                mode: .mouseReporting
            )
        )
        let mouseReportingRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: true)
        )
        expect(
            mouseReportingRoute == .terminalScroll,
            "terminal mouse-reporting scroll input must stay routed to the terminal app"
        )
    }

    private static func verifiesBindClearsStaleScrollbackState() {
        let firstHandle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        let secondHandle = FakeAlanTerminalSurfaceHandle(paneID: "pane_2")
        let controller = AlanTerminalSurfaceController()
        var stateChangeCount = 0
        controller.onSurfaceStateChange = {
            stateChangeCount += 1
        }
        controller.bind(surfaceHandle: firstHandle, paneID: "pane_1")
        firstHandle.emitScrollbackUpdate(
            AlanTerminalScrollbackMetrics(
                totalRows: 220,
                visibleRows: 40,
                firstVisibleRow: 120,
                mode: .normalBuffer
            )
        )

        expect(controller.scrollbackAdapter.state.nativeScrollbarVisible, "first surface must expose scrollback")
        expect(stateChangeCount == 1, "first surface scrollback update must notify once")
        controller.updateMetadata(
            TerminalPaneMetadataSnapshot(
                title: "old pane",
                workingDirectory: "/tmp/old",
                summary: "exited",
                attention: .idle,
                processExited: true,
                lastCommandExitCode: 1,
                lastUpdatedAt: .now
            )
        )
        controller.updateRenderer(
            TerminalRendererSnapshot(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "old renderer failed",
                detail: nil,
                failureReason: "old surface failed",
                recentEvents: []
            )
        )

        controller.bind(surfaceHandle: secondHandle, paneID: "pane_2")
        expect(
            controller.scrollbackAdapter.state == .empty,
            "rebinding to a new surface must clear stale scrollback"
        )
        expect(stateChangeCount == 2, "clearing stale scrollback must notify host UI refresh")

        let staleRoute = controller.routeScroll(
            AlanTerminalScrollInput(deltaX: 0, deltaY: -8, precise: false)
        )
        expect(
            staleRoute == .terminalScroll,
            "new surfaces without scrollback metrics must not issue stale native row scrolls"
        )
        expect(secondHandle.scrollActions.isEmpty, "stale scrollback must not reach the new surface")

        firstHandle.emitScrollbackUpdate(
            AlanTerminalScrollbackMetrics(
                totalRows: 400,
                visibleRows: 50,
                firstVisibleRow: 200,
                mode: .normalBuffer
            )
        )
        expect(
            controller.scrollbackAdapter.state == .empty,
            "old surface scrollback callbacks must be detached after rebind"
        )
        expect(
            controller.surfaceStateSnapshot.childExited == false,
            "rebinding to a new surface must clear stale child-exit metadata"
        )
        expect(
            controller.surfaceStateSnapshot.rendererHealth != "failed",
            "rebinding to a new surface must clear stale renderer failure state"
        )
    }

    private static func verifiesSurfaceCloseRequestIsForwarded() {
        let controller = AlanTerminalSurfaceController()
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        var closeRequests: [Bool] = []

        controller.bind(surfaceHandle: handle, paneID: "pane_1")
        controller.attach(
            to: NSView(),
            bootProfile: nil,
            focused: true,
            onDiagnosticsChange: { _ in },
            onMetadataChange: { _ in },
            onCloseRequest: { requiresConfirmation in
                closeRequests.append(requiresConfirmation)
            }
        )

        handle.requestClose(requiresConfirmation: false)

        expect(closeRequests == [false], "surface close requests must be forwarded to the shell owner")
    }

    private static func verifiesSurfaceSnapshotEqualityIgnoresTimestamp() {
        let older = AlanTerminalSurfaceStateSnapshot.placeholder
        let newer = AlanTerminalSurfaceStateSnapshot(
            readiness: older.readiness,
            terminalMode: older.terminalMode,
            scrollback: older.scrollback,
            search: older.search,
            readonly: older.readonly,
            secureInput: older.secureInput,
            inputReady: older.inputReady,
            rendererHealth: older.rendererHealth,
            childExited: older.childExited,
            lastUpdatedAt: older.lastUpdatedAt.addingTimeInterval(60)
        )
        expect(older != newer, "regular surface snapshot equality must keep timestamp changes visible")
        expect(
            older.equalsIgnoringTimestamp(newer),
            "runtime snapshot diffing must ignore surface timestamp churn"
        )
    }

    private static func verifiesClipboardDeliveryStates() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        let clipboard = AlanTerminalSelectionClipboardAdapter(surfaceHandle: handle)
        let accepted = clipboard.paste("hello")
        expect(accepted.applied, "ready paste must be delivered")
        expect(handle.deliveredText == ["hello"], "ready paste must reach the surface handle")

        _ = handle.teardown()
        let rejected = clipboard.paste("again")
        expect(rejected.applied == false, "closed paste must not report success")
        expect(rejected.errorCode == "terminal_clipboard_unavailable", "closed paste must use a stable clipboard error")
    }

    private static func verifiesSelectionCopyAndPasteUseController() {
        let handle = FakeAlanTerminalSurfaceHandle(paneID: "pane_1")
        handle.selectedText = "selected terminal text"
        let controller = AlanTerminalSurfaceController()
        controller.bind(surfaceHandle: handle, paneID: "pane_1")

        let pasteboard = RecordingTerminalPasteboardWriter()
        expect(
            controller.copySelection(to: pasteboard),
            "controller copy must write selected terminal text to the pasteboard"
        )
        expect(
            pasteboard.string == "selected terminal text",
            "copied terminal selection must be available as pasteboard text"
        )

        let pasteResult = controller.paste("pasted text")
        expect(pasteResult.applied, "controller paste must use failure-aware delivery")
        expect(handle.deliveredText.last == "pasted text", "controller paste must reach the surface handle")

        _ = handle.teardown()
        let rejected = controller.paste("after close")
        expect(rejected.applied == false, "closed controller paste must not report success")
        expect(
            rejected.errorCode == "terminal_clipboard_unavailable",
            "closed controller paste must use the stable clipboard error"
        )
    }

    private static func verifiesMetadataOverlayProjection() {
        let adapter = AlanTerminalMetadataAdapter()
        let failed = adapter.overlayState(
            renderer: TerminalRendererSnapshot(
                kind: .ghosttyLive,
                phase: .failed,
                summary: "internal callback failed",
                detail: nil,
                failureReason: "renderer lost device",
                recentEvents: ["raw callback name"]
            ),
            metadata: .placeholder,
            surface: .unready(reason: .rendererFailed)
        )
        expect(failed?.title == "Terminal cannot draw", "renderer failure overlay must use user-facing language")
        expect(failed?.debugDetail?.contains("renderer lost device") == true, "debug detail must preserve raw failure context")

        let exited = adapter.overlayState(
            renderer: .placeholder,
            metadata: TerminalPaneMetadataSnapshot(
                title: nil,
                workingDirectory: nil,
                summary: nil,
                attention: .idle,
                processExited: true,
                lastCommandExitCode: 2,
                lastUpdatedAt: .now
            ),
            surface: .ready
        )
        expect(exited?.title == "Process exited", "child exit overlay must be terminal-specific")
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: String
    ) {
        guard condition() else {
            fputs("error: \(message)\n", stderr)
            exit(1)
        }
    }
}

@MainActor
private final class RecordingTerminalPasteboardWriter: AlanTerminalPasteboardWriting {
    private(set) var string: String?

    func writeString(_ text: String) -> Bool {
        string = text
        return true
    }
}

private final class RecordingTerminalScrollWheelEvent: NSEvent {
    private let recordedDeltaX: CGFloat
    private let recordedDeltaY: CGFloat

    init(deltaX: CGFloat, deltaY: CGFloat) {
        recordedDeltaX = deltaX
        recordedDeltaY = deltaY
        super.init()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    override var scrollingDeltaX: CGFloat { recordedDeltaX }
    override var scrollingDeltaY: CGFloat { recordedDeltaY }
    override var hasPreciseScrollingDeltas: Bool { true }
    override var momentumPhase: NSEvent.Phase { [] }
}

private final class RecordingTerminalMouseEvent: NSEvent {
    override init() {
        super.init()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }
}
#endif
