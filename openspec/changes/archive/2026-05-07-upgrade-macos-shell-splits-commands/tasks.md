## 1. Implemented Scope

- [x] 1.1 Add split branch ratio, structural identity, migration defaults, and validation/clamping to the shell model.
- [x] 1.2 Add model operations and tests for directional split creation, split resize, equalize, close, spatial focus, pane lift, and cross-tab pane move.
- [x] 1.3 Replace equal `HStack`/`VStack` recursion with a ratio-aware split layout view and native divider resize affordances.
- [x] 1.4 Keep adjacent split panes as one continuous terminal surface with rounded outer corners, no per-pane cards, no fixed gaps, and no bottom pane tab strip.
- [x] 1.5 Add preference-backed inactive split pane dimming and subtle divider treatment.
- [x] 1.6 Add native menu and keyboard command routing for new tabs, directional splits, spatial focus, equalize, close pane, and close tab.
- [x] 1.7 Add `Go to or Command...` actions for spaces, tabs, panes, routing candidates, split/focus/equalize/close actions, and pane lift.
- [x] 1.8 Extend the control plane for pane split, close, lift, cross-tab move, direct focus, and mutation events for created, moved, closed, and focused panes.

## 2. Deferred Scope

The original proposal also explored split zoom/unzoom, drag/drop pane movement,
arbitrary in-tab pane movement, control-plane resize/equalize/zoom commands, and
complete copy/paste/search command ownership. Those remain future work and are
not archived as completed requirements by this change.

## 3. Verification And Archive

- [x] 3.1 Run `clients/apple/scripts/test-shell-split-model.sh`.
- [x] 3.2 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 3.3 Run `clients/apple/scripts/test-terminal-surface-controller.sh`.
- [x] 3.4 Run `clients/apple/scripts/test-terminal-runtime-service.sh`.
- [x] 3.5 Run `clients/apple/scripts/test-shell-runtime-metadata.sh`.
- [x] 3.6 Run `git diff --check`.
- [x] 3.7 Run `openspec validate upgrade-macos-shell-splits-commands --type change --strict --json`.
- [x] 3.8 Run `openspec validate --all --strict --json`.
- [x] 3.9 Build the macOS app with the documented `AlanNative` command.
- [x] 3.10 Sync accepted delta requirements into `openspec/specs/` before archive.
