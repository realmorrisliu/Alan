## ADDED Requirements

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
