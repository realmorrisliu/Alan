## 1. Signal And Metadata Audit

- [x] 1.1 Audit which command-boundary and prompt-mark signals are available from
  the embedded Ghostty surface and existing shell integration.
- [x] 1.2 Document reliability limits for shell integration, tmux, SSH,
  alternate-screen applications, terminal mouse mode, and application-owned
  screen state.
- [x] 1.3 Confirm the focused pane metadata path that should carry semantic
  command state without disturbing existing title/cwd/activity projection.

## 2. Semantic Command Model

- [x] 2.1 Define pane-scoped `CommandSegment` storage for prompt ranges, command
  ranges, output ranges, command text, cwd, exit status, started/ended
  timestamps, and reliable/unavailable boundary state.
- [x] 2.2 Add invalidation and fallback behavior for unavailable or stale
  command boundaries.
- [x] 2.3 Ensure semantic command state remains runtime/session-local metadata
  rather than a long-term command history database.

## 3. Command-Aware Actions

- [x] 3.1 Expose command-aware actions through `Go to or Command...` only when
  reliable boundaries exist: jump previous prompt, jump next prompt, copy last
  command output, and search last command output.
- [x] 3.2 Implement previous/next prompt navigation without changing shell focus,
  split layout, or command history.
- [x] 3.3 Implement copy-last-output through pane-owned buffer ranges and the
  pasteboard without sending input to the terminal process.
- [x] 3.4 Reuse pane-scoped search ownership for search-last-output and fallback
  scrollback search.
- [x] 3.5 Fall back to ordinary scrollback search, selection copy, visible-range
  copy, and normal scrollback navigation when reliable boundaries are missing.
- [x] 3.6 Keep the MVP action-only: no command browser, no visible command
  blocks, no persistent output segmentation.

## 4. Verification

- [x] 4.1 Add focused tests for command segment storage, prompt navigation,
  copy-last-output, search-last-output, unavailable-boundary fallback, and
  alternate-screen suppression.
- [x] 4.2 Run focused Apple shell/terminal scripts covering changed runtime,
  command, search, and clipboard paths.
- [x] 4.3 Run the relevant macOS app build command or document any local blocker.
- [x] 4.4 Run `openspec validate add-semantic-terminal-actions --type change --strict --json`.
- [x] 4.5 Run `openspec validate --all --strict --json`.
- [x] 4.6 Run `git diff --check`.

## 5. Archive Readiness

- [x] 5.1 Review default UI surfaces to confirm semantic actions do not add
  persistent command blocks, browsers, or debug chrome.
- [x] 5.2 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [x] 5.3 Archive the completed OpenSpec change after implementation merges.
