## 1. Surface Controller Structure

- [x] 1.1 Add `AlanTerminalSurfaceController` to bind runtime surface handles to AppKit terminal views.
- [x] 1.2 Add dedicated input, scrollback, selection/clipboard, search, and metadata adapter types.
- [x] 1.3 Move existing terminal event forwarding into the controller without changing behavior.
- [x] 1.4 Expose fake surface handles and fake terminal events for controller tests.

## 2. Scrollback And Rendering State

- [x] 2.1 Add an AppKit scroll view adapter for terminal scrollback and native scrollbar synchronization.
- [x] 2.2 Handle alternate-screen and terminal mouse-mode interactions with scroll input.
- [x] 2.3 Project renderer health, input readiness, readonly state, and child-exit state into pane metadata.
- [x] 2.4 Add truthful fallback UI for renderer failure, missing surface, input-not-ready, and child-exit states.

## 3. Input, Selection, Clipboard, And Search

- [x] 3.1 Normalize keyDown, keyUp, flagsChanged, performKeyEquivalent, marked text, preedit, commit, and cancellation paths.
- [x] 3.2 Normalize primary, secondary, other-button, drag, movement, hover, pressure, and scroll events against terminal mouse modes.
- [x] 3.3 Implement native selection, copy, paste, bracketed-paste-aware delivery when available, and paste failure reporting.
- [x] 3.4 Add pane-scoped terminal search with find, next, previous, match status, and dismissal behavior.
- [x] 3.5 Add compact user-facing terminal overlays while keeping raw surface diagnostics in the inspector debug layer.

## 4. Shell Integration

- [x] 4.1 Update `TerminalPaneView` and host wrappers to compose the surface controller without adding decorative terminal chrome.
- [x] 4.2 Update sidebar/status metadata for title, cwd, bell, attention, process exit, and renderer health.
- [x] 4.3 Update control-plane text delivery and pane summaries to account for input readiness, child exit, readonly state, and renderer failure.
- [x] 4.4 Document any Ghostty surface behavior that remains unsupported after this pass.

## 5. Verification

- [x] 5.1 Add unit tests for scrollback metrics, input normalization, search state, clipboard failure states, and metadata projection with fake surfaces.
- [x] 5.2 Run `git diff --check`.
- [x] 5.3 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 5.4 Build the macOS app with the documented `AlanNative` command.
- [x] 5.5 Manually verify shell scrollback, alternate-screen scrolling, selection/copy, paste, right-click, IME composition, terminal mouse apps, search, child exit, and renderer/fallback state.

## 6. PR And Archive Readiness

- [x] 6.1 Attach screenshot or manual verification notes for terminal overlays and inspector debug separation.
- [x] 6.2 Review the diff for duplicate event forwarding or conflicting responder-chain ownership.
- [x] 6.3 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.4 Archive the OpenSpec change after implementation is merged.
