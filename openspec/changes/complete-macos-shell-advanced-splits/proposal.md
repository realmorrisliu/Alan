## Why

The first split/command pass delivered durable ratios, resize dividers, spatial
focus, native command routing, and pane lift/cross-tab movement. The remaining
advanced terminal workspace work still needs an explicit OpenSpec owner so it
does not disappear inside an archived change.

## What Changes

- Add split zoom and unzoom as a tab-scoped view state that preserves sibling
  pane runtimes and restores the previous split layout.
- Add explicit in-tab pane movement and optional drag/drop movement once it can
  preserve runtime identity without compromising terminal text selection.
- Extend control-plane commands for split resize, equalize, zoom/unzoom, and
  spatial focus with authoritative result semantics.
- Complete command ownership for copy, paste, and terminal search so native
  menu, keyboard, command UI, and terminal surface paths agree on target
  routing.
- Add focused tests and visual review evidence for zoom, in-tab movement,
  drag/drop readiness, control-plane results, and copy/paste/search routing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-workspace-interactions`: Adds split zoom/unzoom, in-tab pane
  movement, drag/drop readiness, and complete command-surface ownership.
- `macos-shell-control-plane-reliability`: Adds authoritative control-plane
  result semantics for resize, equalize, zoom/unzoom, spatial focus, and
  movement commands.
- `macos-shell-terminal-lifecycle`: Clarifies runtime identity preservation for
  zoom/unzoom, in-tab movement, drag/drop movement, and search/copy/paste target
  routing.
- `macos-shell-ui-ux-conformance`: Adds UI requirements for zoom affordances,
  movement affordances, drag/drop quality gates, and search/copy/paste command
  surfaces without toolbar bloat.

## Impact

- Apple client model/controller: `ShellModel.swift`, `ShellHostController.swift`,
  `ShellControlPlane.swift`, and related split mutation tests.
- Apple client UI: `TerminalPaneView.swift`, `MacShellRootView.swift`, native
  `Commands`, menu routing, command UI, and context/drag affordances.
- Runtime ownership: `TerminalRuntimeRegistry.swift`,
  `TerminalSurfaceController.swift`, and terminal host command routing for
  copy/paste/search where needed.
