## Why

Alan's macOS terminal already projects title, cwd, attention, active task, and
surface state into pane metadata, but advanced terminal workflows now need a
durable contract for richer activity and terminal-agent attention. Without a
focused owner, Ghostty-style progress, command completion, CLI coding-agent
status, sidebar tab activity, and notification policy can drift into unrelated
UI fixes.

## What Changes

- Add a unified terminal activity contract for progress, command completion,
  long-running foreground commands, CLI coding-agent lifecycle state, and
  attention/notification policy.
- Define how activity is projected into pane title-bar accessories, sidebar tab
  metadata, accessibility, and optional system notifications without introducing
  dashboard panels or notification-dot clutter.
- Redesign sidebar tab rows as richer attention/identity rows that show a tab
  title plus either the highest-priority tab activity or a worktree/branch
  context fallback, with hover-only close controls.
- Defer semantic command/output actions to `add-semantic-terminal-actions`.
- Defer global Peak quick-terminal behavior to `add-quick-terminal-peak`.
- Keep full Warp-style blocks, Agent View, code review panels, and IDE-like file
  sidebars out of this change.

## Capabilities

### New Capabilities

- `macos-terminal-activity-semantics`: Owns terminal activity state, CLI
  coding-agent status ingestion, activity freshness, priority, sidebar
  projection, pane-title projection, and notification behavior.

### Modified Capabilities

- `macos-terminal-runtime-foundation`: Extends runtime metadata projection from
  title/cwd/attention/readiness into structured activity and agent
  state keyed by stable pane identity.
- `macos-shell-ui-ux-conformance`: Adds UI constraints for lightweight activity
  indicators, progress display, agent status, notification surfaces, and
  sidebar tab activity rows.

## Impact

- Apple runtime and Ghostty bridge: `GhosttyLiveHost.swift`,
  `TerminalHostRuntime.swift`, `TerminalRuntimeService.swift`,
  `TerminalRuntimeRegistry.swift`, and metadata snapshot propagation.
- Apple shell model/projection: `ShellModel.swift`, `ShellHostController.swift`,
  `ShellPaneProjectionService.swift`, shell snapshots, persistence, and control
  plane DTOs where activity state becomes observable.
- Apple shell UI: `TerminalPaneView.swift`, `ShellSidebarView.swift`,
  `MacShellRootView.swift`, native commands, and notification routing.
- Tests and contracts: macOS shell contract scripts, focused Swift script tests,
  screenshot/visual review notes for activity/sidebar/titlebar behavior, and
  OpenSpec validation.
