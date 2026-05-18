## Context

The macOS sidebar already treats split tabs as first-class tab rows: a multi-pane
terminal tab uses a compact leading topology indicator instead of a generic
terminal icon. The current contract covers one pane, two-pane split direction,
and a generic complex case. That leaves common layouts such as three columns,
three rows, or main-pane-plus-stack visually collapsed into the same generic
state.

The indicator lives in a constrained tab-row slot. It must remain scan-friendly,
stable in size, and consistent with Alan's light-mode, material sidebar design.
It should explain topology, not become a miniature pane manager.

## Goals / Non-Goals

**Goals:**

- Classify visible pane trees into a small user-facing topology vocabulary.
- Render common 3-pane layouts, including left/middle/right and top/middle/bottom.
- Render common 4-pane layouts only when they remain legible at tab-row size.
- Render unrecognized or high-pane-count layouts as a single-pane base shape with
  an overlaid pane count.
- Preserve stable tab-row dimensions, accessibility labels, and focus affordances.

**Non-Goals:**

- Persist new topology state or change split mutation semantics.
- Render exact split ratios or arbitrary binary-tree nesting in the sidebar.
- Add a pane-management toolbar, pane selector strip, or persistent labels to tab
  rows.
- Change terminal runtime ownership, pane lifecycle, or control-plane APIs.

## Decisions

### Derive topology from the pane tree at the UI boundary

Introduce or refine a derived topology projection for sidebar use instead of
making the indicator inspect arbitrary tree details inline. The projection should
map the current pane tree and visible pane IDs to a compact enum-like vocabulary:

- single
- two columns / two rows
- three columns / three rows
- three-pane main-with-stack variants
- four columns / four rows, if legible
- grid 2x2
- complex count

This keeps the visual component simple and makes the classification testable
without launching the full app UI.

Alternative considered: draw a proportional miniature of the split tree directly
from pane ratios. That was rejected because the sidebar indicator is too small
for arbitrary ratios to stay legible, and it would make row scanning depend on
implementation detail rather than a stable visual vocabulary.

### Normalize simple same-direction chains

Same-direction split chains should flatten into columns or rows when all visible
leaves are in the same axis and the count remains legible. For example, three
vertical leaves become a left/middle/right indicator, and three horizontal leaves
become top/middle/bottom.

Nested mixed-axis trees should classify as main-plus-stack or grid when the
structure is recognizable. Otherwise the projection should return complex count.

### Complex count overlays the single-pane base

Complex N-pane indicators should use the same visual base as a single pane, with
a compact monospaced count overlaid on top of the shape. The count must not be a
separate trailing badge or adjacent text, because that changes the tab row from
topology into metadata and increases visual noise.

The overlay may be centered or otherwise optically balanced within the indicator,
but it must not resize the row or obscure selection/focus affordances.

## Risks / Trade-offs

- **Risk: too many topology variants become illegible.** Mitigation: keep the
  vocabulary small and route unrecognized or high-count layouts to complex count.
- **Risk: classifier drift from split mutation behavior.** Mitigation: add focused
  tests for representative pane trees, including same-direction chains, nested
  main-stack layouts, 2x2 grid, and complex fallback.
- **Risk: focus state becomes ambiguous in compact 3/4-pane icons.** Mitigation:
  mark focus only when the displayed topology can map a segment back to a visible
  pane; use accessibility labels for the full pane count and focused position.
- **Risk: visual polish regressions are missed by model tests.** Mitigation:
  require screenshot or manual visual evidence for selected, hover, focused, and
  complex-count examples in the light-mode sidebar.
