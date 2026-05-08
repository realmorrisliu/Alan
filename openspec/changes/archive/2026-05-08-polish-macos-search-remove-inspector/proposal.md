## Why

The macOS shell is moving into a UI polish phase, and the right-side inspector no
longer earns its visual and interaction cost. At the same time, `Command-F`
currently routes to pane search but does not feel like the standard macOS Find
experience users expect in a terminal app.

## What Changes

- **BREAKING**: Remove the inspector as a user-facing macOS shell feature,
  including the sidebar toggle, command-palette action, persisted visibility
  preference, right-side pane, voice commands, and UI/spec references that make
  inspector part of the default product surface.
- Keep diagnostic data available through developer/debug-only mechanisms such as
  existing shell snapshots, logs, scripts, or future explicit debug commands
  rather than a persistent in-app inspector.
- Refine terminal search so `Command-F` opens a native-feeling, pane-scoped Find
  bar with a real focused text field, visible result count, next/previous
  controls, close behavior, and predictable keyboard shortcuts.
- Preserve terminal-first layout: opening or closing Find must not resize the
  sidebar, toolbar, split layout, or terminal canvas in a disruptive way.
- Preserve the existing pane-scoped terminal search engine contract; this is a UI
  polish and interaction pass, not a new search backend.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: Removes the inspector contract and adds the
  polished pane-scoped Find bar UX contract for `Command-F`.
- `macos-shell-terminal-lifecycle`: Tightens terminal search lifecycle behavior
  around focus, query updates, result navigation, dismissal, and pane identity.
- `macos-shell-workspace-interactions`: Removes inspector from command and
  toolbar interaction expectations and clarifies search keyboard routing.
- `macos-shell-build-test-contract`: Updates verification requirements so UI
  polish is checked through inspector-removal assertions and Find bar behavior
  instead of inspector screenshots.

## Impact

- Apple client UI: primarily
  `clients/apple/AlanNative/MacShellRootView.swift`,
  `clients/apple/AlanNative/TerminalPaneView.swift`, and
  `clients/apple/AlanNative/TerminalHostView.swift`.
- Apple client search ownership: existing `AlanTerminalSearchAdapter`,
  `AlanTerminalSearchEngine`, `TerminalSurfaceController`, and Ghostty search
  action routing remain the backend contract.
- Apple client commands and affordances: `AlanMacShellCommands`, sidebar command
  launcher/header actions, command palette actions, speech command vocabulary,
  and persisted app storage keys that mention inspector.
- Tests and scripts:
  `clients/apple/scripts/test-terminal-surface-controller.swift`,
  `clients/apple/scripts/check-shell-contracts.sh`, and visual/manual
  verification notes for the default light-mode shell and `Command-F` flow.
- Active UI polish OpenSpec work such as
  `normalize-macos-shell-corner-radii` and `add-macos-pane-title-bars` must
  treat inspector-specific screenshots, radii targets, and debug-surface wording
  as removed or rerouted to explicit developer/debug-only surfaces so inspector
  removal remains the source of truth.
