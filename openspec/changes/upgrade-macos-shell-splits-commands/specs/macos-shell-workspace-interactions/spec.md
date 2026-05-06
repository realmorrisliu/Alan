## ADDED Requirements

### Requirement: Split layout stores durable ratios
Alan's macOS shell SHALL store split branch direction, child identity, and
divider ratio in the shell model so split layouts survive rendering changes and
app state persistence.

#### Scenario: Existing equal split loads
- **WHEN** a tab with an older equal split tree is loaded
- **THEN** the shell model interprets each branch as equal ratios and assigns stable structural identity

#### Scenario: Divider is resized
- **WHEN** the user drags a split divider
- **THEN** the branch ratio updates within usable minimum bounds and the terminal panes keep their runtime identities

#### Scenario: Window resizes
- **WHEN** the window size changes after ratios were set
- **THEN** pane frames are recalculated from stored ratios without resetting the split tree

### Requirement: Split operations are native and reversible
The macOS shell SHALL provide native split operations for creating directional
splits, closing panes, resizing panes, equalizing panes, and zooming a pane.

#### Scenario: Create directional split
- **WHEN** the user invokes split right, left, up, or down from a menu, shortcut, command UI, context menu, or control command
- **THEN** Alan inserts a new pane in the requested direction and focuses the intended pane according to the command semantics

#### Scenario: Equalize splits
- **WHEN** the user invokes equalize for a tab
- **THEN** all split branches in that tab return to equal usable ratios without restarting terminal runtimes

#### Scenario: Zoom focused pane
- **WHEN** the user zooms the focused pane
- **THEN** the focused pane fills the terminal content area while sibling panes remain alive and restorable

#### Scenario: Unzoom focused pane
- **WHEN** the user exits zoom
- **THEN** the previous split layout and pane runtime identities are restored

### Requirement: Spatial focus is first class
The macOS shell SHALL allow users and control clients to move focus spatially
between visible panes using left, right, up, and down directions.

#### Scenario: Focus adjacent pane
- **WHEN** the user invokes focus right from a focused pane with a visible neighbor to the right
- **THEN** shell focus moves to that neighboring pane and terminal focus follows it

#### Scenario: No adjacent pane
- **WHEN** a spatial focus command has no valid target in the requested direction
- **THEN** focus remains unchanged and the command reports a no-target result where a response is required

### Requirement: Pane moves preserve runtime identity
Alan's macOS shell SHALL support pane move operations that change split-tree
placement while preserving pane ID, terminal runtime handle, scrollback,
metadata, and pending delivery state.

#### Scenario: Move pane to another split position
- **WHEN** the user moves a pane within the same tab
- **THEN** the pane appears in the new split position and keeps the same runtime identity

#### Scenario: Move pane to another tab in the same window
- **WHEN** the user moves a pane to another tab in the same shell window
- **THEN** the pane keeps its runtime identity and the source/target tab split trees remain valid

#### Scenario: Move would empty a tab
- **WHEN** a pane move would leave a tab without panes
- **THEN** Alan either closes the empty tab through normal tab-close semantics or rejects the move with a stable reason

### Requirement: Commands use native Mac surfaces
Workspace actions SHALL be available through native menu/command routing,
keyboard shortcuts, command UI, and restrained toolbar/context affordances that
all call the same shell controller mutations.

#### Scenario: Menu command
- **WHEN** the user selects New Terminal Tab, New Alan Tab, Split, Close Pane, Search, Copy, Paste, or Zoom from the menu bar
- **THEN** Alan executes the same controller action used by keyboard and command UI paths

#### Scenario: Keyboard command
- **WHEN** the user invokes a supported command-key shortcut
- **THEN** the responder chain routes it to Alan's workspace command handler or terminal surface command handler as appropriate

#### Scenario: Command UI
- **WHEN** the user opens `Go to or Command...`
- **THEN** workspace actions and routing targets appear with user-facing labels and no raw pane IDs outside debug context

### Requirement: Native window behavior respects Alan organization
The macOS app SHALL keep Alan spaces/tabs as the primary organization model and
avoid native window/tab behavior that conflicts with Alan shell state.

#### Scenario: New window opens
- **WHEN** the user opens a new Alan window
- **THEN** it receives an isolated shell identity and does not merge Alan tabs with another window through native tabbing unexpectedly

#### Scenario: Toolbar default
- **WHEN** the app is in the default terminal workflow
- **THEN** toolbar chrome remains restrained and does not add dashboard controls around terminal content
