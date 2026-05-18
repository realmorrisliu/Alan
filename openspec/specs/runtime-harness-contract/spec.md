# runtime-harness-contract Specification

## Purpose
TBD - created by archiving change consolidate-docs-specs-into-openspec. Update Purpose after archive.
## Requirements
### Requirement: Harness contracts live in OpenSpec
alan SHALL specify normative harness behavior, KPI expectations, self-eval
boundaries, and bridge semantics in OpenSpec, while keeping JSON scenarios and
runner docs as executable fixtures and operator guidance.

#### Scenario: Harness behavior changes
- **WHEN** a change modifies scenario semantics, runner pass/fail criteria,
  KPI meanings, self-eval governance, or bridge message delivery semantics
- **THEN** the requirement is captured in this capability or a more specific
  active OpenSpec capability
- **AND** fixture JSON under `docs/harness/scenarios/` remains data rather than
  the contract source

#### Scenario: Harness docs describe current commands
- **WHEN** `docs/harness/README.md`, self-eval docs, KPI docs, or live
  validation guides document runner usage
- **THEN** they may remain under `docs/` as current validation instructions
- **AND** they point to OpenSpec when they state reusable normative behavior

### Requirement: Harness bridge delivery is explicit
alan SHALL treat external harness bridges as bounded control/data-plane
surfaces with explicit envelope, recovery, consistency, security, and
observability expectations.

#### Scenario: External runner integrates with alan
- **WHEN** a harness runner sends operations, receives events, reconnects, or
  reports assertions
- **THEN** the bridge contract identifies delivery semantics, recovery behavior,
  and failure reporting in OpenSpec before the integration becomes a required
  validation path
