## MODIFIED Requirements

### Requirement: Sidebar matches vertical Arc-like space switching
The default macOS sidebar SHALL remain a single vertical navigation column that
aligns cleanly around the macOS traffic-light area. Spaces SHALL be switched
through a compact bottom borderless icon switcher and horizontal sidebar swipe
gestures, while tabs for the active space remain the primary sidebar list. The
sidebar SHALL be self-explaining through spatial structure, iconography,
selection treatment, hover/focus affordances, and accessibility labels rather
than persistent instructional copy.

#### Scenario: Default sidebar reading order
- **WHEN** a user opens the macOS app
- **THEN** the sidebar reads as command entry, active-space tab list, and bottom space switcher in one vertical column rather than as unrelated dashboard sections or a two-column sidebar

#### Scenario: Space selection
- **WHEN** a user selects a space in the bottom switcher
- **THEN** the tab list updates to show only tabs belonging to that active space

#### Scenario: Sidebar swipe switches spaces
- **WHEN** a user performs a clear horizontal swipe gesture inside the sidebar
- **THEN** Alan switches to the previous or next space without disrupting vertical tab-list scrolling

#### Scenario: Separate creation affordances
- **WHEN** a user creates a new space or a new tab
- **THEN** space creation is presented as a compact bottom-switcher affordance and tab creation is presented in the active-space tab list or toolbar context

#### Scenario: Space switcher is borderless
- **WHEN** the bottom space switcher is visible
- **THEN** space buttons use slim borderless icon styling with selection, hover, and attention conveyed without persistent framed cards or section chrome

#### Scenario: Lightweight tab rows
- **WHEN** the active-space tab list contains terminal and Alan tabs
- **THEN** each tab appears as a skimmable row with a compact marker, title, secondary context, and low-emphasis status rather than as a card or dashboard tile

#### Scenario: Visible copy is minimized
- **WHEN** the default sidebar has at least one space and one tab
- **THEN** the sidebar does not rely on persistent explanatory paragraphs, product slogans, keyboard-shortcut labels, or redundant `Tabs` and `Spaces` headings to explain normal operation

#### Scenario: Accessibility remains explicit
- **WHEN** visible explanatory copy is removed from the sidebar
- **THEN** controls, space switcher items, tab rows, creation buttons, and attention marks retain accessibility labels, help text, or menu labels that expose their purpose to assistive technologies

## ADDED Requirements

### Requirement: Sidebar actions are progressively disclosed
The default macOS sidebar SHALL keep repeated tab and space rows visually quiet
by showing secondary actions through hover, keyboard focus, context menu, or
compact owner-zone controls rather than always-visible explanatory buttons.

#### Scenario: Tab row default state
- **WHEN** a tab row is visible and not hovered or keyboard focused
- **THEN** the row prioritizes icon, title, compact context, selection, and attention state without persistent close/more text buttons

#### Scenario: Tab row interaction state
- **WHEN** a tab row is hovered, keyboard focused, or context-clicked
- **THEN** close, more, move, or related secondary actions become available without resizing the row or shifting neighboring content

#### Scenario: Empty sidebar state
- **WHEN** the sidebar has no user-created spaces or no tabs in the active space
- **THEN** the owning zone exposes a compact creation affordance without showing paragraph-style onboarding copy in the default shell

### Requirement: Split tabs expose compact topology
The default macOS sidebar SHALL show a compact split topology indicator on tab
rows whose active tab contains multiple terminal panes. The indicator SHALL
communicate pane count, dominant split direction when useful, and the currently
focused pane without attempting to render exact split ratios in the tab row.

#### Scenario: Single-pane tab row
- **WHEN** a tab contains one terminal pane
- **THEN** the tab row does not show a split topology indicator

#### Scenario: Two-pane tab row
- **WHEN** a tab contains two visible terminal panes
- **THEN** the tab row shows a compact two-segment indicator that reflects the root split direction and marks the focused pane

#### Scenario: Complex split tab row
- **WHEN** a tab contains three or more visible terminal panes or nested split branches
- **THEN** the tab row summarizes the split with a compact topology mark or pane count instead of a proportional miniature layout

#### Scenario: Attention in split tab
- **WHEN** a non-focused pane inside a split tab needs attention
- **THEN** the split indicator or tab row conveys that attention without exposing raw pane IDs or adding a separate sidebar attention block
