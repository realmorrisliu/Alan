## MODIFIED Requirements

### Requirement: Terminal mode changes survive view changes
The macOS shell host SHALL keep terminal mode metadata such as alternate screen,
mouse reporting, search state, and readonly state with the runtime identity
rather than with transient host views.

#### Scenario: View recreated during alternate screen
- **WHEN** a pane view is recreated while an alternate-screen application is active
- **THEN** the replacement view reflects the current terminal mode rather than reverting to normal-buffer assumptions

#### Scenario: Background pane exits readonly mode
- **WHEN** a background pane changes readonly or input readiness state
- **THEN** the pane metadata updates without selecting that tab

#### Scenario: View recreated during terminal search
- **WHEN** a pane view is recreated while terminal search is active for that pane
- **THEN** the replacement view reflects the pane's current search query, active state, and match metadata without routing search to another pane

## ADDED Requirements

### Requirement: Terminal search lifecycle is pane owned
Terminal search SHALL start, update, navigate, and end through the focused
pane's terminal surface controller and search engine, preserving pane runtime
identity across SwiftUI/AppKit view reconstruction.

#### Scenario: Search starts for focused pane
- **WHEN** `Command-F` is invoked while a terminal pane is focused
- **THEN** Alan starts search through that pane's `AlanTerminalSearchEngine` and records active search state against that pane identity

#### Scenario: Query updates target owning pane
- **WHEN** the user edits the Find query while search is active
- **THEN** Alan sends the query update to the search engine for the pane that owns the Find interaction and does not treat the query text as terminal input

#### Scenario: Navigation targets owning pane
- **WHEN** the user requests next or previous search result
- **THEN** Alan navigates results through the owning pane's search engine and updates the owning pane's selected-match state

#### Scenario: Search ends for owning pane
- **WHEN** the user dismisses Find
- **THEN** Alan ends search through the owning pane's search engine, clears active search UI for that pane, and leaves other pane runtimes unchanged
