# Rust Test Placement Contract

> Status: proposed target contract for Rust test placement in workspace crates.
>
> This document defines where Rust tests belong in Alan's `crates/*` tree so
> implementation files stay readable without forcing internal APIs to become
> public just to satisfy test layout.

## Goal

Alan's Rust test placement must optimize for four things at the same time:

1. Readable implementation files where production logic is easy to scan.
2. Predictable placement so contributors do not invent a new layout per module.
3. Strong white-box testing for private runtime and daemon internals.
4. Strong black-box integration coverage for crate boundaries, public contracts,
   and end-to-end behavior.

## Scope

This contract applies to Rust code under `crates/*`.

It defines **where** tests live. It does not define full test strategy,
coverage targets, provider-harness policy, or CI matrix ownership beyond the
placement rules needed to keep those layers organized.

Non-Rust clients such as `clients/tui/` and `clients/apple/` may adopt similar
principles later, but they are outside the scope of this document.

## Non-Goals

- Defining what every subsystem must test.
- Requiring all existing inline tests to move in one cutover.
- Forcing private implementation details to become `pub` or `pub(crate)` only
  for test access.
- Replacing crate-specific harness docs such as live-provider or live-runtime
  smoke guides.

## Stable Vocabulary

- **Inline unit test**: a `#[cfg(test)] mod tests` block kept in the same file
  as the implementation it exercises.
- **Extracted white-box test file**: a test-only Rust file compiled as a child
  module of the implementation module so it can exercise private details
  without widening production visibility.
- **Integration test**: a crate-level test under `crates/<crate>/tests/`
  compiled outside the library module tree and exercised through the crate's
  external surface or process boundary.
- **Live test**: an integration test that talks to a real provider, daemon, or
  runtime environment and is normally `#[ignore]` plus explicitly opt-in.
- **Test support helper**: fixture-building or assertion helper code that exists
  only to support tests.

## Placement Tiers

Rust tests in Alan must live in one of three placement tiers.

### Tier 1: Inline Unit Tests

Keep tests inline in the implementation file only when all of the following are
true:

1. The tests are short, local, and directly tied to the file's private helper
   logic or invariants.
2. The setup is lightweight and does not require large fixtures, scenario
   matrices, or long async orchestration.
3. Reading the tests next to the implementation materially improves local
   understanding.
4. The inline test block stays small enough that the implementation file
   remains primarily a production-code file rather than a mixed production-plus-
   harness file.

Typical inline test candidates:

- parser and serializer edge cases
- small normalization helpers
- narrow state-transition checks
- simple regression tests for one private helper

Inline tests should not become the default home for scenario suites, large
async flows, or fixture-heavy regression packs.

### Tier 2: Extracted White-Box Test Files

Move tests out of the implementation file but keep them inside the module's
privacy boundary when the tests need private access yet are no longer small and
local.

Use extracted white-box test files for:

- async scenario suites against private runtime or daemon internals
- tests with reusable local fixtures or helper builders
- large regression matrices
- tests whose size makes the implementation file difficult to review
- tests that would otherwise force internal helpers to become public

Stable placement rules:

1. For a flat module file such as `foo.rs`, extracted white-box tests should
   live in a sibling file named `foo_tests.rs` and be loaded with
   `#[cfg(test)] mod foo_tests;`.
2. For a directory-backed module such as `foo/mod.rs`, extracted white-box
   tests should live in `foo/tests.rs` or `foo/tests/<topic>.rs`, loaded from
   `foo/mod.rs` under `#[cfg(test)]`.
3. White-box support helpers that are used only by one module should stay
   adjacent to that module's extracted tests rather than moving into production
   code.
4. Extracted white-box test files remain test-only modules. They must not be
   referenced by production code.

This tier is the default extraction target for large inline test blocks.

### Tier 3: Integration Tests

Use `crates/<crate>/tests/` for black-box behavior that should be validated
through a crate boundary, process boundary, or durable external contract.

