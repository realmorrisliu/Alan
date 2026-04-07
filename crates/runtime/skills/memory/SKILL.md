---
name: memory
description: |
  Manage persistent memory across sessions for long-running tasks.

  Automatically read memory at session start and write updates at session end.
  This enables coherent work that spans multiple context windows.

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

## Memory Layout

All memory files live under `.alan/memory/` in the workspace:

```
.alan/memory/
├── MEMORY.md              # Persistent knowledge index (survives across all sessions)
└── YYYY-MM-DD.md          # Daily incremental work log
```

## Session Start Protocol

When starting a new session (especially if resuming prior work):

1. **Read persistent memory**:
   ```
   read_file .alan/memory/MEMORY.md
   ```

2. **Check recent daily notes**:
   ```
   bash ls -lt .alan/memory/*.md | head -5
   ```
   Then read the most recent daily note for context.

3. **Check git history** for recent changes:
   ```
   bash git log --oneline -10
   ```

4. **Synthesize context**: Combine memory + daily notes + git log to understand:
   - What was the user working on?
   - What decisions were already made?
   - What remains to be done?

## Session End Protocol

Before ending a session or when wrapping up significant work:

1. **Update MEMORY.md** with any new key information:
   - New decisions made
   - Important discoveries or constraints
   - User preferences learned
   - Current project state

2. **Append a dated work log** to `.alan/memory/YYYY-MM-DD.md`:
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

3. **Ensure clean state**: Code should compile, tests should pass, no half-done work.

## MEMORY.md Format

Keep MEMORY.md structured and concise:

```markdown
# Project Memory

## Project Context
Brief description of the project, its goals, and current state.

## Key Decisions
- [2026-02-25] Decision X: chose approach A because...
- [2026-02-24] Decision Y: ...

## In Progress
- Current task or feature being worked on
- Known blockers or issues

## User Preferences
- Communication style, priorities, constraints

## Architecture Notes
- Key design patterns in use
- Important file locations
- Non-obvious dependencies
```

## Rules

1. **Read before write**: Always read existing memory before making changes
2. **Append, don't overwrite**: Add to sections rather than replacing them
3. **Timestamp entries**: Key decisions and changes should have dates
4. **Be concise**: Memory should be scannable, not verbose
5. **Don't duplicate git**: Don't repeat what's already in git history
6. **Respect privacy**: Don't store sensitive information (API keys, passwords)
