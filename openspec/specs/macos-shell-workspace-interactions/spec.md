# macos-shell-workspace-interactions Specification

## Purpose
Define Alan's native macOS shell workspace interactions for terminal splits,
spatial focus, pane lift or cross-tab movement, and shared menu, keyboard, and
command UI routing.
## Requirements
### Requirement: Split layout stores durable ratios
Alan's macOS shell SHALL store split branch direction, child identity, and
divider ratio in the shell model so split layouts survive rendering changes and
app state persistence.

#### Scenario: Existing equal split loads
- **WHEN** a tab with an older equal split tree is loaded
- **THEN** the shell model interprets each branch as equal ratios and preserves stable structural identity

#### Scenario: Divider is resized
- **WHEN** the user drags a split divider
- **THEN** the branch ratio updates within usable minimum bounds and the terminal panes keep their runtime identities

#### Scenario: Window resizes
- **WHEN** the window size changes after ratios were set
- **THEN** pane frames are recalculated from stored ratios without resetting the split tree

### Requirement: Split operations are native and reversible
The macOS shell SHALL provide native split operations for creating directional
splits, closing panes, resizing panes, and equalizing panes.

#### Scenario: Create directional split
- **WHEN** the user invokes split right, left, up, or down from a menu, shortcut, command UI, or control command
- **THEN** Alan inserts a new pane in the requested direction and focuses the intended pane according to the command semantics

#### Scenario: Equalize splits
- **WHEN** the user invokes equalize for a tab
- **THEN** all split branches in that tab return to equal usable ratios without restarting terminal runtimes

#### Scenario: Close focused pane
- **WHEN** the user invokes close pane while a tab has multiple panes
- **THEN** Alan removes the focused pane, repairs the split tree, and keeps the remaining pane runtimes alive

### Requirement: Spatial focus is first class
The macOS shell SHALL allow users to move focus spatially between visible panes
using left, right, up, and down directions.

#### Scenario: Focus adjacent pane
- **WHEN** the user invokes focus right from a focused pane with a visible neighbor to the right
- **THEN** shell focus moves to that neighboring pane and terminal focus follows it

#### Scenario: Preserve perpendicular position
- **WHEN** a tab contains a two-by-two split layout and the lower-left pane is focused
- **THEN** invoking focus right selects the lower-right pane rather than the upper-right pane

#### Scenario: No adjacent pane
- **WHEN** a spatial focus command has no valid target in the requested direction
- **THEN** focus remains unchanged and the command reports a no-target result where a response is required

### Requirement: Pane lift and cross-tab moves preserve runtime identity
Alan's macOS shell SHALL support pane lift and cross-tab pane move operations
that preserve pane ID, terminal runtime handle, scrollback, metadata, and pending
delivery state.

#### Scenario: Lift pane to a new tab
- **WHEN** the user lifts a pane out of a split tab
- **THEN** Alan creates a new tab for that pane and the pane keeps the same runtime identity

#### Scenario: Move pane to another tab in the same window
- **WHEN** the user moves a pane to another tab in the same shell window
- **THEN** the pane keeps its runtime identity and the source and target tab split trees remain valid

#### Scenario: Move would empty a tab
- **WHEN** a pane move would leave a tab without panes
- **THEN** Alan either closes the empty tab through normal tab-close semantics or rejects the move with a stable reason

### Requirement: Commands use native Mac surfaces
Workspace actions SHALL be available through native menu/command routing,
keyboard shortcuts, command input, and any restrained toolbar affordances that
call the same shell controller mutations where the action is shared. The default
`Command-P` command input SHALL accept typed commands without showing persistent
candidate action lists.

#### Scenario: Menu command
- **WHEN** the user selects New Terminal Tab, New Alan Tab, Split, Focus Pane, Equalize Splits, Close Pane, or Close Tab from the menu bar
- **THEN** Alan executes the same shell controller action used by keyboard and command input paths

#### Scenario: Keyboard command
- **WHEN** the user invokes a supported command-key shortcut
- **THEN** the responder chain routes it to Alan's workspace command handler or terminal surface command handler as appropriate

#### Scenario: Command input opens
- **WHEN** the user opens `Go to or Command...`
- **THEN** Alan focuses a single command input field instead of presenting default action, routing, or attention candidate lists

#### Scenario: Command input shortcut toggles
- **WHEN** the user presses `Command-P` while the command input is focused or visible
- **THEN** Alan dismisses the command input instead of opening a duplicate surface

#### Scenario: Typed command resolves
- **WHEN** the user submits a typed command that Alan can resolve to a workspace action or routing target
- **THEN** Alan executes the same shell controller action used by menu and keyboard paths and dismisses the command input

