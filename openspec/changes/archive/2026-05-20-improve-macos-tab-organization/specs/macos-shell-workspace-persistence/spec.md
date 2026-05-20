## ADDED Requirements

### Requirement: Tab Organization Mutations Persist Immediately
The macOS shell SHALL persist Tab reorder, pin/unpin, and Move to Space
mutations to the workspace manifest immediately after the mutation is accepted.

#### Scenario: Tab is reordered
- **WHEN** the user reorders a Tab inside a Space section
- **THEN** alan writes the new per-Space Tab order to the workspace manifest

#### Scenario: Tab is pinned by drag
- **WHEN** the user drags an Unpinned Tab into the Pinned section
- **THEN** alan writes the pinned state and the current pin snapshot to the
  workspace manifest

#### Scenario: Tab is unpinned by drag
- **WHEN** the user drags a Pinned Tab into the Unpinned section
- **THEN** alan writes the unpinned state and updated section order to the
  workspace manifest

#### Scenario: Tab moves to another Space
- **WHEN** a Tab is moved to a different Space
- **THEN** alan writes the source Space order, target Space order, Tab Space
  ownership, pin state, and selected Space/Tab outcome to the manifest

### Requirement: Organization Preserves Runtime Identity
The macOS shell SHALL preserve Tab, pane, split tree, and terminal runtime
identity across reorder, pin/unpin, and Move to Space mutations.

#### Scenario: Tab is reordered
- **WHEN** a Tab changes order inside its Space
- **THEN** its Tab ID, pane IDs, split tree, terminal runtime handles, scrollback,
  metadata, and queued delivery state remain attached to the same Tab

#### Scenario: Tab changes pin state
- **WHEN** a Tab is pinned or unpinned
- **THEN** alan changes organization metadata without restarting terminal
  runtimes or recreating pane identities

#### Scenario: Tab moves across Spaces
- **WHEN** a Tab moves to another Space
- **THEN** the moved Tab keeps its Tab ID, pane IDs, split tree, terminal
  runtime handles, scrollback, metadata, and queued delivery state
