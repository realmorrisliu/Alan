## ADDED Requirements

### Requirement: Sidebar interaction refinement has focused verification
The Apple client SHALL include focused automated checks or documented manual
verification for sidebar selection/focus convergence, sidebar-local space pager
behavior, and coordinated sidebar/window-chrome motion when those interactions
are changed.

#### Scenario: Sidebar selection convergence tested
- **WHEN** sidebar tab or space selection behavior changes
- **THEN** focused tests verify that selecting a tab or space updates shell focused pane, selected tab, selected space, and terminal runtime focus consistently
- **AND** tests or contract checks cover the case where runtime metadata arrives immediately after selection without reverting to the previous tab

#### Scenario: Sidebar-local space pager gesture tested
- **WHEN** horizontal space swipe behavior changes
- **THEN** focused tests cover undecided-axis buffering, horizontal intent lock, vertical scroll pass-through, edge resistance, commit threshold, cancel threshold, phaseful release, phase-less idle release, and fast flick velocity commit
- **AND** verification confirms only the sidebar active-space header and tab list move during the gesture
- **AND** verification confirms the command input, bottom space switcher, sidebar chrome, traffic lights, and workspace terminal surface remain fixed during the gesture

#### Scenario: Pinned sidebar motion reviewed
- **WHEN** pinned sidebar collapse or expansion behavior changes
- **THEN** maintainers can inspect automated invariants, screenshots, or manual notes showing that the sidebar surface, workspace inset, titlebar controls, and standard macOS traffic-light controls move as one coordinated transition

#### Scenario: Floating sidebar chrome reviewed
- **WHEN** collapsed floating-sidebar reveal or hide behavior changes
- **THEN** focused checks or manual notes verify narrow edge hover, hover retention, stable terminal workspace geometry, native traffic-light behavior, and no visible traffic-light jump from the non-floating corner
