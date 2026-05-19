## 1. Current-State Verification

- [x] 1.1 Reproduce or inspect the focused-title readability problem in the current `ShellPaneTitleBarView` styling.
- [x] 1.2 Confirm the current accessory projection order and identify whether `ShellModel.swift` needs explicit layout priority metadata or whether `TerminalPaneView.swift` can own the presentation priority.

## 2. Title-Bar Visual And Layout Implementation

- [x] 2.1 Update `ShellPaneTitleBarView` so the row uses the terminal surface background rather than selected/unselected terminal chrome material fills.
- [x] 2.2 Increase title and accessory foreground contrast for focused and unfocused panes while keeping typography compact and terminal-first.
- [x] 2.3 Replace fixed-width or infinite-width title-bar slots with left-to-right fit-content layout for title, activity/status, cwd/worktree, branch, process/alan, and close.
- [x] 2.4 Implement staged narrow-width fallback so lower-priority accessories degrade from text plus icon to icon-only or hidden before title text or close disappear.
- [x] 2.5 Ensure the title always remains text with middle truncation and never becomes icon-only.
- [x] 2.6 Preserve title-bar click-to-focus, pane-scoped close routing, accessibility labels, and help text.
- [x] 2.7 Preserve terminal canvas hit-testing, selection, mouse reporting, right click, scrollback, and split geometry below the title bar.

## 3. Contract Checks And Focused Tests

- [x] 3.1 Update focused shell contract checks to guard against selected/unselected title-bar overlay fills, fixed-width accessory regressions, and title icon-only fallback.
- [x] 3.2 Add or update focused runtime metadata tests for accessory priority, fallback ordering, and title persistence under narrow-layout assumptions.
- [x] 3.3 Run `clients/apple/scripts/test-shell-runtime-metadata.sh`.
- [x] 3.4 Run `clients/apple/scripts/test-shell-split-model.sh` if pane close or split targeting code changes. Not applicable: pane close and split targeting code did not change.
- [x] 3.5 Run `clients/apple/scripts/test-terminal-surface-controller.sh` if title-bar focus or terminal input boundaries change. Not applicable: terminal input boundary code did not change.
- [x] 3.6 Run `bash clients/apple/scripts/check-shell-contracts.sh`.

## 4. Visual Verification

- [x] 4.1 Inspect or capture a light-mode single-pane view showing the title remains readable when focused.
- [x] 4.2 Inspect or capture light-mode split panes showing title bars integrated with the terminal surface rather than rendered as separate overlay bands.
- [x] 4.3 Inspect or capture a narrow split pane showing accessory degradation while title text and close remain visible.

## 5. Change Validation And Archive Readiness

- [x] 5.1 Run `git diff --check`.
- [x] 5.2 Run `openspec validate polish-macos-pane-title-bar-readability --type change --strict --json`.
- [x] 5.3 Run `openspec validate --all --strict --json`.
- [x] 5.4 Run the relevant macOS app build command for touched Apple client targets, or document why the local environment cannot complete it.
- [x] 5.5 Sync accepted delta requirements into `openspec/specs/` before archive after implementation merges.
- [x] 5.6 Record implementation verification evidence in the change before archive.
