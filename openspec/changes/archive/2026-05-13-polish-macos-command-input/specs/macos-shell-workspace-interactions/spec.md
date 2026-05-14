## MODIFIED Requirements

### Requirement: Commands use native Mac surfaces
Workspace actions SHALL be available through native menu/command routing,
keyboard shortcuts, command input, and any restrained toolbar affordances that
call the same shell controller mutations where the action is shared. The default
`Command-P` command input SHALL accept typed commands without showing persistent
candidate action lists.

#### Scenario: Menu command
- **WHEN** the user selects New Terminal Tab, New Alan Tab, Split, Focus Pane, Equalize Splits, Close Pane, or Close Tab from the menu bar
- **THEN** Alan executes the same shell controller action used by keyboard and command input paths

#### Scenario: Keyboard command
- **WHEN** the user invokes a supported command-key shortcut
- **THEN** the responder chain routes it to Alan's workspace command handler or terminal surface command handler as appropriate

#### Scenario: Command input opens
- **WHEN** the user opens `Go to or Command...`
- **THEN** Alan focuses a single command input field instead of presenting default action, routing, or attention candidate lists

#### Scenario: Command input shortcut toggles
- **WHEN** the user presses `Command-P` while the command input is focused or visible
- **THEN** Alan dismisses the command input instead of opening a duplicate surface

#### Scenario: Typed command resolves
- **WHEN** the user submits a typed command that Alan can resolve to a workspace action or routing target
- **THEN** Alan executes the same shell controller action used by menu and keyboard paths and dismisses the command input

#### Scenario: Typed command is unresolved
- **WHEN** the user submits a typed command that Alan cannot resolve
- **THEN** Alan leaves the command input open and communicates the unresolved state without exposing raw pane IDs or debug routing details
