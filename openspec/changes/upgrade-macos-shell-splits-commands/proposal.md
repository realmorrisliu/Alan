## Why

Alan already models spaces, tabs, panes, and pane trees, but the macOS shell does
not yet behave like a mature native terminal workspace. To approach Ghostty's
quality while preserving Alan's Arc-like sidebar direction, Alan needs resizable
splits, spatial focus, pane movement, native command routing, and restrained
window/menu behavior.

## What Changes

- Upgrade pane-tree layout from simple equal `HStack`/`VStack` recursion to a
  ratio-based split layout with divider resizing, equalization, zoom, and stable
  structural identity.
- Add keyboard and command routing for new tab, new Alan tab, split directions,
  close tab/pane, move focus spatially, resize split, zoom split, copy, paste,
  search, and command palette actions.
- Move durable app/window commands into native macOS menu/command surfaces
  instead of relying only on hidden SwiftUI shortcut buttons and sidebar menus.
- Add pane drag/drop or explicit move flows that preserve pane runtime identity
  and shell state.
- Preserve the product direction: spaces and tabs remain in the Arc-like
  material sidebar; terminal content remains dominant; debug details remain
  inspector-only.
- Clarify multi-window behavior and avoid app-wide tabbing conflicts where
  Alan's custom space/tab model owns organization.

## Capabilities

### New Capabilities
- `macos-shell-workspace-interactions`: Defines native split, focus, command,
  menu, toolbar, and pane movement behavior for Alan's macOS terminal workspace.

### Modified Capabilities
- `macos-shell-ui-ux-conformance`: Split-pane layout, toolbar, menu, and command
  UI requirements become concrete interaction requirements, not just visual
  conformance.
- `macos-shell-terminal-lifecycle`: Pane moves, split zoom, tab moves, and close
  operations must preserve or tear down stable terminal runtime identities
  correctly.
- `macos-shell-control-plane-reliability`: Control-plane pane mutations must
  report authoritative results for resize, move, focus, zoom, and close actions.

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
