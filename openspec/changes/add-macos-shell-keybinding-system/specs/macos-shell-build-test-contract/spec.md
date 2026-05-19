## ADDED Requirements

### Requirement: Keybinding System Is Verified
The Apple client SHALL include focused verification for default shortcut
registry descriptors, conflict detection, target semantics, menu hints, and input
precedence.

#### Scenario: Existing shortcuts are preserved
- **WHEN** registry-backed keybinding descriptors are introduced
- **THEN** tests prove existing shell shortcuts keep their previous key
  equivalents unless a migration is explicitly documented

#### Scenario: Conflicts are detected
- **WHEN** default shortcut descriptors are validated
- **THEN** tests fail on duplicate shortcuts in the same dispatch context and
  include both action IDs in the failure

#### Scenario: Menu hints are verified
- **WHEN** a menu item is backed by an action with a default shortcut descriptor
- **THEN** verification proves the native menu hint comes from the registry
  descriptor

#### Scenario: Keyboard target is verified
- **WHEN** keyboard dispatch invokes Tab or pane actions
- **THEN** tests prove the target is the current selected Tab or focused pane
  rather than a hovered or context-menu row

#### Scenario: Space shortcut scope is verified
- **WHEN** first-version Space shortcut coverage is tested
- **THEN** verification covers next Space, previous Space, numeric Space
  selection, and the absence of default shortcuts for create, rename, and delete
  Space

#### Scenario: Input precedence is verified
- **WHEN** Find is active or terminal input owns a key sequence
- **THEN** tests or script checks prove those handlers take precedence over
  shell action shortcut dispatch

#### Scenario: No customization surface is verified
- **WHEN** the first-version keybinding system is reviewed
- **THEN** tests or code review checklists confirm no shortcut customization UI,
  config file, manifest field, or Command UI integration was added
