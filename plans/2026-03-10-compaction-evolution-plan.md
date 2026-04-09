# Alan Compaction Evolution Plan (2026-03-10)

> Status: active implementation plan for compaction evolution.

## Context

Alan already has a solid compaction foundation:

- `Tape` separates `reference_context`, `summary`, and `messages`.
- automatic compaction uses both message-count and prompt-budget heuristics.
- repeated compactions fold the previous summary into the next compaction input.

Current gaps are mostly in orchestration and recoverability semantics rather than in the
basic tape model:

- manual and automatic compaction share the same implementation path.
- compaction only runs before a turn begins, not during an already-running turn.
- retained history is selected by raw message count instead of semantic boundaries.
- compaction persistence records only the summary text, not trigger/audit metadata.
- failure handling is mostly log-based and is not surfaced clearly to clients.

This plan evolves Alan's compaction system without abandoning its current tape model.

## Current Model

Today the prompt shape is:

1. rendered `reference_context`
2. optional compaction `summary`
3. retained `messages`

This is a good model and should remain the core representation.

The target design should preserve these Alan-native properties:

- `reference_context` remains a distinct layer, not inlined into retained message history.
- compaction updates `summary` and `messages`; it does not own the lifecycle of
  `reference_context`.
- prompt assembly continues to happen via `Tape::prompt_view()`.

## Goals

1. Preserve execution continuity under bounded context.
2. Support three compaction modes with different orchestration semantics:
   - manual
   - automatic pre-turn
   - automatic mid-turn
3. Preserve logical work units better than the current "keep last N messages" policy.
4. Make compaction observable and auditable.
5. Keep rollout compatibility and avoid breaking existing clients.

## Non-Goals

1. Do not replace Alan's tape model with Codex's `replacement_history` model.
2. Do not change summary ordering from `reference_context -> summary -> messages`.
3. Do not require provider-specific tokenizers for compaction heuristics in the first phase.
4. Do not introduce new mandatory frontend event types in the first phase.

## Desired End State

By the end of this work:

- manual compaction can optionally express focus, for example "preserve todos and constraints".
- automatic compaction can run both before a turn and mid-turn.
- compaction retention preserves semantic windows instead of arbitrary tail messages.
- compacted rollout records explain what triggered compaction and what changed.
- failures surface clear warnings and degrade safely.
- repeated compactions remain semantically stable and recoverable.

## Proposed Architecture

Introduce an explicit compaction subsystem instead of keeping the logic embedded in
`agent_loop.rs`.

Suggested runtime types:

```rust
enum CompactionMode {
    Manual,
    AutoPreTurn,
    AutoMidTurn,
}

enum CompactionReason {
    ExplicitRequest,
    WindowPressure,
    ContinuationPressure,
}

struct CompactionRequest {
    mode: CompactionMode,
    reason: CompactionReason,
    focus: Option<String>,
}

struct CompactionOutcome {
    summary: String,
    input_messages: usize,
    output_messages: usize,
    input_tokens: usize,
    output_tokens: usize,
    retry_count: u32,
    duration_ms: u64,
    degraded: bool,
}
```

The subsystem should own:

- trigger evaluation
- compaction-input preparation
- retry and trimming behavior
- retention-window selection
- rollout recording
- warning emission

## Phase Plan

### PR1: Protocol and Audit Substrate

Goal: make compaction observable and extensible without changing runtime behavior yet.

Changes:

- Add a non-breaking manual protocol extension:
  - keep `Op::Compact`
  - add `Op::CompactWithOptions { focus: Option<String> }`
- Keep existing HTTP `/compact` behavior and allow an optional request body later.
- Expand rollout `CompactedItem` with optional audit fields:
  - `trigger`
  - `reason`
  - `focus`
  - `input_messages`
  - `output_messages`
  - `input_tokens`
  - `output_tokens`
  - `duration_ms`
  - `retry_count`
  - `result`
  - `reference_context_revision`
- Add a single runtime helper for recording compaction outcomes instead of persisting only the
  raw summary string.

Compatibility rules:

- all new rollout fields must be optional.
- old rollout files must continue to load unchanged.
- old clients can keep using `Op::Compact`.

Files likely touched:

- `crates/protocol/src/op.rs`
- `crates/runtime/src/rollout.rs`
- `crates/runtime/src/session.rs`
- `crates/alan/src/daemon/routes.rs`

### PR2: Semantic Retention Windows

Goal: stop cutting retained context at arbitrary message boundaries.

Current problem:

- `Tape::compact(summary, keep_last)` keeps the last `keep_last` raw messages.
- this can split a logical interaction, for example preserving a tool response without the
  assistant/tool request that explains why it exists.

Changes:

- Replace raw tail retention with semantic retention windows.
- First implementation should group retained history into user-turn spans:
  - one non-control user message
  - following assistant/tool messages
  - until the next non-control user message
- Retain the newest N spans rather than the newest N messages.
- Keep the summary as a distinct `Context` message, as today.
- Add compaction-input sanitation before summarization:
  - trim oversized tool outputs
  - preserve critical identifiers such as paths, IDs, command names, and tool call IDs
  - drop obvious noise logs where safe

Recommended internal additions:

