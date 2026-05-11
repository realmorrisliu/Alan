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
  below and around the macOS traffic-light area.
- Make spaces available through a bottom, borderless icon switcher and through
  left/right swipe gestures inside the sidebar.
- Reduce visible explanatory copy while preserving discoverability through icon
  choice, placement, hover actions, tooltips, menus, and accessibility labels.
- Keep tabs skimmable and lightweight, with attention and Alan state expressed
  inline.
- Make split tabs legible at a glance by showing a compact topology indicator
  that communicates pane count, dominant split direction, and focused pane.
- Keep `Command-K` entry available from the sidebar, but do not redesign its
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
   area, command entry, active-space tab list, and bottom space switcher. This
   avoids traffic-light alignment problems and matches the Arc-like bottom space
   switcher direction.

   Alternative considered: render a narrow rail and a tab-list column side by
   side. That looks structurally clean in an abstract layout, but it creates
   alignment issues around macOS traffic lights and conflicts with the desired
   Arc-like bottom switcher.

2. Make the bottom space switcher borderless.

   Space switcher items should be compact icon buttons without card borders,
   framed backgrounds, or section chrome. Selection, hover, and attention should
   be expressed through icon tone, subtle glow/tint, dot/count marks, or a small
   backing only when state requires it.

   Alternative considered: keep current rounded square rail items. That is
   functional, but it reads heavier than the desired slim bottom switcher.

3. Support horizontal sidebar swipe for space switching.

   A left/right swipe over the sidebar should switch to the previous/next space.
   This must not steal scroll from the tab list unless the gesture is clearly
   horizontal. Trackpad interaction should feel like switching Arc spaces, not
   like dragging a custom carousel.

4. Remove visible labels where placement is enough.

   Section headings such as `Tabs` and `Spaces`, shortcut-only accessory text,
   and descriptive empty-state paragraphs should not be default chrome. Use
   direct controls, subtle count/attention marks, hover help, context menus, and
   accessibility labels instead.

   Alternative considered: keep labels for clarity. The target product direction
   is a native tool where structure teaches itself; visible labels should be
   reserved for ambiguous controls or text that materially helps repeated use.

5. Use progressive disclosure for secondary actions.

   Close and more actions should appear on row hover/focus or context menu.
   Default rows should prioritize title, compact context, status/attention, and
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
   It should mark the focused pane and optionally attention, but it should not
   try to render exact divider ratios inside the narrow tab row.

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
- Horizontal swipe can conflict with vertical tab scrolling -> Gate space
  switching on clear horizontal intent and keep vertical scroll behavior intact.
- Borderless icons can become too subtle -> Use hover, selection, attention, and
  accessibility labels to maintain discoverability without adding boxes back.
- Split indicators can become noisy in tab rows -> Hide them for single-pane
  tabs, summarize complex splits, and avoid proportional ratio rendering in the
  default row.
- This overlaps visually with material work -> Keep this change focused on
  layout, visible copy, and interaction ownership; leave material tokens to the
  material-system change.
