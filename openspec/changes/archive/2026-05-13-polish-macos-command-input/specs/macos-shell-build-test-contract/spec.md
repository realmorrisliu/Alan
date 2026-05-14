## MODIFIED Requirements

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
- **THEN** maintainers can inspect running-app screenshots or notes for sidebar, terminal, command input, and remaining default-shell overlay states confirming that the UI is smaller-radius, still native, and not visually flat

#### Scenario: Legacy surfaces scoped
- **WHEN** radius inventory finds older or non-primary Apple client surfaces
- **THEN** implementation records whether they are active default shell UI before changing them, instead of silently broadening the polish pass

## ADDED Requirements

### Requirement: Command input polish has focused verification
The Apple client SHALL include focused verification for the `Command-P` input
surface when command UI behavior or material treatment changes.

#### Scenario: Command input keyboard flow is verified
- **WHEN** command input implementation is marked complete
- **THEN** focused tests or manual notes cover open/focus, typing, successful Return submission, unresolved Return behavior, Escape dismissal, click-away dismissal, and terminal focus restoration

#### Scenario: Candidate sections stay removed
- **WHEN** default command input UI changes
- **THEN** shell contract checks or review notes confirm action, routing, attention, best-match, command-row, and microphone affordances are not visible in the default command input surface

#### Scenario: Liquid input visual review is captured
- **WHEN** command input material polish is marked complete
- **THEN** maintainers can inspect screenshots or manual notes showing the input over the active light-mode shell with legible text, restrained depth, and no large panel below the field
