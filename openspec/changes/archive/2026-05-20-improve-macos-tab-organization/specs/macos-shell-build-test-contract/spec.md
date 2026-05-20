## ADDED Requirements

### Requirement: Tab Organization Is Verified
The Apple client SHALL include focused verification for Tab reorder, pin/unpin,
Move to Space, manifest persistence, context targeting, and runtime identity
preservation.

#### Scenario: Drag threshold is verified
- **WHEN** Tab row drag behavior changes
- **THEN** focused tests or script checks verify short clicks still select Tabs
  and drag begins only after the movement threshold

#### Scenario: Cross-section drag is verified
- **WHEN** drag pin/unpin behavior changes
- **THEN** verification covers Unpinned-to-Pinned snapshot creation,
  Pinned-to-Unpinned behavior, and realtime insertion preview

#### Scenario: Context target is verified
- **WHEN** Tab context menu actions change
- **THEN** verification proves actions target the clicked Tab without selecting
  it first

#### Scenario: Move to Space focus is verified
- **WHEN** Move Tab to Space behavior changes
- **THEN** verification covers current Tab follow behavior and non-current Tab
  no-follow behavior

#### Scenario: Runtime identity is verified
- **WHEN** Tab organization mutations are implemented
- **THEN** verification proves Tab IDs, pane IDs, split trees, runtime handles,
  and queued delivery state remain stable across reorder, pin/unpin, and Move
  to Space
