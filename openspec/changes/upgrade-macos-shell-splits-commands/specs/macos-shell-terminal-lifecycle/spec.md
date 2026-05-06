## ADDED Requirements

### Requirement: Split mutations preserve live runtimes
The macOS shell host SHALL preserve pane runtime identity across split resize,
equalize, zoom, focus, and move operations unless the operation explicitly
closes the pane or tab.

#### Scenario: Resize split
- **WHEN** the user resizes a split divider
- **THEN** all panes in the tab keep their existing runtime handles and metadata

#### Scenario: Zoom split
- **WHEN** the user zooms and unzooms a pane
- **THEN** sibling panes remain alive in the runtime service and reattach when visible

#### Scenario: Move pane
- **WHEN** the user moves a pane to another position or tab within the same window
- **THEN** the pane keeps its runtime handle, scrollback, title, cwd, and pending delivery state

### Requirement: Close operations define runtime finalization
The macOS shell host SHALL define explicit terminal runtime finalization
semantics for close pane, close tab, close window, and move operations that
empty containers.

#### Scenario: Close focused pane
- **WHEN** the user invokes close pane
- **THEN** Alan finalizes exactly that pane runtime and repairs the split tree around the removed leaf

#### Scenario: Close tab after moving last pane
- **WHEN** a move operation leaves the source tab empty and Alan closes that tab
- **THEN** Alan does not finalize the moved pane runtime as part of source tab cleanup
