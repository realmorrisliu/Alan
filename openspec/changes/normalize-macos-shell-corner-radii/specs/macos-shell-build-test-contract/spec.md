## ADDED Requirements

### Requirement: Corner-radius conformance is verified
The Apple client SHALL include focused verification for active-shell
corner-radius normalization when default macOS shell chrome is changed.

#### Scenario: Active shell radius check runs
- **WHEN** a change updates active shell visual chrome in `MacShellRootView.swift`, `TerminalPaneView.swift`, or normal-flow `TerminalHostView.swift` fallback surfaces
- **THEN** a focused check or review step verifies that rounded rectangles use the Alan shell radius scale and do not introduce large ad hoc radii

#### Scenario: Capsule usage reviewed
- **WHEN** a change adds `Capsule` usage to active default shell chrome
- **THEN** the change documents why the component is a semantic pill or replaces it with a radius-scale rounded rectangle

#### Scenario: Visual comparison captured
- **WHEN** radius normalization implementation is marked complete
- **THEN** maintainers can inspect running-app screenshots or notes for sidebar, terminal, command palette, and inspector states confirming that the UI is smaller-radius, still native, and not visually flat

#### Scenario: Legacy surfaces scoped
- **WHEN** radius inventory finds older or non-primary Apple client surfaces
- **THEN** implementation records whether they are active default shell UI before changing them, instead of silently broadening the polish pass
