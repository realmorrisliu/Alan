## Context

`upgrade-macos-shell-splits-commands` is archived around the #355 scope: durable
split ratios, dividers, spatial focus, close/equalize, pane lift, cross-tab move,
native menu/shortcut routing, and command UI. The remaining work is less about
basic splits and more about advanced workspace interactions where incorrect
ownership can lose terminal continuity or steal terminal input.

The main constraints are:

- Terminal runtime identity must remain pane-keyed and service-owned.
- Terminal text selection and terminal app input must not be compromised by
  drag/drop or shortcut handling.
- Native menu, keyboard, command UI, context menu, and control-plane commands
  must converge on the same controller mutations.
- Debug and implementation identifiers must stay out of the default UI.

## Goals / Non-Goals

**Goals:**

- Add split zoom/unzoom without closing or rebuilding sibling runtimes.
- Add explicit in-tab pane movement and define the quality bar for drag/drop.
- Extend the control plane for resize, equalize, zoom/unzoom, spatial focus, and
  movement results.
- Finish copy, paste, and terminal search command routing across native menus,
  keyboard shortcuts, command UI, and terminal host surfaces.
- Add tests and visual evidence for advanced split workflows.

**Non-Goals:**

- Revisit the completed #355 split ratio, divider, spatial focus, and basic
  command work except where needed for advanced workflows.
- Add App Intents or system Shortcuts integration.
- Change Alan's spaces/tabs IA or introduce a second window/tab model.

## Decisions

1. Treat zoom as tab-scoped view state, not split-tree mutation.

   Zoom hides sibling panes in the visible layout while leaving the canonical
   split tree and runtime service intact. This avoids rebuilding runtimes and
   keeps unzoom deterministic.

   Alternative considered: replace the split tree with a single leaf while
   zoomed. That is simpler to render but risks losing sibling runtime context
   and complicates control-plane state.

2. Implement explicit move commands before enabling drag/drop by default.

   Controller-owned move commands can validate source/target tabs and preserve
   runtime identity before gesture complexity is introduced. Drag/drop can then
   call the same command path once terminal selection behavior is proven.

   Alternative considered: start with direct drag/drop. That would make it
   harder to separate model bugs from hit-testing and terminal-selection bugs.

3. Route native commands through a command target resolver.

   Copy, paste, search, zoom, focus, and movement need a consistent target:
   selected terminal host, focused pane, selected tab, or command UI row. A
   resolver keeps menu, keyboard, and command UI behavior aligned.

   Alternative considered: let each surface decide locally. That repeats the
   old hidden-button problem and makes terminal-host ownership ambiguous.

4. Make control-plane results semantic, not just boolean.

   Advanced commands should return stable error codes, changed IDs, and resulting
   state fields where useful. This keeps agent automation observable without
   exposing UI-only implementation details.

## Risks / Trade-offs

- Zoom state can drift from selected tab -> Scope zoom to tab ID and clear it
  when the tab or pane disappears.
- Drag/drop can steal terminal selection -> Gate drag/drop behind explicit
  handle/affordance behavior or a quality review before enabling default pane
  dragging.
- Copy/paste/search can conflict with terminal app shortcuts -> Prefer native
  command-key routing and delegate terminal-owned behavior to the focused host.
- Control-plane surface area can become broad -> Add only commands that have a
  controller-owned mutation and clear result contract.
