## Why

Split panes currently show terminal content with only subtle focus treatment and
optional global metadata, so users cannot quickly identify or close a specific
visible pane from the pane itself. A narrow per-pane title bar gives every pane a
stable native affordance while keeping the terminal canvas dominant.

## What Changes

- Add a narrow title bar to each visible terminal pane.
- Show the pane's current terminal display title by consuming the existing
  pane metadata/title-normalization path; do not introduce a second terminal
  title collection contract.
- Add a close button in the pane title bar that closes that pane through the
  shared shell workspace command path.
- Keep the title bar lightweight: no raw pane IDs, runtime phases, debug labels,
  dense toolbar controls, cards, shadows, or redundant metadata.
- Preserve terminal hit-testing: the title bar owns only its explicit controls,
  while terminal text selection, mouse input, and scroll remain inside the
  terminal host surface.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: Adds the default per-pane title bar contract,
  including title content, close affordance, visual weight, and terminal-first
  hit-testing boundaries.
- `macos-shell-build-test-contract`: Adds focused verification for pane title
  display, close routing, and terminal input preservation.

## Impact

- Apple client UI: `clients/apple/AlanNative/TerminalPaneView.swift` and
  adjacent view helpers for the pane leaf wrapper.
- Apple client model/controller: existing `ShellPane`,
  `ShellViewportSnapshot.title`, title normalization helpers,
  `ShellWorkspaceCommand.closePane`, and `ShellHostController` close paths.
- Apple client tests/scripts: focused shell model, runtime metadata, shell
  contract checks, and manual screenshot/interaction verification.
