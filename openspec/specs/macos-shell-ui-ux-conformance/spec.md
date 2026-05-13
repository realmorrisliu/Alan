# macos-shell-ui-ux-conformance Specification

## Purpose
Define the default native macOS shell UI contract: Arc-like space/tab
organization, terminal-first layout, native light-mode material treatment,
restrained toolbar behavior, pane-scoped terminal controls, and progressive
disclosure that keeps debug surfaces out of the default shell.
## Requirements
### Requirement: Sidebar matches space rail plus tab list
The default macOS sidebar SHALL present spaces as a compact vertical rail and
tabs for the active space as the primary sidebar list.

#### Scenario: Default sidebar reading order
- **WHEN** a user opens the macOS app
- **THEN** the sidebar reads as a space switcher plus active-space tab list, not as unrelated dashboard sections

#### Scenario: Space selection
- **WHEN** a user selects a space in the rail
- **THEN** the tab list updates to show only tabs belonging to that active space

#### Scenario: Separate creation affordances
- **WHEN** a user creates a new space or a new tab
- **THEN** space creation is presented as a compact rail affordance and tab creation is presented in the active-space tab list or toolbar context

#### Scenario: Lightweight tab rows
- **WHEN** the active-space tab list contains terminal and Alan tabs
- **THEN** each tab appears as a skimmable row with a compact marker, title, secondary context, and low-emphasis status rather than as a card or dashboard tile

### Requirement: Visual system follows native material guidance
The default macOS shell SHALL use a light-mode-first native material visual
system that feels calm, precise, and terminal-oriented. It SHALL avoid
card-heavy dashboard composition, decorative gradients, hard-coded dominant
theme panels, and ornamental controls.

#### Scenario: Material sidebar
- **WHEN** the app window is visible in the default light appearance
- **THEN** the space rail and tab list use material-backed surfaces, subtle separators, and restrained selection states rather than an opaque themed sidebar panel

#### Scenario: Stable compact controls
- **WHEN** the user hovers, selects, inserts, closes, or switches tabs and spaces
- **THEN** rows, icon controls, counters, and status marks keep stable dimensions and do not resize the sidebar or terminal content

### Requirement: Collapsed sidebar uses a lightweight floating panel
When the sidebar is collapsed, the macOS shell SHALL reveal navigation through a
small floating material panel triggered by intentional edge or titlebar-control
hover, while keeping the terminal workspace stable.

#### Scenario: Narrow reveal target
- **WHEN** the sidebar is collapsed and the pointer approaches the left edge
- **THEN** Alan uses a narrow edge hot zone to reveal the floating sidebar panel rather than a full titlebar or header-width hover region

#### Scenario: Floating panel hover retention
- **WHEN** the pointer moves from the edge hot zone onto the floating sidebar panel or collapsed titlebar controls
- **THEN** the floating panel remains revealed until the pointer leaves those related surfaces

#### Scenario: Floating panel motion
- **WHEN** reduced motion is disabled
- **THEN** the floating sidebar panel enters with a short spring-like leading-edge reveal and exits with a faster low-emphasis hide animation

#### Scenario: Reduced motion respected
- **WHEN** reduced motion is enabled
- **THEN** collapsed-sidebar reveal and hide behavior avoids springy movement while preserving the same hover targets and visibility state

#### Scenario: Workspace stability
- **WHEN** the floating sidebar panel appears or disappears
- **THEN** terminal content, split geometry, and window size remain stable instead of being resized by the transient sidebar surface

#### Scenario: No dashboard treatment
- **WHEN** the user views the default shell
- **THEN** the UI does not present page-like sections, nested cards, large explanatory panels, or marketing-style hero composition

### Requirement: Terminal content is the center of gravity
The main content region SHALL make the active terminal canvas visually dominant
and SHALL avoid nested decorative panels around the terminal in the default
single-pane state.

#### Scenario: Single-pane tab
- **WHEN** a tab contains one pane
- **THEN** the terminal appears nearly full-bleed within the content region and does not show a pane selector strip

#### Scenario: Split-pane tab
- **WHEN** a tab contains multiple panes
- **THEN** pane chrome stays lightweight and focus is conveyed by subtle selection treatment rather than explicit engineering labels

#### Scenario: Alan as optional capability
- **WHEN** a terminal tab is active and no Alan-specific surface has been opened
- **THEN** Alan appears as an optional command or attachment capability layered onto the terminal workflow, not as the structural center of the window

### Requirement: Default UI hides implementation jargon
The default macOS UI SHALL avoid exposing raw pane IDs, `tab_id`, binding,
runtime phases, `window attached`, `title updated`, and other implementation
terms outside explicit debug surfaces.

