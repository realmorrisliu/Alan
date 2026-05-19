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
- [x] 1.5 Document Codex as the first CLI coding-agent adapter target unless
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

- [x] 4.1 Add a small adapter boundary for reliable CLI agent events and unknown
  agent fallback state.
- [x] 4.2 Implement the Codex adapter slice from task 1.5, or update the draft
  with evidence if another first adapter is selected.
- [x] 4.3 Sanitize agent event payloads so default UI never exposes raw hook
  payloads, raw session IDs, or implementation event names.
- [x] 4.4 Add tests for supported agent running, needs-input, complete, error,
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
- [x] 5.5 Project pane title-bar accessories from pane-local detail: activity,
  worktree/cwd, branch, process, and non-duplicated agent/Alan state.
- [x] 5.6 Add low-noise notification routing for user-input-required, error, and
  long-command-complete events according to the task 1.4 policy.
- [x] 5.7 Add accessibility labels or values for activity state on pane and tab
  affordances.
- [x] 5.8 Add visual review evidence for focused progress, background agent
  needs-input, command failure, cleared activity showing worktree/branch
  fallback, split leading topology, and hover close overlay.

## 6. Follow-Up Split

- [x] 6.1 Move semantic command-boundary and command-output action work to
  `add-semantic-terminal-actions`.
- [x] 6.2 Move global quick-terminal Peak work to `add-quick-terminal-peak`.
- [x] 6.3 Keep this change scoped to the completed activity, sidebar, pane-title,
  accessibility, and low-noise notification A-group.

## 7. Verification And Archive Readiness

- [x] 7.1 Run focused Swift script tests for activity model, projection, Ghostty
  source mapping, and CLI agent adapters.
- [x] 7.2 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 7.3 Run the relevant macOS app build command for the touched Apple client
  targets.
- [x] 7.4 Run `git diff --check`.
- [x] 7.5 Run `openspec validate add-advanced-terminal-activity-semantics --type
  change --strict --json`.
- [x] 7.6 Run `openspec validate --all --strict --json`.
- [x] 7.7 Sync accepted delta requirements into `openspec/specs/` before archive.
