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
