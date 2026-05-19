## Why

Alan's macOS shell has several keyboard shortcuts today, but the shortcut
surface is distributed across view code and commands. As Tab and Space
organization grows, shortcuts need one coherent contract: preserve existing Mac
behavior, expose high-frequency navigation and organization actions, and avoid a
customization surface until the product direction is clear.

## What Changes

- Add a macOS shell keybinding system backed by the shell action registry.
- Treat registry action descriptors as the source of truth for default shortcut
  metadata and menu hints.
- Preserve existing shortcuts and fill gaps only for high-frequency shell
  actions.
- Add default shortcut coverage for Tab navigation, current Tab structural
  actions, pane focus/split actions, Find, and Space navigation.
- Keep Space management shortcuts limited to navigation in the first version:
  next/previous Space and direct Space number selection.
- Keep create/rename/delete Space as actions and menu items without default
  shortcuts in the first version.
- Do not add user-custom shortcut configuration, preference UI, or Command UI
  integration in this change.

## Capabilities

### New Capabilities

- `macos-shell-keybinding-system`: Defines registry-backed default keybindings,
  fixed first-version coverage, target semantics, conflict detection, and input
  precedence.

### Modified Capabilities

- `macos-shell-workspace-interactions`: Routes keyboard-driven shell commands
  through the action registry and defines Space shortcut coverage.
- `macos-shell-build-test-contract`: Adds validation requirements for shortcut
  conflicts, preservation of existing shortcuts, target semantics, and input
  precedence.

## Impact

- Depends on `add-macos-shell-action-registry` for action descriptors,
  availability, and handler dispatch.
- Tab pin/move shortcut coverage depends on
  `improve-macos-tab-organization`.
- Apple shell commands, menu declarations, focused view shortcut handling, and
  tests.
