## ADDED Requirements

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
