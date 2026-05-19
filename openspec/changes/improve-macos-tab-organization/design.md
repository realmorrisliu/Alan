## Context

Alan already treats Spaces as durable containers and Tabs as restorable shell
work units. Pinned Tabs have explicit restore snapshots, while unpinned Tabs
have lifecycle semantics. The missing piece is the interaction layer: users need
to organize Tabs directly in the sidebar without breaking terminal continuity.

The visual direction should reference Arc's sidebar: lightweight top Pinned
items, a subtle divider or spacing boundary, and regular Tabs below. This
change does not add folders, nested groups, or a global pinned area.

## Goals / Non-Goals

**Goals:**

- Add per-Space Pinned and Unpinned Tab sections.
- Support whole-row drag reorder with realtime insertion feedback.
- Allow drag across sections to pin or unpin.
- Save the pin snapshot when a Tab is dragged into the Pinned section.
- Move Tabs between Spaces through explicit menu/context actions.
- Preserve Tab, pane, split, and terminal runtime identity for every
  organization mutation.
- Persist ordering, pin state, Space ownership, and pin snapshots immediately.

**Non-Goals:**

- No global Pinned Tabs.
- No folders, tab groups, or nested collections.
- No drag-to-Space-switcher cross-Space movement in the first version.
- No Command UI integration.
- No runtime rebuild when moving or reordering Tabs.

## Decisions

### 1. Pinned is per Space

Pinned Tabs belong to their Space. Moving a Pinned Tab to another Space keeps it
Pinned and inserts it at the end of the target Space's Pinned section. Moving an
Unpinned Tab inserts it at the end of the target Space's Unpinned section.

This preserves the user's local organization without turning Pinned Tabs into a
global browser-style shelf.

### 2. Use two lightweight visual sections

The sidebar should not add heavy section headers. Pinned and Unpinned areas are
separated by spacing and a subtle divider, with Arc-like stable rows and a
lightweight New Tab affordance. Drag insertion lines and row movement should
provide feedback during reorder rather than explanatory text.

### 3. Whole-row drag with a threshold

Tab rows are draggable, but short clicks still select the Tab. Drag begins only
after a movement threshold. Right click, close overlay, and split topology
indicator interactions remain independent and do not accidentally start a
reorder.

### 4. Cross-section drag changes pin state

Dragging from Unpinned to Pinned is equivalent to Pin current state and saves
the current restore snapshot. Dragging from Pinned to Unpinned unpins the Tab.
Dragging within the same section only changes order.

### 5. Cross-Space movement is explicit

The first version uses `Move Tab to Space...` in menus and Tab context menus.
If the moved Tab is currently selected, Alan follows it to the target Space and
keeps it selected. If the moved Tab is not selected, Alan stays on the current
Space.

### 6. Organization never restarts terminal runtimes

Reorder, pin, unpin, and Move to Space mutate shell organization only. Tab IDs,
pane IDs, split trees, terminal runtime handles, scrollback, and pending
delivery state stay attached to the same Tab and panes.
