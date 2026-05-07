## ADDED Requirements

### Requirement: Split UI is terminal first
Split-pane UI SHALL use lightweight dividers, subtle focus treatment, and
stable geometry so the terminal remains the visual center rather than becoming a
card grid or debug layout.

#### Scenario: Multiple panes visible
- **WHEN** a tab contains multiple visible terminal panes
- **THEN** dividers and focus treatment are compact and do not show raw pane IDs, runtime phases, or redundant labels by default

#### Scenario: Split panes share one terminal surface
- **WHEN** a tab contains adjacent visible terminal panes
- **THEN** panes are rendered inside one continuous terminal surface whose outer four corners are rounded, with no per-pane rounded cards, shadows, bottom pane tab strip, or fixed gaps; only a subtle low-contrast beveled split seam separates neighboring panes

#### Scenario: Divider hover
- **WHEN** the user hovers or drags a split divider
- **THEN** the divider provides a clear native resize affordance without resizing unrelated sidebar or toolbar elements

#### Scenario: Inactive split pane
- **WHEN** a split pane is not the active terminal pane
- **THEN** Alan may apply a preference-backed lightweight dim treatment that preserves terminal readability and pointer input while making the active pane and split boundary easier to scan

### Requirement: Command UI owns navigation and actions
The default command entry SHALL present tabs, panes, spaces, and workspace
actions through `Go to or Command...` using user-facing labels and compact rows.

#### Scenario: Command results include panes
- **WHEN** command search lists pane targets
- **THEN** results use tab title, pane title, cwd, or process context as the primary label rather than raw pane IDs

#### Scenario: Command result invokes split action
- **WHEN** the user selects a split, focus, zoom, or move action from command UI
- **THEN** Alan runs the same shell controller mutation used by menu and keyboard paths

### Requirement: Toolbar stays restrained during advanced interactions
Advanced split, focus, resize, move, and zoom affordances SHALL not turn the
toolbar into a dense control strip.

#### Scenario: Pane zoomed
- **WHEN** a pane is zoomed
- **THEN** the UI provides a compact way to exit zoom while preserving the native toolbar contract

#### Scenario: Pane movement available
- **WHEN** pane movement commands are available
- **THEN** the default toolbar remains focused on current tab context, command entry, frequent actions, and inspector toggle
