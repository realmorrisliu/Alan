## 1. Ledger And Storage

- [ ] 1.1 Define memory write ledger record types with `memory_write_id`, target, anchor, confidence, evidence, rationale, source session/turn, timestamps, and revert status.
- [ ] 1.2 Add pure-text ledger layout helpers under `.alan/memory/ledger/` and preserve compatibility with existing memory files.
- [ ] 1.3 Add runtime validation for ledger-target path containment and allowed stable memory targets.

## 2. Write Planning And Validation

- [ ] 2.1 Extend turn-end memory promotion output parsing to carry evidence class, source references, confidence, and consolidation disposition.
- [ ] 2.2 Add validation and canonicalization for direct statements, repeated behavior, and external/repository evidence.
- [ ] 2.3 Route stable memory mutations through a single runtime writer that creates ledger entries and target-file updates together.
- [ ] 2.4 Stage ambiguous, conflicting, or low-confidence observations for consolidation instead of stable promotion.

## 3. Inspection And Revert Surfaces

- [ ] 3.1 Add CLI commands for recent write listing, single-write inspection, and revert.
- [ ] 3.2 Add daemon memory endpoints and endpoint-contract metadata for recent, show, and revert operations.
- [ ] 3.3 Add precise revert mechanics with safe failure when the target content no longer matches the ledger anchor.

## 4. Memory Surface Integration

- [ ] 4.1 Update recall, handoff, session-summary, and daily-note surfaces to keep provenance references bounded.
- [ ] 4.2 Ensure prompt-facing memory surfaces do not duplicate full ledger content.
- [ ] 4.3 Preserve legacy memory entries that do not have reversible ledger metadata.

## 5. Verification

- [ ] 5.1 Add unit tests for write-plan normalization, evidence validation, dedupe, path rejection, and confidence downgrades.
- [ ] 5.2 Add storage tests for ledger creation, target updates, successful revert, already-reverted writes, and manual-resolution-required failures.
- [ ] 5.3 Add daemon and CLI integration tests for recent, show, and revert surfaces.
- [ ] 5.4 Run `cargo test --workspace` or the narrower documented Rust test suites covering runtime and daemon memory behavior.
- [ ] 5.5 Run `openspec validate add-proactive-memory-v2 --strict`.

## 6. PR Review And Archive Readiness

- [ ] 6.1 Review the implementation diff for hidden memory writes, provenance gaps, and prompt-facing ledger leakage.
- [ ] 6.2 After merge, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.3 Archive the completed OpenSpec change after the synced specs validate.
