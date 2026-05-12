## Context

`docs/spec/alan_macos_shell_ui_ux.md` pushed the sidebar toward a separate
space rail plus tab list, but the macOS traffic-light area makes a left/right
split inside the sidebar harder to align cleanly. The preferred interaction is
closer to Arc's vertical sidebar model: tab/navigation content remains in one
vertical column, while spaces are switched through a compact borderless control
strip at the bottom and through horizontal swipe gestures over the sidebar.

The current `ShellSidebarView` is already a vertical stack with a header, command
launcher, tab section, and horizontal space dock. The right direction is not a
second adjacent rail; it is to make that vertical structure quieter, remove
instructional copy, and make the bottom space switcher feel intentional.

## Goals / Non-Goals

**Goals:**

- Keep the sidebar as a single vertical navigation column that aligns cleanly
  below and around the macOS traffic-light area, with an initial narrow macOS
  width target around 264 pt rather than a dashboard-like wide column.
- Make spaces available through a bottom, borderless icon switcher and through
  left/right swipe gestures inside the sidebar that drive a sidebar-local,
  direct-manipulation space preview while the workspace stays stable until
  commit.
- Reduce visible explanatory copy while preserving discoverability through icon
  choice, placement, hover actions, tooltips, menus, and accessibility labels.
- Keep tabs skimmable and lightweight, with Alan state expressed inline and
  attention preserved outside default sidebar notification dots.
- Make split tabs legible at a glance by showing a compact topology indicator
  that communicates pane count, dominant split direction, and focused pane.
- Keep the command input entry available from the sidebar, but do not redesign its
  overlay in this change.

**Non-Goals:**

- Do not redesign material treatment; that is owned by
  `optimize-macos-material-system`.
- Do not implement advanced drag/reorder behavior unless needed to make the
  layout coherent.
- Do not remove product terms such as Space and Tab from accessibility surfaces
  or documentation; the goal is less visible explanation, not less semantic
  structure.

## Decisions

1. Keep a vertical Arc-like sidebar.

   The root sidebar should remain one vertical stack: top identity/window-safe
   area, command entry, active-space tab list, and bottom space switcher. Use a
   restrained width around 264 pt so the sidebar reads as navigation chrome
   instead of a dashboard panel. This avoids traffic-light alignment problems
   and matches the Arc-like bottom space switcher direction.

   Alternative considered: render a narrow rail and a tab-list column side by
   side. That looks structurally clean in an abstract layout, but it creates
   alignment issues around macOS traffic lights and conflicts with the desired
   Arc-like bottom switcher.

2. Make the bottom space switcher borderless.

   Space switcher items should be compact icon buttons without card borders,
   framed backgrounds, section chrome, or notification dots. Selection and hover
   should be expressed through icon tone, subtle glow/tint, or a small backing
   only when state requires it.

   Alternative considered: keep current rounded square rail items. That is
   functional, but it reads heavier than the desired slim bottom switcher.

