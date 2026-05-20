## ADDED Requirements

### Requirement: Shell Action Registry Is Verified
The Apple client SHALL include focused verification for macOS shell action
registry coverage, target resolution, availability, and shortcut conflicts.

#### Scenario: Action IDs are unique
- **WHEN** shell action registry tests run
- **THEN** every registered shell action has a unique stable action ID

#### Scenario: Shortcut conflicts are rejected
- **WHEN** two enabled shell actions in the same keyboard context declare the
  same default shortcut
- **THEN** focused verification fails with enough detail to identify both
  conflicting action IDs

#### Scenario: Context target is preserved
- **WHEN** a context menu action targets a non-selected Tab
- **THEN** focused verification proves the action resolves the context target
  and does not first select the Tab

#### Scenario: Command UI remains unchanged
- **WHEN** the shell action registry is introduced
- **THEN** focused checks confirm new Tab and Space organization actions are not
  added to `Go to or Command...` by this change
