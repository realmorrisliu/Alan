## ADDED Requirements

### Requirement: Zoom affordances stay compact
Split zoom UI SHALL make the zoomed state and escape path clear without adding a
persistent pane-management toolbar.

#### Scenario: Pane zoomed
- **WHEN** a pane is zoomed
- **THEN** the UI provides a compact way to unzoom while keeping the terminal content dominant

#### Scenario: Toolbar remains restrained
- **WHEN** zoom is available for a split pane
- **THEN** the default toolbar does not add a dense split-control strip

### Requirement: Movement affordances protect terminal interaction
Pane movement UI SHALL avoid ambiguous gestures inside terminal content and keep
terminal text selection reliable.

#### Scenario: Movement command shown
- **WHEN** the command UI or context menu offers pane movement
- **THEN** the label describes the destination or action in user-facing terms without raw pane IDs

#### Scenario: Drag affordance visible
- **WHEN** drag/drop pane movement is enabled
- **THEN** the movement affordance is visually distinct from terminal text selection regions

### Requirement: Copy paste and search surfaces are native and pane scoped
Copy, paste, and search command UI SHALL feel native, target the focused pane,
and avoid displacing the sidebar, toolbar, or split layout.

#### Scenario: Search opens
- **WHEN** the user invokes terminal search
- **THEN** the search UI appears as a compact pane-scoped terminal tool

#### Scenario: Copy paste available
- **WHEN** the focused terminal pane can copy or paste
- **THEN** native menu and keyboard commands target that pane without exposing debug routing details
