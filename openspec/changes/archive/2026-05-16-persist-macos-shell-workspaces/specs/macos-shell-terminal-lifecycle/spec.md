## MODIFIED Requirements

### Requirement: Terminal runtimes survive view selection changes
The macOS shell host SHALL keep a tab's terminal process, renderer surface,
runtime metadata, and buffered control state owned by the shell model or a
dedicated runtime registry rather than by the transient SwiftUI/AppKit view that
happens to be visible. Runtime continuity applies while the Tab remains part of
the current shell state; explicit close operations and workspace lifecycle
retirement of inactive unpinned Tabs SHALL finalize the affected terminal
runtimes through the runtime service boundary.

#### Scenario: Switching away from a tab
- **WHEN** a user switches from one tab to another and the first tab is no longer rendered
- **THEN** the first tab's terminal process and runtime record remain alive unless the tab or pane is explicitly closed or the Tab is later retired by the workspace lifecycle contract

#### Scenario: Switching back to a tab
- **WHEN** a user returns to a previously selected tab
- **THEN** the host reattaches the visible view to the existing terminal runtime instead of booting a new shell process

#### Scenario: Closing a tab
- **WHEN** a tab is explicitly closed
- **THEN** all terminal runtimes owned by that tab are torn down exactly once and their final state is reflected in shell state

#### Scenario: Retiring an inactive unpinned Tab
- **WHEN** workspace lifecycle pruning retires an inactive unpinned Tab
- **THEN** terminal runtimes owned by that Tab are finalized through the same runtime service ownership boundary used by explicit close operations

#### Scenario: Restoring a Tab after app restart
- **WHEN** alan restores a Pinned Tab or retained Unpinned Tab from the workspace manifest after app restart
- **THEN** alan creates new terminal runtimes from the restore snapshot instead of claiming continuity with processes from the prior app instance
