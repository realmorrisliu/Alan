# Pure-Text Memory Contract

> Status: target pure-text memory V2 contract.
>
> This document defines Alan's active target direction for memory: pure-text,
> file-backed, auditable memory with no required vector database, graph store,
> SQLite index, or external memory service.

## Goal

Make Alan feel meaningfully smarter across turns and across sessions while
keeping the implementation inspectable, portable, and easy to debug with
ordinary files and shell tools.

The target outcome is not "a bigger `MEMORY.md`". The target outcome is a
runtime-owned memory system with clear separation between:

1. **working memory** for the current live session,
2. **episodic memory** for past sessions and handoffs,
3. **semantic memory** for stable user/workspace knowledge,
4. **procedural memory** for behavior rules and learned operating style.

## Non-Goals

This contract does not require or assume:

1. vector embeddings or ANN search,
2. SQLite or any other database-backed recall layer,
3. hidden provider memory as a source of truth,
4. lossless semantic retrieval over arbitrary historical text,
5. storing every past turn as long-lived curated memory.

Rollout JSONL, tape persistence, and raw session storage still exist, but they
are not the same thing as curated memory.

## Core Principles

1. **Files are the source of truth.** Memory must remain inspectable and
   editable with normal tools.
2. **Runtime owns the read path.** Alan must not rely on the model to remember
   to open the right files before it can answer intelligently.
3. **Curated summaries beat raw search.** Text retrieval quality must come
   primarily from better intermediate files, not from ever broader grep scope.
4. **Stable memory and past-session recall are different jobs.** User identity
   and preferences should not be mixed with historical work logs.
5. **Observation is not confirmation.** The system may stage candidate memory
   from evidence without immediately promoting it into stable user/workspace
   memory.
6. **Writes must stay auditable.** Every durable memory change must retain a
   textual source trail.

## Stable Vocabulary

- **Working memory**: runtime-owned, session-local state needed to continue the
  current task.
- **Episodic memory**: compact summaries of what happened in past sessions.
- **Semantic memory**: stable facts about the user, workspace, constraints,
  conventions, and reusable decisions.
- **Procedural memory**: instructions about how Alan should behave, usually
  expressed through prompts, persona files, and skills.
- **Recall bundle**: the bounded block of memory context injected into a turn
  after runtime retrieval.
- **Session summary**: the curated markdown summary for one finished session.
- **Handoff**: the current cross-session continuation note, optimized for "what
  was happening most recently".
- **Topic page**: a curated markdown page that accumulates durable facts and
  decisions for one recurring subject.
- **Inbox entry**: a candidate memory record that has been observed but not yet
  fully promoted into stable memory.
- **Promotion**: moving a candidate or repeated observation into `USER.md`,
  `MEMORY.md`, or a topic page.

## Memory Layers

### L0: Working Memory

- Carrier: `.alan/memory/working/<session-id>.md`
- Lifecycle: session-local
- Owner: runtime, not the model
- Purpose: keep the current execution coherent

Working memory is the closest analogue to human working memory. It is not a
long-lived knowledge store. It should contain only the minimum state needed to
continue the current task without re-deriving everything from tape.

Required content:

1. current top-level goal,
2. active subgoals,
3. confirmed constraints,
4. unresolved questions,
5. pending verifications,
6. recently established facts that matter for the next few turns,
7. recall hits that were injected into the current session and should remain
   live until superseded.

Rules:

1. Working memory must be rewritten by runtime from structured session state.
2. The model must not directly edit `working/<session-id>.md`.
3. Working memory may be discarded after session finalization once a session
   summary and handoff exist.

### L1: Episodic Memory

- Carrier:
  - `.alan/memory/handoffs/LATEST.md`
  - `.alan/memory/sessions/YYYY/MM/DD/<session-id>.md`
  - `.alan/memory/daily/YYYY-MM-DD.md`
- Lifecycle: multi-session, append-heavy
- Purpose: answer "what happened", "what were we doing", and "what remains"

