# macos-shell-ui-ux-conformance Specification

## Purpose
Define the default native macOS shell UI contract: Arc-like space/tab
organization, terminal-first layout, native light-mode material treatment,
restrained toolbar behavior, and progressive inspector disclosure.
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

#### Scenario: No dashboard treatment
- **WHEN** the user views the default shell without the Debug inspector selected
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
- **THEN** visible copy uses product terms such as Space, Tab, Split, Inspector, Go to or Command, Open in Alan, and Ask Alan

#### Scenario: Command search results
- **WHEN** the command UI shows tabs, panes, actions, routing candidates, or attention items
- **THEN** result titles and summaries use user-facing names where available and do not expose raw pane IDs as the primary label unless the user is in a debug context

#### Scenario: Debug inspector
- **WHEN** the user opens the inspector debug section
- **THEN** implementation details may be shown with clear debug framing

### Requirement: Toolbar is native and restrained
The macOS toolbar/titlebar SHALL feel like native window chrome and contain only
the current tab title/context, one command entry point, a small number of
frequent actions, and an optional inspector toggle.

#### Scenario: Toolbar default state
- **WHEN** no urgent attention item exists
- **THEN** the toolbar does not show attention as a large standalone primary control

#### Scenario: Command entry
- **WHEN** the user invokes the command UI
- **THEN** the entry point is labeled and organized as `Go to or Command...`

### Requirement: Inspector uses progressive disclosure
The inspector SHALL be optional, off by default, and separated into Overview and
Debug layers.

#### Scenario: Overview selected
- **WHEN** the inspector overview is visible
- **THEN** it shows only user-relevant secondary state such as focused tab or pane summary, cwd or repo context, Alan attachment summary, attention summary, and minimal process status

#### Scenario: Debug selected
- **WHEN** the inspector debug layer is visible
- **THEN** debug data such as JSON snapshots, runtime phase, Ghostty data, control paths, and Alan binding details can be inspected without dominating the default layout

### Requirement: UI conformance is verified visually
Mac shell UI changes SHALL be reviewed against the documented UI contract before
the UI conformance tasks are marked complete.

#### Scenario: Default screenshot review
- **WHEN** a UI conformance implementation pass is ready for review
- **THEN** maintainers can inspect a running-app screenshot of the default light-mode window showing the space rail, active-space tab list, terminal-first content area, and inspector-off state

#### Scenario: Inspector screenshot review
- **WHEN** inspector-related UI tasks are marked complete
- **THEN** maintainers can inspect screenshots or recorded notes for both Overview and Debug inspector states, confirming that debug data is hidden from the default workflow and visible in the Debug layer

### Requirement: Terminal overlays use user-facing language
The macOS terminal UI SHALL present terminal search, child-exit, renderer
failure, readonly, input-not-ready, and clipboard states with concise
terminal-user language in the canvas area or inspector overview, while raw
runtime details remain debug-only.

#### Scenario: Renderer failure visible
- **WHEN** a focused terminal pane cannot render
- **THEN** the default UI explains that the terminal cannot draw and offers an actionable next step without showing raw Ghostty callback names or pane IDs

#### Scenario: Child exit visible
- **WHEN** a terminal child process exits
- **THEN** the pane shows a compact terminal exit state rather than debug event names

#### Scenario: Debug layer opened
- **WHEN** the user opens the inspector debug layer
- **THEN** renderer diagnostics, surface identifiers, input mode details, and raw event payloads may be inspected with debug framing

### Requirement: Terminal search does not displace workspace structure
Terminal search UI SHALL be compact, pane scoped, and layered over the terminal
workflow without turning the shell into a dashboard or page layout.

#### Scenario: Search opens
- **WHEN** the user invokes terminal search
- **THEN** the search control appears as a compact terminal tool for the focused pane and the sidebar, toolbar, and split layout keep stable dimensions

#### Scenario: Search closes
- **WHEN** the user dismisses terminal search
- **THEN** keyboard focus returns to the terminal pane that owned the search interaction

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
- **THEN** the default toolbar remains focused on current tab context, command entry, frequent actions, and inspector toggle

#### Scenario: Pane lift available
- **WHEN** pane lift is available through command UI or another explicit non-terminal affordance
- **THEN** the default toolbar does not add a persistent pane-management strip
