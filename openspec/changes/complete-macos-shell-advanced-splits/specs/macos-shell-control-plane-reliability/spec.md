## ADDED Requirements

### Requirement: Advanced split control commands report authoritative results
The macOS shell control plane SHALL return authoritative results for split
resize, equalize, zoom, unzoom, and spatial focus commands.

#### Scenario: Resize command succeeds
- **WHEN** a control client requests a valid split ratio change
- **THEN** the response reports `applied: true` and includes the resulting ratio and affected split or pane IDs

#### Scenario: Equalize command succeeds
- **WHEN** a control client requests equalize for a tab with split branches
- **THEN** the response reports `applied: true` and identifies the tab whose split ratios were reset

#### Scenario: Zoom command succeeds
- **WHEN** a control client zooms or unzooms a valid pane
- **THEN** the response reports `applied: true`, the pane ID, and the tab zoom state

#### Scenario: Spatial focus has no target
- **WHEN** a control client requests spatial focus and no adjacent pane exists
- **THEN** the response reports `applied: false` with a stable no-target error and preserves existing focus

### Requirement: Advanced movement commands report source and destination
Pane move commands SHALL report enough source and destination detail for agents
to observe layout changes without inferring them from raw shell snapshots.

#### Scenario: In-tab move succeeds
- **WHEN** a control client moves a pane within a tab
- **THEN** the response reports the pane ID, source tab, destination tab, direction or position, and preserved runtime identity

#### Scenario: Drag-backed move succeeds
- **WHEN** a drag/drop affordance completes through the control-plane movement path
- **THEN** the response and event stream use the same result semantics as explicit movement commands

### Requirement: Advanced command outcomes emit shell events
Advanced workspace mutations SHALL emit shell events for zoom state, split ratio
changes, equalization, spatial focus, and pane movement.

#### Scenario: Split ratio changes
- **WHEN** a split ratio changes through UI or control-plane resize
- **THEN** the shell event stream records the affected split, tab, and resulting ratio

#### Scenario: Zoom state changes
- **WHEN** a pane is zoomed or unzoomed
- **THEN** the shell event stream records the tab, pane, and resulting zoom state
