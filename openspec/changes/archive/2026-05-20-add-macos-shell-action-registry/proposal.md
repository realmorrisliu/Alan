## Why

Alan's macOS shell already exposes related workspace actions through menu items,
keyboard shortcuts, context menus, command input, and local control paths, but
the action definitions are not yet a single shell-owned contract. Tab movement,
pinning, and the next keybinding pass need a stable shell action registry so
new interaction surfaces do not drift from each other.

## What Changes

- Add a macOS shell-only action registry for Space, Tab, Pane, Find, and shell
  navigation actions.
- Give each shell action a stable `action_id`, user-facing title, target model,
  default shortcut metadata where applicable, availability check, and execution
  handler.
- Route menu bar commands, tab/space context menus, and shell keyboard shortcuts
  through the registry.
- Keep `Go to or Command...` out of scope for this first registry pass; existing
  command input behavior remains unchanged.
- Establish conflict and coverage checks so new shell actions cannot add
  separate shortcut/menu behavior without a registry entry.

## Capabilities

### New Capabilities

- `macos-shell-action-registry`: Owns shell action identity, metadata,
  availability, targeting, shortcut descriptors, and execution routing for
  native macOS shell interactions.

### Modified Capabilities

- `macos-shell-workspace-interactions`: Requires menu, context-menu, and
  keyboard action paths to converge through the shell action registry where
  they share behavior.
- `macos-shell-build-test-contract`: Adds focused validation for registry
  coverage, target resolution, and shortcut conflict detection.

## Impact

- `AlanMacShellCommands.swift`, `ShellSidebarView.swift`, and shell keyboard
  shortcut views.
- `ShellHostController`, `ShellLocalCommandExecutor`, and shell mutation
  helpers that become action handlers or action targets.
- Focused Swift/script tests that verify action availability, handler routing,
  menu/context behavior, and shortcut metadata.
