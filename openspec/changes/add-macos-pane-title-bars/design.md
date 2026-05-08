## Context

`TerminalPaneView` currently renders the selected tab's `ShellPaneTreeNode` into
`ShellTerminalLeafView` leaves. Each leaf owns a `TerminalHostView`, stable
`.id(pane.paneID)`, inactive-pane dimming, and split dividers, but there is no
per-pane visible title or close affordance. The only lightweight metadata strip
is global to the selected pane, so split panes can be hard to identify at a
glance.

The title data already exists in the model and is already owned by the terminal
lifecycle contract. `TerminalHostRuntimeSnapshot` projects terminal metadata
into `ShellPane.viewport?.title`, and the UI already has normalization helpers
such as `shellNormalizedTitle(...)` and `shellDisplayTitle(...)`. This change
only decides how the pane title bar consumes that metadata. Close behavior also
already exists in the model and controller: `ShellStateSnapshot.closingPane(...)`
repairs split trees, falls back to tab close when a tab has one pane, and
preserves remaining pane runtimes through the shared mutation path.

## Goals / Non-Goals

**Goals:**

- Add a narrow title bar to every visible terminal pane, including single-pane
  and split-pane tabs.
- Prefer the current terminal title from `ShellViewportSnapshot.title`, with
  cwd, working-directory, launch-target, and process fallbacks only when the
  terminal title is unavailable.
- Close the exact pane whose title-bar button was clicked, without depending on
  whatever pane was selected before the click.
- Keep the chrome visually quiet and compatible with the existing Arc-like,
  terminal-first macOS shell contract.
- Preserve terminal hit-testing, text selection, mouse reporting, and scroll
  behavior by keeping the title bar outside the terminal host surface.

**Non-Goals:**

- Add a general pane-management toolbar, pane tab strip, breadcrumbs, badges, or
  debug metadata to the pane header.
- Change spaces/tabs IA, split-tree persistence, pane runtime ownership, or
  terminal renderer internals.
- Add drag-to-move, zoom, search, or command palette changes; those remain owned
  by the advanced split/workspace changes.

## Decisions

1. Render pane title bars in `ShellTerminalLeafView`.

   Each leaf already has the pane model, selected state, runtime registry, and
   workspace command hooks. Placing the header there makes the title bar
   per-pane by construction and keeps split branches agnostic to leaf chrome.

   Alternative considered: render one header above `TerminalPaneView`. That
   keeps the layout simple but fails the core requirement because only the
   selected pane would be identified. Alternative considered: overlay the header
   directly on top of `TerminalHostView`. That preserves terminal area but risks
   occluding the first terminal row and reintroducing hit-testing ambiguity.

2. Add a pane-specific title helper that reuses existing normalization but
   changes the priority order.

   The pane title bar should describe the current terminal surface first, so it
   should prefer `shellNormalizedTitle(pane.viewport?.title)`. Working directory
   and process labels remain useful fallbacks. This avoids changing
   `shellDisplayTitle(...)`, whose cwd-first ordering is still appropriate for
   tab/sidebar contexts.

   Alternative considered: use `shellDisplayTitle(...)` unchanged. That would be
   low-risk mechanically, but it can show cwd instead of the actual terminal
   title when both are present.

3. Target close by pane ID, not by selected pane.

   The title-bar close button should call a controller-owned targeted close path
   for the pane represented by that leaf. The implementation can expose the
   existing private `closePane(paneID:)` mutation as a focused controller method
   or add an equivalent wrapper, but it must keep the existing model semantics:
   repair split trees, close a single-pane tab through tab-close semantics, and
   leave the only remaining tab unchanged.

   Alternative considered: focus the pane and then call `closeSelectedPane()`.
   That couples a destructive action to selection order and can close the wrong
   pane if focus changes are coalesced or rejected.

4. Keep title-bar chrome fixed-height and non-invasive.

   The first implementation should use a compact native row: truncated title,
   optional subtle active/inactive treatment, and one icon-only close button with
   tooltip/accessibility label. The title bar should not add cards, large
   padding, shadows, or extra metadata chips. Long titles must truncate without
   resizing panes or split dividers.

5. Verify UI consumption with focused tests plus one visual/manual pass.

   The existing terminal metadata tests already cover title projection into pane
   metadata. This change should only add coverage for the title-bar helper's
   consumption order and targeted close semantics. Hit-testing and visual polish
   still need a running-app screenshot or manual note because the quality bar is
   about terminal input behavior and native layout feel, not just model
   mutation.

## Risks / Trade-offs

- Header height reduces terminal rows -> Keep the row narrow and stable, and
  verify single-pane and split-pane screenshots before marking UI work complete.
- Close button steals terminal input focus -> Keep the button outside the
  terminal host surface and return focus to the next selected terminal after
  successful close.
- Inactive-pane close targets the wrong pane -> Route by pane ID and add a
  focused test for closing a non-selected visible pane.
- Terminal titles can be empty, noisy, or cwd-like -> Normalize and fallback
  consistently, but do not expose raw pane IDs or runtime metadata in default
  UI.
- Existing advanced-splits work may touch the same files -> Keep this change
  narrow: pane header, title derivation, close routing, and verification only.

## Migration Plan

No persisted state or external API migration is required. The change is a local
macOS UI/controller polish pass. If implementation needs to roll back, remove
the leaf title-bar wrapper and targeted close UI while leaving the existing
split model and runtime service intact.

## Open Questions

- Should the title bar remain visible when there is only one total pane in the
  app, or should the close button be disabled while the title remains visible?
  Default decision for implementation: keep the title visible and disable or
  no-op the close control when the existing model reports `lastTab`.
