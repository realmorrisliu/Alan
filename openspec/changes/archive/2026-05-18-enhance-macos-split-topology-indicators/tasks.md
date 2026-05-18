## 1. Topology Projection

- [x] 1.1 Add a sidebar-facing split topology projection that classifies visible pane trees into single, two-column, two-row, three-column, three-row, three-pane main-stack, recognized four-pane, and complex-count cases.
- [x] 1.2 Normalize same-direction split chains into column or row topology when the visible pane count remains legible.
- [x] 1.3 Map recognized topology segments back to visible pane IDs so focused-pane treatment and accessibility labels can stay accurate.

## 2. Sidebar Indicator Rendering

- [x] 2.1 Update the sidebar split indicator renderer to draw three-column, three-row, three-pane main-stack, and recognized four-pane topology shapes within the existing tab-row indicator footprint.
- [x] 2.2 Update complex fallback rendering to use a single-pane-shaped topology base with the pane count overlaid on the shape, not adjacent text or a separate badge.
- [x] 2.3 Preserve stable tab-row dimensions, hover/selected states, and existing focus/cycle interactions while adding the new topology variants.
- [x] 2.4 Update accessibility labels/help text so recognized topology and pane count are described in user-facing terms without raw pane IDs.

## 3. Verification

- [x] 3.1 Add focused tests for topology classification, including two-pane directions, three columns, three rows, main-plus-stack variants, recognized four-pane layouts, and complex fallback.
- [x] 3.2 Add focused tests or review fixtures that verify complex-count rendering keeps the count overlaid on the single-pane base.
- [x] 3.3 Run the relevant focused Apple shell checks and the macOS build command required for this UI surface.
- [x] 3.4 Capture or document light-mode visual evidence for selected, hover, focused-pane, three-pane, four-pane, and complex-count sidebar tab-row states.

## 4. Review And Archive Readiness

- [x] 4.1 Review the implementation against `macos-shell-ui-ux-conformance` to confirm it remains terminal-first, compact, and free of pane-management chrome.
- [x] 4.2 Verify no terminal runtime, pane lifecycle, control-plane API, or persisted pane-tree semantics changed as part of the UI projection work.
- [x] 4.3 Before archiving, sync the accepted delta specs into `openspec/specs/` and validate the full OpenSpec set.
