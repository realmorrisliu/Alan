## Why

Alan already has pure-text memory, turn-end memory promotion, recall bundles,
and pre-compaction memory flushes, but the behavior is still too conservative
and too hard to audit as a first-class product surface. Alan should feel more
intelligent by proactively remembering durable user and workspace facts while
keeping every write inspectable and reversible.

## What Changes

- Add proactive memory write planning that can promote stable facts from direct
  user statements, repeated behavior, and external or repository evidence.
- Add a durable memory write ledger that records provenance, confidence, write
  rationale, target file, and revert state for every stable memory mutation.
- Add low-disturbance review and revert surfaces through CLI and daemon APIs;
  normal turns should not be interrupted by memory write confirmations.
- Ensure reverted memory writes cannot be reintroduced into prompt-facing
  recall, handoff, session-summary, or daily-note surfaces.
- Extend memory validation so invalid, over-broad, duplicate, or unsafe write
  plans are rejected or downgraded before they mutate stable memory.
- Add sensitive-data guardrails so secrets, credentials, and private tokens are
  not written into stable memory or ledger evidence in plaintext.
- Add consolidation rules so ambiguous or conflicting observations can remain
  staged until a stronger consolidation pass resolves them.
- Keep memory writes file-backed, auditable, and compatible with the existing
  pure-text memory direction.

## Capabilities

### New Capabilities

- `runtime-memory-write-audit`: Owns proactive memory write planning, provenance,
  ledger entries, recent write inspection, and revert behavior.

### Modified Capabilities

- `runtime-memory-surfaces`: Memory and handoff surfaces must reference
  proactive write provenance and remain compact continuation aids rather than
  becoming the write ledger.
- `daemon-api-contract`: Daemon APIs must expose recent memory writes, write
  inspection, and revert operations without leaking hidden model reasoning.

## Impact

- Affected runtime modules: `memory_promotion`, `memory_flush`,
  `memory_recall`, prompt assets, session/rollout metadata, and memory layout
  helpers.
- Affected CLI/daemon areas: memory inspection commands and daemon memory
  endpoints.
- Affected storage: `.alan/memory/` gains ledger/recent-write metadata while
  preserving plain Markdown as the source of truth.
- Affected tests: unit tests for write-plan validation and revert mechanics,
  integration tests for daemon/CLI surfaces, and contract tests for auditability.