Episodic memory captures experience, not just facts. It is the primary answer
to cross-session continuity when the user asks about prior work.

### L2: Semantic Memory

- Carrier:
  - `.alan/memory/USER.md`
  - `.alan/memory/MEMORY.md`
  - `.alan/memory/topics/<slug>.md`
  - `.alan/memory/inbox/YYYY/MM/DD/<entry-id>.md`
- Lifecycle: workspace-level
- Purpose: preserve stable, reusable knowledge

Semantic memory stores stable knowledge that should remain valuable beyond a
single session. `USER.md` is narrow and identity-centric. `MEMORY.md` is the
workspace/general stable memory index. Topic pages hold denser subject-specific
memory that would otherwise bloat `MEMORY.md`.

### Procedural Memory

Procedural memory is not primarily stored under `.alan/memory/`.

It lives in:

1. runtime/system prompts,
2. workspace persona files,
3. skills,
4. future prompt-optimization outputs if Alan adopts them.

This separation is intentional. Behavioral rules must not be mixed into user
identity or past-session summaries.

## On-Disk Layout

```text
.alan/
└── memory/
    ├── USER.md
    ├── MEMORY.md
    ├── handoffs/
    │   └── LATEST.md
    ├── working/
    │   └── <session-id>.md
    ├── daily/
    │   └── YYYY-MM-DD.md
    ├── sessions/
    │   └── YYYY/MM/DD/<session-id>.md
    ├── topics/
    │   └── <slug>.md
    └── inbox/
        └── YYYY/MM/DD/<entry-id>.md
```

Rules:

1. `USER.md` and `MEMORY.md` are always present or auto-created.
2. `LATEST.md` is rewritten, not append-only.
3. `working/` is runtime-owned and may be garbage-collected after session end.
4. `sessions/`, `daily/`, `topics/`, and `inbox/` are durable memory assets.
5. Raw rollout files remain under `.alan/sessions/` and are not replaced by
   these markdown files.

## File Contracts

### `USER.md`

Purpose: stable, user-confirmed identity and preference memory.

Recommended structure:

```markdown
# User Memory

## Identity
- ...

## Preferences
- ...

## Stable Constraints
- ...

## Source Notes
- ...
```

Rules:

1. Only store stable user facts and durable preferences.
2. Do not store project status, open TODOs, or temporary current focus here.
3. User-authorized external verification counts as confirmable evidence.
   Example: if the user says "look at my website and you'll know who I am",
   facts directly stated there may be promoted into `Identity`.

### `MEMORY.md`

Purpose: stable workspace-level semantic memory.

Recommended structure:

```markdown
# Workspace Memory

## Project Context

## Stable Decisions

## Durable Constraints

## Important References

## Topic Index
```

Rules:

1. `MEMORY.md` stays curated and relatively short.
2. Dense topic material should move into `topics/<slug>.md`.
3. Daily logs and session summaries must not be copied wholesale into
   `MEMORY.md`.

### `working/<session-id>.md`

Purpose: runtime-owned live continuation state.

Required sections:

```markdown
# Working Memory

session_id: ...
updated_at: ...

## Current Goal

## Active Subgoals

## Confirmed Constraints

## Pending Verification

## Open Loops

## Recent Findings

## Active Recall
```

Rules:

1. This file is machine-maintained.
2. The model may observe it through injected prompt context but does not edit it
   directly.
3. This file is not stable memory and must not be used as the only cross-session
   source.

### `handoffs/LATEST.md`

Purpose: fast cross-session bootstrap for "where were we".

Required sections:

```markdown
# Latest Handoff

session_id: ...
updated_at: ...

## Summary

## Current Direction

## Open Loops

## Important References
```

Rules:

1. `LATEST.md` is rewritten on session finalization.
2. It should describe the most recent actionable continuation state.
3. It must be shorter and more current than a full session summary.

