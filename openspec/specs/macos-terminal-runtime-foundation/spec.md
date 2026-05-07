# macos-terminal-runtime-foundation Specification

## Purpose
TBD - created by archiving change promote-macos-ghostty-runtime. Update Purpose after archive.
## Requirements
### Requirement: Ghostty initialization is process scoped
The macOS app SHALL initialize libghostty, Ghostty resources, terminfo, logging,
and global terminal configuration through a single process-scoped bootstrap
before any pane surface is created.

#### Scenario: First terminal window opens
- **WHEN** the first shell window requests a terminal runtime
- **THEN** the process bootstrap initializes Ghostty exactly once and returns a ready bootstrap state to the window runtime service

#### Scenario: Additional terminal window opens
- **WHEN** another shell window requests a terminal runtime after bootstrap has completed
- **THEN** the app reuses the existing process bootstrap instead of repeating libghostty initialization

#### Scenario: Bootstrap fails
- **WHEN** Ghostty resources, terminfo, or dynamic libraries cannot be prepared
- **THEN** the bootstrap records a stable failure state and pane creation reports non-ready runtime status without pretending terminal input succeeded

### Requirement: Runtime services are window scoped
Each macOS shell window SHALL own a terminal runtime service that maps stable
Alan pane IDs to terminal surface handles for that window only.

#### Scenario: Pane lookup in one window
- **WHEN** a control-plane command targets a pane ID in a shell window
- **THEN** the command resolves that pane through the terminal runtime service for the same window

#### Scenario: Pane ID collision across windows
- **WHEN** two windows contain panes with identical local IDs or restored IDs
- **THEN** each window runtime service resolves and mutates only its own pane handle

### Requirement: Pane surfaces have stable handles
A terminal pane SHALL be represented by a stable surface handle that outlives
SwiftUI/AppKit view creation and stores lifecycle phase, text delivery state,
metadata, and teardown state.

#### Scenario: View is recreated
- **WHEN** SwiftUI recreates the terminal host view for an existing pane
- **THEN** the new view attaches to the existing surface handle without starting a new shell process

#### Scenario: Background pane receives text
- **WHEN** a background pane has a live surface handle and receives `pane.send_text`
- **THEN** the runtime service delivers text through that handle without requiring the pane to become visible

#### Scenario: Surface handle is closing
- **WHEN** text delivery targets a pane whose surface handle is closing or closed
- **THEN** the runtime service rejects or queues the command according to the pane's delivery policy and reports that state explicitly

### Requirement: Host views are runtime adapters
`AlanTerminalHostNSView` and related SwiftUI wrappers SHALL act as adapters for
focus, display metrics, occlusion, frame changes, and input forwarding, and MUST
NOT own Ghostty app lifetime or pane runtime truth.

#### Scenario: Host view attaches
- **WHEN** a terminal host view is mounted for a pane
- **THEN** it receives an existing surface handle from the runtime service and reports view metrics to that handle

#### Scenario: Host view detaches
- **WHEN** a terminal host view is removed because selection or layout changed
- **THEN** the pane surface handle remains alive unless the pane, tab, window, or app is closing

### Requirement: Runtime metadata is projected by pane identity
The runtime service SHALL project terminal title, cwd, process status,
attention, renderer phase, readiness, and delivery diagnostics into Alan shell
state using stable pane IDs.

#### Scenario: Metadata event from background pane
- **WHEN** a background pane emits a title, cwd, process, attention, or renderer-state event
- **THEN** shell state updates the matching pane record without changing user focus

#### Scenario: Metadata event after pane close
- **WHEN** a terminal callback arrives after its pane has reached closed state
- **THEN** the runtime service ignores or records it as late diagnostics without resurrecting the pane

### Requirement: Runtime cleanup is deterministic
Pane, tab, window, and app close paths SHALL transition terminal surface handles
through closing and closed states and release Ghostty resources exactly once.

#### Scenario: Closing one pane
- **WHEN** a user closes a split pane
- **THEN** the runtime service tears down that pane surface exactly once and preserves other pane handles in the same tab

#### Scenario: Closing a tab
- **WHEN** a user closes a tab with multiple panes
- **THEN** the runtime service tears down every pane surface in that tab exactly once and publishes final closed state

#### Scenario: App terminates
- **WHEN** the app terminates while terminal panes are live
- **THEN** the runtime service performs best-effort teardown and records closed or interrupted terminal state for persisted diagnostics

