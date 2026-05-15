## 1. Selection And Focus Convergence

- [x] 1.1 Add shell-controller helpers that resolve a target pane for a selected tab or space, preferring an existing focused pane in that tab and otherwise using a stable pane from the tab tree.
- [x] 1.2 Change `select(spaceID:)`, `select(tabID:)`, indexed space selection, adjacent space selection, and sidebar click paths so committed sidebar selection updates authoritative `shellState.focusedPaneID` through the shell focus mutation path.
- [x] 1.3 Ensure committed sidebar selection requests terminal focus for the selected pane when a runtime is available, without stealing focus during transient preview-only gestures.
- [x] 1.4 Add focused tests proving tab clicks and space clicks are not reverted by immediate terminal runtime metadata or control-plane state publication.
- [x] 1.5 Add split-tab coverage proving sidebar selection chooses a stable pane without changing split trees or divider ratios.

## 2. Coordinated Sidebar And Window Chrome Motion

- [x] 2.1 Replace pinned-sidebar conditional insertion/removal with a mounted sidebar presentation state that drives visible width, content opacity or offset, and workspace leading inset.
- [x] 2.2 Tune pinned collapse and expansion timing to be short and crisp in normal motion and non-springy under reduced motion.
- [x] 2.3 Extend the window chrome bridge so standard macOS traffic-light controls move with the sidebar/titlebar-control motion and settle to corrected final AppKit frames.
- [x] 2.4 Preserve collapsed floating-sidebar reveal behavior: narrow edge hover, toolbar-hover retention, stable terminal workspace geometry, and no traffic-light appearance ahead of panel reveal.
- [x] 2.5 Add or update focused window-placement tests for hidden traffic lights, floating surface origin, pinned motion final frames, and native traffic-light behavior.

## 3. Sidebar-local Space Content Pager

- [x] 3.1 Replace the root-level `ShellSpacePagerState` usage with a sidebar-local pager state that tracks source index, target index, drag offset, sidebar content page width, commit/cancel state, and settlement phase.
- [x] 3.2 Preserve gesture axis arbitration in `ShellSidebarSwipeMonitor`, including undecided buffering, vertical scroll pass-through, phaseful release, phase-less idle release, momentum handoff, and fast flick velocity.
- [x] 3.3 Render current and adjacent sidebar active-space content pages from the same pager offset so users can see the target space edge while dragging, without moving command input, the bottom space switcher, sidebar chrome, or the terminal workspace surface.
- [x] 3.4 Keep command input, the bottom space switcher, sidebar chrome, traffic lights, and the terminal workspace surface visually fixed during sidebar-local pager motion while preserving terminal runtime identities.
- [x] 3.5 Apply bounded edge resistance at the first and last spaces and prevent accidental wraparound.
- [x] 3.6 Commit the target space through the authoritative selection/focus path at the transition point so concurrent runtime updates cannot snap the UI back during settlement.
- [x] 3.7 Cancel below-threshold gestures back to the source page without changing selected space, selected tab, focused pane, split tree, or divider ratios.

## 4. Verification

- [x] 4.1 Extend focused shell tests for sidebar selection/focus convergence and runtime-update race coverage.
- [x] 4.2 Extend sidebar swipe monitor or pager tests for horizontal, vertical, undecided, edge, cancel, commit, phaseful, phase-less, and fast-flick cases.
- [x] 4.3 Update shell contract checks so default shell code cannot reintroduce view-local-only sidebar selection or full-window space pager semantics.
- [x] 4.4 Run focused Apple checks: `clients/apple/scripts/test-shell-runtime-metadata.sh`, `clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh`, `clients/apple/scripts/test-shell-window-placement.sh`, and `clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 4.5 Build or run the macOS app and capture manual verification notes or screenshots for pinned collapse/expand, floating reveal/hide, tab click persistence, space click persistence, and space swipe pager motion.
  - 2026-05-15: macOS Debug build passed with project-local DerivedData and manual verification notes were added in `manual-verification.md`. Live visual interaction was not performed in this run, so this remains unchecked for human acceptance.

## 5. OpenSpec And Review Readiness

- [x] 5.1 Keep `proposal.md`, `design.md`, `tasks.md`, and all delta specs aligned if implementation discoveries change the contract.
- [x] 5.2 Run `openspec validate refine-macos-sidebar-interactions --strict` after spec edits and after implementation.
- [x] 5.3 Run `openspec validate --all --strict` before opening or updating a PR.
- [x] 5.4 Prepare archive readiness notes after implementation so the delta specs can be synced into `openspec/specs/` before archiving.
