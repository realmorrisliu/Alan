## ADDED Requirements

### Requirement: Material hierarchy separates navigation from content
The default macOS shell SHALL use material roles that distinguish the functional
navigation/control layer from the content layer. Liquid Glass-style treatment
SHALL be reserved for navigation, command entry, compact controls, and transient
interactive affordances, while workspace and terminal content surfaces SHALL use
standard materials, tonal surfaces, or stable opaque fills that preserve
readability.

#### Scenario: Sidebar uses functional material
- **WHEN** the default shell renders the sidebar command entry, active-space tab list, bottom space switcher, and compact sidebar controls
- **THEN** those navigation surfaces use a consistent functional material treatment with legible foreground content and restrained selection states

#### Scenario: Terminal content avoids decorative glass
- **WHEN** the active terminal pane or terminal surround is visible
- **THEN** Alan does not apply Liquid Glass-style decorative transparency to the terminal content layer and keeps terminal text contrast stable

#### Scenario: Workspace backdrop is semantic
- **WHEN** the shell renders the main workspace background outside terminal panes
- **THEN** the background uses a semantic material or tonal role chosen for hierarchy rather than hard-coded theme color dominance

### Requirement: Active shell controls use semantic material roles
Buttons, key hints, close controls, hover affordances, and command-entry controls SHALL use
shared semantic material/control roles in the active macOS shell and MUST avoid one-off white,
opaque, or ad hoc translucent fills in default shell chrome.

#### Scenario: Compact icon button
- **WHEN** a compact icon button appears in the sidebar, title bar, terminal chrome, or command entry
- **THEN** its background, hover, pressed, disabled, and selected appearances come from shared shell control roles and keep stable dimensions

#### Scenario: Foreground on material
- **WHEN** text or symbols render on top of a material-backed shell control
- **THEN** Alan uses system-vibrant foreground styles or approved shell tokens that remain legible across light appearance, reduced transparency, and increased contrast

#### Scenario: AppKit bridge remains isolated
- **WHEN** a SwiftUI shell view needs an AppKit-backed visual effect material
- **THEN** the view uses a reusable support-layer wrapper rather than creating `NSVisualEffectView` bridge details inline

### Requirement: Active shell surfaces use semantic elevation
The active macOS shell SHALL pair its material roles with a small semantic
radius and shadow scale. Surface elevation MUST communicate hierarchy and
interaction state rather than decorate every translucent control.

#### Scenario: Primary terminal surface anchors elevation
- **WHEN** the active terminal surface is visible
- **THEN** it uses the primary content-surface treatment with continuous 12pt corners, a focused adaptive contact shadow, and restrained rim/highlight treatment

#### Scenario: Static controls stay quiet
- **WHEN** sidebar command launchers, titlebar ghost buttons, or compact static controls are idle
- **THEN** they avoid default shadows and use material tint, stroke, hover, or highlight to show affordance

#### Scenario: Selected navigation uses light elevation
- **WHEN** a sidebar row or space switcher item is selected or previewed
- **THEN** it may use a very light adaptive contact shadow that is smaller than floating overlay shadows and does not produce dirty dark halos in light mode

#### Scenario: Floating surfaces carry stronger elevation
- **WHEN** the command input, pane Find bar, or collapsed sidebar panel floats above the shell
- **THEN** it uses semantic floating-surface shadows that are visible, focused, and adaptive while keeping the terminal content visually dominant

#### Scenario: Radius scale remains role-based
- **WHEN** active shell visual chrome is updated
- **THEN** micro indicators, compact controls, rows, floating inputs, primary surfaces, collapsed panels, and semantic pill inputs use the shared shell radius roles instead of local one-off values
