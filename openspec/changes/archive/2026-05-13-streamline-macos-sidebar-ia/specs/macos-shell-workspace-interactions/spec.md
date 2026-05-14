## ADDED Requirements

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
