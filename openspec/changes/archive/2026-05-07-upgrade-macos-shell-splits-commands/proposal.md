## Why

Alan already models spaces, tabs, panes, and pane trees, but the macOS shell
needed the first terminal-grade split and command layer. This change focuses on
the behavior that is now merged in #355: durable split ratios, native split
dividers, directional split commands, spatial focus, close/equalize commands,
pane lift/cross-tab move, and a restrained command UI.

## What Changes

- Upgrade pane-tree layout from simple equal `HStack`/`VStack` recursion to a
  ratio-based split layout with divider resizing, equalization, and stable
  structural identity.
- Add keyboard and command routing for new terminal tab, new Alan tab, split
  directions, close tab/pane, move focus spatially, equalize splits, and command
  UI actions.
- Move common shell workspace commands into native macOS menu/command surfaces
  instead of relying only on hidden SwiftUI shortcut buttons and sidebar menus.
- Add pane lift and cross-tab pane move flows that preserve pane runtime
  identity and shell state.
- Preserve the product direction: spaces and tabs remain in the Arc-like
  material sidebar; terminal content remains dominant; debug details remain
  inspector-only.
- Keep adjacent split panes visually continuous, with rounded outer terminal
  corners, no per-pane rounded cards, no fixed gaps, no bottom pane tab strip,
  and preference-backed inactive-pane dimming.

## Deferred

The broader exploration included split zoom/unzoom, drag/drop movement,
arbitrary in-tab pane movement, control-plane resize/equalize/zoom commands, and
complete copy/paste/search command ownership. Those are intentionally excluded
from this archived change and should be handled by future focused changes.

## Capabilities

### New Capabilities
- `macos-shell-workspace-interactions`: Defines native split, focus, command,
  menu, toolbar, and pane lift/move behavior for Alan's macOS terminal workspace.

### Modified Capabilities
- `macos-shell-ui-ux-conformance`: Split-pane layout, toolbar, menu, and command
  UI requirements become concrete interaction requirements, not just visual
  conformance.
- `macos-shell-terminal-lifecycle`: Split resize, equalize, focus, pane lift or
  move, and close operations must preserve or tear down stable terminal runtime
  identities correctly.
- `macos-shell-control-plane-reliability`: Control-plane pane mutations must
  report authoritative results for split, close, lift, move, and focus actions.

## Impact

- Apple client UI/model: `ShellModel.swift`, `ShellHostController.swift`,
  `TerminalPaneView.swift`, `MacShellRootView.swift`, `TerminalRuntimeRegistry.swift`,
  and new split/command support files.
- Native app behavior: menu bar and keyboard shortcuts become first-class Mac
  surfaces for terminal workspace actions.
- Control plane: existing pane mutation commands may gain stronger result
  semantics and new commands for resize/zoom/spatial focus if needed.
- Visual QA: requires screenshots or running-app checks for single pane,
  split-pane, inspector-off, command UI, and native menu/keyboard flows.
