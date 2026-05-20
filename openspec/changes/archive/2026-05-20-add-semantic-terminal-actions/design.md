## Context

`add-advanced-terminal-activity-semantics` shipped the A-group activity model:
activity ingestion, freshness, sidebar projection, pane-title projection,
accessibility, and notification routing. During that design pass, semantic
terminal behavior was explicitly kept lighter than Warp-style command blocks and
deferred until the activity foundation was stable.

This change owns that deferred B-group: prompt/command boundaries and the
focused actions that become possible when those boundaries are reliable.

## Goals / Non-Goals

**Goals:**

- Model prompt marks, command start/end, command text, output ranges, cwd, exit
  status, and timestamps as pane-scoped semantic terminal metadata.
- Keep semantic command state reliability explicit so Alan can disable actions
  when shell, tmux, SSH, alternate-screen, or application-mode constraints make
  boundaries unavailable.
- Add command-aware actions through `Go to or Command...` for previous/next
  prompt, copy last output, and search last output.
- Reuse pane-scoped terminal search and clipboard ownership.
- Preserve normal terminal rendering, scrollback, focus, and process state.

**Non-Goals:**

- No Warp-style visible command blocks.
- No command browser or persistent output segmentation in the MVP.
- No long-term command history database.
- No guessing output ranges from visible screen text.
- No quick terminal Peak behavior; that is owned by `add-quick-terminal-peak`.

## Decisions

### 1. Store semantic command state per pane

Alan should attach command-boundary state to the terminal pane that produced it.
The state is runtime/session-local metadata, not a workspace history database.
It can contain recent `CommandSegment` records with prompt range, command range,
output range, command text, cwd, exit status, started/ended timestamps, and a
reliability state.

This keeps split panes independent and avoids making semantic data a global
terminal service.

### 2. Gate actions on reliability

Command-aware actions should appear only when Alan has reliable boundaries for
the focused pane. If boundaries are unavailable, stale, or invalidated by
terminal mode, Alan should fall back to ordinary scrollback search, selection
copy, visible-range copy, and normal scrollback navigation.

The action surface must not present precise "last command output" behavior when
it would be guessing from screen text.

### 3. Keep the MVP action-only

The first semantic terminal slice should add actions, not new persistent visual
chrome. `Go to or Command...` is the right default surface because it keeps the
terminal first while making command-aware affordances discoverable.

Visible command blocks, command browsers, and segmented output views can be
evaluated later if user workflows prove they are needed.

### 4. Reuse terminal search ownership

Search-last-output should reuse pane-scoped terminal search ownership and focus
return behavior. A command-output scoped search is a bounded variant of terminal
search, not a global search panel.

This avoids duplicating focus, dismissal, and match-navigation behavior.
