## Why

Alan's macOS shell now has persistent Spaces, Tabs, pin snapshots, and stable
terminal runtimes, but users still cannot organize Tabs with the direct
manipulation expected from an Arc-like sidebar. Per-Space pinned sections,
drag-based ordering, and explicit Move to Space actions are needed before the
shortcut system can expose reliable Tab organization commands.

## What Changes

- Split each Space's Tab list into lightweight Pinned and Unpinned visual
  sections inspired by Arc's sidebar treatment.
- Allow whole-row Tab dragging with a movement threshold so short clicks still
  select the Tab.
- Support realtime insertion previews while reordering within a section or
  dragging across Pinned and Unpinned sections.
- Treat Unpinned-to-Pinned drag as Pin current state and save the pin snapshot.
- Treat Pinned-to-Unpinned drag as Unpin.
- Add registry-backed menu and context-menu actions for Pin, Unpin, Move Tab
  Left/Right, and Move Tab to Space.
- Keep cross-Space Tab movement in menus/context menus for the first version;
  dragging a Tab to the Space switcher is out of scope.
- Preserve Tab, pane, and terminal runtime identity across reorder, pin/unpin,
  and Move to Space.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-workspace-interactions`: Adds per-Space Pinned/Unpinned Tab
  sections, drag reorder, drag pin/unpin, and Move Tab to Space semantics.
- `macos-shell-workspace-persistence`: Persists per-Space Tab order, pin state,
  pin snapshots, and Space ownership immediately after organization mutations.
- `macos-shell-ui-ux-conformance`: Adds Arc-like sidebar visual constraints for
  Pinned/Unpinned Tab sections and drag insertion feedback.
- `macos-shell-control-plane-reliability`: Adds authoritative mutation results
  and events for Tab reorder, pin/unpin, and Move Tab to Space.
- `macos-shell-build-test-contract`: Adds focused validation for drag behavior,
  target semantics, manifest writes, and runtime identity preservation.

## Impact

- Depends on `add-macos-shell-action-registry` for shared Tab organization
  actions and target resolution.
- Apple shell model and manifest mutation helpers for Tab order, pin state, and
  Space ownership.
- Sidebar Tab list UI, context menus, and drag/drop state.
- Shell events, local command execution, and focused Apple tests.