```rust
struct MessageSpan {
    start: usize,
    end: usize,
    kind: SpanKind,
}

enum SpanKind {
    UserTurn,
    Control,
}
```

Files likely touched:

- `crates/runtime/src/tape.rs`
- `crates/runtime/src/session.rs`
- new `crates/runtime/src/runtime/compaction.rs`

### PR3: Mid-Turn Automatic Compaction

Goal: allow a long-running turn to compact and continue without waiting for the next turn.

Current problem:

- automatic compaction runs before user input is appended for a new turn.
- once a turn is already running, Alan has no separate mid-turn compaction path.
- long tool loops or long follow-up chains can grow until the next LLM request becomes risky.

Alan should not copy Codex's history-reinjection design directly because Alan already has
separate `reference_context` rendering. The right Alan-native design is:

- keep `reference_context` unchanged
- update only `summary` and retained `messages`
- let `Tape::prompt_view()` continue to prepend `reference_context` automatically

Trigger point:

- after assistant output and tool results have been written to tape
- before the next LLM request in a continuing turn
- only when the turn is going to continue

Suggested integration point:

- `turn_executor.rs`, in the loop path where tool orchestration returns
  `ContinueTurnLoop`

Suggested state additions:

```rust
struct TurnState {
    // existing fields...
    compactions_this_turn: u32,
    last_compaction_prompt_tokens: Option<usize>,
}
```

Safety rules:

- set a small per-turn compaction cap, for example `1` or `2`
- only compact when the projected prompt is still above threshold
- avoid re-compacting when the previous compaction did not materially shrink prompt size

Mode differences:

- `Manual`: explicit user request, optional focus
- `AutoPreTurn`: happens before new user input is added
- `AutoMidTurn`: happens only to preserve current-turn continuity

### PR4: Failure Degradation and Memory Coordination

Goal: make compaction failure predictable and user-visible.

Changes:

- Introduce soft and hard trigger thresholds:
  - `soft_threshold`: optionally flush memory first
  - `hard_threshold`: compact immediately
- On compaction failure:
  - preserve the original tape
  - emit `Event::Warning`
  - do not silently continue as if compaction succeeded
- Add a degraded fallback path:
  - trim oversized tool outputs from the summarize region
  - preserve recent semantic spans
  - keep existing summary if a new one cannot be produced
- After repeated failures, emit a stronger warning recommending a new session.

This phase should align runtime behavior with `docs/spec/compaction_contract.md`.

## Prompt Strategy

Keep the existing compaction prompt as the baseline and extend it by mode.

Manual mode additions:

- preserve user-provided focus if present
- bias toward retaining todos, constraints, and requested deliverables

Mid-turn mode additions:

- emphasize active execution continuity
- preserve:
  - current objective
  - currently active tool chain
  - unresolved errors
  - next executable step
  - critical identifiers such as paths, IDs, and commands

The summary should remain concise and structured, but should be optimized for immediate reuse by
the next LLM request, not just for human readability.

## Rollout and Recovery

Rollout should eventually make compaction causality reconstructible:

- what triggered compaction
- what was summarized
- what was retained
- whether retries or degraded fallback occurred

Recovery requirements:

- old `CompactedItem { message, timestamp }` records must remain readable
- new records should be loadable even if some fields are missing
- replay should preserve the latest summary and latest reference-context revision snapshot

## Testing Plan

### Unit Tests

- `Tape`:
  - semantic-span extraction
  - compact retention keeps whole spans
  - summary ordering remains `reference_context -> summary -> messages`
  - compaction token estimate shrinks after compaction
- compaction subsystem:
  - manual focus propagates into prompt construction
  - previous summary is incorporated into new compaction input
  - trimming retry preserves context messages and drops oldest non-context messages first

### Integration Tests

- manual compact:
  - produces summary
  - records audit metadata
  - preserves recent semantic spans
- auto pre-turn compact:
  - triggers on token ratio
  - does not mutate `reference_context`
- auto mid-turn compact:
  - triggers during a continuing turn
  - allows the same turn to continue and finish
  - respects per-turn compaction budget
- failure handling:
  - summary failure preserves original tape
  - warning reaches the event stream
  - degraded fallback behaves deterministically

### Compatibility Tests

- old rollout files load without migration
- old `Op::Compact` clients continue to work
- new rollout records can be ignored safely by older code during review/migration

## Suggested Execution Order

1. PR1: protocol and rollout audit substrate
2. PR2: semantic retention windows
3. PR3: mid-turn automatic compaction
4. PR4: failure degradation and memory coordination

This order keeps compatibility work first, then improves history quality, then adds the most
behaviorally significant runtime change.

## Open Questions

1. Should manual compact support only a single `focus: Option<String>`, or a richer structured
   request later?
2. Is turn-span retention sufficient, or do we need explicit tool-batch retention in phase one?
3. Should Alan add dedicated compaction lifecycle events, or rely on warnings plus rollout audit
   until clients demand richer progress UI?
4. What shrinkage threshold should block repeated mid-turn compaction attempts?

## Recommendation

Adopt the plan above without replacing Alan's existing tape model.

The main design principle is:

- borrow Codex's orchestration lessons
- keep Alan's own `reference_context + summary + messages` architecture

That gives Alan the biggest practical improvement with the least conceptual churn.
