## ADDED Requirements

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
