## ADDED Requirements

### Requirement: Streamlined sidebar has focused verification
The Apple client SHALL include focused verification for sidebar information
architecture changes that remove visible copy or restructure space/tab
navigation.

#### Scenario: Sidebar reading order is reviewed
- **WHEN** sidebar IA implementation is marked complete
- **THEN** maintainers can inspect screenshots or manual notes showing the vertical sidebar, active-space tab list, bottom borderless space switcher, separate creation affordances, and no persistent explanatory sidebar blocks

#### Scenario: Sidebar interaction states are reviewed
- **WHEN** tab or space row secondary actions are progressively disclosed
- **THEN** verification covers default, hover, selected, no-notification-dot, and empty states without row resizing or layout shifts

#### Scenario: Sidebar space swipe is reviewed
- **WHEN** horizontal space switching is implemented in the sidebar
- **THEN** verification covers gesture-tracked left and right sidebar previews, translation-first drag mapping, fast horizontal flick commit, full-width space-title and tab-list motion, stationary hold, zero-delta release, horizontal/vertical axis lock, later vertical movement during a horizontal swipe, no static side padding gaps during page motion, a stable workspace during drag, commit, cancel, edge resistance, and confirms vertical tab-list scrolling still works

#### Scenario: Split tab indicator is reviewed
- **WHEN** split-aware tab row implementation is marked complete
- **THEN** verification covers single-pane, two-pane horizontal, two-pane vertical, complex split, focused-pane, no-notification-dot, pointer activation, and keyboard or accessibility activation states

#### Scenario: Accessibility copy is preserved
- **WHEN** visible sidebar text is removed or shortened
- **THEN** review confirms accessibility labels, help text, menu labels, or equivalent nonvisual descriptions still identify the affected controls
