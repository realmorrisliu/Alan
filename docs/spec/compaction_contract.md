# Compaction Contract

> Status: partially implemented current contract with ongoing quality work.
>
> Current reality: manual compaction, automatic pressure evaluation,
> soft/hard thresholds, pre-compaction memory flush, structured audit
> snapshots, and reconnect/session-read recovery are all implemented. The main
> remaining work is around quality calibration and finalizing the public shape.

## Goals

Compaction is not "delete history." It is the core mechanism for preserving execution continuity under bounded context.

It must guarantee:

1. Context-size reduction.
2. Preservation of key decisions and unfinished work.
3. No breakage of downstream recoverability.

## Current Implementation Snapshot

Implemented in the current tree:

1. Manual compaction via `Op::CompactWithOptions { focus? }`.
2. Automatic pre-turn and mid-turn compaction pressure evaluation using both
   message-count guardrails and context-window utilization ratios.
3. Soft-threshold automatic memory flush before `AutoPreTurn` compaction, with
   structured skip/failure/success outcomes.
4. Structured `compaction_observed` and `memory_flush_observed` events, plus
   recovery of the latest attempts through session reads and reconnect
   snapshots.
5. Rollout/session persistence of compaction attempts and degraded/failure
   outcomes.

Still evolving:

1. Summary-quality enforcement is still heuristic rather than a separately
   scored contract.
2. Reference markers remain optional and are not yet a separate public object.

## Trigger Types

### 1) Manual Trigger

- Triggered by explicit operation (`Op::CompactWithOptions { focus }`).
- May include focus instructions (for example "preserve todos and constraints").

### 2) Automatic Trigger

- Triggered near context-window limits.
- Runtime should evaluate both:
  - secondary message-count guardrail
  - configurable prompt utilization ratio against `context_window_tokens`
- `context_window_tokens` should come from explicit config override first, then
  resolved model metadata.
- Dual-threshold strategy:
  - `below_soft`: do not compact yet
  - `soft_threshold`: `AutoPreTurn` may run one silent pre-compaction memory flush, then continue
    with compaction
  - `hard_threshold`: compaction is mandatory immediately
- `AutoMidTurn`, message-count guardrails, and emergency near-limit pressure should be treated as
  `hard_threshold` behavior and bypass pre-compaction memory flush.

## Input Scope Contract

Compaction input should include:

1. Current session history useful for ongoing reasoning (messages, tool results, key system context).
2. Active policy/context boundaries when required as non-droppable information.

Compaction input should exclude:

1. Large irrelevant tool outputs (can be trimmed before summarization).
2. Noise logs that are unsafe or useless to reuse.

Recent retained context should prefer semantic windows (for example complete user-turn spans)
instead of arbitrary raw tail messages.
When a single recent span exceeds the retention budget, runtime may fall back to a raw-tail cut
to guarantee meaningful context reduction.

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

Current pre-compaction flush behavior on automatic compaction:

1. Persist high-value long-term info to L1 memory.
2. Flush turn should be silent by default.
3. Flush should run only for `AutoPreTurn` requests at `soft_threshold`.
4. Each compaction cycle should attempt at most one automatic flush.
5. On flush failure, emit warning and continue compaction.
6. On flush skip, emit a structured skip result rather than warning-only text.

## Events and Audit Fields

Each compaction must persist at least:

1. `trigger` (`manual/auto`)
2. `reason` (`window_pressure/explicit_request`)
3. `focus` (optional manual guidance)
4. `pressure_level` (`soft/hard` for automatic compaction)
5. `memory_flush_attempt_id` (optional link to the producing pre-compaction flush)
6. `input_size` / `output_size`
7. `summary_id` (or equivalent ref)
8. `duration_ms`
9. `result` (`success/retry/degraded/failure/skipped`)

On retry, include `retry_count` and failure reason.

Each automatic memory flush attempt must persist at least:

1. `compaction_mode`
2. `pressure_level`
3. `result` (`success/skipped/failure`)
4. `skip_reason` when skipped
5. `source_messages`
6. `output_path` when a daily note was written
7. `warning_message` / `error_message` when applicable

External visibility requirements:

1. Each attempted compaction should emit a structured `compaction_observed` event carrying the
   attempt snapshot.
2. Manual compaction should preserve the initiating `submission_id` in the attempt snapshot so
   `/compact` callers can correlate submission acceptance with the final outcome.
3. Session snapshot reads should expose `latest_compaction_attempt`.
4. Reconnect snapshots should expose `execution.latest_compaction_attempt`.
5. Each attempted automatic memory flush should emit a structured `memory_flush_observed` event.
6. Session snapshot reads should expose `latest_memory_flush_attempt`.
7. Reconnect snapshots should expose `execution.latest_memory_flush_attempt`.

## Failure Degradation Strategy

1. **Summary failure**: preserve original context and return recoverable error; never silently clear context.
2. **Partial failure**: degrade to "trim large tool output + preserve recent window", and if possible emit a deterministic fallback summary rather than silently dropping compaction.
3. **Repeated failure**: emit explicit warnings, persist auditable failure markers, and recommend new session/run.

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
