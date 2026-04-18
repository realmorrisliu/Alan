# Memory Architecture

> Status: mixed current/target contract.
>
> The active target direction for Alan memory is the pure-text contract in
> [`pure_text_memory_contract.md`](./pure_text_memory_contract.md). This
> document is the shorter architectural summary: what memory is for, how the
> layers relate, and which pieces are already present versus still target-only.

## Goal

Give Alan durable continuity across turns and sessions without requiring hidden
provider memory or external indexing infrastructure.

The memory architecture must make four things simultaneously true:

1. Alan can continue active work without re-deriving everything from raw tape.
2. Alan can recall what happened in earlier sessions.
3. Alan can preserve stable user/workspace knowledge without bloating prompts.
4. Operators can inspect and debug the whole system with ordinary files.

## Core Principles

1. **Files are the source of truth.**
2. **Runtime owns bootstrap and recall.**
3. **Curated summaries are more important than broad raw search.**
4. **Stable memory and past-session recall are different layers.**
5. **Observation and confirmation must remain distinct.**

## Current Implementation Snapshot

Implemented in the current tree:

1. L0 execution memory persists through tape and rollout.
2. Basic workspace memory exists under the active workspace Alan state
   directory's `memory/` folder.
3. Automatic pre-compaction memory flush writes durable context to a daily note
   surface before soft-threshold compaction.
4. Automatic confirmed-turn memory promotion uses a model-mediated structured
   write plan instead of a brittle phrase parser.
5. Latest memory-flush attempt state is recoverable via replay, thread reads,
   reconnect snapshots, and rollout fallback.

Still incomplete or target-only:

1. Runtime-owned working-memory files are not yet the primary continuity
   mechanism.
2. Session summaries, handoffs, topic pages, and inbox/promotion flows are not
   yet fully implemented as first-class runtime surfaces.
3. Pre-turn recall is still too dependent on model initiative rather than
   deterministic runtime bootstrap and routing.

## Memory Layers

### L0: Working Memory

- Carrier: runtime-owned session-local text plus live in-memory state
- Lifecycle: current session only
- Purpose: maintain immediate continuity and task focus

Working memory should hold the current goal, active subgoals, confirmed
constraints, pending verification, and the most recent recall hits that still
matter. It is not a durable knowledge base.

### L1: Episodic Memory

- Carrier: handoffs, session summaries, and daily notes
- Lifecycle: cross-session
- Purpose: answer "what happened", "what were we doing", and "what remains"

Episodic memory is Alan's pure-text answer to long-horizon session recall. It
must be readable by both runtime and humans without replaying whole JSONL logs.

### L2: Semantic Memory

- Carrier: `USER.md`, `MEMORY.md`, topic pages, and candidate/inbox entries
- Lifecycle: workspace-level
- Purpose: preserve stable user facts, reusable workspace facts, decisions, and
  conventions

Semantic memory is durable knowledge, not recent chronology.

### Procedural Memory

- Carrier: runtime/system prompts, workspace persona, and skills
- Lifecycle: agent-definition-level
- Purpose: tell Alan how to behave

Procedural memory must remain separate from user identity and session history.

## On-Disk Surfaces

The target pure-text layout is:

```text
.alan/memory/
├── USER.md
├── MEMORY.md
├── handoffs/LATEST.md
├── working/<session-id>.md
├── daily/YYYY-MM-DD.md
├── sessions/YYYY/MM/DD/<session-id>.md
├── topics/<slug>.md
└── inbox/YYYY/MM/DD/<entry-id>.md
```

Role split:

1. `USER.md` stores stable user identity/preferences only.
2. `MEMORY.md` stores stable workspace-level semantic memory.
3. `LATEST.md` is the most recent cross-session continuation note.
4. `working/` is runtime-owned and session-local.
5. `daily/` is append-heavy chronology and compaction-preservation surface.
6. `sessions/` stores one curated summary per finished session.
7. `topics/` prevents `MEMORY.md` from becoming a dumping ground.
8. `inbox/` stages useful but not-yet-promoted observations.

## Read Boundary

The target read path is:

1. bootstrap `USER.md`, `MEMORY.md`, `LATEST.md`, and the newest daily note at
   session start,
2. route per-turn recall by intent,
3. prefer curated markdown surfaces before any raw rollout search,
4. build a small source-labeled recall bundle for injection.

Retrieval order should narrow before it broadens:

1. exact file reads,
2. curated lexical search over topic pages and session summaries,
3. recent daily notes,
4. raw rollout grep only as a fallback.

This architecture intentionally avoids making semantic search, vector stores, or
SQLite a requirement.

## Write Boundary

The target write path is:

1. **confirmed writes** into `USER.md`, `MEMORY.md`, or topic pages,
2. **observed captures** into inbox entries or daily notes,
3. **session finalization** into a session summary plus handoff,
4. **consolidation** that promotes or prunes memory over time.

Important distinctions:

1. not every useful observation should go directly into stable memory,
2. not every past session detail belongs in `MEMORY.md`,
3. working memory must never masquerade as long-term memory.
4. automatic promotion should be model-mediated and schema-validated, with
   runtime retaining sole authority over durable writes.

## Relationship to Compaction

Compaction and memory are adjacent but different concerns:

1. compaction keeps the current session within context limits,
2. memory preserves what should survive beyond the current context window.

The automatic pre-compaction memory flush remains useful, but it should be seen
as one write source among several. Its natural landing zone is the daily-note
surface, not direct mutation of stable user memory.

## Relationship to Rollout

Tape and rollout remain the high-fidelity execution record.

Memory files are curated projections derived from that record:

1. rollout is the raw log,
2. session summaries and handoffs are distilled episodic memory,
3. `USER.md`, `MEMORY.md`, and topic pages are distilled semantic memory.

The architecture therefore depends on both surfaces:

1. rollout for fidelity and audit,
2. curated text memory for continuity and retrieval quality.

## Non-Goals

This architecture does not aim to:

1. preserve every turn verbatim in stable memory,
2. solve recall purely by widening lexical search,
3. require hidden provider memory,
4. require vector databases or SQLite indexes for baseline correctness.

## Acceptance Criteria

1. Alan can answer identity and prior-work questions from pure-text memory when
   the evidence already exists on disk.
2. Runtime bootstrap and recall no longer depend on the model deciding to
   inspect the right files.
3. Working, episodic, semantic, and procedural memory have distinct
   responsibilities.
4. Stable memory remains small and curated because chronology is pushed into
   handoffs, session summaries, and daily notes.
5. The entire memory system remains inspectable with normal file reads and
   lexical search tools.