### `sessions/YYYY/MM/DD/<session-id>.md`

Purpose: one markdown summary per finished session.

Required frontmatter:

```yaml
---
session_id: ...
created_at: ...
workspace: ...
title: ...
tags: [ ... ]
entities: [ ... ]
source_rollout: ...
---
```

Required sections:

```markdown
# Session Summary

## What Happened

## Key Decisions

## Durable Facts

## Open Loops

## References
```

Rules:

1. Session summaries are the primary pure-text substitute for indexed
   transcript search.
2. The `title`, `tags`, and `entities` fields exist specifically to improve
   lexical recall quality.

### `daily/YYYY-MM-DD.md`

Purpose: append-only same-day work log and compaction preservation surface.

Rules:

1. Daily notes may contain multiple timestamped entries.
2. Automatic pre-compaction flush output lands here, not in `USER.md`.
3. Daily notes are an intermediate memory surface, not the final stable memory
   index.

### `topics/<slug>.md`

Purpose: curated subject memory that survives many sessions.

Required frontmatter:

```yaml
---
title: ...
aliases: [ ... ]
tags: [ ... ]
entities: [ ... ]
updated_at: ...
source_sessions: [ ... ]
---
```

Required sections:

```markdown
# <Topic Title>

## Summary

## Stable Facts

## Key Decisions

## Open Questions

## References
```

Rules:

1. Topic pages are the main long-horizon substitute for semantic search.
2. Aliases and entities must be curated so plain-text search stays effective.
3. Topic pages should aggregate repeated evidence from sessions and daily notes.

### `inbox/YYYY/MM/DD/<entry-id>.md`

Purpose: candidate memory staging area.

Required frontmatter:

```yaml
---
id: ...
kind: user_identity | user_preference | workspace_fact | topic_fact | workflow_rule
status: observed | confirmed | rejected | expired
target: USER.md | MEMORY.md | topics/<slug>.md
confidence: low | medium | high
created_at: ...
updated_at: ...
source_sessions: [ ... ]
---
```

Required body:

```markdown
## Observation

## Evidence

## Promotion Rationale
```

Rules:

1. Inbox entries are durable text records, not hidden internal state.
2. Promotion into stable memory must cite inbox entry ids or source sessions.
3. Rejection and expiration remain visible by status change rather than silent
   deletion.

## Runtime Read Path

### Session Bootstrap

At the start of every new session, runtime must build a bootstrap bundle from:

1. `USER.md`
2. `MEMORY.md`
3. `handoffs/LATEST.md`
4. the newest `daily/YYYY-MM-DD.md` if present

This bootstrap bundle is injected automatically. It is not optional model
behavior and does not rely on the memory skill being selected.

### Pre-Turn Recall Routing

Before generating a response for a user turn, runtime should decide whether
additional recall is needed.

Recall triggers include:

1. explicit references such as "remember", "last time", "之前", "上次", "还记得",
2. identity or preference questions,
3. references to previously discussed projects, people, or recurring topics,
4. a working-memory state that contains unresolved references likely answered by
   episodic or semantic memory.

Recall routing order:

1. **Identity / preferences**:
   - `USER.md`
   - relevant inbox entries targeting `USER.md`
2. **Current project / ongoing work**:
   - `MEMORY.md`
   - `handoffs/LATEST.md`
   - newest daily note
3. **Past-session recall**:
   - recent session summaries
   - recent daily notes
4. **Topic recall**:
   - `topics/*.md`
   - then session summaries mentioning the same aliases/entities
5. **Fallback**:
   - grep recent raw rollout JSONL only when curated markdown did not answer the
     question

### Lexical Retrieval Rules

Pure-text retrieval must prefer deterministic lexical search over broad file
loading.

Search widening order:

1. exact file reads for known targets,
2. exact phrase match,
3. alias/entity/title match,
4. tokenized OR-style lexical search,
5. recency-biased fallback over recent summaries,
6. raw rollout grep as last resort

