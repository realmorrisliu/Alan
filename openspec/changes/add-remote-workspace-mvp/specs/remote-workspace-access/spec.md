## ADDED Requirements

### Requirement: Account-bound device enrollment
Alan SHALL bind each remote-capable Mac and iPhone app installation to an Alan
account and a stable device identity before allowing Remote Workspace access.

#### Scenario: Mac enrolls after account login
- **WHEN** a user signs in to Alan Desktop on macOS
- **THEN** the Mac is registered as a device owned by that account
- **AND** the Mac receives device-bound credentials suitable for remote
  availability
- **AND** those credentials are stored in the platform secure store rather than
  in workspace files

#### Scenario: iPhone signs in to the same account
- **WHEN** a user signs in to Alan on iPhone with the same account as the Mac
- **THEN** the iPhone is registered as a device owned by that account
- **AND** the iPhone can request access only to devices associated with that
  account

### Requirement: Automatic Mac remote availability
Alan Desktop SHALL automatically keep the signed-in Mac remotely connectable
while the app is running and the user has not disabled Remote Workspace.

#### Scenario: Desktop starts while signed in
- **WHEN** Alan Desktop starts with a valid signed-in account and device binding
- **THEN** it establishes an outbound encrypted connection to the Alan remote
  service without requiring inbound network configuration
- **AND** the user is not asked for public IP, router, VPN, tunnel, SSH, or port
  forwarding settings

#### Scenario: Desktop loses remote connectivity
- **WHEN** the Mac loses network connectivity or its outbound remote connection
  drops
- **THEN** Alan Desktop retries connection in the background
- **AND** Alan Cloud marks the device stale or offline without changing local
  runtime state

### Requirement: User-owned device discovery
Alan SHALL let a signed-in iPhone discover the user's own online Alan Desktop
devices without exposing relay or tunnel implementation details.

#### Scenario: iPhone lists available Macs
- **WHEN** the iPhone app requests Remote Workspace devices for the signed-in
  account
- **THEN** the response includes only devices owned by that account
- **AND** each device includes product-facing status such as online/offline,
  last seen, and connectability
- **AND** the response does not require the iPhone user to provide a daemon URL,
  public IP, tunnel URL, or relay node token

#### Scenario: Device is offline
- **WHEN** a previously enrolled Mac is not connected to Alan Cloud
- **THEN** the iPhone may show the Mac as offline or unavailable
- **AND** the iPhone MUST NOT offer state-advancing actions against that Mac
  until it reconnects

### Requirement: Workspace and agent status discovery
Alan SHALL expose enough Mac-authored workspace and session status for iPhone
users to choose what to continue remotely.

#### Scenario: Mac publishes connectable workspace status
- **WHEN** Alan Desktop is online
- **THEN** it publishes connectable workspace status for the signed-in user
- **AND** status includes whether a workspace is connectable and whether an
  agent/session is currently active
- **AND** the Mac remains the authority for workspace identity and session
  liveness

#### Scenario: iPhone chooses a workspace
- **WHEN** the iPhone user selects an online Mac
- **THEN** the iPhone can list or select the Mac-authored connectable
  workspaces/sessions
- **AND** the UI presents the action as continuing work on another Alan device
  rather than connecting to infrastructure

### Requirement: Remote session control
Alan SHALL allow iPhone to continue a Mac workspace/session by sending normal
Alan control operations to the Mac through the remote service.

#### Scenario: iPhone sends a message
- **WHEN** the iPhone user sends a message to a remote workspace/session
- **THEN** the request is routed to the selected Mac
- **AND** the Mac validates authorization and applies the operation through the
  same session/runtime path used by local clients

#### Scenario: iPhone interrupts an active run
- **WHEN** the iPhone user interrupts a running remote session
- **THEN** the interrupt is routed to the selected Mac
- **AND** the Mac remains responsible for applying or rejecting the interrupt
  according to runtime state

