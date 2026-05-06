## 1. Activation Boundary

- [ ] 1.1 Add `TerminalHostActivationDelegate` as a main-actor class-bound protocol with a pane activation method.
- [ ] 1.2 Make `ShellHostController` conform to the activation delegate and route requests to `focus(paneID:)`.
- [ ] 1.3 Thread the delegate through `TerminalHostView`, `TerminalRuntimeRegistry.hostView(...)`, and `AlanTerminalHostNSView.configure(...)`.
- [ ] 1.4 Store the activation delegate weakly on `AlanTerminalHostNSView` and refresh it whenever the host is configured.

## 2. Terminal Event Ownership

- [ ] 2.1 Remove terminal-pane selection ownership from `ShellTerminalLeafView` by deleting the terminal `.onTapGesture(perform: onSelect)` path and any now-unused terminal-leaf `onSelect` plumbing.
- [ ] 2.2 Add a host-owned activation helper that validates the current pane ID, asks the weak delegate to activate it, and requests terminal focus.
- [ ] 2.3 Call the activation helper before forwarding primary, secondary, and other mouse-down events to Ghostty.
- [ ] 2.4 Preserve existing mouse-up, drag, movement, scroll, pressure, key, IME, copy, paste, and command-key behavior through the AppKit terminal host.
- [ ] 2.5 Keep explicit SwiftUI controls, including pane selector buttons, on their existing selection actions.

## 3. Hit-Testing And Dragging Boundaries

- [ ] 3.1 Make `AlanGhosttyCanvasView` transparent to AppKit hit-testing while preserving `mouseDownCanMoveWindow == false`.
- [ ] 3.2 Make `AlanTerminalFallbackCanvasView` transparent to AppKit hit-testing while preserving `mouseDownCanMoveWindow == false`.
- [ ] 3.3 Make passive terminal placeholder/diagnostic overlay views non-interactive unless they contain explicit controls.
- [ ] 3.4 Confirm terminal pane clicks and drags still opt out of native background window dragging.

## 4. Contract Checks

- [ ] 4.1 Extend `clients/apple/scripts/check-shell-contracts.sh` to require the weak activation delegate boundary.
- [ ] 4.2 Extend the contract check to reject SwiftUI terminal leaf `.onTapGesture(perform: onSelect)`.
- [ ] 4.3 Extend the contract check to require transparent hit-testing for Ghostty and fallback rendering canvases.
- [ ] 4.4 Keep the existing occlusion and background-dragging checks intact.

## 5. Verification

- [ ] 5.1 Run `git diff --check`.
- [ ] 5.2 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 5.3 Build the macOS app with the documented Xcode command for `AlanNative`.
- [ ] 5.4 Manually verify click-to-select, immediate typing after selecting a pane, drag selection, right click, scroll, and background window dragging in the running app.
- [ ] 5.5 Review the diff for retain cycles, especially registry-owned host views retaining shell controller state.

## 6. PR And Archive Readiness

- [ ] 6.1 Update the PR with the implementation summary and verification results.
- [ ] 6.2 Resolve any review comments tied to terminal event ownership after confirming the running app behavior.
- [ ] 6.3 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.4 Archive the OpenSpec change after implementation is merged.
