## 1. Inspector Removal

- [x] 1.1 Remove `alanShellShowsInspector`, `showsInspector` bindings, right-side inspector layout, and inspector animations from `MacShellRootView.swift`.
- [x] 1.2 Delete `ShellInspectorView`, `ShellInspectorSection`, `InspectorCard`, and inspector-only helper code after confirming any needed diagnostics remain available from shell snapshots, logs, scripts, or tests.
- [x] 1.3 Remove inspector sidebar/header toggle controls, accessibility labels, tooltips, and user-facing copy.
- [x] 1.4 Remove inspector actions and keywords from `ShellCommandTabAction`, default command-palette results, command execution, and dynamic show/hide title/detail logic.
- [x] 1.5 Remove inspector phrases from `ShellVoiceCommandController` vocabulary and handling.
- [x] 1.6 Update active UI polish changes or verification notes that still require inspector screenshots, inspector radii, or inspector smoke coverage.

## 2. Native Find Bar UI

- [x] 2.1 Add a compact pane-scoped Find bar component with editable text field, previous/next icon controls, close control, and match-count/no-result feedback.
- [x] 2.2 Render the Find bar for the focused pane without resizing the sidebar, toolbar, split tree, or terminal canvas.
- [x] 2.3 Focus and select the Find query field when `Command-F` opens search.
- [x] 2.4 Apply query edits through the owning pane's `TerminalSurfaceController.updateSearchQuery(_:)` instead of sending printable query text as terminal input.
- [x] 2.5 Display search adapter state for active query, total matches, selected match, searching/no-result state, and inactive state without raw pane IDs or Ghostty action names.

## 3. Find Keyboard And Focus Routing

- [x] 3.1 Keep `Command-F` routed to the focused pane's terminal search owner and open the Find bar.
- [x] 3.2 Route Return and `Command-G` to the next search result for the pane that owns the active Find interaction.
- [x] 3.3 Route Shift-Return and Shift-`Command-G` to the previous search result for the pane that owns the active Find interaction.
- [x] 3.4 Route Escape and the close control to `dismissSearch()` for the owning pane.
- [x] 3.5 Return first responder/focus to the owning terminal pane after Find is dismissed.
- [x] 3.6 Verify split-pane focus changes cannot send query edits or navigation to the wrong pane.

## 4. Tests And Contract Checks

- [x] 4.1 Update `clients/apple/scripts/test-terminal-surface-controller.swift` to cover Find start, query update, navigation, engine callbacks, and dismissal for the owning pane.
- [x] 4.2 Add or update focused tests for `Command-F`, `Command-G`, Shift-`Command-G`, Return, Shift-Return, Escape, and printable query text behavior while Find is active.
- [x] 4.3 Update `clients/apple/scripts/check-shell-contracts.sh` so inspector UI/actions are not required and stale inspector affordances are flagged if reintroduced.
- [x] 4.4 Update UI smoke/manual verification expectations from inspector overview/debug coverage to default-shell-without-inspector and `Command-F` Find bar coverage.
- [x] 4.5 Search Apple client code, scripts, docs, and active OpenSpec changes for stale user-facing inspector references and remove or reframe them where this change owns the surface.

## 5. Verification

- [x] 5.1 Run `clients/apple/scripts/test-terminal-surface-controller.sh`.
- [x] 5.2 Run other focused Apple shell scripts affected by command UI or shell UI changes.
- [x] 5.3 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 5.4 Run `git diff --check`.
- [x] 5.5 Run `openspec validate polish-macos-search-remove-inspector --type change --strict --json`.
- [x] 5.6 Run `openspec validate --all --strict --json`.
- [x] 5.7 Build the macOS app with the documented `AlanNative` Debug command.
- [x] 5.8 Capture or document light-mode screenshots/manual notes for default shell without inspector, split-pane Find ownership, query editing, match navigation, no-result feedback, and Escape dismissal back to terminal.

## 6. Archive Readiness

- [x] 6.1 Sync accepted inspector-removal and Find bar requirements into `openspec/specs/` before archive.
- [x] 6.2 Record implementation verification evidence and any adjusted active-change dependencies in this change before archive.
