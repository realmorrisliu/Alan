## ADDED Requirements

### Requirement: Terminal lifecycle ownership is service backed
The macOS shell host SHALL route terminal process, renderer surface, runtime
metadata, pending delivery buffer, and teardown ownership through the terminal
runtime service rather than through transient host views.

#### Scenario: Runtime survives SwiftUI reconstruction
- **WHEN** SwiftUI reconstructs the shell content view while a pane remains part of shell state
- **THEN** the terminal runtime service keeps the pane surface alive and the new view attaches to the same runtime identity

#### Scenario: Runtime no longer exists
- **WHEN** shell state references a pane whose terminal runtime has irrecoverably failed or closed
- **THEN** lifecycle metadata reports the non-ready state and the UI/control plane do not treat the pane as a ready terminal

### Requirement: Pane close finalizes runtime identity
The macOS shell host SHALL make pane, tab, and window close operations call the
runtime service finalizer for each affected pane before the pane is removed from
authoritative runtime state.

#### Scenario: Closing a split pane
- **WHEN** a user closes one pane in a split tab
- **THEN** the runtime service finalizes only that pane's surface and the remaining panes keep their runtime identities

#### Scenario: Closing a window
- **WHEN** a shell window closes
- **THEN** every pane runtime owned by that window transitions to closing or closed state before the window control identity is released

### Requirement: Reattachment preserves terminal continuity
Visible terminal views SHALL reattach to existing runtime handles and MUST NOT
restart shell processes, clear scrollback, or reset pane metadata solely because
selection, split layout, or window visibility changed.

#### Scenario: Tab selection changes repeatedly
- **WHEN** a user switches between terminal tabs several times
- **THEN** each tab keeps its existing terminal process, scrollback, title, cwd, and runtime phase

#### Scenario: Split layout changes
- **WHEN** a pane is moved, resized, or temporarily hidden by split zoom
- **THEN** its runtime handle remains continuous and reattaches when visible again
