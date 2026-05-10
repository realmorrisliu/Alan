## ADDED Requirements

### Requirement: Architecture warning debt is reduced by focused slices
The Apple client SHALL reduce tracked architecture-maintainability warnings
through focused, behavior-preserving refactor slices. Each slice MUST identify
the warning class it resolves, the owner boundary it clarifies, and the
verification commands that protect the moved behavior.

#### Scenario: Focused slice resolves a warning
- **WHEN** a refactor slice removes one or more warnings from
  `check-architecture-maintainability.sh`
- **THEN** the slice updates `clients/apple/ARCHITECTURE.md` with the new
  warning count and removes or narrows the corresponding debt entry

#### Scenario: Slice changes a terminal owner
- **WHEN** a slice moves code from a terminal runtime, host, or surface owner
- **THEN** focused terminal runtime or terminal surface scripts are run in
  addition to the architecture report

#### Scenario: Slice changes a shell controller owner
- **WHEN** a slice moves controller, store, projection, or command-routing code
  out of `ShellHostController.swift`
- **THEN** shell contract validation is run and the shared
  `ShellWorkspaceCommand` vocabulary remains the command boundary

#### Scenario: Slice changes console or mobile owners
- **WHEN** a slice moves code from `Views/Console/ContentView.swift`
- **THEN** the primary macOS shell path remains distinguishable from
  console/mobile surfaces by folder, naming, or project grouping

### Requirement: Architecture validation expectations track reduced debt
The architecture-maintainability gate SHALL keep current warning expectations
aligned with the tracked debt ledger. A PR that resolves a warning MUST update
the report expectations and documentation in the same change so the warning
cannot silently reappear.

#### Scenario: Warning count decreases
- **WHEN** `check-architecture-maintainability.sh` reports fewer warnings than
  the documented debt ledger
- **THEN** the implementation updates the ledger and any script expectations
  before the PR is considered complete

#### Scenario: Warning count does not decrease
- **WHEN** a refactor slice moves architecture code but does not reduce the
  warning count
- **THEN** the PR explains why the moved boundary is an intermediate step and
  leaves the debt ledger accurate

#### Scenario: New or broadened warning appears
- **WHEN** a change introduces a new architecture warning or broadens an
  existing warning while reducing another one
- **THEN** the change either resolves the new warning before merge or records a
  concrete follow-up boundary in the debt ledger
