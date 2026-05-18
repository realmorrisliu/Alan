## MODIFIED Requirements

### Requirement: Split tabs expose compact topology
The default macOS sidebar SHALL show a compact split topology indicator on tab
rows whose active tab contains at least one visible terminal pane. The indicator
SHALL communicate pane count, common split topology, and the currently focused
pane when that topology can be mapped to visible pane segments, without
attempting to render exact split ratios or arbitrary tree nesting in the tab row.

#### Scenario: Single-pane tab row
- **WHEN** a tab contains one terminal pane
- **THEN** the tab row shows a compact single-pane topology indicator with stable width

#### Scenario: Two-pane tab row
- **WHEN** a tab contains two visible terminal panes
- **THEN** the tab row shows a compact two-segment indicator that reflects the root split direction and marks the focused pane

#### Scenario: Three-column tab row
- **WHEN** a tab contains three visible terminal panes that normalize to left, middle, and right columns
- **THEN** the tab row shows a compact three-column topology indicator with stable width and a segment-level focused-pane mark when focus is inside one of those panes

#### Scenario: Three-row tab row
- **WHEN** a tab contains three visible terminal panes that normalize to top, middle, and bottom rows
- **THEN** the tab row shows a compact three-row topology indicator with stable width and a segment-level focused-pane mark when focus is inside one of those panes

#### Scenario: Three-pane main stack tab row
- **WHEN** a tab contains three visible terminal panes that normalize to one main pane plus a two-pane stack on the opposite side
- **THEN** the tab row shows a compact main-plus-stack topology indicator that preserves the main pane side or edge and marks the focused pane when focus maps to a displayed segment

#### Scenario: Four-pane recognizable tab row
- **WHEN** a tab contains four visible terminal panes that normalize to a legible four-column, four-row, or 2x2 grid topology
- **THEN** the tab row shows the corresponding compact four-pane topology indicator without widening the tab row, adding text labels, or rendering proportional split ratios

#### Scenario: Complex split tab row
- **WHEN** a tab contains a visible split topology that is not one of the recognized compact topologies or exceeds the legible indicator pane count
- **THEN** the tab row shows a single-pane-shaped topology base with the pane count overlaid on that shape
- **AND** the pane count is not rendered as adjacent text, a separate trailing badge, a notification dot, or a separate sidebar metadata block

#### Scenario: Split tab avoids notification dots
- **WHEN** a non-focused pane inside a split tab needs attention
- **THEN** the split indicator and tab row do not add notification dots, expose raw pane IDs, or add a separate sidebar attention block

#### Scenario: Split topology remains accessible
- **WHEN** assistive technology reads a tab row with a split topology indicator
- **THEN** the accessibility label or help text communicates the pane count and recognized topology in user-facing terms without exposing raw pane IDs or implementation names
