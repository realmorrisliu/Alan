## ADDED Requirements

### Requirement: Terminal-area events are owned by the terminal host
The macOS shell host SHALL route mouse events that occur inside terminal pixels
through the pane's AppKit terminal host rather than through SwiftUI tap gesture
wrappers around the terminal view.

#### Scenario: First click activates and reaches the terminal
- **WHEN** a user clicks a visible terminal pane that is not currently selected
- **THEN** the shell selects that pane, makes its terminal host first responder, and forwards the same mouse-down event to the terminal renderer

#### Scenario: Terminal text selection starts on first drag
- **WHEN** a user begins a drag inside a visible terminal pane
- **THEN** the drag is handled by the terminal host and can start terminal text selection without requiring a prior selection-only click

#### Scenario: Terminal host lifetime remains pane-keyed
- **WHEN** SwiftUI recreates the terminal leaf view for an existing pane
- **THEN** the registry reuses the pane-keyed terminal host and refreshes its weak activation boundary without transferring terminal event ownership to the SwiftUI view

### Requirement: Terminal activation does not retain shell controllers
Registry-owned terminal host views SHALL use a weak activation boundary when
requesting pane selection from the shell controller.

#### Scenario: Host requests activation
- **WHEN** a terminal host receives a mouse-down event for a pane with a stable pane ID
- **THEN** it calls the weak activation boundary for that pane before requesting terminal focus

#### Scenario: Activation boundary is unavailable
- **WHEN** a terminal host has no activation delegate available
- **THEN** terminal input handling remains local to the host and the host does not keep a strong closure that can retain the shell controller
