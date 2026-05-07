## ADDED Requirements

### Requirement: Split zoom is reversible
The macOS shell SHALL let users zoom and unzoom the focused pane without
mutating the canonical split tree or closing sibling panes.

#### Scenario: Zoom focused pane
- **WHEN** the user zooms a focused split pane
- **THEN** the focused pane fills the terminal content area and sibling panes remain alive and restorable

#### Scenario: Unzoom focused pane
- **WHEN** the user exits zoom
- **THEN** the previous split layout, divider ratios, and pane runtime identities are restored

### Requirement: In-tab pane movement is explicit and reversible
The macOS shell SHALL support explicit pane movement within the same tab while
preserving pane identity and keeping the split tree valid.

#### Scenario: Move pane within current tab
- **WHEN** the user moves a pane to a valid position in the current tab
- **THEN** Alan updates the split-tree placement and keeps the moved pane's runtime identity

#### Scenario: Move target invalid
- **WHEN** the requested in-tab move would create an invalid split tree or move a pane onto itself
- **THEN** Alan rejects the move with a stable reason and leaves the current layout unchanged

### Requirement: Drag/drop movement has a terminal-selection quality gate
Pane drag/drop SHALL only be enabled by default after it uses the same controller
mutation path as explicit moves and preserves terminal text selection behavior.

#### Scenario: Drag starts inside terminal text
- **WHEN** the user drags inside terminal-rendered text
- **THEN** Alan treats the drag as terminal selection or terminal input rather than pane movement

#### Scenario: Drag uses a movement affordance
- **WHEN** the user drags a supported pane movement affordance to another valid target
- **THEN** Alan runs the same move mutation used by explicit move commands

### Requirement: Copy paste and search commands route consistently
Copy, paste, and terminal search SHALL resolve the same focused terminal target
across native menus, keyboard shortcuts, command UI, and terminal host surfaces.

#### Scenario: Copy focused terminal selection
- **WHEN** the user invokes Copy while a terminal pane owns a selection
- **THEN** the command is delivered to that terminal host rather than to shell debug text

#### Scenario: Paste into focused terminal
- **WHEN** the user invokes Paste while a terminal pane is focused
- **THEN** the command is delivered to that pane's terminal host

#### Scenario: Search focused terminal
- **WHEN** the user invokes terminal search from a native command surface
- **THEN** the search UI is scoped to the focused terminal pane
