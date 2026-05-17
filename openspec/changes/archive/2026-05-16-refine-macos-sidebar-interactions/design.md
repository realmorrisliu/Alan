## Context

The current macOS shell sidebar has three related interaction problems:

- Sidebar tab and space clicks update `selectedSpaceID` / `selectedTabID` without updating the authoritative `shellState.focusedPaneID`. Later runtime metadata or state publication calls `synchronizeSelection()` and restores selection from the old focused pane.
- Pinned-sidebar collapse is expressed as conditional SwiftUI insertion/removal, while titlebar controls and AppKit traffic-light placement are synchronized through a separate bridge. This produces uncoordinated motion and can make collapse appear instantaneous.
- Pinning from a revealed collapsed floating sidebar currently crosses two presentation paths: the floating overlay is removed, then the pinned sidebar expands from its collapsed/offscreen state. That can read as a brief hide-then-show instead of one Arc-like morph from the visible floating surface into the pinned layout.
- Collapsed floating-sidebar reveal is retained by SwiftUI view-local hover on the narrow reveal zone, the floating panel, and titlebar controls. After double-click visible-frame zoom, the left AppKit resize frame can take pointer ownership before the SwiftUI hover surfaces see the pointer as still inside the reveal neighborhood, so the sidebar schedules a hide even though the user is still intentionally working the left edge.
- Horizontal space swipe is represented as a discontinuous sidebar-local source/target transition. It previews the sidebar header and tab list only, commits after settle, and does not model the ordered space sequence as a continuous sidebar-local content pager.

The implementation must preserve alan's terminal-first layout, native material sidebar, hidden-titlebar window behavior, and terminal event ownership boundaries.

## Goals / Non-Goals

**Goals:**

- Make sidebar tab and space selection persist by routing selection through authoritative shell focus.
- Give pinned sidebar collapse, expansion, and floating-to-pinned pinning one coordinated presentation model across sidebar width, terminal content inset, sidebar toolbar controls, and standard macOS traffic lights.
- Keep collapsed floating-sidebar reveal stable across AppKit window-frame hit testing, including the left resize cursor region after visible-frame zoom.
- Replace discontinuous sidebar space swipe behavior with a continuous,
  sidebar-local content pager over the ordered `ShellSpace` sequence.
- Keep collapsed floating sidebar reveal transient: it must not resize terminal content except during an explicit user-initiated pin morph, and it must retain the narrow edge/titlebar-control hover behavior.
- Add focused automated and contract verification for the new interaction guarantees.

**Non-Goals:**

- Redesign the sidebar information architecture, visual palette, or command input.
- Add new space types, workspace persistence formats, or cross-window space movement.
- Rework terminal pane input ownership or Ghostty rendering.
- Implement a full visual snapshot automation system beyond the focused checks needed for this change.
- Reproduce Arc's exact animation physics, duration, or private implementation details.

## Decisions

1. **Selection actions update focus, not only view-local selection.**

   Sidebar tab and space selection will resolve a target pane from the selected tab and call the existing shell focus mutation path. This keeps `selectedSpaceID`, `selectedTabID`, `shellState.focusedSpaceID`, `shellState.focusedTabID`, and `shellState.focusedPaneID` converged. The target pane should prefer the tab's current focused pane when present, otherwise the first pane in that tab's pane tree.

   Alternative considered: preserve a separate "preview selection" state and delay focus until terminal click. That matches passive preview behavior but keeps the current flashback failure mode and makes sidebar navigation feel unreliable.

2. **Sidebar presentation uses one model, not separate pinned/floating surfaces.**

   `MacShellRootView` should derive sidebar rendering and window chrome from one presentation snapshot rather than directly branching on booleans such as `isSidebarCollapsed`, `isSidebarPanelRevealed`, and `pinnedSidebarPresentationProgress`. The model should represent the durable modes (`pinned`, `collapsedHidden`, `floatingRevealed`) and the transient mode needed for pinning (`morphingFloatingToPinned`). Each snapshot should provide the layout reservation progress, visible surface origin, surface width, content opacity/offset, corner treatment, floating shadow, hit-testing role, titlebar tool position, and `ShellWindowChromeSurface` values.

   The pinned sidebar should still remain mounted while its visible width, content opacity/offset, and workspace leading inset animate from expanded to collapsed. When a revealed floating sidebar is pinned, the visible surface should not be removed first. It should morph from the floating origin and floating treatment into the pinned origin and layout-reserved treatment, while the workspace leading inset opens continuously. Only after the morph settles should transient floating state be cleared.

   Alternative considered: tune the existing `.transition(.move(edge: .leading))`. This would not coordinate the HStack layout, titlebar overlay, and AppKit traffic lights because they still change through separate state paths.

   Alternative considered: keep separate pinned and floating views and bridge them with `matchedGeometryEffect`. That still leaves AppKit traffic lights and window chrome outside the matched geometry system, so it risks two animations with different owners.

