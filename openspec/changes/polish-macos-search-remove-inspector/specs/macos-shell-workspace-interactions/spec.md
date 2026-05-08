## MODIFIED Requirements

### Requirement: Commands use native Mac surfaces
Workspace actions SHALL be available through native menu/command routing,
keyboard shortcuts, command UI, and any restrained toolbar affordances that call
the same shell controller mutations where the action is shared.

#### Scenario: Menu command
- **WHEN** the user selects New Terminal Tab, New Alan Tab, Split, Focus Pane, Equalize Splits, Close Pane, or Close Tab from the menu bar
- **THEN** Alan executes the same shell controller action used by keyboard and command UI paths

#### Scenario: Keyboard command
- **WHEN** the user invokes a supported command-key shortcut
- **THEN** the responder chain routes it to Alan's workspace command handler or terminal surface command handler as appropriate

#### Scenario: Command UI
- **WHEN** the user opens `Go to or Command...`
- **THEN** workspace actions and routing targets appear with user-facing labels and no raw pane IDs outside debug context

#### Scenario: Inspector command removed
- **WHEN** the user opens menus, command UI, or other native command surfaces
- **THEN** Alan does not expose inspector show, hide, open, close, or toggle commands

## ADDED Requirements

### Requirement: Find keyboard routing follows Mac conventions
Terminal Find keyboard handling SHALL follow common macOS search conventions
while preserving terminal-first focus ownership.

#### Scenario: Open Find
- **WHEN** the user presses `Command-F` with a terminal pane focused
- **THEN** Alan opens the focused pane's Find bar, focuses the Find text field, and preserves the current query where pane search state already exists

#### Scenario: Next result
- **WHEN** Find is active and the user presses Return or `Command-G`
- **THEN** Alan navigates to the next result for the owning pane without sending Return or `g` to the terminal

#### Scenario: Previous result
- **WHEN** Find is active and the user presses Shift-Return or Shift-`Command-G`
- **THEN** Alan navigates to the previous result for the owning pane without sending those keys to the terminal

#### Scenario: Close Find
- **WHEN** Find is active and the user presses Escape
- **THEN** Alan closes Find for the owning pane and returns keyboard focus to that terminal pane
