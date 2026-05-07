## ADDED Requirements

### Requirement: Split workspace mutations preserve live runtimes
The macOS shell host SHALL preserve pane runtime identity across split resize,
equalize, focus, pane lift, and cross-tab pane move operations unless the
operation explicitly closes the pane or tab.

#### Scenario: Resize split
- **WHEN** the user resizes a split divider
- **THEN** all panes in the tab keep their existing runtime handles and metadata

#### Scenario: Equalize splits
- **WHEN** the user equalizes splits in a tab
- **THEN** all panes in the tab keep their existing runtime handles and metadata

#### Scenario: Lift pane
- **WHEN** the user lifts a pane to its own tab
- **THEN** the pane keeps its runtime handle, scrollback, title, cwd, and pending delivery state

#### Scenario: Move pane to another tab
- **WHEN** the user moves a pane to another tab within the same window
- **THEN** the pane keeps its runtime handle, scrollback, title, cwd, and pending delivery state

### Requirement: Split close operations define runtime finalization
The macOS shell host SHALL define explicit terminal runtime finalization
semantics for close pane, close tab, close window, pane lift, and pane move
operations that empty containers.

#### Scenario: Close focused pane
- **WHEN** the user invokes close pane
- **THEN** Alan finalizes exactly that pane runtime and repairs the split tree around the removed leaf

#### Scenario: Close tab after moving last pane
- **WHEN** a move operation leaves the source tab empty and Alan closes that tab
- **THEN** Alan does not finalize the moved pane runtime as part of source tab cleanup
