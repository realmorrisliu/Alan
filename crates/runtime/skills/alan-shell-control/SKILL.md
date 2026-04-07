---
name: alan-shell-control
description: |
  Inspect and operate the native Alan shell control surface.

  Use this when:
  - The user asks about spaces, tabs, surfaces, panes, focus, or splits
  - Alan needs to decide which pane or surface should receive an action
  - Alan needs to create a space, split a pane, focus a pane, or send text
  - The task depends on understanding the outer terminal app rather than only
    understanding Alan session state

  This skill assumes the local `alan shell` CLI namespace backed by the shell's
  IPC/socket API. If that CLI is unavailable, report that clearly and fall back
  to other available context.

metadata:
  short-description: Control the native Alan terminal shell
  tags: [shell, terminal, panes, spaces, routing, focus]
capabilities:
  required_tools: [bash]
---

# Alan Shell Control

The shell model and the Alan runtime model are separate.

Shell model:

```text
Window -> Space -> Surface -> PaneTree -> Pane
```

Alan model:

```text
Session -> Turn/Run -> Yield/Checkpoint -> Event history
```

A pane may optionally expose Alan metadata, but a pane is not an Alan session.

## Commands

Prefer these commands when available:

```bash
alan shell state
alan shell space list
alan shell surface list
alan shell pane list
alan shell pane snapshot --pane <id>
alan shell pane focus --pane <id>
alan shell pane split --pane <id> --direction <horizontal|vertical>
alan shell pane send-text --pane <id> --text "..."
alan shell space create --cwd <path>
alan shell space open-alan --cwd <path>
alan shell attention inbox
alan shell routing candidates --pane <id>
```

## Workflow

1. Query state before taking action. Do not guess the target pane.
2. Use shell IDs, not visible labels, as the real target identity.
3. When several panes are plausible targets, inspect snapshots and routing
   candidates before mutating anything.
4. Prefer focus changes and explicit pane selection over broadcasting text.
5. Treat `send-text`, split/move/close, and pane or space creation as
   mutations. Confirm the target is correct first.

## Target Selection Rules

When choosing a pane, prioritize in this order:

1. Explicit pane or space ID given by the user
2. Currently focused pane, if it matches the request
3. Pane whose process or metadata matches the task
4. Pane with relevant Alan binding
5. Pane with relevant cwd or title

If two or more panes remain equally plausible, ask the user rather than acting
blindly.

## Snapshot Use

Use pane snapshots to answer questions like:

1. What is happening in this pane right now?
2. Is this pane running `alan-tui`, a shell, or some other process?
3. Is the pane waiting on the user?
4. Is the pane likely the correct destination for the next action?

Prefer summaries, visible viewport data, and explicit metadata over scraping the
entire scrollback by default.

## Safety Rules

1. Query before mutate.
2. Never assume the focused pane is correct without checking.
3. Do not treat shell structure as equivalent to Alan session structure.
4. If `alan shell` is unavailable, say so explicitly.
5. If the shell reports stale or missing state, re-query before acting.
