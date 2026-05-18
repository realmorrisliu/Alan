## Why

Pane title bars currently read like a separate overlay above the terminal, and
their selected state can make the title effectively disappear in normal use.
This weakens split-pane scanning at exactly the moment the title bar needs to
identify the focused terminal clearly.

## What Changes

- Make pane title bars visually immersive by matching the terminal surface
  background instead of adding a selected/unselected chrome wash above it.
- Increase title and accessory foreground contrast so focused and unfocused
  panes remain readable in light mode.
- Keep the pane title as the primary text and never degrade it to an icon-only
  representation.
- Lay out title-bar content from left to right as fit-content items:
  title, activity/status, cwd or worktree, branch, process or alan state, and
  close.
- Add deterministic narrow-width fallback: lower-priority accessories degrade
  from text plus icon to icon-only, then hide before the title text or close
  affordance disappear.
- Preserve existing pane-scoped close routing, title metadata projection, split
  geometry, focus ownership, and terminal input ownership.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: Refines the pane title-bar contract for
  readability, terminal-surface integration, left-to-right fit-content layout,
  and responsive accessory degradation.
- `macos-shell-build-test-contract`: Extends pane title-bar verification to
  guard against unreadable selected titles, overlay-style title-bar chrome, and
  fixed-width accessory regressions.

## Impact

- Apple client UI: `clients/apple/alan-macos/TerminalPaneView.swift`, especially
  `ShellPaneTitleBarView` and its accessory views.
- Apple shell model/projection: `clients/apple/alan-macos/ShellModel.swift`
  if accessory priority or projection needs to become explicit for layout.
- Apple shell contract checks and focused tests:
  `clients/apple/scripts/check-shell-contracts.sh`,
  `clients/apple/scripts/test-shell-runtime-metadata.swift`, and visual/manual
  verification notes for light-mode single-pane and split-pane title bars.
