## 1. Activation Boundary

- [x] 1.1 Add `TerminalHostActivationDelegate` as a main-actor class-bound protocol with a pane activation method.
- [x] 1.2 Make `ShellHostController` conform to the activation delegate and route requests to `focus(paneID:)`.
- [x] 1.3 Thread the delegate through `TerminalHostView`, `TerminalRuntimeRegistry.hostView(...)`, and `AlanTerminalHostNSView.configure(...)`.
- [x] 1.4 Store the activation delegate weakly on `AlanTerminalHostNSView` and refresh it whenever the host is configured.

## 2. Terminal Event Ownership

- [x] 2.1 Remove terminal-pane selection ownership from `ShellTerminalLeafView` by deleting the terminal `.onTapGesture(perform: onSelect)` path and any now-unused terminal-leaf `onSelect` plumbing.
- [x] 2.2 Add a host-owned activation helper that validates the current pane ID, asks the weak delegate to activate it, and requests terminal focus.
- [x] 2.3 Call the activation helper before forwarding primary, secondary, and other mouse-down events to Ghostty.
- [x] 2.4 Preserve existing mouse-up, drag, movement, scroll, pressure, key, IME, copy, paste, and command-key behavior through the AppKit terminal host.
- [x] 2.5 Keep explicit SwiftUI controls, including pane selector buttons, on their existing selection actions.

## 3. Hit-Testing And Dragging Boundaries

- [x] 3.1 Make `AlanGhosttyCanvasView` transparent to AppKit hit-testing while preserving `mouseDownCanMoveWindow == false`.
- [x] 3.2 Make `AlanTerminalFallbackCanvasView` transparent to AppKit hit-testing while preserving `mouseDownCanMoveWindow == false`.
- [x] 3.3 Make passive terminal placeholder/diagnostic overlay views non-interactive unless they contain explicit controls.
- [x] 3.4 Confirm terminal pane clicks and drags still opt out of native background window dragging.

## 4. Contract Checks

- [x] 4.1 Extend `clients/apple/scripts/check-shell-contracts.sh` to require the weak activation delegate boundary.
- [x] 4.2 Extend the contract check to reject SwiftUI terminal leaf `.onTapGesture(perform: onSelect)`.
- [x] 4.3 Extend the contract check to require transparent hit-testing for Ghostty and fallback rendering canvases.
- [x] 4.4 Keep the existing occlusion and background-dragging checks intact.

## 5. Verification

- [x] 5.1 Run `git diff --check`.
- [x] 5.2 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 5.3 Build the macOS app with the documented Xcode command for `AlanNative`.
- [x] 5.4 Manually verify click-to-select, immediate typing after selecting a pane, drag selection, right click, scroll, and background window dragging in the running app.
- [x] 5.5 Review the diff for retain cycles, especially registry-owned host views retaining shell controller state.

## 6. PR And Archive Readiness

- [x] 6.1 Update the PR with the implementation summary and verification results.
- [ ] 6.2 Resolve any review comments tied to terminal event ownership after confirming the running app behavior.
- [ ] 6.3 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.4 Archive the OpenSpec change after implementation is merged.
