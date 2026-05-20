## Context

The macOS shell already has menu commands for common shell operations, sidebar
context menus for tab pinning, existing SwiftUI keyboard shortcuts, and shell
controller mutations. Those surfaces are close but not authoritative: an action
can exist in one entrypoint without the same label, availability, or target
rules elsewhere.

This change creates the first shell-only registry. It is deliberately narrower
than a whole-app or cross-client command system.

## Goals / Non-Goals

**Goals:**

- Define stable action IDs for shell operations.
- Centralize user-facing label, optional default shortcut, target kind,
  availability check, and execution handler.
- Support menu bar commands, tab/space context menus, and keyboard shortcuts.
- Keep targets explicit: current selection/focused pane for keyboard and menu
  actions, context tab or context space for right-click actions.
- Let unavailable actions be disabled in menus and context menus.
- Add validation that prevents duplicate shortcuts and unregistered shell menu
  actions.

**Non-Goals:**

- No app-wide action registry for console, settings, onboarding, or daemon UI.
- No daemon/TUI protocol-level action table.
- No user-customizable shortcuts.
- No expansion of `Go to or Command...`; this registry can be a future input,
  but this change does not add typed command behavior or command result
  filtering.

## Decisions

### 1. Keep the registry shell-only

The first registry should only cover macOS shell actions. It may include Space,
Tab, Pane, Find, terminal-search, and shell navigation operations, but it should
not try to model global app commands or daemon protocol actions.

This keeps the registry close to the existing shell controller and avoids
turning this interaction pass into a broader architecture rewrite.

### 2. Use stable action IDs

Action IDs should be stable strings such as `shell.tab.pin`,
`shell.tab.move_to_space`, `shell.space.select_next`, or
`shell.pane.split_right`. Labels can change, but IDs are the durable contract
used by tests, menus, shortcuts, and future command surfaces.

### 3. Make target resolution explicit

Keyboard shortcuts and menu bar actions target the current selected shell
context by default: selected Space, selected Tab, and focused pane. Context menu
actions target the object that opened the menu without first changing
selection. Actions that require an additional target, such as `Move Tab to
Space...`, resolve that target through a menu/submenu or picker owned by the
surface.

### 4. Keep Command UI out of this change

`Go to or Command...` remains a separate surface. This registry should not
register new tab/space operations into Command UI, add typed command parsing, or
define Command UI filtering. A future change can connect Command UI once its
product shape is settled.

### 5. Prefer disabled menu items over hidden menu items

Menus and context menus should keep unavailable actions visible but disabled
when that preserves discoverability. Keyboard shortcuts for unavailable actions
should be side-effect free. Tests should cover that availability and handler
execution are derived from the same registry entry.
