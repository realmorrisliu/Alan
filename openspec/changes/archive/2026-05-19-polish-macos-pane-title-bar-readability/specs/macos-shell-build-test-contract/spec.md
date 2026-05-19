## MODIFIED Requirements

### Requirement: Pane title bars have focused verification
The Apple client SHALL include focused automated or documented verification for
pane title-bar consumption, pane-scoped close routing, terminal input
ownership, selected-title readability, responsive accessory layout, and
terminal-surface integration when pane title bars are changed.

#### Scenario: Pane title-bar consumption tested
- **WHEN** pane title-bar helpers receive existing terminal title, working-directory, cwd, launch-target, and process metadata combinations
- **THEN** focused tests verify title-bar priority, fallback ordering, long-title handling, and suppression of raw pane IDs or debug terms without retesting terminal title capture itself

#### Scenario: Pane close routing tested
- **WHEN** a title-bar close action targets a selected pane, an inactive split pane, a single-pane tab with other tabs, or the final remaining pane
- **THEN** focused tests verify the shell mutation result, selected pane after close, split tree repair, and final-pane protection

#### Scenario: Terminal input preservation reviewed
- **WHEN** pane title-bar implementation is ready for review
- **THEN** maintainers can inspect automated shell contract checks or manual notes covering terminal click-to-focus, typing, selection drag, right click, scrolling, and close-button interaction

#### Scenario: Title readability guarded
- **WHEN** pane title-bar UI polish changes selected or unfocused title styling
- **THEN** focused checks or documented visual review verify that the focused title remains visible as text against the terminal surface background in light mode
- **AND** contract checks fail or review blocks the change if selected title-bar styling can hide, wash out, or replace the title text with icon-only content

#### Scenario: Responsive layout guarded
- **WHEN** pane title-bar layout changes
- **THEN** focused checks or review verify that title-bar accessories use fit-content layout and staged responsive fallback instead of fixed-width accessory columns
- **AND** narrow title bars preserve title text and close affordance while lower-priority accessories degrade first

#### Scenario: Terminal-surface integration guarded
- **WHEN** pane title-bar background or material roles change
- **THEN** focused checks or documented visual review verify that the title bar matches the terminal surface background and does not reintroduce a selected/unselected overlay band above the pane

#### Scenario: Window chrome and collapsed sidebar guardrails run
- **WHEN** hidden-titlebar window chrome, titlebar double-click behavior, local app launch behavior, or collapsed-sidebar floating-panel behavior changes
- **THEN** focused checks verify launch presents one primary window, empty titlebar double-click zoom targets only non-control chrome, system traffic-light buttons keep their normal behavior, and collapsed-sidebar reveal uses narrow hover targets with stable workspace geometry

#### Scenario: Visual evidence captured
- **WHEN** pane title-bar UI polish is marked complete
- **THEN** maintainers can inspect a running-app screenshot or manual note showing light-mode single-pane and split-pane tabs with compact pane title bars, readable titles, restrained close buttons, responsive accessory fallback, and no default debug labels
