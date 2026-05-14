## MODIFIED Requirements

### Requirement: Default UI hides implementation jargon
The default macOS UI SHALL avoid exposing raw pane IDs, `tab_id`, binding,
runtime phases, `window attached`, `title updated`, and other implementation
terms outside explicit debug surfaces.

#### Scenario: Normal terminal workflow
- **WHEN** a user creates, selects, splits, or closes tabs and panes
- **THEN** visible copy uses product terms such as Space, Tab, Split, Go to or Command, Open in Alan, and Ask Alan

#### Scenario: Command input routing states
- **WHEN** the command input opens, submits a supported typed command, or reports an unresolved typed command
- **THEN** the input and inline status use user-facing names where available and do not expose raw pane IDs, routing internals, or debug identifiers as the primary label
- **AND** Alan does not open default tabs, panes, actions, routing-candidate, attention, best-match, or command-row sections below the field

#### Scenario: Debug surfaces
- **WHEN** implementation details are needed
- **THEN** they remain in explicit debug-only surfaces, logs, scripts, or snapshots rather than default shell chrome

### Requirement: Radius normalization preserves shell hierarchy
Radius normalization SHALL make Alan feel calmer and more precise without
turning the UI into a flat grid or weakening control affordances.

#### Scenario: Sidebar remains skimmable
- **WHEN** sidebar spaces, tabs, command entry, and creation controls are visible
- **THEN** smaller radii preserve row scanning, hover states, selected states, and stable dimensions

#### Scenario: Command input remains readable
- **WHEN** the command input is open
- **THEN** the floating input surface, text field, close control, and inline unresolved state use distinct but restrained radii so hierarchy is visible without turning the command input into a large palette or decorative card

#### Scenario: Overlays remain secondary
- **WHEN** the command input, Find bar, or another remaining default-shell overlay is visible
- **THEN** that surface uses restrained radii and does not read as a large decorative card competing with the terminal

### Requirement: Command UI owns navigation and shell actions
The default command entry SHALL provide a typed `Go to or Command...` input for
supported shell workspace actions and routing targets. It SHALL execute
resolved typed submissions through the shared shell controller mutation path
where the action is shared, and it SHALL avoid default visible candidate rows or
multi-section command-palette chrome.

#### Scenario: Command input opens
- **WHEN** the user invokes `Command-P` or activates `Go to or Command...`
- **THEN** Alan focuses a single floating command input field instead of presenting default tabs, panes, actions, routing-candidate, attention, or best-match lists

#### Scenario: Command input executes supported action
- **WHEN** the user submits typed text that Alan can resolve to a supported workspace action or routing target
- **THEN** Alan runs the same shell controller mutation used by menu and keyboard paths where that action is shared
- **AND** the command input dismisses and restores focus to the previously focused terminal pane when available

#### Scenario: Command input reports unresolved text
- **WHEN** the user submits typed text that Alan cannot resolve
- **THEN** Alan leaves the command input open and communicates the unresolved state inline without opening candidate rows or exposing raw debug identifiers

## ADDED Requirements

### Requirement: Command input opens as a Liquid Glass input
The macOS shell SHALL present `Command-P` as a single floating Liquid
Glass-style input layer that captures text entry without rendering default
candidate sections below the input.

#### Scenario: Command input opens
- **WHEN** the user presses `Command-P` or activates the sidebar command entry
- **THEN** Alan opens a floating material-backed input field, focuses the text field, and does not show action, routing, attention, or best-match lists below it

#### Scenario: Command input toggles from shortcut
- **WHEN** the command input is already open and the user presses `Command-P`
- **THEN** Alan dismisses the input and returns keyboard focus to the previously focused terminal pane when available

#### Scenario: Command input is visually restrained
- **WHEN** the command input is visible
- **THEN** the surface uses a restrained native material treatment, stable geometry, and compact controls rather than a large card, dashboard panel, or multi-section palette
- **AND** it appears and disappears with an opacity-only fade instead of moving down from the top edge

#### Scenario: Command input dismisses
- **WHEN** the user presses Escape, clicks outside the input, activates a close affordance, or successfully submits a resolved command
- **THEN** Alan dismisses the input and returns keyboard focus to the previously focused terminal pane when available

#### Scenario: No default voice affordance
- **WHEN** the command input is visible
- **THEN** the input does not show a microphone or voice-listening affordance unless a future voice-specific requirement explicitly adds one

#### Scenario: Unresolved command stays input-only
- **WHEN** the user submits text that cannot be resolved to a supported command or destination
- **THEN** Alan keeps the command surface input-only and communicates the unresolved state without opening candidate rows below the field
