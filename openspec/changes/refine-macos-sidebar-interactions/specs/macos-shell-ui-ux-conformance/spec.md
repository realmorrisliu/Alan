## MODIFIED Requirements

### Requirement: Collapsed sidebar uses a lightweight floating panel
When the sidebar is collapsed, the macOS shell SHALL reveal navigation through a
small floating material panel triggered by intentional edge or titlebar-control
hover, while keeping the terminal workspace stable.

#### Scenario: Narrow reveal target
- **WHEN** the sidebar is collapsed and the pointer approaches the left edge
- **THEN** alan uses a narrow edge hot zone to reveal the floating sidebar panel rather than a full titlebar or header-width hover region

#### Scenario: Floating panel hover retention
- **WHEN** the pointer moves from the edge hot zone onto the floating sidebar panel or collapsed titlebar controls
- **THEN** the floating panel remains revealed until the pointer leaves those related surfaces

#### Scenario: Window-edge hover retention
- **WHEN** the sidebar is collapsed, the floating panel is revealed, and the pointer crosses from the edge hot zone or floating panel into the left window resize frame
- **THEN** alan treats that pointer position as part of the collapsed-sidebar reveal neighborhood and keeps the floating panel revealed
- **AND** alan does not schedule a hide merely because AppKit has switched the cursor or hit-test state to a window-resize affordance
- **AND** native window resizing remains available if the user presses and drags in the resize frame

#### Scenario: Visible-frame zoom edge retention
- **WHEN** the shell window has been double-click zoomed to the current screen's visible work area and its left edge is flush with the usable screen boundary
- **AND** the sidebar is collapsed and revealed from the left edge
- **THEN** moving the pointer along the left edge or through the resize-cursor strip does not cause the floating sidebar to auto-hide while the pointer remains in the window-level reveal neighborhood

#### Scenario: Floating panel owns traffic lights
- **WHEN** the sidebar is collapsed and the floating panel is hidden
- **THEN** the standard macOS traffic-light controls are hidden with the sidebar surface instead of remaining on the bare window corner
- **AND WHEN** the floating sidebar panel is revealed
- **THEN** the standard macOS traffic-light controls reappear on that floating sidebar surface without appearing ahead of the panel reveal timing, jumping from the non-floating corner, or changing terminal workspace geometry

#### Scenario: Floating panel motion
- **WHEN** reduced motion is disabled
- **THEN** the floating sidebar panel enters with a short spring-like leading-edge reveal and exits with a faster low-emphasis hide animation
- **AND** the standard macOS traffic-light controls and lightweight sidebar titlebar controls move with the visible floating surface instead of snapping after the panel has moved

#### Scenario: Reduced motion respected
- **WHEN** reduced motion is enabled
- **THEN** collapsed-sidebar reveal and hide behavior avoids springy movement while preserving the same hover targets and visibility state

#### Scenario: Workspace stability
- **WHEN** the floating sidebar panel appears or disappears
- **THEN** terminal content, split geometry, and window size remain stable instead of being resized by the transient sidebar surface

#### Scenario: No dashboard treatment
- **WHEN** the user views the default shell
- **THEN** the UI does not present page-like sections, nested cards, large explanatory panels, or marketing-style hero composition

## ADDED Requirements

### Requirement: Pinned sidebar motion is continuous and coordinated
Pinned sidebar collapse and expansion SHALL be represented as a coordinated
motion of the sidebar surface, workspace inset, lightweight sidebar titlebar
controls, and standard macOS traffic-light controls rather than as independent
insertions, removals, or frame jumps.

#### Scenario: Sidebar collapses
- **WHEN** the user hides the pinned sidebar and reduced motion is disabled
- **THEN** the sidebar surface moves or narrows out with a short, crisp animation
- **AND** the terminal workspace adjusts its leading inset continuously with the sidebar motion
- **AND** lightweight sidebar titlebar controls and standard macOS traffic-light controls move with the same visual timing instead of jumping to their final positions

#### Scenario: Sidebar expands
- **WHEN** the user pins or expands the sidebar and reduced motion is disabled
- **THEN** the sidebar surface, terminal workspace inset, lightweight sidebar titlebar controls, and standard macOS traffic-light controls move together with a short, non-dragging animation
- **AND** the expanded state settles without delayed toolbar drift or terminal content relayout after the visual motion has completed

#### Scenario: Reduced motion collapse
- **WHEN** reduced motion is enabled and the pinned sidebar is hidden or shown
- **THEN** alan avoids springy movement while still applying one coherent final layout for sidebar surface, workspace inset, titlebar controls, and traffic-light controls

#### Scenario: Native traffic-light behavior preserved
- **WHEN** sidebar or titlebar chrome moves during pinned or floating sidebar transitions
- **THEN** alan continues using the standard macOS traffic-light controls for close, minimize, and zoom behavior rather than drawing custom replacements
