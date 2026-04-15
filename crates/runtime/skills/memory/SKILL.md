---
name: memory
description: |
  Manage persistent memory across sessions for long-running tasks.

  Alan's target direction is runtime-owned pure-text memory bootstrap and recall.
  This skill helps the model work with that file-backed memory intentionally
  when durable updates or deeper inspection are needed.

  Use this when:
  - Resuming work from a previous session
  - Working on a long-running project that spans multiple conversations
  - User references prior decisions, context, or history
  - Important facts, preferences, or decisions need to be preserved

  Alan ships this as a built-in capability package.

metadata:
  short-description: Persistent memory across sessions
  tags: [memory, persistence, long-running, context-management]
capabilities:
  required_tools: [read_file, write_file, edit_file, bash]
---

# Memory Skill

Manage durable, file-based memory that persists across sessions.

For the active target contract, see `docs/spec/pure_text_memory_contract.md`.

## Memory Layout

All memory files live under the active workspace Alan state directory's `memory/` folder.

Examples:

```text
repo workspace:    .alan/memory/
default workspace: memory/
```

Layout:

```text
memory/
├── USER.md                # Stable, user-confirmed identity/preferences
├── MEMORY.md              # Stable workspace-level semantic memory
├── handoffs/LATEST.md     # Most recent cross-session continuation note
├── daily/YYYY-MM-DD.md    # Daily incremental work log
├── sessions/...           # Curated per-session summaries
├── topics/<slug>.md       # Long-lived topic pages
└── inbox/...              # Candidate memory awaiting promotion
```

Do not assume the current workspace always has a visible `.alan/` prefix in relative paths.
Resolve the active memory location from the runtime/workspace context when possible.
Treat `USER.md` as stable, user-confirmed identity/preference memory only.
Treat `MEMORY.md` as a curated index, not a dumping ground for chronology.
Put evolving work state in handoffs, session summaries, topic pages, inbox entries, or daily notes
depending on durability and confirmation level.

## Session Start Protocol

When starting a new session (especially if resuming prior work):

1. **Prefer runtime bootstrap context first**.
   The target design is for runtime to inject a bootstrap bundle built from:
   - `USER.md`
   - `MEMORY.md`
   - `handoffs/LATEST.md`
   - the newest daily note

2. **Do not re-read all memory files by default** if the needed context is already present in the
   prompt.

3. **Inspect memory files with tools only when needed**, for example when:
   - recall seems incomplete or inconsistent,
   - you need to verify the exact on-disk content before editing,
   - the user asks about older work that likely lives in session summaries or topic pages.

4. **Prefer narrow reads before broad search**:
   - identity/preferences: `USER.md`
   - current direction: `handoffs/LATEST.md`
   - stable workspace facts: `MEMORY.md`
   - recurring subject: `topics/<slug>.md`
   - recent chronology: latest daily note or recent session summary

## Session End Protocol

Before ending a session or when wrapping up significant work:

1. **Update stable user context**:
   - If the user explicitly asks you to remember stable identity or preference details across
     sessions, update `USER.md`.
   - Do not write guessed personality traits, inferred preferences, or temporary current focus to
     `USER.md`.

2. **Update `MEMORY.md` or a topic page** with new stable key information:
   - New decisions made
   - Important discoveries or constraints
   - Reusable workspace conventions
   - Long-lived references

3. **Append a dated work log** to `{active_memory_dir}/daily/YYYY-MM-DD.md`:
   ```markdown
   ## {timestamp}

   ## What was done
   - ...

   ## Key decisions
   - ...

   ## Next steps
   - ...

   ## Open questions
   - ...
   ```

Automatic runtime note:
- Soft-threshold pre-compaction memory flush may also append a structured entry to the same daily
  note file.
- That automatic flush is meant to preserve durable blockers/constraints before L0 compaction.
- Keep `MEMORY.md` curated and stable; do not treat automatic flush output as a replacement for
  maintaining the long-lived index.

4. **Let runtime own the session-handoff surfaces**.
   Target direction:
   - runtime writes/refreshes `handoffs/LATEST.md`
   - runtime writes a curated session summary under `sessions/`
   - runtime may stage candidate memory under `inbox/`

5. **Ensure clean state**: Code should compile, tests should pass, no half-done work.

## MEMORY.md Format

Keep MEMORY.md structured and concise:

```markdown
# Project Memory

## Project Context
Brief description of the project, its goals, and current state.

## Key Decisions
- [2026-02-25] Decision X: chose approach A because...
- [2026-02-24] Decision Y: ...

## Durable Constraints
- Stable constraints that persist across sessions

## Architecture Notes
- Key design patterns in use
- Important file locations
- Non-obvious dependencies

## Topic Index
- topic slug -> why it matters
```

## Rules

1. **Read before write**: Always read existing memory before making changes
2. **Append, don't overwrite**: Add to sections rather than replacing them
3. **Timestamp entries**: Key decisions and changes should have dates
4. **Be concise**: Memory should be scannable, not verbose
5. **Don't duplicate git**: Don't repeat what's already in git history
6. **Respect privacy**: Don't store sensitive information (API keys, passwords)
7. **Keep `USER.md` narrow**: Store only stable, user-confirmed identity/preferences there
8. **Prefer topic pages over bloat**: If one subject grows large or recurs across sessions, move it
   into `topics/<slug>.md` and keep only the summary/index link in `MEMORY.md`
9. **Use inbox-style staging when unsure**: Useful but not fully confirmed information belongs in
   candidate memory or daily/session notes before promotion into stable memory
