## Context

Alan's target memory direction is pure-text, file-backed, auditable memory. The
runtime already has turn-end memory promotion, memory recall bundles, and
pre-compaction memory flushes, but stable memory writes are not yet a complete
product surface: writes are difficult to inspect, revert, or connect to a clear
evidence trail.

The desired behavior is more proactive than "only remember when the user says
remember this." Alan may learn from direct user statements, repeated behavior,
and external or repository evidence, but the system must stay quiet by default
and must not become a hidden provider-memory black box.

## Goals / Non-Goals

**Goals:**

- Promote durable user/workspace facts proactively when evidence and confidence
  justify it.
- Keep every stable memory mutation auditable through a durable write ledger.
- Provide low-disturbance review and precise revert surfaces.
- Preserve the existing pure-text memory layout and ordinary file inspection
  workflow.
- Allow ambiguous or conflicting observations to be staged until consolidation.

**Non-Goals:**

- Add vector search, graph storage, SQLite, or an external memory service.
- Interrupt every memory write with a confirmation prompt.
- Store every turn as stable semantic memory.
- Let the model directly mutate memory files outside runtime validation.
- Build the final macOS UI for memory review in this change.

## Decisions

### Decision: Add a memory write ledger instead of relying on inline comments

Every stable memory mutation gets a `memory_write_id` and a ledger record under
`.alan/memory/ledger/`. The target Markdown files remain readable, while the
ledger owns audit details: target path, inserted anchor or range, normalized
observation, confidence, evidence, rationale, source session/turn, timestamp,
and revert status.

Alternatives considered:

- Inline provenance beside every memory line. This keeps context nearby but
  bloats the files that are injected into prompts.
- Rollout-only provenance. This preserves audit data but makes recent write
  review and revert too expensive.

### Decision: Keep runtime as the only memory writer

Model-mediated write planning remains useful for semantic judgment, but runtime
owns target canonicalization, dedupe, confidence downgrades, text bounds,
path-safety checks, ledger creation, and file mutation.

Alternatives considered:

- Give the memory skill permission to edit stable memory directly. This is too
  hard to audit and revert reliably.
- Replace model judgment with Rust heuristics. This would miss many useful
  stable facts and accumulate brittle phrase matching.

### Decision: Use low-disturbance review by default

Alan writes eligible stable memory without interrupting the user. Review happens
through `alan memory recent`, `alan memory show`, `alan memory revert`, and
daemon equivalents.

Alternatives considered:

- Confirm all writes. This prevents bad memory but makes Alan feel needy and
  slows normal work.
- Never auto-promote inferred facts. This is safer but fails the product goal of
  proactive intelligence.

### Decision: Treat evidence class as part of validation

The write planner must identify whether evidence came from direct user
statement, repeated behavior, or external/repository evidence. Direct stable
statements can promote immediately at high confidence. Repeated behavior needs
multiple evidence points or an existing stable-memory update. External evidence
must include source paths, URLs, commands, or issue/PR references.

Alternatives considered:

- Single confidence field only. This is too weak for review and future policy.
- Separate memory stores per evidence class. This adds complexity without
  improving the first implementation.

### Decision: Defer complex consolidation to a separate pass

Turn-end memory observation should stay cheap and bounded. If the write planner
finds ambiguity, conflicts, or cross-session patterns that need synthesis, it
stages the observation and marks it for consolidation rather than forcing a
stable write.

Alternatives considered:

- Always run deep consolidation at turn end. This is expensive and slows normal
  interactions.
- Never consolidate automatically. This allows inbox and daily notes to drift.

## Risks / Trade-offs

- Incorrect stable memory write -> Mitigate with provenance, confidence,
  recent-write review, and precise revert.
- Ledger and target files drift -> Mitigate by writing ledger and target updates
  as one runtime operation where possible and by validating ledger targets in
  tests.
- Prompt pollution from provenance -> Mitigate by keeping detailed provenance
  in the ledger and injecting only bounded recall bundles.
- Revert becomes hard after manual edits -> Mitigate with anchored blocks and
  fallback status that marks a write as requiring manual resolution instead of
  applying a risky patch.
- Proactive writes reveal hidden reasoning -> Mitigate by storing observations,
  evidence, and rationale, not private reasoning traces.

## Migration Plan

1. Add the ledger layout and APIs without changing existing memory write
   behavior.
2. Route existing `memory_promotion` stable writes through the ledger.
3. Add recent/show/revert CLI and daemon read surfaces.
4. Expand write planning and validation to support direct statements, repeated
   behavior, and external evidence.
5. Add consolidation staging for ambiguous or conflicting observations.

Existing memory files remain valid. Existing stable facts that lack ledger
entries are treated as legacy memory and are not automatically reversible.

## Open Questions

- Whether ledger files should be one file per write or monthly JSONL plus a
  generated recent Markdown view.
- Whether the first implementation should expose `alan memory revert --dry-run`.
- How much macOS UI should be included after the CLI/API surfaces are stable.
