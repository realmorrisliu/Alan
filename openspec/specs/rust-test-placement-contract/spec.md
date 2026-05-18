# rust-test-placement-contract Specification

## Purpose
Defines Rust test placement rules for inline unit tests, extracted white-box
suites, crate-level integration tests, migration policy, and behavior-boundary
coverage.

## Requirements
### Requirement: Rust test placement contracts live in OpenSpec
alan SHALL specify Rust test placement rules, extraction triggers, migration
policy, and relationship to integration tests in OpenSpec.

#### Scenario: Rust tests are added or materially edited
- **WHEN** a change adds or materially edits Rust tests
- **THEN** the author chooses inline unit tests, extracted white-box tests, or
  crate-level integration tests based on the OpenSpec placement rules
- **AND** new placement guidance is not authored as a long-form `docs/spec/`
  contract

#### Scenario: Legacy Rust test placement doc is opened
- **WHEN** `docs/spec/rust_test_placement_contract.md` is reached during
  migration
- **THEN** the file points to this OpenSpec capability as a bridge

### Requirement: Test location matches behavior boundary
alan SHALL place tests near the behavior boundary they verify, with larger or
cross-module behavior using extracted white-box or integration suites rather
than oversized inline modules.

#### Scenario: Test needs private implementation access
- **WHEN** a Rust test needs private module access but is too large for a small
  inline unit block
- **THEN** it uses an extracted white-box test file adjacent to the
  implementation module

#### Scenario: Test verifies public crate behavior
- **WHEN** a Rust test verifies cross-module or public crate behavior
- **THEN** it belongs in a crate-level integration test unless private access is
  the core reason for the test
