## ADDED Requirements

### Requirement: Pane title bars have focused verification
The Apple client SHALL include focused automated or documented verification for
pane title-bar consumption, pane-scoped close routing, and terminal input
ownership when pane title bars are changed.

#### Scenario: Pane title-bar consumption tested
- **WHEN** pane title-bar helpers receive existing terminal title, working-directory, cwd, launch-target, and process metadata combinations
- **THEN** focused tests verify title-bar priority, fallback ordering, long-title handling, and suppression of raw pane IDs or debug terms without retesting terminal title capture itself

#### Scenario: Pane close routing tested
- **WHEN** a title-bar close action targets a selected pane, an inactive split pane, a single-pane tab with other tabs, or the final remaining pane
- **THEN** focused tests verify the shell mutation result, selected pane after close, split tree repair, and final-pane protection

#### Scenario: Terminal input preservation reviewed
- **WHEN** pane title-bar implementation is ready for review
- **THEN** maintainers can inspect automated shell contract checks or manual notes covering terminal click-to-focus, typing, selection drag, right click, scrolling, and close-button interaction

#### Scenario: Visual evidence captured
- **WHEN** pane title-bar UI polish is marked complete
- **THEN** maintainers can inspect a running-app screenshot or manual note showing light-mode single-pane and split-pane tabs with compact pane title bars, readable titles, restrained close buttons, and no default debug labels
