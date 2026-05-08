## ADDED Requirements

### Requirement: Corner radii are restrained and tokenized
The default Alan macOS shell UI SHALL use a small role-based corner-radius scale
for rounded rectangular surfaces and controls. It SHALL avoid large ad hoc
radii and capsule-heavy default chrome.

#### Scenario: Radius scale applied
- **WHEN** the active macOS shell renders sidebar rows, command rows, pane title bars, inspector cards, terminal surrounds, inline panels, or overlay surfaces
- **THEN** those rounded rectangular elements use the Alan shell radius scale rather than one-off numeric radii

#### Scenario: Default shell avoids large radii
- **WHEN** a default shell surface is visible in normal light-mode use
- **THEN** rounded rectangular chrome does not use radii larger than the overlay radius unless a specific exception is documented in the UI contract

#### Scenario: Capsule use is limited
- **WHEN** the default shell shows text chips, keycap hints, metadata chips, command badges, sidebar controls, or pane title controls
- **THEN** those controls use restrained rounded rectangles rather than `Capsule` shapes unless the component is explicitly defined as a semantic pill

#### Scenario: True circles remain semantic
- **WHEN** the shell shows attention dots, status indicators, traffic-light-like indicators, or intentionally round icon-only controls
- **THEN** those elements may remain circular because the circle communicates state or system-like control behavior

#### Scenario: Terminal surface remains precise
- **WHEN** a single pane or split-pane tab is visible
- **THEN** terminal panes keep a shared continuous terminal surround with smaller outer corners and no per-pane rounded card treatment

### Requirement: Radius normalization preserves shell hierarchy
Radius normalization SHALL make Alan feel calmer and more precise without
turning the UI into a flat grid or weakening control affordances.

#### Scenario: Sidebar remains skimmable
- **WHEN** sidebar spaces, tabs, command entry, and creation controls are visible
- **THEN** smaller radii preserve row scanning, hover states, selected states, and stable dimensions

#### Scenario: Command UI remains readable
- **WHEN** the command palette is open
- **THEN** the outer overlay, search field, and result rows use distinct but restrained radii so hierarchy is visible without large bubble-like cards

#### Scenario: Inspector remains secondary
- **WHEN** the inspector is visible
- **THEN** overview and debug surfaces use restrained radii and do not read as large decorative cards competing with the terminal
