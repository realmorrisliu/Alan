## MODIFIED Requirements

### Requirement: Visual system follows native material guidance
The default macOS shell SHALL use a light-mode-first native material visual
system that feels calm, precise, and terminal-oriented. It SHALL avoid
card-heavy dashboard composition, decorative gradients, hard-coded dominant
theme panels, persistent secondary inspector panes, and ornamental controls.

#### Scenario: Material sidebar
- **WHEN** the app window is visible in the default light appearance
- **THEN** the space rail and tab list use material-backed surfaces, subtle separators, and restrained selection states rather than an opaque themed sidebar panel

#### Scenario: Stable compact controls
- **WHEN** the user hovers, selects, inserts, closes, or switches tabs and spaces
- **THEN** rows, icon controls, counters, and status marks keep stable dimensions and do not resize the sidebar or terminal content

#### Scenario: No dashboard treatment
- **WHEN** the user views the default shell
- **THEN** the UI does not present page-like sections, nested cards, large explanatory panels, persistent inspector panes, or marketing-style hero composition

### Requirement: Default UI hides implementation jargon
The default macOS UI SHALL avoid exposing raw pane IDs, `tab_id`, binding,
runtime phases, `window attached`, `title updated`, and other implementation
terms outside explicit developer debug surfaces.

#### Scenario: Normal terminal workflow
- **WHEN** a user creates, selects, splits, searches, or closes tabs and panes
- **THEN** visible copy uses product terms such as Space, Tab, Split, Find, Go to or Command, Open in Alan, and Ask Alan

#### Scenario: Command search results
- **WHEN** the command UI shows tabs, panes, actions, routing candidates, or attention items
- **THEN** result titles and summaries use user-facing names where available and do not expose raw pane IDs as the primary label unless the user is in an explicit debug context

#### Scenario: Developer diagnostics
- **WHEN** a developer opens an explicit debug surface such as a shell snapshot, log, test fixture, or debug command
- **THEN** implementation details may be shown with clear debug framing outside the default product UI

### Requirement: Toolbar is native and restrained
The macOS toolbar/titlebar SHALL feel like native window chrome and contain only
the current tab title/context, one command entry point, and a small number of
frequent actions.

#### Scenario: Toolbar default state
- **WHEN** no urgent attention item exists
- **THEN** the toolbar does not show attention as a large standalone primary control

#### Scenario: Command entry
- **WHEN** the user invokes the command UI
- **THEN** the entry point is labeled and organized as `Go to or Command...`

#### Scenario: Inspector absent
- **WHEN** the user scans the toolbar, sidebar header, and default shell chrome
- **THEN** there is no persistent inspector toggle or right-side inspector affordance

### Requirement: UI conformance is verified visually
Mac shell UI changes SHALL be reviewed against the documented UI contract before
the UI conformance tasks are marked complete.

#### Scenario: Default screenshot review
- **WHEN** a UI conformance implementation pass is ready for review
- **THEN** maintainers can inspect a running-app screenshot of the default light-mode window showing the space rail, active-space tab list, terminal-first content area, and no inspector pane or inspector toggle

#### Scenario: Find bar screenshot review
- **WHEN** terminal search UI tasks are marked complete
- **THEN** maintainers can inspect screenshots or recorded notes for `Command-F`, typed search text, match navigation, no-result state, and dismissal back to terminal focus

### Requirement: Terminal overlays use user-facing language
The macOS terminal UI SHALL present terminal search, child-exit, renderer
failure, readonly, input-not-ready, and clipboard states with concise
terminal-user language in the canvas area or focused pane chrome, while raw
runtime details remain developer-debug-only.

#### Scenario: Renderer failure visible
- **WHEN** a focused terminal pane cannot render
- **THEN** the default UI explains that the terminal cannot draw and offers an actionable next step without showing raw Ghostty callback names or pane IDs

#### Scenario: Child exit visible
- **WHEN** a terminal child process exits
- **THEN** the pane shows a compact terminal exit state rather than debug event names

#### Scenario: Debug surface opened
- **WHEN** a developer opens an explicit debug surface outside the default product UI
- **THEN** renderer diagnostics, surface identifiers, input mode details, and raw event payloads may be inspected with debug framing

### Requirement: Terminal search does not displace workspace structure
Terminal search UI SHALL be compact, pane scoped, and presented as a
native-feeling Find bar for the focused terminal pane without turning the shell
into a dashboard or page layout.

#### Scenario: Search opens
- **WHEN** the user invokes `Command-F`
- **THEN** the focused pane shows a compact Find bar with a focused editable text field, previous and next controls, close control, and result-count feedback while the sidebar, toolbar, and split layout keep stable dimensions

#### Scenario: Search updates
- **WHEN** the user types or edits text in the Find bar
- **THEN** query changes are applied to the focused pane's terminal search engine without sending those characters as terminal input

#### Scenario: Search navigates
- **WHEN** matches exist and the user presses Return, `Command-G`, Shift-Return, Shift-`Command-G`, or the previous/next controls
- **THEN** Alan moves between matches in the focused pane and updates visible match feedback such as `2 of 9`

#### Scenario: Search closes
- **WHEN** the user dismisses terminal search with Escape or the close control
- **THEN** the Find bar closes, terminal search ends for the owning pane, and keyboard focus returns to the terminal pane that owned the search interaction

### Requirement: Command UI owns navigation and shell actions
The default command entry SHALL present tabs, panes, spaces, routing candidates,
attention items, and common shell workspace actions through `Go to or Command...`
using user-facing labels and compact rows.

#### Scenario: Command results include panes
- **WHEN** command search lists pane targets
- **THEN** results use tab title, pane title, cwd, process context, or routing context as the primary label rather than raw pane IDs

#### Scenario: Command result invokes split action
- **WHEN** the user selects a split, focus, equalize, close, or pane lift action from command UI
- **THEN** Alan runs the same shell controller mutation used by menu and keyboard paths where that action is shared

#### Scenario: Inspector action absent
- **WHEN** the user opens `Go to or Command...` or searches for inspector-related terms
- **THEN** the command UI does not offer show, hide, open, close, or toggle inspector actions

### Requirement: Toolbar stays restrained during split interactions
Advanced split, focus, resize, equalize, close, and pane lift affordances SHALL
not turn the toolbar into a dense control strip.

#### Scenario: Multiple panes visible
- **WHEN** a tab contains multiple panes
- **THEN** the default toolbar remains focused on current tab context, command entry, and frequent actions without adding an inspector toggle or persistent pane-management strip

#### Scenario: Pane lift available
- **WHEN** pane lift is available through command UI or another explicit non-terminal affordance
- **THEN** the default toolbar does not add a persistent pane-management strip

## REMOVED Requirements

### Requirement: Inspector uses progressive disclosure
**Reason**: The inspector no longer justifies a persistent product surface in
the terminal-first macOS shell. It duplicates developer diagnostics, consumes
horizontal space, adds command and toolbar clutter, and weakens the Arc-like
terminal workspace direction.

**Migration**: Remove inspector UI and user-facing actions. Keep needed
diagnostics available through explicit developer surfaces such as shell
snapshots, logs, focused scripts, test fixtures, or future debug commands rather
than a right-side in-app inspector.
