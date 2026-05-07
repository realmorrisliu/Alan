## ADDED Requirements

### Requirement: Pane workspace mutation commands report authoritative results
The macOS shell control plane SHALL return authoritative results for pane split,
pane close, pane lift, cross-tab pane move, and direct pane focus commands after
the mutation is accepted or rejected.

#### Scenario: Split command succeeds
- **WHEN** a control client requests a valid directional pane split
- **THEN** the response reports `applied: true` and includes the resulting focused pane ID

#### Scenario: Split command invalid
- **WHEN** a control client requests a pane split against a missing pane or without a direction
- **THEN** the response reports `applied: false` with a stable error code and leaves shell state unchanged

#### Scenario: Move command succeeds
- **WHEN** a control client moves a pane to a valid destination tab in the same window
- **THEN** the response reports `applied: true` and the resulting focused pane ID while preserving the pane runtime identity

#### Scenario: Close command succeeds
- **WHEN** a control client closes a pane
- **THEN** the response reflects both shell model removal and the remaining focused pane

### Requirement: Pane focus commands are observable
Direct pane focus commands SHALL report whether focus changed to the requested
pane or why the target could not be focused.

#### Scenario: Direct focus changes
- **WHEN** a control client requests focus for an existing pane
- **THEN** the response reports `applied: true` and the requested pane ID

#### Scenario: Direct focus target missing
- **WHEN** a control client requests focus for a missing pane
- **THEN** the response reports `applied: false` with a stable missing-pane error and preserves existing focus

### Requirement: Workspace mutation events are observable
Workspace mutations SHALL emit shell events with enough detail for agents to
observe pane creation, closure, movement, metadata changes, attention changes,
and focus changes.

#### Scenario: Split creates a pane
- **WHEN** the user or a control client creates a split
- **THEN** the shell event stream records the created pane and its tab

#### Scenario: Move changes a pane tab
- **WHEN** the user or a control client moves a pane to another tab
- **THEN** the shell event stream records the previous and current tab or space identity for the moved pane

#### Scenario: Focus changes
- **WHEN** the user or a control client changes focused pane
- **THEN** the shell event stream records the previous and current focused pane IDs