#### Scenario: Normal terminal workflow
- **WHEN** a user creates, selects, splits, or closes tabs and panes
- **THEN** visible copy uses product terms such as Space, Tab, Split, Go to or Command, Open in Alan, and Ask Alan

#### Scenario: Command search results
- **WHEN** the command UI shows tabs, panes, actions, routing candidates, or attention items
- **THEN** result titles and summaries use user-facing names where available and do not expose raw pane IDs as the primary label unless the user is in a debug context

#### Scenario: Debug surfaces
- **WHEN** implementation details are needed
- **THEN** they remain in explicit debug-only surfaces, logs, scripts, or snapshots rather than default shell chrome

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

#### Scenario: Empty titlebar zoom
- **WHEN** a user double-clicks an empty, non-control area of the hidden-titlebar chrome
- **THEN** Alan toggles the window between its previous frame and the current screen's visible work area while leaving the system traffic-light buttons, including the green button, on their normal macOS behavior

#### Scenario: Native fullscreen chrome
- **WHEN** the hidden-titlebar shell window enters native macOS fullscreen and the system takes over or hides the traffic-light controls
- **THEN** Alan moves its lightweight titlebar controls to the leading edge without reserving traffic-light space
- **AND WHEN** the window is actively live-resized
- **THEN** Alan continuously resynchronizes the standard traffic-light controls during the resize interaction rather than only correcting the final resting position
- **AND WHEN** the window exits native fullscreen or finishes resizing
- **THEN** Alan keeps the standard traffic-light controls at their intended inset and returns its titlebar controls to the post-traffic-light position

### Requirement: Default shell does not expose inspector chrome
The default macOS shell SHALL not include a persistent right-side inspector,
inspector toggle, or inspector command surface.

#### Scenario: Default shell opened
- **WHEN** the user opens the macOS shell
- **THEN** no inspector pane, inspector toggle, or inspector-specific command appears in the default UI

#### Scenario: Diagnostics needed
- **WHEN** maintainers need runtime diagnostics
- **THEN** diagnostics remain available through shell snapshots, logs, scripts, tests, or an explicit future debug surface rather than a default inspector

### Requirement: UI conformance is verified visually
Mac shell UI changes SHALL be reviewed against the documented UI contract before
the UI conformance tasks are marked complete.

#### Scenario: Default screenshot review
- **WHEN** a UI conformance implementation pass is ready for review
- **THEN** maintainers can inspect a running-app screenshot of the default light-mode window showing the space rail, active-space tab list, terminal-first content area, and no inspector surface

#### Scenario: Removed-inspector review
- **WHEN** inspector-removal UI tasks are marked complete
- **THEN** maintainers can inspect screenshots or recorded notes confirming the default shell has no right-side inspector and no inspector toggle

### Requirement: Terminal overlays use user-facing language
The macOS terminal UI SHALL present child-exit, renderer failure, readonly,
input-not-ready, and clipboard states with concise terminal-user language in
the canvas area, while raw runtime details remain debug-only.

#### Scenario: Renderer failure visible
- **WHEN** a focused terminal pane cannot render
- **THEN** the default UI explains that the terminal cannot draw and offers an actionable next step without showing raw Ghostty callback names or pane IDs

#### Scenario: Child exit visible
- **WHEN** a terminal child process exits
- **THEN** the pane shows a compact terminal exit state rather than debug event names

#### Scenario: Debug diagnostics inspected
- **WHEN** maintainers inspect explicit debug diagnostics
- **THEN** renderer diagnostics, surface identifiers, input mode details, and raw event payloads use debug framing outside the default shell

### Requirement: Terminal search does not displace workspace structure
Terminal search UI SHALL be compact, pane scoped, and layered over the terminal
workflow without turning the shell into a dashboard or page layout.

#### Scenario: Search opens
- **WHEN** the user invokes terminal search
- **THEN** the search control appears as a compact terminal tool for the focused pane and the sidebar, toolbar, and split layout keep stable dimensions

#### Scenario: Search closes
- **WHEN** the user dismisses terminal search
- **THEN** keyboard focus returns to the terminal pane that owned the search interaction

### Requirement: Native Find bar owns terminal search UI
Terminal search SHALL render through a compact pane-scoped Find bar rather than
the passive terminal overlay card, and query edits SHALL be routed to the
owning terminal surface search controller.

#### Scenario: Find opens
- **WHEN** the user invokes `Command-F` for a focused pane
- **THEN** Alan shows a compact Find bar for that pane, focuses the query field, and does not send printable query text to the terminal application

#### Scenario: Find navigates
- **WHEN** the Find bar owns an active query
- **THEN** Return, `Command-G`, and Shift-`Command-G` navigate matches through the pane's search owner

