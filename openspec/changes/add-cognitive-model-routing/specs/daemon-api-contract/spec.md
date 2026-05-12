## ADDED Requirements

### Requirement: Cognitive Routing API Metadata
The daemon SHALL expose cognitive routing metadata in session, turn, read, fork,
and reconnect surfaces where request-control metadata is already reported.

#### Scenario: Session response includes routing metadata
- **WHEN** a session is created with cognitive routing enabled
- **THEN** the daemon response includes the configured routing mode and the
  active cognitive model binding metadata available at startup

#### Scenario: Turn read includes selected system
- **WHEN** a client reads session history or reconnect state after a routed turn
- **THEN** the response includes selected cognitive system, routing source,
  model binding id, provider, model, effective reasoning effort, and bounded
  routing reason when available

### Requirement: Cognitive Routing API Overrides
The daemon SHALL accept explicit cognitive-system override intent on session,
fork, and turn submission surfaces where doing so preserves existing governance
and request-control boundaries.

#### Scenario: Turn override supersedes session override
- **WHEN** a session was created with a cognitive-system override
- **AND** a submitted turn includes its own cognitive-system override
- **THEN** runtime treats the turn override as the effective routing intent for
  that turn before applying deterministic gates

#### Scenario: Turn requests System 2
- **WHEN** a client submits a turn with a System 2 override
- **THEN** runtime receives turn-scoped routing intent and applies it before
  provider dispatch

#### Scenario: Turn requests System 1 in gated context
- **WHEN** a client submits a turn with a System 1 override
- **AND** runtime detects a configured high-risk or high-complexity gate
- **THEN** runtime rejects or supersedes the forced System 1 override and
  reports the routing decision in metadata

#### Scenario: Invalid override is rejected
- **WHEN** a client requests an unknown cognitive system or a cognitive system
  whose model binding is not configured
- **THEN** the daemon or runtime rejects the request with a diagnostic rather
  than silently using the default route

#### Scenario: Routing endpoints are registered
- **WHEN** cognitive routing DTOs or routes are added
- **THEN** endpoint metadata and generated client drift checks cover the new
  public surface
