## ADDED Requirements

### Requirement: Memory Write API Contract
The daemon SHALL expose endpoint-contract-backed APIs for recent memory write
listing, single-write inspection, and memory write revert.

#### Scenario: Recent memory writes endpoint
- **WHEN** a client calls the recent memory writes endpoint
- **THEN** the daemon returns bounded ledger metadata for recent stable memory
  writes without hidden reasoning content

#### Scenario: Memory write inspection endpoint
- **WHEN** a client requests a memory write by id
- **THEN** the daemon returns the write detail, provenance, target, confidence,
  and revert status

#### Scenario: Memory write revert endpoint
- **WHEN** a client requests revert for a memory write by id
- **THEN** the daemon attempts the runtime memory revert operation and returns a
  success, already-reverted, not-found, or manual-resolution-required result

#### Scenario: Memory endpoints are registered
- **WHEN** memory write APIs are added
- **THEN** each route has canonical endpoint metadata and generated client
  helpers participate in drift checks
