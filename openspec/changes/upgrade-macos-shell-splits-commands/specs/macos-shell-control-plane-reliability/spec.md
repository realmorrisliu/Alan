## ADDED Requirements

### Requirement: Split control commands report authoritative results
The macOS shell control plane SHALL return authoritative results for split
creation, pane focus, split resize, equalize, zoom, move, and close after the
mutation is accepted or rejected.

#### Scenario: Resize command succeeds
- **WHEN** a control client requests a valid split ratio change
- **THEN** the response reports `applied: true` and includes the resulting ratio and affected pane or split IDs

#### Scenario: Resize command invalid
- **WHEN** a control client requests a split ratio outside accepted bounds or against a missing split
- **THEN** the response reports `applied: false` with a stable error code and leaves shell state unchanged

#### Scenario: Move command succeeds
- **WHEN** a control client moves a pane to a valid destination in the same window
- **THEN** the response reports the pane ID, source location, destination location, and preserved runtime identity

#### Scenario: Close command succeeds
- **WHEN** a control client closes a pane
- **THEN** the response reflects both shell model removal and runtime finalization state

### Requirement: Spatial focus commands are observable
Spatial focus commands SHALL report whether focus changed, the previous pane,
the new pane, and the reason focus did not change when no target exists.

#### Scenario: Spatial focus changes
- **WHEN** a control client requests focus left and an adjacent pane exists
- **THEN** the response reports `applied: true`, previous focused pane, and new focused pane

#### Scenario: Spatial focus has no target
- **WHEN** a control client requests focus up and no adjacent pane exists
- **THEN** the response reports `applied: false` with a no-target error and preserves existing focus

### Requirement: Command outcomes emit shell events
Workspace mutations SHALL emit shell events from menu, keyboard, command UI,
and control-plane paths with enough detail for agents to observe layout and
focus changes.

#### Scenario: Keyboard split command
- **WHEN** the user creates a split with a keyboard shortcut
- **THEN** the shell event stream records the new pane, split direction, selected tab, and focused pane

#### Scenario: Menu close command
- **WHEN** the user closes a pane from the menu bar
- **THEN** the shell event stream records pane removal and runtime finalization outcome
