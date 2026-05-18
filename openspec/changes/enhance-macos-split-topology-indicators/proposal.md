## Why

Split tabs currently communicate only a small subset of pane topology in the sidebar.
As Alan adds richer split workflows, the tab list needs a compact way to distinguish
common multi-pane layouts without turning the sidebar into a pane-management panel.

## What Changes

- Extend the sidebar split indicator contract so it can represent common 3-pane and
  4-pane topologies, including three-column and three-row layouts.
- Keep the default indicator compact and topology-first: it should remain a small
  tab-row affordance, not a text label, pane strip, or debug surface.
- Treat complex N-pane layouts as a single-pane-shaped indicator with the pane count
  overlaid on the shape, rather than placing the count beside the icon.
- Preserve lightweight focus affordances where the topology can identify individual
  panes without making the tab row resize.
- Add verification expectations for topology classification and sidebar visual
  coverage.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: Define the expanded split topology indicator
  behavior for sidebar tab rows.
- `macos-shell-build-test-contract`: Require coverage for topology classification
  and visual stability of the sidebar indicators.

## Impact

- Affects macOS shell sidebar tab UI, especially the split indicator rendered for
  terminal tabs.
- Affects derived shell UI projection logic that classifies pane trees into
  user-facing topology categories.
- Does not change terminal runtime ownership, split mutation semantics, control-plane
  APIs, or persisted pane tree data.
