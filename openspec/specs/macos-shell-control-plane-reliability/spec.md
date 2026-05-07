# macos-shell-control-plane-reliability Specification

## Purpose
Define reliability requirements for the macOS shell control plane, including
window-scoped identities, bounded IPC, authoritative mutation results, and
observable persistence or event failures.
## Requirements
### Requirement: Windows have isolated shell identities
The Alan macOS shell control plane SHALL have one active primary shell window
identity per running native app instance. Duplicate window or duplicate process
launch paths MUST NOT create competing shell control directories, socket paths,
event streams, persisted shell state files, or terminal runtime registries.

#### Scenario: Opening a second window
- **WHEN** the user invokes a second-window path such as New Window or `Command-N`
- **THEN** Alan focuses or reopens the existing primary shell window and does not create another `window_id`, socket path, control directory, or persisted state file

#### Scenario: Reading window state
- **WHEN** an agent queries the primary window's shell state
- **THEN** the response contains only spaces, tabs, panes, events, and focus state for the singleton primary shell window

#### Scenario: Forced duplicate process
- **WHEN** a forced second app process starts while the primary app instance owns the shell control plane
- **THEN** the second process exits without publishing a second socket, state file, event stream, or terminal runtime registry

#### Scenario: Reopening primary window
- **WHEN** the existing app process reopens the primary shell window after it was closed
- **THEN** the reopened window uses the app instance's singleton shell identity instead of allocating an independent window-scoped control plane

### Requirement: IPC requests are bounded
The local shell control socket SHALL bound request size, request duration, and
per-client work so a stalled or oversized client cannot block other control
requests indefinitely.

#### Scenario: Client never sends newline
- **WHEN** a socket client connects and does not complete a request within the configured deadline
- **THEN** the server closes that client and continues accepting later clients

#### Scenario: Client sends oversized request
- **WHEN** a socket client sends more than the maximum accepted request bytes
- **THEN** the server rejects that request, closes the client, and keeps serving later requests

#### Scenario: Main actor command handling is slow
- **WHEN** a command requires main-actor handling and the handler exceeds the response deadline
- **THEN** the server returns or records a timeout failure instead of hanging the socket loop

### Requirement: Mutations report authoritative results
Control-plane mutation commands SHALL return responses derived from authoritative
shell/runtime state after the requested mutation has been accepted or rejected.

#### Scenario: Missing target
- **WHEN** a mutation references a missing space, tab, or pane ID
- **THEN** the response reports `applied: false` with a stable error code

#### Scenario: Runtime-dependent mutation
- **WHEN** a mutation depends on terminal runtime availability
- **THEN** the response reflects whether the runtime accepted, queued, or rejected the operation

### Requirement: Persistence and event failures are observable
The shell control plane SHALL surface state, event, command, and binding file IO
failures through logs, diagnostics, or control responses instead of ignoring all
write/read errors.

#### Scenario: State file cannot be written
- **WHEN** publishing shell state fails to write the state file
- **THEN** the control plane records a diagnostic that can be inspected during debugging

#### Scenario: Command file cannot be decoded
- **WHEN** a file-command request cannot be decoded
- **THEN** the control plane records or writes a failure result rather than silently deleting the only evidence

### Requirement: Runtime-dependent commands use service state
The macOS shell control plane SHALL derive runtime-dependent command results
from the terminal runtime service after resolving the target window and pane.

#### Scenario: Text delivery succeeds through runtime service
- **WHEN** `pane.send_text` targets a pane whose service-owned surface accepts the bytes
- **THEN** the response reports `applied: true`, the accepted byte count, and the pane runtime phase observed by the service

#### Scenario: Target pane has no service handle
- **WHEN** a runtime-dependent command targets a pane that shell state still lists but the runtime service cannot resolve
- **THEN** the response reports `applied: false` with a stable runtime-missing error and does not claim delivery

### Requirement: Pending delivery is pane scoped and observable
If the runtime service supports queued text delivery, the queue SHALL be scoped
to one pane surface handle and observable through shell diagnostics or command
responses.

#### Scenario: Text is queued for an attachable pane
- **WHEN** `pane.send_text` targets an attachable pane whose surface is not currently ready to accept text
- **THEN** the response reports queued state with the pane ID, queued byte count, and delivery policy

#### Scenario: Queued text is flushed
- **WHEN** the pane surface becomes ready after text was queued
- **THEN** the runtime service flushes the pane-specific queue and records whether the bytes were accepted or rejected

#### Scenario: Pane closes with queued text
- **WHEN** a pane closes while text remains queued
- **THEN** the runtime service drops or fails that queue with a diagnostic tied to the closed pane

### Requirement: Runtime service publishes command diagnostics
Runtime-dependent command failures SHALL be visible in control-plane responses
and diagnostics rather than only in view-local logs.

#### Scenario: Surface rejects text
- **WHEN** a service-owned surface rejects delivered text
- **THEN** the control response includes a stable error code and the service records a pane diagnostic for inspector/debug use

#### Scenario: Runtime command times out
- **WHEN** the runtime service cannot complete a runtime-dependent command inside the control-plane deadline
- **THEN** the response reports timeout without blocking later control requests for the same window

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