#### Scenario: Typed command is unresolved
- **WHEN** the user submits a typed command that Alan cannot resolve
- **THEN** Alan leaves the command input open and communicates the unresolved state without exposing raw pane IDs or debug routing details

### Requirement: Sidebar swipe previews spaces without moving the workspace
Horizontal swipe gestures that originate inside the macOS sidebar SHALL drive a
sidebar-local space transition preview. The preview SHALL include the sidebar's
active-space header and tab list, SHALL keep the workspace terminal surface on
the current space while the gesture is active, and SHALL avoid mutating durable
shell selection until the gesture commits.

#### Scenario: Gesture-tracked sidebar preview
- **WHEN** a user horizontally swipes inside the sidebar and an adjacent space exists
- **THEN** the current sidebar space header and tab list move with the gesture while the adjacent space previews from the side
- **AND** the space header and tab list use the same full sidebar page width for horizontal offsets
- **AND** the preview movement is rendered directly from horizontal finger translation instead of being amplified, quantized, or shaped by the commit threshold
- **AND** the space header pager is not narrowed by row padding or trailing creation controls
- **AND** the sidebar pager avoids static left or right padding gaps while pages move
- **AND** the workspace terminal surface remains visually stable on the original selected space
- **AND** vertical tab-list scrolling does not move while horizontal intent is locked
- **AND** later vertical finger movement during the same horizontal swipe does not move the tab list vertically

#### Scenario: Undecided axis does not leak movement
- **WHEN** a sidebar scroll gesture has not yet crossed the horizontal or vertical intent threshold
- **THEN** Alan buffers the initial mixed deltas instead of applying partial vertical tab-list scrolling
- **AND** the gesture is routed only after horizontal or vertical intent is locked

#### Scenario: Commit updates shell selection
- **WHEN** the user releases a space swipe past the commit threshold
- **THEN** Alan updates selected space and tab through the shell controller selection path
- **AND** the workspace terminal surface and terminal focus follow the committed space after the transition settles
- **AND** release is honored even when the macOS ended or momentum-start event carries zero scroll delta

#### Scenario: Cancel preserves shell selection
- **WHEN** the user releases a space swipe before the commit threshold
- **THEN** Alan returns the sidebar preview to the original space
- **AND** selected space, selected tab, terminal focus, split tree, and divider ratios remain unchanged
- **AND** release is honored even when the macOS ended or momentum-start event carries zero scroll delta

#### Scenario: Stationary gesture holds preview
- **WHEN** a user pauses a phaseful horizontal trackpad swipe while their fingers remain on the trackpad
- **THEN** Alan keeps the sidebar preview at the current gesture offset
- **AND** Alan does not commit or cancel until the gesture ends, is cancelled, or enters momentum

#### Scenario: Release uses last finger velocity
- **WHEN** a phaseful horizontal trackpad swipe ends or enters momentum
- **THEN** Alan evaluates commit using the current preview progress and the last effective finger velocity before release
- **AND** Alan does not replace that velocity with a zero-delta ended event

#### Scenario: Fast flick can commit
- **WHEN** a user performs a fast horizontal flick inside the sidebar
- **THEN** Alan recognizes the dominant horizontal release or momentum handoff as a space swipe
- **AND** Alan may commit from velocity even when the gesture produced only a short visible translation before release

#### Scenario: Phase-less gesture settles
- **WHEN** a horizontal sidebar swipe comes from a scroll device that does not provide gesture phases
- **THEN** Alan may treat a short idle gap as release to avoid leaving the preview stuck
- **AND** shell selection follows the same commit threshold as other sidebar swipes

#### Scenario: Vertical scroll is not captured
- **WHEN** a user's gesture is primarily vertical in the sidebar tab list
- **THEN** the native vertical tab-list scroll receives the gesture and the workspace space transition does not begin
- **AND** horizontal sidebar preview movement is not applied while vertical intent is locked

### Requirement: Sidebar split indicators can focus panes
Split topology indicators in the macOS sidebar SHALL route pane focus through
the same shell controller focus model used by terminal split interactions.

#### Scenario: Two-pane segment clicked
- **WHEN** a user clicks a segment in a two-pane tab row split indicator
- **THEN** Alan selects that pane and terminal focus follows it without changing the split tree or divider ratios

#### Scenario: Complex split indicator clicked
- **WHEN** a user clicks a compact indicator for a tab with three or more panes
- **THEN** Alan performs a predictable pane-focus action or opens a compact pane picker, and the action does not mutate the split tree

#### Scenario: Split indicator keyboard access
- **WHEN** a split tab row or its split indicator has keyboard focus
- **THEN** keyboard or accessibility activation can focus panes without relying on pointer-only interaction
