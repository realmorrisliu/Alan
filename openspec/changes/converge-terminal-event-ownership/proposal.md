## Why

The macOS terminal pane currently splits ownership of the same mouse event
across SwiftUI selection gestures, AppKit terminal input handling, Ghostty canvas
rendering, and NSWindow background dragging. That makes small chrome changes
fragile: a pane click can accidentally become selection-only, terminal input,
or window dragging depending on which layer wins hit-testing.

## What Changes

- Move terminal-pane activation from the SwiftUI `.onTapGesture` wrapper into
  the AppKit terminal host that already owns terminal mouse, keyboard, IME,
  selection, scroll, paste, and Ghostty forwarding.
- Keep SwiftUI responsible for layout, shell selection state, and explicit
  controls such as pane selector buttons.
- Make Ghostty and fallback canvas subviews drawing surfaces rather than event
  owners, so terminal mouse events are handled consistently by the host view.
- Add a narrow weak activation delegate from the AppKit host back to the shell
  controller instead of storing long-lived strong SwiftUI closures in
  registry-owned host views.
- Preserve the existing background-window-dragging contract: non-interactive
  shell background remains draggable, while terminal pane clicks never drag the
  window.
- Add contract checks and focused verification notes so future changes cannot
  reintroduce SwiftUI tap ownership over the terminal pane.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `macos-shell-terminal-lifecycle`: terminal focus and pane selection must follow
  stable pane identity while terminal-area events are owned by the terminal host
  rather than transient SwiftUI gesture wrappers.
- `macos-shell-ui-ux-conformance`: terminal content remains the center of
  gravity, with terminal clicks behaving as terminal activation/input and
  background dragging limited to non-terminal, non-interactive chrome.
- `macos-shell-build-test-contract`: shell contract checks must cover the
  terminal event boundary so regressions are caught without relying only on
  manual visual testing.

## Impact

- Apple client files: `TerminalPaneView.swift`, `TerminalHostView.swift`,
  `TerminalRuntimeRegistry.swift`, `ShellHostController.swift`,
  `GhosttyLiveHost.swift`, and `check-shell-contracts.sh`.
- Runtime behavior: terminal pane clicks select/focus the pane and continue to
  forward mouse input to Ghostty through a single AppKit path.
- UI behavior: pane selector buttons and other explicit SwiftUI controls keep
  their existing action ownership; only terminal-canvas selection is moved.
- No external API, daemon protocol, persistence format, or Ghostty dependency
  changes are intended.
