## ADDED Requirements

### Requirement: Split zoom is reversible
The macOS shell SHALL let users zoom and unzoom the focused PaneSlot without
mutating the canonical split tree or closing sibling PaneSlots or terminal content.

#### Scenario: Zoom focused pane
- **WHEN** the user zooms a focused split PaneSlot
- **THEN** the focused PaneSlot fills the shell content area and sibling PaneSlots remain alive and restorable

#### Scenario: Unzoom focused pane
- **WHEN** the user exits zoom
- **THEN** the previous split layout, divider ratios, PaneSlot identities, and mounted ContentInstance identities are restored

### Requirement: In-tab pane movement is explicit and reversible
The macOS shell SHALL support explicit PaneSlot movement within the same tab while
preserving PaneSlot identity, mounted ContentInstance identity, and split tree validity.

#### Scenario: Move pane within current tab
- **WHEN** the user moves a PaneSlot to a valid position in the current tab
- **THEN** alan updates the split-tree placement and keeps the moved PaneSlot and mounted ContentInstance identities

#### Scenario: Move target invalid
- **WHEN** the requested in-tab move would create an invalid split tree or move a PaneSlot onto itself
- **THEN** alan rejects the move with a stable reason and leaves the current layout unchanged

### Requirement: Drag/drop movement has a terminal-selection quality gate
Pane drag/drop SHALL only be enabled by default after it uses the same controller
mutation path as explicit moves and preserves terminal text selection behavior.

#### Scenario: Drag starts inside terminal text
- **WHEN** the user drags inside terminal-rendered text
- **THEN** alan treats the drag as terminal selection or terminal input rather than pane movement

#### Scenario: Drag uses a movement affordance
- **WHEN** the user drags a supported PaneSlot movement affordance to another valid target
- **THEN** alan runs the same move mutation used by explicit move commands

### Requirement: Copy paste and search commands route consistently
Copy, paste, and terminal search SHALL resolve the same focused terminal target
across native menus, keyboard shortcuts, command UI, and terminal host surfaces.

#### Scenario: Copy focused terminal selection
- **WHEN** the user invokes Copy while focused terminal content owns a selection
- **THEN** the command is delivered to that terminal host rather than to shell debug text

#### Scenario: Paste into focused terminal
- **WHEN** the user invokes Paste while a PaneSlot that mounts terminal content is focused
- **THEN** the command is delivered to that terminal ContentInstance host

#### Scenario: Search focused terminal
- **WHEN** the user invokes terminal search from a native command surface
- **THEN** the search UI is scoped to the focused terminal ContentInstance
