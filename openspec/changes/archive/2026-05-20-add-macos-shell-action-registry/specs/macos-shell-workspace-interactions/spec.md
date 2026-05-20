## MODIFIED Requirements

### Requirement: Commands use native Mac surfaces
Workspace actions SHALL be available through native menu/command routing,
keyboard shortcuts, command input, and any restrained toolbar affordances that
call the same shell controller mutations where the action is shared. Menu bar,
context menu, and keyboard shortcut paths SHALL resolve shared shell actions
through the macOS shell action registry. The default `Command-P` command input
SHALL accept typed commands without showing persistent candidate action lists;
this registry change SHALL NOT add new Command UI behaviors.

#### Scenario: Menu command
- **WHEN** the user selects New Terminal Tab, New alan Tab, Split, Focus Pane,
  Equalize Splits, Close Pane, or Close Tab from the menu bar
- **THEN** alan executes the registered shell action used by matching keyboard
  and context paths where that behavior is shared

#### Scenario: Keyboard command
- **WHEN** the user invokes a supported command-key shortcut
- **THEN** the responder chain routes it to alan's shell action registry or
  terminal surface command handler as appropriate

#### Scenario: Context command
- **WHEN** the user invokes a supported Tab or Space context menu command
- **THEN** alan resolves the registry action with the context Tab or Space
  target rather than first changing shell selection

#### Scenario: Command input opens
- **WHEN** the user opens `Go to or Command...`
- **THEN** alan focuses a single command input field instead of presenting
  default action, routing, or attention candidate lists
- **AND** this registry change does not add new Tab or Space organization
  commands to the Command UI

#### Scenario: Command input shortcut toggles
- **WHEN** the user presses `Command-P` while the command input is focused or
  visible
- **THEN** alan dismisses the command input instead of opening a duplicate
  surface

#### Scenario: Typed command resolves
- **WHEN** the user submits a typed command that alan can resolve to an existing
  workspace action or routing target
- **THEN** alan executes the existing command input behavior and dismisses the
  command input

#### Scenario: Typed command is unresolved
- **WHEN** the user submits a typed command that alan cannot resolve
- **THEN** alan leaves the command input open and communicates the unresolved
  state without exposing raw pane IDs or debug routing details