#### Scenario: Find dismisses
- **WHEN** the user presses Escape or clicks the close control
- **THEN** Alan dismisses the Find interaction and returns focus to the owning terminal pane

### Requirement: Terminal panes have unambiguous hit-testing boundaries
The macOS shell UI SHALL keep terminal-rendering surfaces from intercepting
mouse events that must be handled by the terminal host, while preserving
explicit SwiftUI/AppKit controls outside the terminal pane.

#### Scenario: Rendering canvas is clicked
- **WHEN** a user clicks the Ghostty or fallback rendering canvas inside a terminal pane
- **THEN** AppKit hit-testing delivers the event to the terminal host rather than treating the canvas as a separate interactive owner

#### Scenario: Passive terminal overlay is visible
- **WHEN** a non-interactive terminal placeholder or diagnostic overlay is visible over the terminal canvas
- **THEN** the overlay does not prevent the terminal host from receiving pane activation clicks

#### Scenario: Pane selector button is clicked
- **WHEN** a user clicks an explicit pane selector button outside the terminal canvas
- **THEN** that SwiftUI control handles selection through its own action without routing the click through the terminal host

### Requirement: Window dragging excludes terminal panes
The macOS shell UI SHALL allow non-interactive window background regions to drag
the hidden-titlebar shell window and SHALL prevent terminal-pane interactions
from initiating window dragging.

#### Scenario: Background chrome is dragged
- **WHEN** a user drags a non-interactive shell background area outside terminal panes and controls
- **THEN** the window moves according to the native movable-background behavior

#### Scenario: Terminal pane is dragged
- **WHEN** a user drags inside a terminal pane
- **THEN** the drag is handled as terminal input or terminal selection and does not move the window

### Requirement: Split UI is terminal first
Split-pane UI SHALL use lightweight dividers, subtle focus treatment, and
stable geometry so the terminal remains the visual center rather than becoming a
card grid or debug layout.

#### Scenario: Multiple panes visible
- **WHEN** a tab contains multiple visible terminal panes
- **THEN** dividers and focus treatment are compact and do not show raw pane IDs, runtime phases, or redundant labels by default

#### Scenario: Split panes share one terminal surface
- **WHEN** a tab contains adjacent visible terminal panes
- **THEN** panes are rendered inside one continuous terminal surface whose outer four corners are rounded, with no per-pane rounded cards, shadows, bottom pane tab strip, or fixed gaps; only a subtle low-contrast beveled split seam separates neighboring panes

#### Scenario: Divider hover
- **WHEN** the user hovers or drags a split divider
- **THEN** the divider provides a clear native resize affordance without resizing unrelated sidebar or toolbar elements

#### Scenario: Inactive split pane
- **WHEN** a split pane is not the active terminal pane
- **THEN** Alan may apply a preference-backed lightweight dim treatment that preserves terminal readability and pointer input while making the active pane and split boundary easier to scan

### Requirement: Terminal panes expose narrow title bars
Each visible macOS terminal pane SHALL include a compact title bar at the top of
the pane that identifies the terminal and provides a pane-scoped close
affordance while keeping terminal content visually dominant.

#### Scenario: Single pane title visible
- **WHEN** a terminal tab contains one visible pane
- **THEN** the pane shows a narrow title bar above the terminal canvas with a user-facing terminal title and one slim close button

#### Scenario: Split pane titles visible
- **WHEN** a terminal tab contains multiple visible panes
- **THEN** every pane leaf shows its own title bar and close button without adding a pane selector strip, card grid, or debug labels

#### Scenario: Long title fits
- **WHEN** a pane title is long or changes while the pane is visible
- **THEN** the title truncates within a stable fixed-height title bar without resizing split dividers, sidebar rows, toolbar content, or sibling panes

### Requirement: Pane title bars consume terminal metadata
Pane title bars SHALL consume the current terminal title already projected into
pane metadata, and SHALL use existing user-facing fallback labels only when the
terminal title is unavailable.

#### Scenario: Terminal title exists
- **WHEN** a pane has a non-empty `viewport.title`
- **THEN** the title bar shows the normalized terminal title rather than raw pane IDs, cwd-first labels, runtime phases, or debug event text

#### Scenario: Terminal title missing
- **WHEN** a pane has no usable terminal title
- **THEN** the title bar falls back to cwd leaf, working-directory name, launch target, process name, or `Terminal` using user-facing copy

#### Scenario: Debug terms suppressed
- **WHEN** terminal metadata contains implementation-oriented summaries such as `title updated`, `window attached`, or raw runtime state
- **THEN** the title bar does not expose those terms outside explicit developer/debug-only surfaces

