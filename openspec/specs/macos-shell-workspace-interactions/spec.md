# macos-shell-workspace-interactions Specification

## Purpose
Define alan's native macOS shell workspace interactions for terminal splits,
spatial focus, pane lift or cross-tab movement, and shared menu, keyboard, and
command UI routing.
## Requirements
### Requirement: Split layout stores durable ratios
alan's macOS shell SHALL store split branch direction, child identity, and
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
- **THEN** alan inserts a new pane in the requested direction and focuses the intended pane according to the command semantics

#### Scenario: Equalize splits
- **WHEN** the user invokes equalize for a tab
- **THEN** all split branches in that tab return to equal usable ratios without restarting terminal runtimes

#### Scenario: Close focused pane
- **WHEN** the user invokes close pane while a tab has multiple panes
- **THEN** alan removes the focused pane, repairs the split tree, and keeps the remaining pane runtimes alive

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
alan's macOS shell SHALL support pane lift and cross-tab pane move operations
that preserve pane ID, terminal runtime handle, scrollback, metadata, and pending
delivery state.

#### Scenario: Lift pane to a new tab
- **WHEN** the user lifts a pane out of a split tab
- **THEN** alan creates a new tab for that pane and the pane keeps the same runtime identity

#### Scenario: Move pane to another tab in the same window
- **WHEN** the user moves a pane to another tab in the same shell window
- **THEN** the pane keeps its runtime identity and the source and target tab split trees remain valid

#### Scenario: Move would empty a tab
- **WHEN** a pane move would leave a tab without panes
- **THEN** alan either closes the empty tab through normal tab-close semantics or rejects the move with a stable reason

### Requirement: Commands use native Mac surfaces
Workspace actions SHALL be available through native menu/command routing,
keyboard shortcuts, command input, and any restrained toolbar affordances that
call the same shell controller mutations where the action is shared. Menu bar,
context menu, and keyboard shortcut paths SHALL resolve shared shell actions
through the macOS shell action registry. The default `Command-P` command input
SHALL accept typed commands without showing persistent candidate action lists;
this registry change SHALL NOT add new Command UI behaviors.

#### Scenario: Menu command
- **WHEN** the user selects New Terminal Tab, New alan Tab, Split, Focus Pane,
  Equalize Splits, Close Pane, or Close Tab from the menu bar
- **THEN** alan executes the registered shell action used by matching keyboard
  and context paths where that behavior is shared

#### Scenario: Keyboard command
- **WHEN** the user invokes a supported command-key shortcut
- **THEN** the responder chain routes it to alan's shell action registry or
  terminal surface command handler as appropriate

#### Scenario: Context command
- **WHEN** the user invokes a supported Tab or Space context menu command
- **THEN** alan resolves the registry action with the context Tab or Space
  target rather than first changing shell selection

#### Scenario: Command input opens
- **WHEN** the user opens `Go to or Command...`
- **THEN** alan focuses a single command input field instead of presenting
  default action, routing, or attention candidate lists
- **AND** this registry change does not add new Tab or Space organization
  commands to the Command UI

#### Scenario: Command input shortcut toggles
- **WHEN** the user presses `Command-P` while the command input is focused or
  visible
- **THEN** alan dismisses the command input instead of opening a duplicate
  surface

#### Scenario: Typed command resolves
- **WHEN** the user submits a typed command that alan can resolve to an existing
  workspace action or routing target
- **THEN** alan executes the existing command input behavior and dismisses the
  command input

#### Scenario: Typed command is unresolved
- **WHEN** the user submits a typed command that alan cannot resolve
- **THEN** alan leaves the command input open and communicates the unresolved
  state without exposing raw pane IDs or debug routing details

### Requirement: Sidebar swipe previews spaces without moving the workspace
Horizontal swipe gestures that originate inside the macOS sidebar SHALL drive
a sidebar-local, finger-tracked space content pager. The moving page SHALL
include only the sidebar's active-space header and active-space tab list. The
command input, bottom space switcher, sidebar material surface, sidebar chrome,
macOS traffic-light placement, and workspace terminal surface SHALL remain
visually fixed while the gesture is active. alan SHALL avoid mutating durable
shell selection until the gesture commits. The pager SHALL keep the current
space centered in a bounded five-page rendering window: up to two previous
spaces, the current space, and up to two next spaces.

