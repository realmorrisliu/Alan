## ADDED Requirements

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
