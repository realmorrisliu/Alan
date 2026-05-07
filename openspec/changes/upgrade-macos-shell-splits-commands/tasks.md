## 1. Split Model

- [x] 1.1 Add split branch ratio, structural identity, and validation/clamping to the shell model.
- [x] 1.2 Add migration/loading behavior for existing equal split trees.
- [ ] 1.3 Add model operations for split directional creation, resize, equalize, zoom, unzoom, close, focus, and move.
- [ ] 1.4 Add focused model tests for ratio persistence, invalid ratios, tree repair, and empty-container handling.

## 2. Split Layout UI

- [x] 2.1 Replace equal `HStack`/`VStack` recursion with a ratio-aware split layout view.
- [x] 2.2 Add native divider resize affordances with stable minimum pane sizes.
- [ ] 2.3 Add split zoom and unzoom UI that preserves sibling runtime handles.
- [ ] 2.4 Add visual treatment for focused panes that stays lightweight and hides debug identifiers.

## 3. Native Commands And Menus

- [ ] 3.1 Add shell command routing for new terminal tab, new Alan tab, split directions, close pane/tab, focus directions, resize, equalize, zoom, copy, paste, search, and command UI.
- [ ] 3.2 Wire native menu bar commands to shell controller actions.
- [ ] 3.3 Wire keyboard shortcuts through responder-chain or SwiftUI command routing without stealing terminal-app input.
- [ ] 3.4 Update `Go to or Command...` results for spaces, tabs, panes, and workspace actions using user-facing labels.
- [ ] 3.5 Keep toolbar changes restrained and verify the default terminal workflow remains inspector-off and terminal-first.

## 4. Pane Movement And Control Plane

- [ ] 4.1 Add explicit pane move operations within a tab and across tabs in the same window.
- [ ] 4.2 Add drag/drop movement only if it preserves runtime identity and does not compromise terminal selection behavior.
- [ ] 4.3 Extend control-plane commands for resize, equalize, zoom, spatial focus, move, and close if existing commands are insufficient.
- [ ] 4.4 Emit shell events for split, focus, zoom, move, and close outcomes from every command path.
- [ ] 4.5 Confirm native window behavior does not conflict with Alan's custom spaces/tabs organization.

## 5. Verification

- [ ] 5.1 Add unit tests for split ratios, spatial focus, zoom, pane moves, close repair, and command result semantics.
- [ ] 5.2 Run `git diff --check`.
- [ ] 5.3 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 5.4 Build the macOS app with the documented `AlanNative` command.
- [ ] 5.5 Capture or record review evidence for single-pane, split-pane, divider resize, zoom, command UI, menu shortcuts, and inspector-off default UI.

## 6. PR And Archive Readiness

- [ ] 6.1 Review shortcut conflicts against common terminal applications and adjust routing notes.
- [ ] 6.2 Review pane movement for runtime identity preservation before enabling drag/drop by default.
- [ ] 6.3 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.4 Archive the OpenSpec change after implementation is merged.