#### Scenario: Metadata stays in title chrome
- **WHEN** terminal status, branch, attention, or Alan binding metadata is useful in the default pane UI
- **THEN** Alan presents it as lightweight pane-title-bar accessories rather than as a persistent bottom status strip below the terminal canvas

### Requirement: Pane close button targets its pane
The pane title bar close button SHALL close the pane represented by that title
bar through the shared shell controller mutation path.

#### Scenario: Inactive split pane closed
- **WHEN** a user clicks the close button on a non-selected visible split pane
- **THEN** Alan closes that pane, repairs the split tree, and keeps the remaining pane runtimes alive without closing a different selected pane

#### Scenario: Single pane tab closed
- **WHEN** a user clicks the close button for the only pane in a tab and other tabs remain
- **THEN** Alan applies the existing tab-close semantics for that tab and focuses a remaining terminal pane

#### Scenario: Last remaining pane protected
- **WHEN** a close button targets the only pane in the only remaining tab
- **THEN** Alan keeps the shell state valid and does not remove the final workspace surface

### Requirement: Pane title bars preserve terminal input ownership
Pane title bars SHALL own only their explicit title and button controls, and
SHALL not intercept terminal input, selection, mouse reporting, scrollback, or
renderer hit-testing inside the terminal canvas.

#### Scenario: Terminal canvas clicked below title bar
- **WHEN** a user clicks, drags, scrolls, or right-clicks inside the terminal canvas below a pane title bar
- **THEN** the terminal host receives the event according to the terminal event ownership contract

#### Scenario: Close button clicked
- **WHEN** a user clicks the close button in the pane title bar
- **THEN** the button handles the pane close action without routing that click through terminal text input

#### Scenario: Title area clicked
- **WHEN** a user clicks the non-button title area
- **THEN** Alan may focus the pane, but it does not send text, mouse reports, or scroll events to the terminal application

### Requirement: Corner radii are restrained and tokenized
The default Alan macOS shell UI SHALL use a small role-based corner-radius scale
for rounded rectangular surfaces and controls. It SHALL avoid large ad hoc
radii and capsule-heavy default chrome.

#### Scenario: Radius scale applied
- **WHEN** the active macOS shell renders sidebar rows, command rows, pane title bars, terminal surrounds, inline panels, or overlay surfaces
- **THEN** those rounded rectangular elements use the Alan shell radius scale rather than one-off numeric radii

#### Scenario: Default shell avoids large radii
- **WHEN** a default shell surface is visible in normal light-mode use
- **THEN** rounded rectangular chrome does not use radii larger than the overlay radius unless a specific exception is documented in the UI contract

#### Scenario: Capsule use is limited
- **WHEN** the default shell shows text chips, keycap hints, metadata chips, command badges, sidebar controls, or pane title controls
- **THEN** those controls use restrained rounded rectangles rather than `Capsule` shapes unless the component is explicitly defined as a semantic pill

#### Scenario: True circles remain semantic
- **WHEN** the shell shows attention dots, status indicators, traffic-light-like indicators, or intentionally round icon-only controls
- **THEN** those elements may remain circular because the circle communicates state or system-like control behavior

#### Scenario: Terminal surface remains precise
- **WHEN** a single pane or split-pane tab is visible
- **THEN** terminal panes keep a shared continuous terminal surround with smaller outer corners and no per-pane rounded card treatment

### Requirement: Radius normalization preserves shell hierarchy
Radius normalization SHALL make Alan feel calmer and more precise without
turning the UI into a flat grid or weakening control affordances.

#### Scenario: Sidebar remains skimmable
- **WHEN** sidebar spaces, tabs, command entry, and creation controls are visible
- **THEN** smaller radii preserve row scanning, hover states, selected states, and stable dimensions

#### Scenario: Command UI remains readable
- **WHEN** the command palette is open
- **THEN** the outer overlay, search field, and result rows use distinct but restrained radii so hierarchy is visible without large bubble-like cards

#### Scenario: Overlays remain secondary
- **WHEN** the command palette or another remaining default-shell overlay is visible
- **THEN** that surface uses restrained radii and does not read as a large decorative card competing with the terminal

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

### Requirement: Toolbar stays restrained during split interactions
Advanced split, focus, resize, equalize, close, and pane lift affordances SHALL
not turn the toolbar into a dense control strip.

#### Scenario: Multiple panes visible
- **WHEN** a tab contains multiple panes
- **THEN** the default toolbar remains focused on current tab context, command entry, and frequent actions

#### Scenario: Pane lift available
- **WHEN** pane lift is available through command UI or another explicit non-terminal affordance
- **THEN** the default toolbar does not add a persistent pane-management strip
