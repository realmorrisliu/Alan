## 1. Title Derivation

- [ ] 1.1 Add a pane-title helper that consumes existing `ShellViewportSnapshot.title` before cwd, working-directory, launch-target, process, or `Terminal` fallbacks.
- [ ] 1.2 Add focused tests for title-bar consumption priority, fallback ordering, long title truncation readiness, and debug/raw-ID suppression without retesting terminal title capture itself.

## 2. Pane Leaf UI

- [ ] 2.1 Wrap each `ShellTerminalLeafView` terminal host in a compact fixed-height title bar plus terminal canvas layout.
- [ ] 2.2 Render a single-line truncated pane title with active/inactive styling that stays visually lighter than terminal content.
- [ ] 2.3 Add an icon-only pane close button with tooltip/accessibility label and stable dimensions.
- [ ] 2.4 Keep the title bar outside the terminal host surface so terminal clicks, drags, right clicks, and scroll events stay owned by the terminal host.

## 3. Pane-Scoped Close Routing

- [ ] 3.1 Expose a controller-owned targeted pane close path that routes by pane ID and reuses existing `ShellStateSnapshot.closingPane(...)` semantics.
- [ ] 3.2 Wire the title-bar close button to close the pane represented by that leaf, including inactive split panes.
- [ ] 3.3 Keep final-pane protection aligned with the existing `lastTab` model behavior and avoid leaving the shell without a valid surface.
- [ ] 3.4 Add focused tests for selected-pane close, inactive split-pane close, single-pane tab close, and final-pane protection.

## 4. Verification

- [ ] 4.1 Run `clients/apple/scripts/test-shell-split-model.sh`.
- [ ] 4.2 Run `clients/apple/scripts/test-shell-runtime-metadata.sh`.
- [ ] 4.3 Run `clients/apple/scripts/test-terminal-surface-controller.sh` if terminal input or host wrapper behavior changes.
- [ ] 4.4 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 4.5 Run `git diff --check`.
- [ ] 4.6 Run `openspec validate add-macos-pane-title-bars --type change --strict --json`.
- [ ] 4.7 Run `openspec validate --all --strict --json`.
- [ ] 4.8 Build the macOS app with the documented `AlanNative` Debug command.
- [ ] 4.9 Capture or document light-mode single-pane and split-pane screenshots plus manual terminal interaction notes.

## 5. Archive Readiness

- [ ] 5.1 Sync accepted delta requirements into `openspec/specs/` before archiving after implementation merges.
- [ ] 5.2 Record implementation verification evidence in the change before archive.
