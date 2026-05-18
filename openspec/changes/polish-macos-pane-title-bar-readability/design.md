## Context

`TerminalPaneView.swift` already renders `ShellPaneTitleBarView` above every
visible terminal leaf. The current row is fixed height, uses selected and
unselected terminal-chrome material fills, gives the title an infinite-width
slot, and gives accessories fixed maximum widths. That made the first
implementation compact, but it now works against the desired terminal-first
presentation: the title bar reads as an overlay on top of the pane, selected
state can wash out the title, and accessory layout is not fit-content.

The title and accessory data already exist. `ShellModel.swift` derives the
pane title from terminal metadata and projects activity, status, cwd/worktree,
branch, process, and alan detail into pane-local accessories. This change is
therefore a presentation and layout refinement, not a metadata or focus-model
rewrite.

## Goals / Non-Goals

**Goals:**

- Make pane title bars feel like part of the terminal surface instead of a
  separate chrome strip.
- Keep title text readable in focused and unfocused panes, especially in light
  mode.
- Render title-bar items left to right as fit-content content.
- Preserve title text in every responsive fallback state; the title may
  truncate, but it must not become icon-only.
- Degrade lower-priority accessories predictably when pane width is narrow.
- Keep terminal input ownership, split geometry, title derivation, and targeted
  pane close behavior unchanged.

**Non-Goals:**

- Redesign sidebar tab rows, terminal activity semantics, or terminal metadata
  ingestion.
- Add a pane toolbar, pane tab strip, breadcrumbs, hover menu, drag-to-reorder
  behavior, or new pane-management commands.
- Change how focused pane identity is stored or synchronized.
- Add a custom general-purpose SwiftUI layout unless the staged fit-content
  approach proves insufficient during implementation.

## Decisions

1. Title bars use terminal-surface background, not selected chrome material.

   The row should read as the first line of the pane surface. Selected state
   should not be expressed through a title-bar background wash, because that
   is the source of the current overlay feel and likely contributes to poor
   selected-title contrast. Focus remains a pane-level concern expressed by the
   existing terminal surface, split boundary, or dim treatment.

   Alternative considered: keep a lighter material wash for the selected title
   bar. That would make focus more local to the title row, but it would keep
   the title bar visually separate from the terminal.

2. Title and close are persistent; accessories are responsive.

   The title stays as text in every width state and uses middle truncation when
   needed. The close affordance remains available because it is the pane-scoped
   action that the title bar owns. Accessories degrade first because they are
   secondary detail.

   Alternative considered: allow the title to collapse to a terminal or folder
   icon. That saves space, but it removes the core scanning value of the title
   bar and makes split panes harder to identify.

3. Use staged `ViewThatFits`-style fallback before a custom layout.

   The first implementation should provide three stable presentations:
   full accessories with icon and text, compact accessories as icon-only for
   lower-priority items, and minimal accessories containing only actionable or
   high-priority state. This keeps implementation understandable while still
   honoring fit-content layout.

   Alternative considered: write a custom measuring layout that decides item
   visibility one by one. That can be more precise but adds maintenance cost
   for a small title-bar row.

4. Accessory priority is semantic and left-to-right.

   The row order is title, activity/status, cwd or worktree, branch, process or
   alan state, close. Narrow-width fallback should remove or iconize from the
   lowest-priority end before touching activity/status or title.

   Alternative considered: keep the current accessory projection order and
   fixed widths. That is lower risk mechanically, but it does not solve the
   fit-content and narrow-pane behavior.

5. Tests should guard contracts, not screenshots.

   Automated checks should verify the code keeps title text persistent,
   avoids selected/unselected title-bar fills, uses staged fallback, and does
   not reintroduce fixed-width accessory slots. Visual review remains required
   for actual light-mode readability because contrast and composition are
   perceptual.

## Risks / Trade-offs

- Reduced local focus signal in the title row -> Preserve pane-level focus
  treatment and verify split panes remain scannable in a running app.
- Icon-only accessories may become cryptic -> Keep help/accessibility labels
  on icon-only states and reserve icon-only fallback for secondary detail.
- `ViewThatFits` fallback may be too coarse for very narrow panes -> Keep the
  staged approach first; upgrade to a custom layout only if implementation
  evidence shows unacceptable truncation or hiding behavior.
- Higher contrast text may feel heavier -> Use role-specific typography and
  foreground tokens instead of increasing font size or adding a background.
- Existing contract checks may be too pattern-specific -> Update checks to
  assert durable behavior while avoiding brittle exact SwiftUI structure where
  possible.

## Migration Plan

No persisted state, protocol, or runtime migration is required. The change is
local to macOS shell presentation and verification. Rollback is a UI-only
revert of the title-bar visual/layout changes while leaving existing title
metadata and pane close routing intact.

## Open Questions

None. The selected direction is the staged fit-content title bar with persistent
text title, terminal-surface background, higher foreground contrast, and
accessory degradation before title degradation.
