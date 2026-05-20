## ADDED Requirements

### Requirement: Tabs Are Organized Into Per-Space Pinned And Unpinned Sections
The macOS shell SHALL organize Tabs inside each Space into a Pinned section and
an Unpinned section. Pinning is scoped to the owning Space and SHALL NOT create
a global pinned Tab shelf.

#### Scenario: Space contains pinned and unpinned Tabs
- **WHEN** a Space has both Pinned and Unpinned Tabs
- **THEN** alan presents those Tabs as two ordered sections within that Space

#### Scenario: Pinned Tab moves to another Space
- **WHEN** a Pinned Tab is moved to a different Space
- **THEN** alan keeps the Tab pinned and inserts it at the end of the target
  Space's Pinned section

#### Scenario: Unpinned Tab moves to another Space
- **WHEN** an Unpinned Tab is moved to a different Space
- **THEN** alan keeps the Tab unpinned and inserts it at the end of the target
  Space's Unpinned section

### Requirement: Tab Rows Support Direct Reorder And Pin State Changes
The macOS shell SHALL allow users to drag Tab rows to reorder Tabs within a
section and to change pin state by dragging across the Pinned and Unpinned
section boundary.

#### Scenario: Short click selects Tab
- **WHEN** the user clicks a Tab row without crossing the drag threshold
- **THEN** alan selects that Tab normally

#### Scenario: Drag reorders inside a section
- **WHEN** the user drags a Tab row to another position inside the same section
- **THEN** alan reorders the Tab within that section without changing its pin
  state

#### Scenario: Drag pins Tab
- **WHEN** the user drags an Unpinned Tab into the Pinned section and drops it
- **THEN** alan pins the Tab using its current restorable state and inserts it
  at the previewed Pinned position

#### Scenario: Drag unpins Tab
- **WHEN** the user drags a Pinned Tab into the Unpinned section and drops it
- **THEN** alan unpins the Tab and inserts it at the previewed Unpinned position

#### Scenario: Drag shows insertion preview
- **WHEN** the user drags a Tab row within or across sections
- **THEN** alan shows a realtime insertion preview before mutating durable Tab
  order

### Requirement: Move Tab To Space Is Explicit In The First Version
The macOS shell SHALL support Move Tab to Space through menu and Tab context
actions. The first version SHALL NOT require dragging a Tab to the Space
switcher to move it across Spaces.

#### Scenario: Move selected Tab follows target
- **WHEN** the user moves the current selected Tab to another Space
- **THEN** alan selects the target Space and keeps the moved Tab selected

#### Scenario: Move non-selected Tab stays put
- **WHEN** the user moves a non-selected Tab to another Space through its
  context menu
- **THEN** alan keeps the current Space, selected Tab, and focused pane
  unchanged

#### Scenario: Move target missing
- **WHEN** the user or a control path requests moving a Tab to a missing Space
- **THEN** alan rejects the move with a stable reason and leaves Tab order,
  Space ownership, and focus unchanged

### Requirement: Tab Context Menus Use Context Targets
Tab context menu actions SHALL target the Tab that opened the menu without first
changing selected Space, selected Tab, or focused pane.

#### Scenario: Context pin targets clicked Tab
- **WHEN** the user opens a context menu on a non-selected Tab and chooses Pin
- **THEN** alan pins the clicked Tab without selecting it first

#### Scenario: Context move targets clicked Tab
- **WHEN** the user opens a context menu on a non-selected Tab and chooses Move
  Tab to Space
- **THEN** alan moves the clicked Tab and keeps the current selection unchanged
  unless the clicked Tab was already selected
