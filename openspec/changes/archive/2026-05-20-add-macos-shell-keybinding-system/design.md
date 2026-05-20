## Context

Keyboard control is central to a terminal-first macOS shell. The current shell
already has some native shortcuts for actions such as opening Tabs, focusing
panes, splitting panes, and Find. New Tab and Space organization should not add
ad hoc shortcut handling in each view. Instead, shortcuts should flow through
the same action registry used by menus and context menus.

This change deliberately avoids user-custom shortcut editing. The first version
should provide a stable default surface and a conflict-tested foundation.

## Goals / Non-Goals

**Goals:**

- Preserve existing working shortcuts.
- Register default shortcuts from shell action descriptors.
- Provide shortcut coverage for high-frequency Tab, pane, Find, and Space
  navigation actions.
- Use current selected Tab and focused pane as the keyboard action target.
- Display native shortcut hints in menus.
- Detect duplicate or conflicting default shortcuts in tests.
- Define input precedence between Find, terminal input, and shell actions.

**Non-Goals:**

- No user-custom shortcut preferences, config file, or import/export format.
- No Command UI integration.
- No default shortcut for every action.
- No hover-targeted shortcut behavior.
- No default shortcuts for create, rename, or delete Space in the first version.

## Decisions

### 1. Registry descriptors own default shortcuts

Each shortcut-enabled action declares its default shortcut in the shell action
registry. Menus and direct keyboard dispatch read that same descriptor so labels,
availability, target semantics, and shortcut hints stay aligned.

### 2. Preserve before filling gaps

Existing macOS shell shortcuts remain the baseline. The new system fills gaps for
high-frequency operations rather than remapping core habits. If an existing
shortcut conflicts with a proposed new default, the existing shortcut wins unless
the implementation plan explicitly accepts a migration.

### 3. First-version coverage is intentionally narrow

Default shortcuts should cover:

- Tab: new, close, next, previous, move left, move right, pin/unpin current Tab.
- Pane: split, close/focus where already present, equalize where already
  present or newly standardized.
- Find: preserve existing Find shortcut and Find-mode behavior.
- Space: next Space, previous Space, direct numeric Space selection.

Create Space, rename Space, delete Space, and Move Tab to Space remain available
through menus/actions without default shortcuts in the first version.

### 4. Keyboard target is current selection

Keyboard shortcuts operate on the current selected Space, selected Tab, and
focused pane. They do not target a hovered row or a Tab that merely owns an open
context menu.

### 5. Input precedence is explicit

When Find is active, Find-owned shortcuts and text handling take precedence over
shell actions. Terminal-reserved input remains terminal input. Shell action
shortcuts are handled only when they match registered actions and the action is
available for the current shell state.

### 6. Unavailable actions are disabled, not hidden

Shortcut-triggered unavailable actions are no-ops with stable diagnostics where
appropriate. Menus show disabled actions and their shortcut hints, matching the
registry availability state.
