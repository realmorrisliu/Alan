## ADDED Requirements

### Requirement: Cognitive Routing API Metadata
The daemon SHALL expose cognitive routing metadata in session, turn, read, fork,
and reconnect surfaces where request-control metadata is already reported.

#### Scenario: Session response includes routing metadata
- **WHEN** a session is created with cognitive routing enabled
- **THEN** the daemon response includes the configured routing mode and the
  active cognitive profile metadata available at startup

#### Scenario: Turn read includes selected system
- **WHEN** a client reads session history or reconnect state after a routed turn
- **THEN** the response includes selected cognitive system, routing source,
  profile id, model, effective reasoning effort, and bounded routing reason
  when available

### Requirement: Cognitive Routing API Overrides
The daemon SHALL accept explicit cognitive-system override intent on session,
fork, and turn submission surfaces where doing so preserves existing governance
and request-control boundaries.

#### Scenario: Turn requests System 2
- **WHEN** a client submits a turn with a System 2 override
- **THEN** runtime receives turn-scoped routing intent and applies it before
  provider dispatch

#### Scenario: Invalid override is rejected
- **WHEN** a client requests an unknown cognitive system or a system that is not
  configured
- **THEN** the daemon or runtime rejects the request with a diagnostic rather
  than silently using the default route

#### Scenario: Routing endpoints are registered
- **WHEN** cognitive routing DTOs or routes are added
- **THEN** endpoint metadata and generated client drift checks cover the new
  public surface
