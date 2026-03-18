# Memory Architecture

> Status: VNext contract (evolves from current Tape + Workspace Memory capabilities).

## Goals

Decouple short-lived model context from long-lived persistent knowledge with an explainable, maintainable, auditable memory model.

Core principles:

1. **Files are the factual source of truth**, not implicit model memory.
2. **Retrieval is a capability layer**, not a state layer.
3. **Writes must be policy-driven**, not opportunistic.

## Three-Layer Memory Model

### L0: Execution Memory

- Carrier: `Tape + Rollout`
- Lifecycle: Session-level
- Purpose: continuity of current execution
- Traits: high fidelity, growth-prone, requires compaction

### L1: Workspace Memory

- Carrier: `{workspace}/.alan/memory/`
- Lifecycle: Workspace-level
- Purpose: stable preferences, decisions, constraints, key facts
- Traits: human-readable, editable, versionable

Recommended base files:

- `MEMORY.md`: long-lived stable memory (rules, preferences, context)
- `memory/YYYY-MM-DD.md`: daily incremental log, including automatic pre-compaction flush entries

### L2: Retrieval Memory (Optional Index Layer)

- Carrier: vector/hybrid index (pluggable)
- Lifecycle: rebuildable
- Purpose: semantic recall efficiency across long horizons
- Traits: cache layer, not source of truth, always rebuildable

## Current Implementation Mapping

Currently available:

1. L0: persisted `Tape` and `rollout`.
2. L1 (basic): workspace memory directory + memory skill.

Missing pieces:

1. Unified memory tool contract (`search/get`).
2. Pre-compaction auto flush policy.
3. L2 index conventions and backend interfaces.

## Write Policy Contract

### When to write to L1

1. User explicitly asks to remember something.
2. Reusable decisions emerge (rules, constraints, preferences).
3. Compaction is imminent and high-value info is not yet persisted (pre-compaction flush).

### What should not be written

1. Short-term noise.
2. Highly volatile facts that are better read live from source systems.
3. Sensitive data unless explicitly allowed by governance policy.

## Read Policy Contract

1. First decide whether long-term memory is relevant.
2. Retrieval should go from narrow to broad:
   - precise file reads (`MEMORY.md`, same-day notes)
   - then semantic search (L2)
3. Retrieved results should include source paths for auditability.

## Coordination with Compaction

### Pre-compaction memory flush (recommended)

Before soft compaction threshold:

1. Run a silent pass to persist high-value info to L1.
2. No user-visible reply by default unless necessary.
3. Trigger once per compaction cycle.
4. Append successful flush output to `.alan/memory/YYYY-MM-DD.md`, not directly to `MEMORY.md`.

### Contract requirements

1. Flush failure should not block main flow, but must be logged.
2. Flush skip conditions must be explicit (for example `already_flushed_this_cycle`,
   `read_only_memory_dir`, or `no_durable_content`).
3. Hard-threshold and mid-turn compaction should bypass pre-compaction flush entirely.
4. The latest flush attempt should be recoverable through event replay, thread reads, reconnect
   snapshots, and rollout recovery.

## Data Governance and Audit

1. Each memory write should include: `who/when/why/source`.
2. Optional fields: confidence, expiration, sensitivity level.
3. Deletions and rewrites must be traceable.

## L2 Index Abstraction Interface (Draft)

```text
index.upsert(path, content, metadata)
index.delete(path)
index.search(query, options) -> snippets[]
index.read(path, range) -> text
```

Requirements:

1. Index failure must not block L1 file reads/writes.
2. Backend is replaceable (sqlite/vector/hybrid).
3. Retrieval results must link back to L1 source text.

## Acceptance Criteria

1. L0/L1/L2 responsibilities are clear and non-overlapping.
2. Critical context remains recoverable through L1 after compaction.
3. Memory write/read chains are auditable.
