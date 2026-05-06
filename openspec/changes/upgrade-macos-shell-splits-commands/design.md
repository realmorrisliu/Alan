## Context

Alan already has a shell model for spaces, tabs, panes, and split trees. The UI
direction is also clear: Arc-like spaces and tabs in a material sidebar with the
terminal as the center of gravity. What is missing is mature native terminal
workspace behavior: resizable split ratios, spatial focus, pane movement, split
zoom, command routing, menu bar integration, and restrained toolbar/window
behavior.

Ghostty provides a strong reference for terminal window interactions, but Alan
must preserve its own space/tab model and agent control plane. This change turns
Alan's shell model into a real native terminal workspace rather than a static
split renderer.

## Goals / Non-Goals

**Goals:**

- Replace equal recursive split layout behavior with ratio-based split nodes and
  stable structural identity.
- Add resize, equalize, zoom, spatial focus, and pane move operations that
  preserve terminal runtime identity when panes remain alive.
- Promote common terminal workspace actions into native menu and keyboard
  command surfaces.
- Keep the toolbar restrained and native, with the command entry point and
  frequent actions only.
- Extend the control plane with authoritative results for split/focus/move/close
  operations that agents need.

**Non-Goals:**

- Complete low-level terminal surface parity. That belongs to
  `complete-macos-terminal-surface`.
- Add App Intents or Shortcuts integration. That belongs to
  `add-macos-shell-automation-tests`.
- Change Alan's brand direction into Ghostty's exact UI.

## Decisions

1. Introduce a ratio-based `ShellSplitTree`.

   Each branch stores direction, child IDs, and ratio. Leaves reference pane IDs.
   Ratios persist with the tab and are clamped to usable minimums. Structural IDs
   let the view diff and controller mutate split nodes without confusing pane
   runtime identity.

   Alternative considered: derive ratios from view geometry only. That would
   make split state non-durable and impossible for the control plane to inspect.

2. Add a split layout controller separate from the terminal runtime service.

   The split controller mutates the shell model, validates geometry constraints,
   and coordinates focus/zoom/move actions. The runtime service owns pane
   lifetimes; the split controller only tells it when panes are closed or moved.

   Alternative considered: let each split view mutate local state. That makes
   keyboard commands, menu commands, and control-plane operations diverge.

3. Route workspace commands through the responder chain and shell controller.

   Native menu items, command palette actions, toolbar buttons, context menus,
   and keyboard shortcuts all call the same command functions. The focused pane
   and selected tab determine the target.

   Alternative considered: keep hidden SwiftUI buttons for shortcuts. That is
   quick but not robust enough for a Mac terminal-grade app.

4. Preserve pane runtime identity across movement and zoom.

   Moving a pane changes its position in the split tree, not its pane ID or
   runtime handle. Zoom hides sibling layout temporarily without closing their
   terminal surfaces.

   Alternative considered: rebuild panes during moves or zoom. That would lose
   scrollback/process continuity and violate the terminal lifecycle contract.

5. Keep window chrome quiet.

   The toolbar/titlebar should not become a dashboard. Space and tab
   organization remains in the sidebar; commands are available through menu,
   keyboard, and a compact `Go to or Command...` entry point.

## Risks / Trade-offs

- Ratio persistence can make layout migration tricky -> Provide a one-time
  migration from existing equal split trees and clamp invalid ratios at load.
- Keyboard shortcuts can conflict with terminal apps -> Route command-key
  shortcuts through native command handling and leave non-command terminal keys
  to the terminal surface adapter.
- Split zoom can confuse control-plane focus -> Treat zoom as view state tied to
  the tab and keep pane runtime state unchanged.
- Drag/drop pane movement may be hard to perfect -> Support explicit move
  commands first if drag/drop quality is not ready.
- Native app tabbing can conflict with Alan tabs -> Disable or scope native
  tabbing where Alan's custom tabs own organization.

## Migration Plan

1. Add ratio and structural identity fields while reading existing split trees as
   equal ratios.
2. Implement split mutations in the shell controller and route existing UI
   actions through them.
3. Replace recursive equal layout rendering with a ratio-aware split view and
   native dividers.
4. Add command/menu/keyboard routing for the same controller actions.
5. Add pane movement, zoom, equalize, and control-plane results.
6. Verify single-pane, multi-pane, command UI, menu shortcuts, and inspector-off
   screenshots.

## Open Questions

- What shortcut set should Alan reserve for its shell versus pass to terminal
  apps beyond standard command-key commands?
- Should split zoom persist per tab across app restart or remain a transient
  window interaction?
- How much pane drag/drop is necessary for the first version if explicit move
  commands are reliable?