3. **AppKit traffic lights are animated through the window chrome bridge.**

   `ShellWindowPlacement` should receive enough surface motion information to move standard traffic-light controls with the same timing as the sidebar toolbar. Visibility changes should avoid showing controls before the floating panel reveal and avoid leaving controls visible after the surface is hidden.

   Alternative considered: draw custom traffic-light replicas in SwiftUI. That would break native macOS traffic-light behavior and conflicts with the existing hidden-titlebar contract.

4. **Collapsed reveal retention is a window-level pointer judgment.**

   The collapsed floating sidebar should still use a narrow edge trigger, but
   once revealed the retention decision should be made against a window-level
   set of related regions: the edge trigger neighborhood, the floating sidebar
   surface, collapsed titlebar controls, and the adjacent left window resize
   frame. The retention path should observe pointer location in window/screen
   coordinates rather than relying only on SwiftUI `onHover(false)` from
   individual transparent views.

   This keeps the Arc-like behavior when a visible-frame-zoomed window is flush
   with the screen's left usable boundary. Moving the pointer through the
   resize-cursor strip should not count as leaving the sidebar reveal intent.
   Native resize must remain owned by AppKit; the sidebar logic should only
   decide whether to cancel a pending hide, not steal mouse-down/drag behavior
   from the resize frame.

   Alternative considered: widen the SwiftUI edge hot zone. That may mask the
   failure on some displays, but it still loses to AppKit frame hit-testing and
   makes the collapsed trigger less intentional.

5. **Space swipe is a sidebar-local five-page content pager.**

   The gesture model should track `sourceIndex`, `targetIndex`, `dragOffset`,
   `pageWidth`, and settlement phase for the sidebar's active-space content
   area. The rendered strip should keep the current space at the center with up
   to two previous and two next spaces mounted, so reversing direction during a
   swipe does not replace the target page and overdrag can reveal a sliver of
   the second adjacent page. The moving page includes only the active space
   title/header and the active space tab list. Command input, the bottom space
   switcher, sidebar material/chrome, traffic lights, and the terminal workspace
   surface remain visually fixed while dragging. Commit and cancel use the same
   pager model, but shell selection and terminal focus change only when the
   gesture commits to a target space. A single gesture may commit at most one
   adjacent space; any movement beyond one page is bounded to a small physical
   overdrag gap for feel rather than multi-space navigation.

   Alternative considered: make the entire shell content area a continuous
   space pager. That breaks the accepted terminal-first layout because the
   terminal workspace slides, duplicates, and exposes artifacts during a
   sidebar navigation gesture.

6. **Commit focus is applied at the authoritative transition point.**

   When a pager commit is selected, shell focus should update through the controller selection path before or at the start of the settle-to-target phase, while sidebar content pager state keeps rendering the old and new sidebar pages until animation completion. This prevents runtime metadata updates from snapping the UI back during the settle animation.

   Alternative considered: wait until animation completion before changing shell state. That is the current source of race-prone behavior because unrelated runtime updates can arrive while the visual transition is in flight.

## Risks / Trade-offs

- **Risk: traffic-light animation fights AppKit layout corrections.** → Keep the AppKit bridge as the only owner of standard button frames, coalesce chrome syncs, and ensure final frames are corrected without visible jumps.
- **Risk: a unified presentation model grows too broad.** → Keep it limited to shell chrome presentation data: surface origin, layout progress, treatment, visibility, hit-testing, and window chrome values. Sidebar content, space selection, and terminal runtime identity remain owned by their existing components.
- **Risk: floating-to-pinned morph shows duplicate sidebar content.** → During the morph, render one visible sidebar surface for the active presentation. The pinned layout reservation may open underneath, but the user should not see both the floating panel and a second pinned sidebar copy.
- **Risk: window-level pointer retention blocks native resizing.** → Keep AppKit as the owner of resize hit-testing and only use pointer location to keep or cancel the sidebar hide timer; do not install a mouse-down-consuming overlay on the resize frame.
- **Risk: immediate focus commit changes terminal runtime focus while the old sidebar page is still visible during settle.** → Keep a short sidebar content pager state that decouples rendering from the already-committed focus for the duration of settlement.
- **Risk: sidebar-local pager accidentally moves fixed shell regions.** → Keep command input, the bottom space switcher, sidebar chrome, traffic lights, and the terminal workspace outside the moving page.
- **Risk: swipe velocity makes one gesture skip multiple spaces.** → Clamp visual drag to one page plus a small overdrag gap and keep commit targets limited to the immediate previous or next space.
- **Risk: gesture axis arbitration regresses vertical sidebar scrolling.** → Preserve the existing intent lock behavior and add tests for undecided, vertical, phaseful, phase-less, momentum, and fast flick paths.

## Migration Plan

Implement in narrow stages: first selection/focus convergence, then pinned sidebar/chrome motion, then sidebar-local content pager refactor. Keep existing persisted shell state format unchanged. If a later stage regresses, it can be reverted independently because selection convergence does not require the pager implementation.

## Open Questions

None.
