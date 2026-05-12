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

### Decision: Use one Markdown ledger file per stable write

V1 should store each stable write as a separate Markdown file, grouped by date,
for example `.alan/memory/ledger/YYYY/MM/<memory_write_id>.md`. This makes each
write easy to inspect with normal tools, gives revert code a stable record to
load, and avoids rewriting a large append-only JSONL file when revert status
changes.

Recent-write listing can scan the dated ledger directories or maintain a small
derived index, but the per-write Markdown ledger file remains the durable source
of truth.

Alternatives considered:

- Monthly JSONL. This is compact and append-friendly, but precise revert status
  updates either rewrite a shared file or require secondary tombstones.
- Inline Markdown blocks only. This is easy to read in target files but weak for
  bounded prompt recall and audit filtering.

### Decision: Keep runtime as the only memory writer

Model-mediated write planning remains useful for semantic judgment, but runtime
owns target canonicalization, dedupe, confidence downgrades, text bounds,
path-safety checks, ledger creation, and file mutation.
When `[memory].enabled = false`, runtime must treat all proactive memory write
targets as ineligible and skip stable, staged, inbox, daily-note, consolidation,
and ledger persistence.

Alternatives considered:

- Give the memory skill permission to edit stable memory directly. This is too
  hard to audit and revert reliably.
- Replace model judgment with Rust heuristics. This would miss many useful
  stable facts and accumulate brittle phrase matching.

### Decision: Use low-disturbance review by default

Alan writes eligible stable memory without interrupting the user. Review happens
through `alan memory recent`, `alan memory show`, `alan memory revert`, and
daemon equivalents. Daemon memory write APIs must bind every recent, show, and
revert request to an explicit workspace or session scope before reading or
mutating a ledger. Session-scoped requests use the authorized session as the
workspace authority. Workspace-scoped requests require host/admin authorization
or an authorized session for that workspace with the read or admin authority
needed by the operation.

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

Evidence must also be reviewable later. Ledger records should store the source
kind, stable source locator, observed-at timestamp when relevant, and either a
bounded excerpt, line/range, command summary, or content hash sufficient to
explain why the fact was written without preserving large raw artifacts.

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

### Decision: Reject or redact sensitive material before durable memory persistence

The memory writer must treat API keys, access tokens, passwords, private
credentials, and secret-like values as unsafe memory material. A useful fact may
still be recorded after redaction, for example "the project uses a GitHub token
from the host secret store," but plaintext secrets must not be written to stable
memory, staged observations, inbox entries, daily notes, consolidation queues,
or ledger evidence.

Alternatives considered:

- Let provenance store exact command output. This is useful for debugging but
  makes memory a secret sink.
- Rely on user review after the fact. This violates the low-disturbance model
  because bad writes may sit unnoticed.

### Decision: Reverted memory must disappear from prompt-facing surfaces

Revert must not leave a bad fact available for future recall just because it is
visibly marked as reverted in `USER.md`, `MEMORY.md`, or a topic page. The
preferred path is to remove the inserted stable-memory block when the ledger
anchor still matches. If a target file keeps a tombstone or reverted marker for
human auditability, every prompt-facing memory renderer must exclude that block
from recall, handoff, session-summary, and daily-note surfaces.

Alternatives considered:

- Only mark reverted content inline. This is readable to humans, but prompt
  assembly reads pure text and could re-inject the bad fact.
- Store reverted content only in the ledger. This is safer for prompts and is
  acceptable as long as the stable memory target no longer exposes it.

## Implementation Ownership

This change is primarily a runtime memory/storage change, not a memory-skill
permission expansion.

- Runtime owns memory write planning orchestration, target canonicalization,
  dedupe, confidence downgrade, sensitive-data validation, ledger writes, target
  file mutation, and precise revert.
- Prompt and memory surface renderers may display bounded summaries or
  provenance references, but they must not become the ledger, reintroduce
  reverted content, or rely on prompts as the secret-redaction boundary.
- Skills may suggest candidate observations or explain memory behavior, but they
  must not directly mutate stable memory, staged memory, inbox entries, daily
  notes, or ledger files outside the runtime writer.
- Daemon and CLI APIs expose recent/show/revert review surfaces with explicit
  workspace or session scope; they do not choose memory targets without runtime
  validation.
- Client UI can make memory review easier later, but the first implementation's
  correctness belongs to runtime validation, storage, and API contracts.

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
- Reverted content leaks back into prompts -> Mitigate by removing reverted
  stable-memory content or requiring prompt-facing surface renderers to filter
  reverted blocks.
- Proactive writes reveal hidden reasoning -> Mitigate by storing observations,
  evidence, and rationale, not private reasoning traces.
- Proactive writes capture secrets -> Mitigate by scanning candidates and
  evidence for secret-like material and rejecting or redacting before durable
  stable, staged, inbox, daily-note, or ledger persistence.

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

- Whether the first implementation should expose `alan memory revert --dry-run`.
- How much macOS UI should be included after the CLI/API surfaces are stable.
