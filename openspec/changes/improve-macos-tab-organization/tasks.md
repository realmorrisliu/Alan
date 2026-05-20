## 1. Model And Manifest

- [x] 1.1 Extend shell model helpers so each Space exposes stable Pinned and
  Unpinned Tab ordering.
- [x] 1.2 Persist Tab order, pin state, Space ownership, and selected Tab after
  reorder, pin/unpin, and Move to Space mutations.
- [x] 1.3 Save a pin snapshot when an Unpinned Tab is dragged into the Pinned
  section or otherwise pinned from the organization surface.
- [x] 1.4 Preserve existing update-pin behavior for already Pinned Tabs without
  updating the snapshot on ordinary reorder.

## 2. Registry Actions And Mutations

- [x] 2.1 Add registry-backed Tab actions for pin, unpin, move left, move right,
  and Move Tab to Space.
- [x] 2.2 Make context-menu Tab actions target the clicked Tab without selecting
  it first.
- [x] 2.3 Implement Move Tab to Space insertion rules: keep pin state and insert
  at the end of the target Space's corresponding section.
- [x] 2.4 Follow the moved Tab only when the moved Tab was the current selected
  Tab; otherwise keep the current Space selected.
- [x] 2.5 Emit shell events for reorder, pin/unpin, and Move Tab to Space.

## 3. Sidebar Drag UI

- [x] 3.1 Render per-Space Pinned and Unpinned sections with Arc-like lightweight
  separation and without heavy group headings.
- [x] 3.2 Support whole-row drag with a movement threshold so short clicks still
  select the Tab.
- [x] 3.3 Show realtime insertion preview within and across sections.
- [x] 3.4 Keep close overlay, right-click menu, and split topology indicator
  interactions from accidentally starting drag reorder.
- [x] 3.5 Keep `New Tab` as a lightweight list action rather than a toolbar-like
  primary button.

## 4. Runtime Continuity

- [x] 4.1 Prove reorder, pin/unpin, and Move to Space preserve Tab ID, pane ID,
  split tree, terminal runtime handle, metadata, scrollback, and queued delivery
  state.
- [x] 4.2 Ensure moving the current Tab follows to the target Space and focuses
  the same preferred pane.
- [x] 4.3 Ensure moving a non-current Tab does not change the current selected
  Space, selected Tab, or focused pane.

## 5. Verification

- [x] 5.1 Add focused model tests for same-section reorder, cross-section
  pin/unpin, snapshot creation, Move to Space, and current/non-current focus
  behavior.
- [x] 5.2 Add focused sidebar tests or script checks for drag threshold,
  insertion preview, context target, and split indicator/close overlay
  coexistence.
- [x] 5.3 Add manifest persistence tests for immediate order, pin state, snapshot
  and Space ownership writes.
- [x] 5.4 Run relevant Apple shell scripts and the macOS app build command, or
  document any local blocker.
- [x] 5.5 Run `openspec validate improve-macos-tab-organization --type change --strict --json`.
- [x] 5.6 Run `openspec validate --all --strict --json`.
- [x] 5.7 Run `git diff --check`.

## 6. Archive Readiness

- [x] 6.1 Confirm the final UI stays Arc-like and does not introduce tab folders,
  global pinned tabs, heavy section headers, or Command UI scope.
- [ ] 6.2 Before archive, sync accepted delta requirements into
  `openspec/specs/`.
- [ ] 6.3 Archive the OpenSpec change after implementation merges.
