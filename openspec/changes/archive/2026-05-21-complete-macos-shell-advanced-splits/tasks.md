## 1. Split Zoom

- [x] 1.1 Add tab-scoped zoom state that leaves the canonical split tree unchanged.
- [x] 1.2 Render zoomed PaneSlots full-area while keeping sibling terminal ContentInstance runtimes registered and restorable.
- [x] 1.3 Add menu, keyboard, command UI, and compact visible affordances for zoom and unzoom.
- [x] 1.4 Add model/UI tests for zoom, unzoom, disappearing panes, and selected-tab changes.

## 2. Pane Movement

- [x] 2.1 Add explicit in-tab pane move operations with stable validation and tree repair.
- [x] 2.2 Route pane movement commands through the shared shell controller mutation path.
- [x] 2.3 Add drag/drop movement only after proving terminal text selection remains reliable.
- [x] 2.4 Add tests for in-tab movement, invalid movement, drag-backed movement routing, and PaneSlot / mounted ContentInstance identity preservation.

## 3. Control Plane

- [x] 3.1 Add control-plane commands for resize, equalize, zoom, unzoom, and spatial focus.
- [x] 3.2 Return stable result payloads for changed split IDs, ratios, tab zoom state, previous/current focus, and no-target errors.
- [x] 3.3 Emit shell events for split ratio changes, equalization, zoom state changes, spatial focus, and advanced movement.
- [x] 3.4 Add control-plane tests for success, invalid target, no target, and unchanged-state outcomes.

## 4. Copy Paste And Search

- [x] 4.1 Add a shared command target resolver for native menu, keyboard, command UI, context menu, and terminal host paths.
- [x] 4.2 Route Copy and Paste to the focused terminal host when terminal selection or input owns the command.
- [x] 4.3 Route terminal search to a terminal-content-scoped UI owned by the focused terminal ContentInstance runtime identity.
- [x] 4.4 Add tests for command routing precedence and terminal-host ownership.

## 5. Verification And Archive Readiness

- [x] 5.1 Run `clients/apple/scripts/test-shell-split-model.sh`.
- [x] 5.2 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 5.3 Run the Apple shell controller/runtime script tests touched by this change.
- [x] 5.4 Run `git diff --check`.
- [x] 5.5 Run `openspec validate complete-macos-shell-advanced-splits --type change --strict --json`.
- [x] 5.6 Run `openspec validate --all --strict --json`.
- [x] 5.7 Build the macOS app with the renamed `alan-macos` command.
- [x] 5.8 Sync accepted delta requirements into `openspec/specs/` before archive.
