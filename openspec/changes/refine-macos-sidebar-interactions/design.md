## Context

The current macOS shell sidebar has three related interaction problems:

- Sidebar tab and space clicks update `selectedSpaceID` / `selectedTabID` without updating the authoritative `shellState.focusedPaneID`. Later runtime metadata or state publication calls `synchronizeSelection()` and restores selection from the old focused pane.
- Pinned-sidebar collapse is expressed as conditional SwiftUI insertion/removal, while titlebar controls and AppKit traffic-light placement are synchronized through a separate bridge. This produces uncoordinated motion and can make collapse appear instantaneous.
- Horizontal space swipe is represented as a sidebar-local source/target transition. It previews the sidebar header and tab list only, commits after settle, and does not model spaces as a continuous sequence.

The implementation must preserve alan's terminal-first layout, native material sidebar, hidden-titlebar window behavior, and terminal event ownership boundaries.

## Goals / Non-Goals

**Goals:**

- Make sidebar tab and space selection persist by routing selection through authoritative shell focus.
- Give pinned sidebar collapse and expansion one coordinated motion model across sidebar width, terminal content inset, sidebar toolbar controls, and standard macOS traffic lights.
- Replace sidebar-only swipe semantics with a continuous pager over the ordered `ShellSpace` sequence.
- Keep collapsed floating sidebar reveal transient: it must not resize terminal content, and it must retain the narrow edge/titlebar-control hover behavior.
- Add focused automated and contract verification for the new interaction guarantees.

**Non-Goals:**

- Redesign the sidebar information architecture, visual palette, or command input.
- Add new space types, workspace persistence formats, or cross-window space movement.
- Rework terminal pane input ownership or Ghostty rendering.
- Implement a full visual snapshot automation system beyond the focused checks needed for this change.

## Decisions

1. **Selection actions update focus, not only view-local selection.**

   Sidebar tab and space selection will resolve a target pane from the selected tab and call the existing shell focus mutation path. This keeps `selectedSpaceID`, `selectedTabID`, `shellState.focusedSpaceID`, `shellState.focusedTabID`, and `shellState.focusedPaneID` converged. The target pane should prefer the tab's current focused pane when present, otherwise the first pane in that tab's pane tree.

   Alternative considered: preserve a separate "preview selection" state and delay focus until terminal click. That matches passive preview behavior but keeps the current flashback failure mode and makes sidebar navigation feel unreliable.

2. **Pinned sidebar motion uses a continuous presentation progress.**

   The pinned sidebar should remain mounted while its visible width, content opacity/offset, and workspace leading inset animate from expanded to collapsed. The same progress drives titlebar tool position and the `ShellWindowChromeSurface` origin/visibility passed to AppKit.

   Alternative considered: tune the existing `.transition(.move(edge: .leading))`. This would not coordinate the HStack layout, titlebar overlay, and AppKit traffic lights because they still change through separate state paths.

3. **AppKit traffic lights are animated through the window chrome bridge.**

   `ShellWindowPlacement` should receive enough surface motion information to move standard traffic-light controls with the same timing as the sidebar toolbar. Visibility changes should avoid showing controls before the floating panel reveal and avoid leaving controls visible after the surface is hidden.

   Alternative considered: draw custom traffic-light replicas in SwiftUI. That would break native macOS traffic-light behavior and conflicts with the existing hidden-titlebar contract.

4. **Space swipe is a pager over the ordered space sequence.**

   The gesture model should track `sourceIndex`, `targetIndex`, `dragOffset`, `pageWidth`, and settlement phase. Adjacent pages are rendered from the same offset so the user can see the next or previous space edge while dragging. The visible space page includes the sidebar navigation content and the terminal workspace surface for the source and adjacent target spaces, while preserving terminal runtime identity. Commit and cancel use the same pager model rather than a sidebar-only source/target transition.

   Alternative considered: keep sidebar-only preview and only fix commit timing. That would solve some failures but would not match the virtual-desktop physical model requested for spaces.

5. **Commit focus is applied at the authoritative transition point.**

   When a pager commit is selected, shell focus should update through the controller selection path before or at the start of the settle-to-target phase, while visual transition state keeps rendering the old and new pages until animation completion. This prevents runtime metadata updates from snapping the UI back during the settle animation.

   Alternative considered: wait until animation completion before changing shell state. That is the current source of race-prone behavior because unrelated runtime updates can arrive while the visual transition is in flight.

## Risks / Trade-offs

- **Risk: traffic-light animation fights AppKit layout corrections.** → Keep the AppKit bridge as the only owner of standard button frames, coalesce chrome syncs, and ensure final frames are corrected without visible jumps.
- **Risk: immediate focus commit changes terminal runtime focus while the old page is still visible during settle.** → Keep a short visual transition overlay/pager state that decouples rendering from the already-committed focus for the duration of settlement.
- **Risk: continuous pager could create multiple live terminal hosts during drag.** → Reuse existing pane runtimes and render only the current and adjacent space pages needed for the gesture.
- **Risk: gesture axis arbitration regresses vertical sidebar scrolling.** → Preserve the existing intent lock behavior and add tests for undecided, vertical, phaseful, phase-less, momentum, and fast flick paths.

## Migration Plan

Implement in narrow stages: first selection/focus convergence, then pinned sidebar/chrome motion, then space pager refactor. Keep existing persisted shell state format unchanged. If a later stage regresses, it can be reverted independently because selection convergence does not require the pager implementation.

## Open Questions

None.
