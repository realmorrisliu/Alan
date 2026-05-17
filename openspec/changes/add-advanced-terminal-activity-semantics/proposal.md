## Why

Alan's macOS terminal already projects title, cwd, attention, active task, and
surface state into pane metadata, but advanced terminal workflows now need a
durable contract for richer activity, semantic command boundaries, and a global
quick terminal Peak surface. Without a draft owner, Ghostty-style progress,
Warp-style agent status, prompt navigation, command-output browsing, and
summonable quick-terminal work can drift into unrelated UI fixes.

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
- Add semantic terminal requirements for prompt/command boundaries, current
  command/output ranges, prompt navigation, copy-last-output, and search/browse
  flows that build on shell integration and terminal surface state.
- Add a quick terminal draft contract for a detached global macOS Peak window
  that can be summoned from any macOS Space, reused as a single instance, and
  promoted into Alan spaces/tabs without creating a second terminal runtime
  model.
- Keep full Warp-style blocks, Agent View, code review panels, and IDE-like file
  sidebars out of this change.

## Capabilities

### New Capabilities

- `macos-terminal-activity-semantics`: Owns terminal activity state, CLI
  coding-agent status ingestion, semantic command boundaries, prompt navigation,
  command-output actions, and quick terminal behavior.

### Modified Capabilities

- `macos-terminal-runtime-foundation`: Extends runtime metadata projection from
  title/cwd/attention/readiness into structured activity, command, and agent
  state keyed by stable pane identity.
- `macos-terminal-surface-parity`: Extends terminal surface parity from
  scrollback/search/clipboard into semantic command-output browsing and
  prompt-scoped actions.
- `macos-shell-ui-ux-conformance`: Adds UI constraints for lightweight activity
  indicators, progress display, agent status, notification surfaces, semantic
  terminal controls, and quick terminal presentation.
- `macos-shell-workspace-interactions`: Adds quick terminal global
  summon/dismiss, focus restoration, single-instance lifecycle, promotion into
  tabs/spaces, and command ownership semantics.

## Impact

- Apple runtime and Ghostty bridge: `GhosttyLiveHost.swift`,
  `TerminalHostRuntime.swift`, `TerminalRuntimeService.swift`,
  `TerminalRuntimeRegistry.swift`, and metadata snapshot propagation.
- Apple shell model/projection: `ShellModel.swift`, `ShellHostController.swift`,
  `ShellPaneProjectionService.swift`, shell snapshots, persistence, and control
  plane DTOs where activity state becomes observable.
- Apple shell UI: `TerminalPaneView.swift`, `ShellSidebarView.swift`,
  `MacShellRootView.swift`, native commands, notification routing, and quick
  terminal window/panel ownership.
- Tests and contracts: macOS shell contract scripts, focused Swift script tests,
  screenshot/visual review notes for activity/sidebar/titlebar behavior, and
  OpenSpec validation.
