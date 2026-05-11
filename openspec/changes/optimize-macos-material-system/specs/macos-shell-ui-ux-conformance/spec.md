## ADDED Requirements

### Requirement: Material hierarchy separates navigation from content
The default macOS shell SHALL use material roles that distinguish the functional
navigation/control layer from the content layer. Liquid Glass-style treatment
SHALL be reserved for navigation, command entry, compact controls, and transient
interactive affordances, while workspace and terminal content surfaces SHALL use
standard materials, tonal surfaces, or stable opaque fills that preserve
readability.

#### Scenario: Sidebar uses functional material
- **WHEN** the default shell renders the space rail, active-space tab list, and compact sidebar controls
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