3. Support horizontal sidebar swipe as direct manipulation, not a trigger.

   A left/right swipe over the sidebar should begin a transient space
   transition. While the gesture is active, the sidebar's current space header
   and tab list should move with the finger and the adjacent space should
   preview from the side. The workspace terminal surface should remain visually
   stable during the gesture and update only after release commits the new
   selected space. The header and tab-list previews should compute their
   horizontal offsets from the same full sidebar page width, not from the
   padded content width or the space-title row's accessory width, so they move
   at the same pixel speed and avoid visible side gaps during the swipe. The
   space-title row should stay a quiet label rather than adding a persistent
   trailing creation button, and the title pager itself should remain
   full-width. Dragging should use a translation-first model: the sidebar pages
   render directly from the current horizontal finger/scroll translation, while
   normalized progress is derived only for release-time commit decisions. The
   commit threshold should not amplify, quantize, or otherwise shape the drag
   preview. Alan should commit the selected space only after release crosses a
   distance or velocity threshold; otherwise it should cancel back to the
   original space. Fast horizontal flicks should be able to lock and commit
   from the release/momentum handoff even if they do not produce a long stream
   of intermediate changed events.

   This transition belongs in view state, not in `ShellHostController`
   selection state. `ShellHostController` should still change selected space via
   the existing selection path once the gesture commits. This keeps terminal
   focus, runtime snapshots, and control-plane state stable while the gesture is
   only previewing.

   The gesture recognizer should use an explicit axis lock before routing
   scroll-wheel events. During the initial dead zone, mixed scroll deltas should
   be buffered rather than leaked to the vertical tab list. Once horizontal
   intent locks, vertical deltas are ignored, native vertical tab-list scrolling
   is disabled for that gesture, and the sidebar preview is the only movement.
   Once vertical intent locks, the sidebar space preview does not begin and
   native vertical tab-list scrolling receives the gesture. Phaseful
   trackpad gestures should remain fully direct: if the fingers pause while
   still touching the trackpad, the sidebar preview should hold its current
   offset rather than settling. Alan should settle only when the gesture ends,
   is cancelled, or enters momentum. The recognizer must process
   ended/cancelled/momentum phases before ignoring zero-delta scroll events,
   because macOS may deliver the release event with no scroll delta. It should
   settle using the last effective finger velocity rather than recomputing
   velocity from a zero-delta release event. Phase-less scroll devices may use a
   short idle fallback to avoid leaving the preview stuck. At the first or last
   space, the transition should rubber-band rather than wrapping; keyboard
   shortcuts may continue to wrap through the existing discrete command path.

   Alternative considered: keep a scroll-delta threshold and call
   `selectAdjacentSpace`. That is cheaper, but it feels like a page trigger
   rather than native direct manipulation and does not match the desired Arc-like
   or Apple interaction quality.

   Alternative considered: drag the entire workspace surface with the sidebar.
   That made too much of the window move during a sidebar navigation gesture and
   did not match Arc's interaction model, where the current page stays stable
   until the sidebar space switch commits.

4. Remove visible labels where placement is enough.

   Section headings such as `Tabs` and `Spaces`, shortcut-only accessory text,
   and descriptive empty-state paragraphs should not be default chrome. Use
   direct controls, subtle counts where they are useful, hover help, context
   menus, and accessibility labels instead.

   Alternative considered: keep labels for clarity. The target product direction
   is a native tool where structure teaches itself; visible labels should be
   reserved for ambiguous controls or text that materially helps repeated use.

5. Use progressive disclosure for secondary actions.

   Close and more actions should appear on row hover/focus or context menu.
   Default rows should prioritize title, compact context, Alan attachment, and
   selection.

6. Empty states should be actionable, not explanatory.

   If no spaces or tabs exist, the sidebar should show a compact creation
   affordance in the owning zone. It should avoid paragraph-style instruction
   like "Create a space to start..." in normal chrome.

7. Represent split tabs with a topology indicator, not a full thumbnail.

   A split tab row should show a small indicator only when the tab contains more
   than one pane. For two panes, the indicator can mirror the root split
   direction with two segments. For three or more panes, it should summarize the
   topology with a compact cluster or dominant-direction glyph plus pane count.
   It should mark the focused pane, but it should not render notification dots
   or try to show exact divider ratios inside the narrow tab row.

   Alternative considered: draw a miniature proportional split layout. That
   works for simple two-column splits, but Alan's terminal split tree supports
   nested horizontal/vertical splits and arbitrary ratios, which would become
   unreadable at sidebar size.

8. Make the split indicator a quick focus target.

   For two panes, clicking the visible segment should focus that pane. For more
   complex layouts, clicking the indicator should provide a predictable focus
   action such as cycling to the next pane, and a hover/focus or click-expanded
   popover may expose a larger topology map for direct pane selection. This
   keeps the row compact while preserving a path to precise focus.

## Risks / Trade-offs

- Removing visible copy can hurt first-run clarity -> Mitigate with stable
  placement, recognizable symbols, hover help, and accessibility labels.
- Horizontal swipe can conflict with vertical tab scrolling -> Lock horizontal
  intent before consuming scroll-wheel events, keep vertical scrolling native,
  and make commit/cancel happen only on release.
- Delaying workspace updates until commit can feel abrupt if the sidebar settle
  is too long -> Keep the settle short, commit through the existing selection
  path after release, and avoid moving the large terminal surface during drag.
- Borderless icons can become too subtle -> Use hover, selection, and
  accessibility labels to maintain discoverability without adding boxes back.
- Split indicators can become noisy in tab rows -> Hide them for single-pane
  tabs, summarize complex splits, and avoid proportional ratio rendering in the
  default row.
- This overlaps visually with material work -> Keep this change focused on
  layout, visible copy, and interaction ownership; leave material tokens to the
  material-system change.
