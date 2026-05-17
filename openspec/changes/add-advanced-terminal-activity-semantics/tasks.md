## 1. A-Group Refinement Gate

- [x] 1.1 Audit current terminal metadata sources in `GhosttyLiveHost`,
  `TerminalHostRuntime`, `TerminalRuntimeService`, `ShellHostController`, and
  `ShellPaneProjectionService`.
- [x] 1.2 Finalize normalized activity state names, source kinds, source-first
  display labels, priority order, freshness rules, and user-facing labels for
  progress, command, Alan, and CLI agent states.
- [x] 1.3 Finalize sidebar row projection details for title, activity-or-context
  fallback, progress rail, leading topology/kind slot, and hover close overlay.
- [x] 1.4 Encode default notification policy for focused, visible, background,
  unfocused, successful, failed, and user-input-required activity.
- [ ] 1.5 Document Codex as the first CLI coding-agent adapter target unless
  implementation evidence shows another adapter is lower risk.

## 2. Activity Model And Projection

- [x] 2.1 Add pane-scoped terminal activity value types for source, status,
  priority, progress, command outcome, agent metadata, freshness, timestamps,
  and source-first display labels.
- [x] 2.2 Extend terminal metadata snapshots and shell projection paths to carry
  normalized activity without removing existing title/cwd/attention fields.
- [x] 2.3 Add deterministic activity merge and priority logic for concurrent
  progress, command completion, bell, Alan, and agent signals, including
  tab-level highest-priority activity selection.
- [x] 2.4 Add unit/script coverage for activity merge order, stale progress,
  focused successful command completion exclusion from sidebar activity,
  progress-over-running priority, and user-input-required priority.
- [x] 2.5 Add freshness coverage for progress stale/clear, command failure
  demotion after focus or timeout, brief bell, persistent exited state, and
  persistent needs-input/error until replaced.

## 3. Ghostty And Command Activity Sources

- [x] 3.1 Map `GHOSTTY_ACTION_PROGRESS_REPORT` into structured activity including
  determinate, indeterminate, paused, error, clear, and stale states.
- [x] 3.2 Map `GHOSTTY_ACTION_COMMAND_FINISHED` into command-completion activity
  with success/failure status and duration when available.
- [x] 3.3 Keep bell and child-exit attention compatible with the normalized
  activity model without changing existing terminal overlay semantics.
- [x] 3.4 Add focused tests for progress update, progress clear, command success,
  command failure, and stale-progress timeout behavior.

## 4. CLI Coding-Agent Activity

- [ ] 4.1 Add a small adapter boundary for reliable CLI agent events and unknown
  agent fallback state.
- [ ] 4.2 Implement the Codex adapter slice from task 1.5, or update the draft
  with evidence if another first adapter is selected.
- [ ] 4.3 Sanitize agent event payloads so default UI never exposes raw hook
  payloads, raw session IDs, or implementation event names.
- [ ] 4.4 Add tests for supported agent running, needs-input, complete, error,
  malformed payload, and unsupported-agent fallback.

## 5. Activity UI And Notifications

- [x] 5.1 Render normalized activity in pane title-bar accessories without
  resizing pane title bars or terminal canvases.
- [x] 5.2 Refactor sidebar tab rows to show a leading topology/kind slot, title
  line, activity-or-worktree/branch secondary line, optional same-source
  progress rail, and hover-only close overlay.
- [x] 5.3 Move split topology into the leading slot for split tabs and make
  activation cycle focus through panes in stable order.
- [x] 5.4 Keep single-pane tabs on semantic kind or supported agent icons in the
  leading slot.
- [ ] 5.5 Project pane title-bar accessories from pane-local detail: activity,
  worktree/cwd, branch, process, and non-duplicated agent/Alan state.
- [x] 5.6 Add low-noise notification routing for user-input-required, error, and
  long-command-complete events according to the task 1.4 policy.
- [x] 5.7 Add accessibility labels or values for activity state on pane and tab
  affordances.
- [ ] 5.8 Add visual review evidence for focused progress, background agent
  needs-input, command failure, cleared activity showing worktree/branch
  fallback, split leading topology, and hover close overlay.

## 6. B-Group Semantic Terminal Draft Follow-Up

- [ ] 6.1 Audit which command-boundary and prompt-mark signals are available from
  the embedded Ghostty surface and existing shell integration.
- [ ] 6.2 Define pane-scoped `CommandSegment` storage for prompt ranges, command
  ranges, output ranges, command text, cwd, exit status, started/ended
  timestamps, and reliable/unavailable boundary state.
- [ ] 6.3 Expose command-aware actions through `Go to or Command...` only when
  reliable boundaries exist: jump previous prompt, jump next prompt, copy last
  command output, and search last command output.
- [ ] 6.4 Fall back to ordinary scrollback search, selection copy, visible-range
  copy, and normal scrollback navigation when reliable boundaries are missing.
- [ ] 6.5 Keep the MVP action-only: no command browser, no visible command blocks,
  no persistent output segmentation, and no long-term command history database.
- [ ] 6.6 Reuse pane-scoped search ownership for search-last-output and fallback
  scrollback search.

## 7. Quick Terminal Draft Follow-Up

- [ ] 7.1 Add a single global quick-terminal slot that reuses one terminal
  runtime across hide/show and summons it onto the current macOS Space/display.
- [ ] 7.2 Add shared shell command routing for the configurable global toggle
  shortcut, draft default `Option+Space`, plus explicit show, hide, focus, close,
  and promote commands.
- [ ] 7.3 Present the quick terminal through a detached native macOS Peak window
  that does not depend on or raise Alan's main window and that uses normal
  terminal runtime service ownership.
- [ ] 7.4 Preserve quick terminal runtime state across hide/show, keep `Esc`
  routed to the terminal, avoid focus-loss auto-hide, and tear down the runtime
  only through explicit close semantics.
- [ ] 7.5 Apply quick-terminal cwd creation rules: existing instance cwd,
  focused Alan pane cwd, last quick-terminal cwd, then home.
- [ ] 7.6 Implement `Open in Space` promotion as a move into the selected Alan
  Space/tab that hides the Peak and clears the global quick slot.
- [ ] 7.7 Add focus, display/Space placement, hide/show, close, promote, and
  hidden-activity notification tests.

## 8. Verification And Archive Readiness

- [ ] 8.1 Run focused Swift script tests for activity model, projection, Ghostty
  source mapping, and CLI agent adapters.
- [ ] 8.2 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 8.3 Run the relevant macOS app build command for the touched Apple client
  targets.
- [ ] 8.4 Run `git diff --check`.
- [ ] 8.5 Run `openspec validate add-advanced-terminal-activity-semantics --type
  change --strict --json`.
- [ ] 8.6 Run `openspec validate --all --strict --json`.
- [ ] 8.7 Sync accepted delta requirements into `openspec/specs/` before archive.
