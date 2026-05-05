## ADDED Requirements

### Requirement: Terminal runtimes survive view selection changes
The macOS shell host SHALL keep a tab's terminal process, renderer surface,
runtime metadata, and buffered control state owned by the shell model or a
dedicated runtime registry rather than by the transient SwiftUI/AppKit view that
happens to be visible.

#### Scenario: Switching away from a tab
- **WHEN** a user switches from one tab to another and the first tab is no longer rendered
- **THEN** the first tab's terminal process and runtime record remain alive unless the tab or pane is explicitly closed

#### Scenario: Switching back to a tab
- **WHEN** a user returns to a previously selected tab
- **THEN** the host reattaches the visible view to the existing terminal runtime instead of booting a new shell process

#### Scenario: Closing a tab
- **WHEN** a tab is explicitly closed
- **THEN** all terminal runtimes owned by that tab are torn down exactly once and their final state is reflected in shell state

### Requirement: Pane text delivery is truthful
The macOS shell host SHALL only acknowledge `pane.send_text` as applied when the
target pane runtime accepts the text or queues it in a durable pane-specific
delivery buffer that will be flushed when the runtime is attached.

#### Scenario: Visible pane accepts text
- **WHEN** `pane.send_text` targets a visible pane with a ready terminal runtime
- **THEN** the response reports `applied: true` and includes the accepted byte count

#### Scenario: Background pane accepts text
- **WHEN** `pane.send_text` targets a background pane with an existing terminal runtime
- **THEN** the text is delivered to that runtime without requiring the tab to become visible

#### Scenario: Target pane cannot accept text
- **WHEN** `pane.send_text` targets a missing, closed, or not-yet-bootable pane
- **THEN** the response reports `applied: false` with a specific error code and does not claim accepted bytes

### Requirement: Focus and metadata follow runtime identity
The macOS shell host SHALL associate focus, cwd, title, process status,
attention, renderer phase, and last-command metadata with stable pane IDs.

#### Scenario: Runtime metadata arrives for a background pane
- **WHEN** a background pane reports cwd, title, process, or attention changes
- **THEN** the shell state for that pane updates without changing the user's selected tab

#### Scenario: Visible focus changes
- **WHEN** the user focuses a visible pane
- **THEN** shell state updates the focused pane while preserving the runtime records for all other panes

### Requirement: Host fallback state is user-safe
The macOS shell host SHALL make unavailable Ghostty or failed terminal runtime
states explicit and actionable without presenting a fake usable terminal.

#### Scenario: Ghostty is unavailable
- **WHEN** the app launches without a linked or loadable Ghostty runtime
- **THEN** the affected pane reports a non-ready terminal state and the UI provides setup/debug information without accepting terminal input as if it succeeded

#### Scenario: Surface creation fails
- **WHEN** a terminal surface cannot be created for a pane
- **THEN** the pane records the failure reason and control-plane mutations against that pane fail or queue according to the delivery contract