#### Scenario: Gesture-tracked sidebar content pager
- **WHEN** a user horizontally swipes inside the sidebar and an adjacent space exists
- **THEN** the current sidebar space header and tab list move with the gesture while the adjacent space content previews from the side
- **AND** the space header and tab list use the same full sidebar content page width for horizontal offsets
- **AND** movement is rendered directly from horizontal finger translation instead of being amplified, quantized, or shaped by the commit threshold
- **AND** the pager keeps previous, current, and next page slots stable while direction changes instead of replacing the rendered target page based only on current drag sign
- **AND** visual drag is clamped to one page plus a small overdrag gap that can reveal part of the second adjacent page for physical feedback
- **AND** the space header pager is not narrowed by row padding or trailing creation controls
- **AND** the sidebar pager avoids static left or right padding gaps while pages move
- **AND** the command input remains fixed
- **AND** the bottom space switcher remains fixed as the stable space navigation control
- **AND** the workspace terminal surface remains visually stable on the original selected space
- **AND** visible terminal panes keep their runtime identities instead of being restarted, duplicated, or horizontally offset as a side effect of the drag
- **AND** vertical tab-list scrolling does not move while horizontal intent is locked
- **AND** later vertical finger movement during the same horizontal swipe does not move the tab list vertically

#### Scenario: Undecided axis buffers mixed deltas
- **WHEN** a sidebar scroll gesture has not yet crossed the horizontal or vertical intent threshold
- **THEN** alan buffers the initial mixed deltas instead of applying partial vertical tab-list scrolling or horizontal pager movement
- **AND** the gesture is routed only after horizontal or vertical intent is locked

#### Scenario: Content pager reaches sequence edge
- **WHEN** a user swipes past the first or last available space
- **THEN** alan applies bounded edge resistance to the moving sidebar content rather than wrapping unexpectedly or showing a nonexistent space page
- **AND** releasing before a valid target is selected returns the content pager to the current space

#### Scenario: Commit updates focus at the authoritative transition point
- **WHEN** the user releases a space swipe past the commit threshold or with sufficient release velocity toward an adjacent space
- **THEN** alan commits the target space through the shell controller selection and focus path
- **AND** the sidebar content pager settles smoothly to the committed space without being reverted by concurrent runtime updates
- **AND** the workspace terminal surface and terminal focus follow the committed space after shell selection commits
- **AND** release is honored even when the macOS ended or momentum-start event carries zero scroll delta
- **AND** a single release commits at most the immediately adjacent previous or next space, never multiple spaces

#### Scenario: Cancel preserves focus and layout
- **WHEN** the user releases a space swipe before the commit threshold
- **THEN** alan animates the sidebar content pager back to the original space
- **AND** selected space, selected tab, terminal focus, split tree, and divider ratios remain unchanged
- **AND** release is honored even when the macOS ended or momentum-start event carries zero scroll delta

#### Scenario: Phaseful gesture waits for real release
- **WHEN** a user pauses a phaseful horizontal trackpad swipe while their fingers remain on the trackpad
- **THEN** alan keeps the sidebar content pager at the current drag offset
- **AND** alan does not commit or cancel until the gesture ends, is cancelled, or enters momentum

#### Scenario: Release uses last effective velocity
- **WHEN** a phaseful horizontal trackpad swipe ends or enters momentum
- **THEN** alan evaluates commit using current pager progress and the last effective finger velocity before release
- **AND** alan does not replace that velocity with a zero-delta ended event

#### Scenario: Fast flick can commit
- **WHEN** a user performs a fast horizontal flick inside the sidebar
- **THEN** alan recognizes the dominant horizontal release or momentum handoff as a space switch
- **AND** alan may commit from velocity even when the gesture produced only a short visible translation before release

#### Scenario: Phase-less gesture settles
- **WHEN** a horizontal sidebar swipe comes from a scroll device that does not provide gesture phases
- **THEN** alan may treat a short idle gap as release to avoid leaving the content pager stuck
- **AND** shell selection follows the same commit threshold as other sidebar swipes

#### Scenario: Vertical scroll is not captured
- **WHEN** a user's gesture is primarily vertical in the sidebar tab list
- **THEN** the native vertical tab-list scroll receives the gesture and the workspace space transition does not begin
- **AND** horizontal sidebar content pager movement is not applied while vertical intent is locked

### Requirement: Sidebar split indicators can focus panes
Split topology indicators in the macOS sidebar SHALL route pane focus through
the same shell controller focus model used by terminal split interactions.

#### Scenario: Two-pane segment clicked
- **WHEN** a user clicks a segment in a two-pane tab row split indicator
- **THEN** alan selects that pane and terminal focus follows it without changing the split tree or divider ratios

#### Scenario: Complex split indicator clicked
- **WHEN** a user clicks a compact indicator for a tab with three or more panes
- **THEN** alan performs a predictable pane-focus action or opens a compact pane picker, and the action does not mutate the split tree

#### Scenario: Split indicator keyboard access
- **WHEN** a split tab row or its split indicator has keyboard focus
- **THEN** keyboard or accessibility activation can focus panes without relying on pointer-only interaction

