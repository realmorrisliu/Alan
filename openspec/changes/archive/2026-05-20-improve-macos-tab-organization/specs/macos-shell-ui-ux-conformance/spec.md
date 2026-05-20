## ADDED Requirements

### Requirement: Tab Organization Follows Lightweight Arc-Like Sections
The macOS sidebar SHALL present per-Space Pinned and Unpinned Tab sections with
a restrained Arc-like visual treatment that preserves scan speed and avoids
heavy group chrome.

#### Scenario: Pinned and Unpinned sections render
- **WHEN** a Space contains Pinned and Unpinned Tabs
- **THEN** alan separates the sections with subtle spacing or a divider rather
  than large boxed panels, cards, or heavy section headers

#### Scenario: Tab rows remain stable
- **WHEN** the user hovers, selects, drags, pins, unpins, or reorders Tabs
- **THEN** Tab rows keep stable height and sidebar geometry without resizing
  terminal content

#### Scenario: New Tab remains lightweight
- **WHEN** the sidebar shows the New Tab affordance
- **THEN** it appears as a lightweight list action rather than a large toolbar
  or dashboard-style primary button

#### Scenario: No folder scope
- **WHEN** the first Tab organization pass ships
- **THEN** alan does not introduce tab folders, nested tab groups, or a global
  pinned shelf

### Requirement: Drag Feedback Is Direct And Minimal
Tab drag feedback SHALL communicate target section and insertion position
without explanatory copy or persistent drag chrome.

#### Scenario: Drag insertion preview
- **WHEN** the user drags a Tab row over a valid insertion point
- **THEN** alan shows a direct insertion preview in the target section

#### Scenario: Drag crosses section boundary
- **WHEN** the user drags a Tab row from Pinned to Unpinned or Unpinned to
  Pinned
- **THEN** the target section and insertion position are visually clear before
  drop

#### Scenario: Invalid drag target
- **WHEN** the user drags over a target that cannot accept the Tab
- **THEN** alan avoids committing the mutation and preserves current Tab order
  without showing raw debug identifiers
