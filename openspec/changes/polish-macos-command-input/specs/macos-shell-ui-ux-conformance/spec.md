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