Implementation may use `rg` or an internal Rust lexical scanner, but the
behavior contract is:

1. search must stay bounded,
2. recall results must include source paths,
3. runtime must compose a small recall bundle instead of dumping whole files.

### Recall Bundle Contract

The injected recall bundle should be bounded and source-labeled, for example:

```text
## Memory Recall

[Source: .alan/memory/USER.md]
- Preferred name: Morris

[Source: .alan/memory/handoffs/LATEST.md]
- Current direction: finish memory spec
```

Rules:

1. Recall bundles must be treated as informational background, not new user
   input.
2. Multiple sources should be merged into one bounded bundle.
3. Source paths are mandatory for auditability.

## Runtime Write Path

### Confirmed Writes

Runtime may promote information directly into stable memory when one of the
following is true:

1. the user explicitly says to remember it,
2. the user directly states the fact as stable identity/preference/constraint,
3. the user explicitly authorizes a source lookup that directly states the fact,
4. the fact is already in stable memory and the new turn is an update/rewrite.

### Observed Captures

When information seems useful but is not yet strong enough for stable memory,
runtime should record it as an inbox entry or daily note instead of dropping it.

Observed captures are appropriate for:

1. likely identity inference from external evidence,
2. inferred workspace conventions that are useful but not yet repeated,
3. provisional blockers or hypotheses,
4. repeated but still unconfirmed behavioral patterns.

### Session Finalization

At session end, runtime should:

1. finalize `working/<session-id>.md`,
2. write `sessions/YYYY/MM/DD/<session-id>.md`,
3. refresh `handoffs/LATEST.md`,
4. append a concise entry to the current daily note,
5. optionally stage inbox entries for unresolved but important observations.

### Consolidation

Pure-text memory quality depends on regular consolidation.

Consolidation responsibilities:

1. merge repeated daily/session facts into topic pages,
2. promote confirmed inbox entries into `USER.md` or `MEMORY.md`,
3. prune or resolve stale open loops in `LATEST.md`,
4. keep `MEMORY.md` short by linking outward to topic pages,
5. mark obsolete candidate entries as rejected or expired instead of silently
   overwriting history.

Consolidation may happen:

1. at explicit session end,
2. during background maintenance,
3. before or after compaction,
4. on explicit user request.

## Relationship to Compaction

The existing pre-compaction memory flush remains valid, but its role changes:

1. it preserves durable context into the daily note surface,
2. it must not be treated as the only memory write path,
3. it must not write directly into `USER.md`,
4. it may create or enrich inbox entries when the output is durable but not yet
   confirmed.

Compaction and memory are related but separate:

1. compaction protects the current session from context overflow,
2. memory preserves information beyond the current session.

## Relationship to Rollout and Tape

1. Tape + rollout remain the high-fidelity execution log.
2. Session summaries, handoffs, and topic pages are curated projections derived
   from that log.
3. Retrieval should prefer curated text files before touching raw rollout.
4. Raw rollout grep is a fallback, not the primary pure-text recall mechanism.

## Privacy, Safety, and Audit

1. Stable memory should not silently absorb sensitive data.
2. Source trails must remain textual and inspectable.
3. User-facing identity memory should favor explicit confirmation or
   user-authorized source verification.
4. Rewrites and deletions should leave a visible trail through summary,
   frontmatter status, or source references.

## Acceptance Criteria

1. Alan can answer "who am I" or "what were we doing" from pure-text memory
   without asking the user to repeat themselves when the evidence already
   exists.
2. Runtime, not the model alone, performs session bootstrap and pre-turn recall.
3. `USER.md`, `MEMORY.md`, handoffs, session summaries, topic pages, and inbox
   entries have distinct responsibilities.
4. Working memory is runtime-owned and does not masquerade as long-term memory.
5. Cross-session continuity works without any required vector database or
   SQLite service.
6. The system remains debuggable with ordinary file inspection and `rg`.
