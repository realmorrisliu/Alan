## ADDED Requirements

### Requirement: Terminal interaction regressions have focused verification
The Apple client SHALL include focused automated tests, shell contract checks,
or documented manual verification for terminal keyboard delivery, tab cwd
inheritance, and shell child-exit lifecycle changes.

#### Scenario: TUI keyboard verification
- **WHEN** terminal input routing is changed
- **THEN** verification covers Vim or an equivalent TUI receiving Escape, Tab, Backspace, control-key navigation, printable input, and command-mode transitions in a focused terminal pane

#### Scenario: Native command routing verification
- **WHEN** terminal keyboard routing is changed
- **THEN** verification covers app-reserved `Command` shortcuts and visible command-input keys so terminal input ownership does not break native macOS commands

#### Scenario: New tab cwd verification
- **WHEN** terminal tab creation is changed
- **THEN** verification covers runtime cwd metadata, pane snapshot cwd fallback, explicit control-plane cwd, and default/home fallback

#### Scenario: Exit lifecycle verification
- **WHEN** child-exit handling is changed
- **THEN** verification covers `exit` from a split pane, `exit` from a single-pane tab, final-pane fallback behavior, and rejection of later text delivery to an exited runtime
