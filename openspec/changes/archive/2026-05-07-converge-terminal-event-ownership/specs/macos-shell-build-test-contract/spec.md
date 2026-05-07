## ADDED Requirements

### Requirement: Terminal event ownership is contract-checked
The Apple client SHALL include focused shell contract checks that preserve the
terminal event ownership boundary between SwiftUI layout, AppKit terminal host
input, rendering canvases, and native window background dragging.

#### Scenario: SwiftUI terminal tap wrapper is reintroduced
- **WHEN** a code change wraps the terminal native view in a SwiftUI tap gesture for pane selection
- **THEN** the shell contract check fails with an error explaining that terminal-area selection belongs to the terminal host

#### Scenario: Activation delegate strongly retains controller state
- **WHEN** a code change stores terminal activation as a strong registry-owned closure
- **THEN** the shell contract check fails or the focused review checklist requires replacing it with the weak activation boundary

#### Scenario: Rendering canvas becomes interactive owner
- **WHEN** a code change lets Ghostty or fallback rendering canvas views receive terminal mouse-down hit tests as independent owners
- **THEN** the shell contract check fails or the focused review checklist requires routing those events through the terminal host

#### Scenario: Focused manual verification is performed
- **WHEN** event ownership implementation is ready for review
- **THEN** verification covers click-to-select, immediate typing, drag selection, right click, scrolling, and background window dragging in the running macOS app
