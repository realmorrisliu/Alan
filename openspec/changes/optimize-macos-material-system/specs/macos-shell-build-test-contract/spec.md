## ADDED Requirements

### Requirement: Material hierarchy has focused verification
The Apple client SHALL include focused verification for active-shell material
changes so native material polish does not reduce terminal readability or
reintroduce hard-coded visual effects.

#### Scenario: Material review is captured
- **WHEN** active macOS shell material roles, background surfaces, or compact control treatments change
- **THEN** maintainers can inspect screenshots or manual notes covering the default light-mode sidebar, terminal content area, command entry, compact controls, and floating overlays

#### Scenario: Accessibility material settings are reviewed
- **WHEN** material hierarchy implementation is marked complete
- **THEN** verification includes reduced-transparency or increased-contrast review notes, or a documented reason those settings could not be exercised locally

#### Scenario: One-off material fills are checked
- **WHEN** a change adds new active-shell material or translucent fills
- **THEN** focused review or a lightweight check confirms the fill is attached to a shared semantic material/control role rather than a local hard-coded effect