Integration tests are the correct location for:

- crate public API behavior
- CLI flows
- HTTP route and websocket contract behavior
- cross-module orchestration that does not need private access
- persistence, restart, and replay flows viewed from the crate boundary
- protocol contract and event-sequence validation
- smoke tests and live-provider or live-runtime harnesses

Stable placement rules:

1. Integration tests must not require widening production visibility solely to
   make the test compile.
2. Shared integration-test helpers belong under `crates/<crate>/tests/support/`
   or a similarly explicit test-only support module under `tests/`.
3. Live tests should remain integration tests, normally `#[ignore]`, with
   explicit opt-in environment variables and companion docs/scripts when needed.

## Required Placement Decisions

When adding or modifying a Rust test, contributors must choose the narrowest
placement tier that preserves both readability and test value:

1. Start with inline only if the test is genuinely small and local.
2. If the test needs private access but is no longer small and local, extract
   it into a white-box test file instead of leaving it inline.
3. If the behavior can be validated from outside the module boundary, prefer a
   crate-level integration test.

The location choice is part of the design, not an afterthought.

## Mandatory Restrictions

The following patterns are not allowed for new code:

1. Expanding a production API to `pub` or `pub(crate)` solely so a black-box
   test under `crates/<crate>/tests/` can reach internal details.
2. Keeping large async scenario suites inline once they stop being small local
   unit tests.
3. Placing black-box contract tests inside `src/` when they do not require
   private access.
4. Moving general-purpose test support helpers into production modules when
   they exist only to support tests.

## Extraction Triggers

The contract intentionally avoids a single hard line-count limit, but the
following conditions are mandatory extraction signals.

A `#[cfg(test)] mod tests` block should be moved out of the implementation file
when any of the following is true:

1. The test block has become a substantial share of the file and the production
   implementation is no longer easy to scan top-to-bottom.
2. The tests introduce fixture builders, helper layers, scenario matrices, or
   multi-step async orchestration.
3. The tests are best organized by behavior topic rather than by one flat local
   `tests` module.
4. Reviewing the production implementation now requires scrolling through a
   large harness section to recover context.

In practice, very large mixed files should be treated as already past the
extraction threshold even if the implementation remains correct.

## Migration Policy

This contract is forward-looking and applies immediately to new or materially
edited Rust test code.

Existing inline tests are temporarily grandfathered, but they should be moved
when one of the following happens:

1. The surrounding implementation file is already large enough that tests are
   materially harming readability.
2. The touched change adds more scenario coverage to an already oversized
   inline test block.
3. The work is already refactoring the implementation module and the test move
   can be done without destabilizing unrelated behavior.

The first migration wave should prioritize the largest mixed production-plus-
test files in `alan-runtime` and `alan`.

## Relationship To Other Docs

- [docs/testing_strategy.md](../testing_strategy.md) defines the current test
  layers and protocol-drift strategy. This contract defines where those tests
  belong in the repository.
- [docs/spec/app_server_protocol.md](./app_server_protocol.md),
  [docs/spec/durable_run_contract.md](./durable_run_contract.md), and adjacent
  runtime specs define behavior contracts that integration tests may validate.
- Live test operational guides remain in dedicated docs such as
  `docs/live_provider_harness.md` and `docs/live_runtime_smoke.md`.

## Acceptance Criteria

This contract is satisfied when all of the following are true:

1. New Rust tests under `crates/*` choose one of the three placement tiers
   deliberately rather than defaulting to inline `#[cfg(test)]`.
2. Large private-access scenario suites move to extracted white-box test files
   instead of remaining embedded in production files.
3. Crate-boundary, protocol, CLI, route, smoke, and live tests live under
   `crates/<crate>/tests/`.
4. Production visibility is not widened solely for test placement.
5. Test-only support helpers live in test-only locations.
6. Oversized mixed implementation files shrink over time as they are touched
   and migrated toward this contract.
