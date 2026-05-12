## ADDED Requirements

### Requirement: Memory Write API Contract
The daemon SHALL expose endpoint-contract-backed APIs for recent memory write
listing, single-write inspection, and memory write revert. These APIs SHALL bind
each request to an explicit workspace or session scope before reading or
mutating memory ledger state. Workspace-scoped requests SHALL require host/admin
authorization or SHALL be bound to an authorized session for that workspace.

#### Scenario: Recent memory writes endpoint
- **WHEN** a client calls the recent memory writes endpoint
- **AND** the request identifies a workspace or session scope
- **THEN** the daemon returns bounded ledger metadata for recent stable memory
  writes in that scope without hidden reasoning content

#### Scenario: Memory write inspection endpoint
- **WHEN** a client requests a memory write by id
- **AND** the request identifies a workspace or session scope
- **THEN** the daemon returns the write detail, provenance, target, confidence,
  and revert status from that scope

#### Scenario: Memory write revert endpoint
- **WHEN** a client requests revert for a memory write by id
- **AND** the request identifies a workspace or session scope
- **THEN** the daemon attempts the runtime memory revert operation and returns a
  success, already-reverted, not-found, or manual-resolution-required result

#### Scenario: Workspace-scoped memory request is authorized
- **WHEN** a client calls a recent, show, or revert memory write endpoint with a
  workspace scope
- **THEN** the daemon requires host/admin authorization or an authorized session
  for that workspace with the read or admin authority needed by the operation
  before reading or mutating the ledger

#### Scenario: Unauthorized workspace scope is rejected
- **WHEN** a client calls a recent, show, or revert memory write endpoint with a
  workspace scope that is not covered by host/admin authorization or an
  authorized session for that workspace
- **THEN** the daemon rejects the request before reading or mutating any memory
  ledger

#### Scenario: Unscoped memory write request is rejected
- **WHEN** a client calls a recent, show, or revert memory write endpoint without
  a workspace or session scope
- **THEN** the daemon rejects the request before reading or mutating any memory
  ledger

#### Scenario: Memory endpoints are registered
- **WHEN** memory write APIs are added
- **THEN** each route has canonical endpoint metadata and generated client
  helpers participate in drift checks
- **AND** the endpoint metadata records the required session scope and any
  workspace-scope authorization requirement
