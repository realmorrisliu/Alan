## Context

The active macOS shell UI is centered in `MacShellRootView.swift`, with
`TerminalPaneView.swift` and `TerminalHostView.swift` owning terminal rendering
and input. The current shell still treats the inspector as a product surface:
`MacShellRootView` stores `alanShellShowsInspector`, renders a right-side
`ShellInspectorView`, exposes a sidebar toggle, includes a command-palette
`toggleInspector` action, and teaches speech recognition inspector commands.

That inspector duplicates information already available in shell snapshots,
runtime metadata, logs, and focused test/debug scripts. It also works against
the current visual direction: terminal first, fewer controls, native material,
and progressive disclosure through explicit debug tools rather than a permanent
secondary pane.

Terminal search already has a useful backend boundary. `Command-F` routes to the
focused pane, `TerminalSurfaceController` owns an `AlanTerminalSearchAdapter`,
and Ghostty-backed surfaces receive `start_search`, `search:<query>`,
`navigate_search:*`, and `end_search` actions. The weak part is the user
experience: search text is entered by intercepting terminal key events and
presented as a generic terminal overlay, so it does not behave like a normal
macOS Find control.

## Goals / Non-Goals

**Goals:**

- Remove inspector from the default macOS shell product surface and interaction
  model.
- Keep terminal/runtime diagnostics available outside the default UI through
  existing developer surfaces.
- Make `Command-F` open a focused, native-feeling Find bar for the focused pane.
- Route query edits, match navigation, and dismissal through the existing
  pane-scoped terminal search engine.
- Keep sidebar, split layout, and terminal geometry stable while Find is open.
- Update tests and contract checks so inspector does not drift back in as a
  default affordance.

**Non-Goals:**

- Remove shell snapshots, runtime metadata, logging, or debugging capability.
- Add global workspace search, transcript search, semantic search, or search
  across multiple panes/tabs.
- Replace Ghostty's search backend or redefine search-result computation.
- Redesign the command palette beyond removing inspector actions and stale copy.
- Implement a dark-mode pass.

## Decisions

1. Remove inspector completely from user-facing shell UI.

   Delete the right-side `ShellInspectorView`, `ShellInspectorSection`,
   `InspectorCard`, sidebar toggle, command-palette `toggleInspector` action,
   inspector speech commands, and `alanShellShowsInspector` storage. The default
   shell should have two structural regions: material sidebar and terminal
   workspace.

   Alternative considered: keep a hidden debug inspector behind a menu item.
   That preserves current code but keeps a product concept that the user has
   judged low-value. Debug needs should use explicit developer surfaces instead
   of keeping default IA alive for a rarely used pane.

2. Preserve diagnostics through developer/debug surfaces, not product chrome.

   Any data currently visible only in the inspector should be checked against
   existing alternatives before deletion: `alan shell state`, copy-snapshot
   command paths, runtime logs, focused scripts, and test fixtures. If an
   inspector-only datum is still needed, add it to one of those explicit debug
   paths rather than creating another in-app panel.

   Alternative considered: move inspector content into the command palette.
   That would clutter the command UI and keep debug state too close to normal
   navigation.

3. Introduce a real SwiftUI/AppKit Find bar instead of printable-key capture.

   `Command-F` should show a compact pane-scoped Find bar with a focused text
   field. While the field is focused, normal text editing keys belong to the
   text field, not to `TerminalHostView`'s terminal key interception. Query
   changes call `updateSearchQuery(_:)`, and the existing
   `AlanTerminalSearchAdapter` remains the state bridge for query, total
   matches, selected match, and active/inactive state.

   Alternative considered: improve the existing terminal overlay copy only.
   That would still feel unlike standard macOS Find because there is no real
   editable field, selection, cursor, or native focus model.

4. Anchor Find to the focused terminal pane without resizing the workspace.

   The first implementation should render the Find bar as a compact overlay or
   leaf-level tool surface inside the focused pane's chrome layer. It should not
   open a command-palette sheet, add a sidebar section, or change split ratios.
   In split mode, only the focused pane shows the active Find bar; switching pane
   focus either moves the bar to that pane's active search state or dismisses the
   previous visual affordance according to the pane-owned search state.

   Alternative considered: add a full-width window find bar below the toolbar.
   That is common in document apps, but Alan's search target is a terminal pane;
   a window-wide bar makes split-pane targeting ambiguous.

5. Match macOS Find keyboard expectations.

   Required shortcuts:

   - `Command-F`: open Find for the focused pane and focus/select the query
     field.
   - `Return` or `Command-G`: next match.
   - `Shift-Return` or `Shift-Command-G`: previous match.
   - `Escape`: close Find and return focus to the owning terminal pane.

   Optional controls should be icon-first: previous, next, close, and a match
   count such as `2 of 9` or `No results`. The UI must avoid raw pane IDs,
   Ghostty action names, and debug routing details.

6. Keep backend search lifecycle pane-owned.

   Search start/update/navigation/end continues through the focused pane's
   `TerminalSurfaceController` and `AlanTerminalSearchEngine`. View rebuilds
   should not lose active search state for the pane runtime. Closing Find calls
   `endSearch()` and returns first responder to the terminal host when possible.

## Risks / Trade-offs

- Removing inspector can hide a useful debug datum -> Before deleting each
  section, confirm the data is available from shell snapshot, logs, tests, or an
  explicit debug command; move only truly needed data.
- Stale active OpenSpec changes mention inspector -> Update or rebase those
  changes so this removal remains the source of truth.
- Find bar focus can steal terminal input unexpectedly -> Confine typed query
  input to the focused text field and make Escape reliably restore terminal
  focus.
- Search state can target the wrong pane after focus changes -> Keep all search
  actions pane-scoped and add focused tests for split-pane search ownership.
- Overlay placement can occlude terminal content -> Keep the bar compact,
  material-backed, and anchored where it does not resize or obscure split chrome;
  verify with screenshots.

## Migration Plan

No persisted data migration is required beyond removing or ignoring
`alanShellShowsInspector`. Implementation should remove inspector UI and command
surface references first, then introduce the Find bar on top of the existing
search adapter/engine. Visual verification should cover the default light-mode
window, split panes, `Command-F`, search navigation, and dismissal back to the
terminal.

## Open Questions

None. Default decision: delete inspector as a product feature now, and use
explicit debug commands/scripts if later work needs deeper shell diagnostics.
