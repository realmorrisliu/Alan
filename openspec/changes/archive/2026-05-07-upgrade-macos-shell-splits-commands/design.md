## Context

Alan already has a shell model for spaces, tabs, panes, and split trees. The UI
direction is also clear: Arc-like spaces and tabs in a material sidebar with the
terminal as the center of gravity. This change captures the first completed
native workspace layer from #355: durable split ratios, resize dividers,
directional splits, spatial focus, equalize and close commands, pane lift or
cross-tab move, menu/keyboard routing, and a restrained command UI.

Ghostty provides a strong reference for terminal window interactions, but Alan
must preserve its own space/tab model and agent control plane. This change turns
Alan's shell model into an interactive split workspace rather than a static
split renderer, while leaving zoom and drag/drop movement to later work.

## Goals / Non-Goals

**Goals:**

- Replace equal recursive split layout behavior with ratio-based split nodes and
  stable structural identity.
- Add resize, equalize, spatial focus, close, pane lift, and cross-tab pane move
  operations that preserve terminal runtime identity when panes remain alive.
- Promote common terminal workspace actions into native menu, keyboard, and
  command UI surfaces.
- Keep the toolbar restrained and native, with the command entry point and
  frequent actions only.
- Extend the control plane with authoritative results and events for split,
  focus, lift, move, and close operations that agents need.

**Non-Goals:**

- Complete low-level terminal surface parity. That belongs to
  `complete-macos-terminal-surface`.
- Add App Intents or Shortcuts integration. That belongs to
  `add-macos-shell-automation-tests`.
- Change Alan's brand direction into Ghostty's exact UI.
- Add split zoom/unzoom, drag/drop movement, arbitrary in-tab pane movement,
  control-plane resize/equalize/zoom commands, or complete copy/paste/search
  command ownership.

## Decisions

1. Introduce a ratio-based `ShellSplitTree`.

   Each branch stores direction, child IDs, and ratio. Leaves reference pane IDs.
   Ratios persist with the tab and are clamped to usable minimums. Structural IDs
   let the view diff and controller mutate split nodes without confusing pane
   runtime identity.

   Alternative considered: derive ratios from view geometry only. That would
   make split state non-durable and impossible for the control plane to inspect.

2. Route split layout mutations through the shell controller while keeping
   runtime ownership separate.

   The shell controller mutates the shell model, validates geometry constraints,
   and coordinates split, resize, equalize, focus, lift, move, and close actions.
   The runtime service owns pane lifetimes; controller actions only finalize
   runtimes when panes or tabs are actually closed.

   Alternative considered: let each split view mutate local state. That makes
   keyboard commands, menu commands, and control-plane operations diverge.

3. Route workspace commands through the responder chain and shell controller.

   Native menu items, command palette actions, toolbar buttons, and keyboard
   shortcuts all call the same command functions. The focused pane and selected
   tab determine the target.

   Alternative considered: keep hidden SwiftUI buttons for shortcuts. That is
   quick but not robust enough for a Mac terminal-grade app.

4. Preserve pane runtime identity across resize, focus, lift, and cross-tab move.

   Lifting a pane to a tab or moving it across tabs changes model placement, not
   its pane ID or runtime handle. Resize, equalize, and focus are layout/focus
   mutations only and do not restart terminal runtimes.

   Alternative considered: rebuild panes during moves. That would lose
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
- Split seams can be either too visually loud or too subtle -> Use a subtle
  two-pixel seam plus preference-backed inactive-pane dimming instead of
  per-pane cards or fixed gaps.
- Drag/drop and arbitrary in-tab movement are larger interaction surfaces ->
  Keep this archive to pane lift and cross-tab move semantics already merged.
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
5. Add pane lift, cross-tab move, equalize, and control-plane results/events.
6. Verify single-pane, multi-pane, command UI, menu shortcuts, and inspector-off
   screenshots.

## Open Questions

- What shortcut set should Alan reserve for its shell versus pass to terminal
  apps beyond standard command-key commands?
- Should split zoom persist per tab across app restart or remain a transient
  window interaction?
- How much pane drag/drop or arbitrary in-tab pane movement is necessary beyond
  pane lift and cross-tab move?