#### Scenario: iPhone resumes a pending yield
- **WHEN** a remote session is waiting for confirmation or structured input
- **THEN** the iPhone can submit a resume response if its token has the required
  resume scope
- **AND** the Mac validates the pending request before advancing execution

### Requirement: Realtime remote event flow
Alan SHALL support realtime remote delivery of session events and streamed
assistant output from the Mac to the iPhone.

#### Scenario: Session streams output remotely
- **WHEN** a Mac-authored remote session emits text, thinking, tool, warning,
  error, yield, or turn-boundary events
- **THEN** the iPhone receives those events in near real time over the remote
  transport
- **AND** event IDs, sequence, session IDs, submission IDs, turn IDs, and item
  IDs remain authored by the Mac

#### Scenario: Relay transports events
- **WHEN** realtime events are delivered through Alan Cloud relay
- **THEN** the relay forwards event transport without becoming the authority for
  event ordering or runtime state
- **AND** the iPhone can still recover missed events with the Mac-authored
  cursor replay APIs

### Requirement: Reconnect and gap recovery
Alan SHALL recover remote iPhone sessions after app backgrounding, network
changes, and relay reconnects without duplicating execution.

#### Scenario: iPhone reconnects with a valid cursor
- **WHEN** the iPhone reconnects with its latest observed event cursor
- **THEN** Alan returns events after that cursor
- **AND** runtime execution is not restarted or re-driven by reconnect

#### Scenario: iPhone cursor has a gap
- **WHEN** the iPhone reconnects after its cursor is no longer in the replay
  buffer
- **THEN** Alan returns a gap indication
- **AND** the iPhone can rebuild actionable state from the reconnect snapshot
  before continuing event consumption

### Requirement: Node-authoritative execution boundary
Alan SHALL keep remote workspace execution, tool access, governance, and local
workspace reads authoritative on the user's Mac.

#### Scenario: Cloud receives a state-changing remote request
- **WHEN** Alan Cloud receives a request to submit, interrupt, resume, fork,
  compact, roll back, or delete a remote session
- **THEN** Alan Cloud routes the request to the selected Mac
- **AND** Alan Cloud MUST NOT execute tools, read local workspace files, decide
  policy outcomes, or mutate runtime state on behalf of the Mac

#### Scenario: Mac rejects unauthorized request
- **WHEN** a remote request lacks the required account, device, session, or
  operation scope
- **THEN** the Mac or Cloud rejects the request with a machine-readable
  authorization error
- **AND** no runtime state is advanced

### Requirement: Remote access security and revocation
Alan SHALL protect Remote Workspace with encrypted transport, device binding,
scoped short-lived authorization, and revocation.

#### Scenario: Remote connection is established
- **WHEN** iPhone connects to a Mac through Alan Remote Workspace
- **THEN** the connection uses encrypted transport
- **AND** access tokens are scoped to the signed-in account, client device,
  target Mac device, and permitted operations

#### Scenario: Device is revoked
- **WHEN** a user revokes a Mac or iPhone device
- **THEN** Alan invalidates future remote access for that device
- **AND** active remote connections using that device credential are closed or
  rejected before additional state-changing operations are accepted

### Requirement: Zero-configuration product language
Alan SHALL present Remote Workspace as device-to-device workspace continuation,
not as remote desktop or user-managed networking.

#### Scenario: User opens Remote Workspace on iPhone
- **WHEN** the iPhone user opens the remote workspace surface
- **THEN** the primary UI language describes online Alan devices, workspaces,
  sessions, runs, messages, and approvals
- **AND** it does not require or foreground VPN, tunnel, Cloudflare, SSH, port
  mapping, router configuration, public IP, or daemon URL concepts

#### Scenario: Debug details are needed
- **WHEN** a developer opens an explicit debug or diagnostics surface
- **THEN** Alan may expose relay, node, routing, and connection diagnostics
- **AND** those diagnostics remain outside the default user workflow
