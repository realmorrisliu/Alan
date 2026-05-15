## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Sidebar swipe previews spaces without moving the workspace
Horizontal swipe gestures that originate inside the macOS sidebar SHALL drive
a sidebar-local, finger-tracked space content pager. The moving page SHALL
include only the sidebar's active-space header and active-space tab list. The
command input, bottom space switcher, sidebar material surface, sidebar chrome,
macOS traffic-light placement, and workspace terminal surface SHALL remain
visually fixed while the gesture is active. Alan SHALL avoid mutating durable
shell selection until the gesture commits.

#### Scenario: Gesture-tracked sidebar content pager
- **WHEN** a user horizontally swipes inside the sidebar and an adjacent space exists
- **THEN** the current sidebar space header and tab list move with the gesture while the adjacent space content previews from the side
- **AND** the space header and tab list use the same full sidebar content page width for horizontal offsets
- **AND** movement is rendered directly from horizontal finger translation instead of being amplified, quantized, or shaped by the commit threshold
- **AND** the command input remains fixed
- **AND** the bottom space switcher remains fixed as the stable space navigation control
- **AND** the workspace terminal surface remains visually stable on the original selected space
- **AND** visible terminal panes keep their runtime identities instead of being restarted, duplicated, or horizontally offset as a side effect of the drag
- **AND** vertical tab-list scrolling does not move while horizontal intent is locked

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

#### Scenario: Cancel preserves focus and layout
- **WHEN** the user releases a space swipe before the commit threshold
- **THEN** alan animates the sidebar content pager back to the original space
- **AND** selected space, selected tab, terminal focus, split tree, and divider ratios remain unchanged

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
