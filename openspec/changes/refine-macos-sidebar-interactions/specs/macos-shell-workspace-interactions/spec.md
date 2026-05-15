## REMOVED Requirements

### Requirement: Sidebar swipe previews spaces without moving the workspace
**Reason**: Sidebar-only previews make spaces feel like a local list animation rather than a continuous workspace sequence, and delaying shell selection until after settlement leaves room for runtime focus updates to snap the UI back to the previous tab or space.

**Migration**: Use the new continuous space pager and authoritative sidebar selection requirements in this change.

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

### Requirement: Space switching uses a continuous pager
Horizontal space switching SHALL model spaces as an ordered, continuous sequence
with a current page index, adjacent page preview, drag offset, and commit/cancel
settlement, so swiping behaves like a native carousel or virtual desktop. The
space page SHALL include the sidebar navigation content and the terminal
workspace surface for each visible source or adjacent target space while
preserving terminal runtime identity.

#### Scenario: Gesture-tracked pager preview
- **WHEN** a user horizontally swipes inside the sidebar and an adjacent space exists
- **THEN** alan renders the current and adjacent space pages from the same horizontal drag offset
- **AND** the user can see the edge of the adjacent space while dragging toward it
- **AND** the sidebar active-space header, sidebar tab list, and terminal workspace surface move as parts of the same space page
- **AND** visible terminal panes keep their runtime identities instead of being restarted or recreated as a side effect of the drag
- **AND** movement is rendered directly from horizontal finger translation instead of being amplified, quantized, or shaped by the commit threshold
- **AND** vertical tab-list scrolling does not move while horizontal intent is locked

#### Scenario: Undecided axis buffers mixed deltas
- **WHEN** a sidebar scroll gesture has not yet crossed the horizontal or vertical intent threshold
- **THEN** alan buffers the initial mixed deltas instead of applying partial vertical tab-list scrolling or horizontal pager movement
- **AND** the gesture is routed only after horizontal or vertical intent is locked

#### Scenario: Pager reaches sequence edge
- **WHEN** a user swipes past the first or last available space
- **THEN** alan applies bounded edge resistance rather than wrapping unexpectedly or showing a nonexistent space page
- **AND** releasing before a valid target is selected returns the pager to the current space

#### Scenario: Commit updates focus at the authoritative transition point
- **WHEN** the user releases a space swipe past the commit threshold or with sufficient release velocity toward an adjacent space
- **THEN** alan commits the target space through the shell controller selection and focus path
- **AND** the visual pager settles smoothly to the committed space without being reverted by concurrent runtime updates
- **AND** terminal focus follows the selected pane when the pane runtime is available

#### Scenario: Cancel preserves focus and layout
- **WHEN** the user releases a space swipe before the commit threshold
- **THEN** alan animates the pager back to the original space
- **AND** selected space, selected tab, terminal focus, split tree, and divider ratios remain unchanged

#### Scenario: Phaseful gesture waits for real release
- **WHEN** a user pauses a phaseful horizontal trackpad swipe while their fingers remain on the trackpad
- **THEN** alan keeps the pager at the current drag offset
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
- **THEN** alan may treat a short idle gap as release to avoid leaving the pager stuck
- **AND** shell selection follows the same commit threshold as other sidebar swipes

#### Scenario: Vertical scroll is not captured
- **WHEN** a user's gesture is primarily vertical in the sidebar tab list
- **THEN** the native vertical tab-list scroll receives the gesture and the workspace space transition does not begin
- **AND** horizontal pager movement is not applied while vertical intent is locked