### Requirement: Sidebar selection commits authoritative shell focus
Sidebar tab and space selection SHALL update the authoritative shell focused
pane through the same shell controller focus model used by terminal activation,
so sidebar selection, focused space, focused tab, focused pane, and terminal
runtime focus converge.

#### Scenario: Tab row clicked
- **WHEN** a user clicks a tab row in the active sidebar space
- **THEN** alan resolves the target tab's preferred pane and updates shell focus to that pane through the shell controller focus path
- **AND** later terminal runtime metadata, state publication, or selection synchronization does not restore the previously focused tab

#### Scenario: Space switcher clicked
- **WHEN** a user clicks a space in the bottom sidebar space switcher
- **THEN** alan selects that space, resolves the target tab and pane for that space, and updates shell focus through the shell controller focus path
- **AND** terminal focus follows the selected pane when the pane runtime is available

#### Scenario: Selected tab contains multiple panes
- **WHEN** a sidebar selection targets a tab with multiple panes
- **THEN** alan prefers the tab's currently focused pane when that pane belongs to the selected tab
- **AND** alan otherwise chooses a stable pane from the tab's pane tree without changing split structure or divider ratios

#### Scenario: Runtime update races selection
- **WHEN** terminal runtime metadata or control-plane state publication occurs immediately after sidebar selection
- **THEN** the committed sidebar selection remains on the selected tab and space because the shell focused pane already matches the selection

### Requirement: New terminal tabs inherit focused pane cwd
The macOS shell SHALL create user-requested terminal tabs in the focused pane's
current working directory unless the caller supplies an explicit working
directory or no valid focused-pane cwd exists.

#### Scenario: Runtime cwd is current
- **WHEN** the user invokes New Terminal Tab from a focused pane whose runtime metadata reports cwd `/repo/app`
- **THEN** the new tab's initial pane starts in `/repo/app`
- **AND** the new tab's pane snapshot records `/repo/app` as its cwd

#### Scenario: Snapshot cwd is fallback
- **WHEN** the focused pane has no runtime cwd metadata but its shell pane snapshot records cwd `/repo/app`
- **THEN** the new terminal tab starts in `/repo/app`

#### Scenario: Explicit cwd wins
- **WHEN** a control-plane or command path opens a new terminal tab with an explicit cwd `/tmp/work`
- **THEN** alan starts the new tab in `/tmp/work` even if the focused pane has a different cwd

#### Scenario: Missing focused cwd falls back
- **WHEN** a new terminal tab is requested and alan cannot resolve a valid cwd from the focused pane or request
- **THEN** alan falls back to the workspace default working directory or the user's home directory

#### Scenario: Split and tab creation agree
- **WHEN** a user creates a split pane and a new terminal tab from the same focused pane
- **THEN** both new terminal runtimes use the same cwd resolution order

### Requirement: Keyboard Shell Commands Route Through The Action Registry
Keyboard-triggered macOS shell commands SHALL resolve and execute through the
shell action registry so keyboard shortcuts, menus, and context menus share
action availability and handler semantics.

#### Scenario: Keyboard shortcut invokes Tab action
- **WHEN** the user presses a Tab-related shell shortcut
- **THEN** alan resolves the registered Tab action and applies it to the current
  selected Tab

#### Scenario: Keyboard shortcut invokes Space action
- **WHEN** the user presses a Space-related shell shortcut
- **THEN** alan resolves the registered Space action and applies it to the
  current selected Space context

#### Scenario: Keyboard shortcut invokes pane action
- **WHEN** the user presses a pane-related shell shortcut
- **THEN** alan resolves the registered pane action and applies it to the
  focused pane

### Requirement: First-Version Space Shortcuts Are Navigation Only
The first version of macOS shell Space shortcuts SHALL cover Space navigation
only and SHALL NOT provide default shortcuts for Space creation, rename, or
deletion.

#### Scenario: Next Space shortcut
- **WHEN** the user presses the default Next Space shortcut
- **THEN** alan selects the next Space in workspace order

#### Scenario: Previous Space shortcut
- **WHEN** the user presses the default Previous Space shortcut
- **THEN** alan selects the previous Space in workspace order

#### Scenario: Numeric Space shortcut
- **WHEN** the user presses a numeric Space selection shortcut for an existing
  Space index
- **THEN** alan selects that Space

#### Scenario: Numeric Space target is missing
- **WHEN** the user presses a numeric Space selection shortcut for a missing
  Space index
- **THEN** alan leaves the current Space selected and reports a stable
  unavailable reason for diagnostics where appropriate

#### Scenario: Create Space has no default shortcut
- **WHEN** the first-version Space action registry exposes create Space
- **THEN** alan exposes the action without a default keyboard shortcut

#### Scenario: Rename or delete Space has no default shortcut
- **WHEN** the first-version Space action registry exposes rename or delete Space
- **THEN** alan exposes those actions without default keyboard shortcuts
