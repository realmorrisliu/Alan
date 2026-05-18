## ADDED Requirements

### Requirement: Split topology indicators have focused verification
The Apple client SHALL include focused automated or documented verification for
sidebar split topology classification and visual stability when the split
topology indicator changes.

#### Scenario: Topology classification is tested
- **WHEN** split topology indicator logic is implemented or changed
- **THEN** focused tests verify single pane, two columns, two rows, three columns, three rows, three-pane main-plus-stack variants, four-pane recognized layouts, and complex-count fallback classification

#### Scenario: Complex count rendering is verified
- **WHEN** a tab's split topology falls back to complex count
- **THEN** focused tests or visual evidence verify that the count overlays a single-pane-shaped topology base rather than rendering beside the indicator as adjacent text or a separate badge

#### Scenario: Sidebar indicator visuals are reviewed
- **WHEN** split topology indicator UI implementation is marked complete
- **THEN** maintainers can inspect running-app screenshots or manual notes covering light-mode selected, hover, focused-pane, three-pane, four-pane, and complex-count tab-row states without row resizing or layout shifts
