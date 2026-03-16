# Compaction Contract

> Status: VNext contract (extends current `compact` capability with trigger/quality/audit semantics).

## Goals

Compaction is not "delete history." It is the core mechanism for preserving execution continuity under bounded context.

It must guarantee:

1. Context-size reduction.
2. Preservation of key decisions and unfinished work.
3. No breakage of downstream recoverability.

## Trigger Types

### 1) Manual Trigger

- Triggered by explicit operation (`Op::Compact`).
- Forward-compatible clients may use `Op::CompactWithOptions { focus }`.
- May include focus instructions (for example "preserve todos and constraints").

### 2) Automatic Trigger

- Triggered near context-window limits.
- Runtime should evaluate both:
  - secondary message-count guardrail
  - configurable prompt utilization ratio against `context_window_tokens`
- `context_window_tokens` should come from explicit config override first, then
  resolved model metadata.
- Recommended dual-threshold strategy:
  - `hard_threshold`: compaction is mandatory
  - `soft_threshold`: run pre-compaction memory flush first

## Input Scope Contract

Compaction input should include:

1. Current session history useful for ongoing reasoning (messages, tool results, key system context).
2. Active policy/context boundaries when required as non-droppable information.

Compaction input should exclude:

1. Large irrelevant tool outputs (can be trimmed before summarization).
2. Noise logs that are unsafe or useless to reuse.

Recent retained context should prefer semantic windows (for example complete user-turn spans)
instead of arbitrary raw tail messages.

## Output Contract

Post-compaction session must include at least:

1. **Compaction summary item** (structured summary record).
2. **Recent window** (latest critical messages kept verbatim).
3. **Reference marker** (optional) for covered ranges and source references.

Summary minimum content:

1. Key decisions.
2. Active constraints.
3. Unfinished work and next steps.
4. Critical identifiers (IDs, paths, command context) without distortion.

## Quality Constraints

1. **Factual safety**: no fabricated facts.
2. **Identifier fidelity**: IDs/paths/hashes must remain accurate.
3. **Actionability**: summary should directly support next-step execution.

## Coordination with Memory

Recommended pre-compaction flush on automatic compaction:

1. Persist high-value long-term info to L1 memory.
2. Flush turn should be silent by default.
3. On flush failure, emit warning and continue compaction.

## Events and Audit Fields

Each compaction must persist at least:

1. `trigger` (`manual/auto`)
2. `reason` (`window_pressure/explicit_request`)
3. `focus` (optional manual guidance)
4. `input_size` / `output_size`
5. `summary_id` (or equivalent ref)
6. `duration_ms`
7. `result` (`success/failure/retry`)

On retry, include `retry_count` and failure reason.

## Failure Degradation Strategy

1. **Summary failure**: preserve original context and return recoverable error; never silently clear context.
2. **Partial failure**: degrade to "trim large tool output + preserve recent window".
3. **Repeated failure**: emit explicit warnings and recommend new session/run.

## Idempotency and Reentrancy

1. Re-running compaction on identical input snapshot should be semantically equivalent.
2. Avoid infinite compaction loops inside the same turn.
3. Compaction must be interruptible without corrupting session consistency.

## Relationship with Rollback/Fork

1. Rollback must respect compaction boundaries and preserve summary consistency.
2. Fork should inherit necessary summary context to keep branch executability.

## Acceptance Criteria

1. Token usage drops meaningfully while behavior stays continuous.
2. Summary covers decisions/constraints/todos/critical identifiers.
3. Audit logs reconstruct compaction causality end-to-end.
