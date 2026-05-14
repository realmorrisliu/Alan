## MODIFIED Requirements

### Requirement: Terminal content is the center of gravity
The main content region SHALL make the active terminal canvas visually dominant
and SHALL avoid nested decorative panels around the terminal in the default
single-pane state.

#### Scenario: Single-pane tab
- **WHEN** a tab contains one pane
- **THEN** the terminal appears nearly full-bleed within the content region and
  does not show a pane selector strip

#### Scenario: Split-pane tab
- **WHEN** a tab contains multiple panes
- **THEN** pane chrome stays lightweight and focus is conveyed by subtle
  selection treatment rather than explicit engineering labels

#### Scenario: alan as optional capability
- **WHEN** a terminal tab is active and no alan-specific surface has been opened
- **THEN** alan appears as an optional command or attachment capability layered
  onto the terminal workflow, not as the structural center of the window

### Requirement: Default UI hides implementation jargon
The default macOS UI SHALL avoid exposing raw pane IDs, `tab_id`, binding,
runtime phases, `window attached`, `title updated`, and other implementation
terms outside explicit debug surfaces. It SHALL also avoid obsolete product
labels such as `AlanNative` or `Alan Shell` in visible app chrome.

#### Scenario: Normal terminal workflow
- **WHEN** a user creates, selects, splits, or closes tabs and panes
- **THEN** visible copy uses product terms such as Space, Tab, Split, Go to or
  Command, Open in alan, and Ask alan

#### Scenario: Command input routing states
- **WHEN** the command input opens, submits a supported typed command, or
  reports an unresolved typed command
- **THEN** the input and inline status use user-facing names where available and
  do not expose raw pane IDs, routing internals, or debug identifiers as the
  primary label
- **AND** alan does not open default tabs, panes, actions, routing-candidate,
  attention, best-match, or command-row sections below the field

#### Scenario: Debug surfaces
- **WHEN** implementation details are needed
- **THEN** they remain in explicit debug-only surfaces, logs, scripts, or
  snapshots rather than default shell chrome

## ADDED Requirements

### Requirement: Visible macOS app copy follows product brand identity
The default macOS app UI SHALL render the public product brand as `alan` and
SHALL use `alan for macOS` only where platform distinction is useful.

#### Scenario: App chrome is visible
- **WHEN** the Dock name, app menu, window title, toolbar labels, command
  palette labels, sidebar buttons, help text, or accessibility labels name the
  product
- **THEN** they use lowercase `alan`
- **AND** they do not use `Alan`, `AlanNative`, `alanterm`, or `Alan Shell` as
  visible product names

#### Scenario: Terminal app category is visible
- **WHEN** the UI or docs explain the native app's category
- **THEN** they call it a terminal emulator or terminal workspace
- **AND** they do not call the product a shell

#### Scenario: Shell is a technical command namespace
- **WHEN** a debug-only surface, script, or CLI-oriented help message refers to
  the `alan shell` namespace
- **THEN** it presents that phrase as literal command syntax or control-plane
  implementation language
- **AND** the default product UI remains `alan`, not `alan shell`
