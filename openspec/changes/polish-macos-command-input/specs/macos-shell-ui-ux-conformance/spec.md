## ADDED Requirements

### Requirement: Command-K opens a Liquid Glass input
The macOS shell SHALL present `Command-K` as a single floating Liquid
Glass-style input layer that captures text entry without rendering default
candidate sections below the input.

#### Scenario: Command input opens
- **WHEN** the user presses `Command-K` or activates the sidebar command entry
- **THEN** Alan opens a floating material-backed input field, focuses the text field, and does not show action, routing, attention, or best-match lists below it

#### Scenario: Command input is visually restrained
- **WHEN** the command input is visible
- **THEN** the surface uses a restrained native material treatment, stable geometry, and compact controls rather than a large card, dashboard panel, or multi-section palette

#### Scenario: Command input dismisses
- **WHEN** the user presses Escape, clicks outside the input, activates a close affordance, or successfully submits a resolved command
- **THEN** Alan dismisses the input and returns keyboard focus to the previously focused terminal pane when available

#### Scenario: No default voice affordance
- **WHEN** the `Command-K` input is visible
- **THEN** the input does not show a microphone or voice-listening affordance unless a future voice-specific requirement explicitly adds one

#### Scenario: Unresolved command stays input-only
- **WHEN** the user submits text that cannot be resolved to a supported command or destination
- **THEN** Alan keeps the command surface input-only and communicates the unresolved state without opening candidate rows below the field
