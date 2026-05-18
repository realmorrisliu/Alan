## ADDED Requirements

### Requirement: Terminal interaction regressions have focused verification
The Apple client SHALL include focused automated tests, shell contract checks,
or documented manual verification for terminal keyboard delivery, tab cwd
inheritance, and shell child-exit lifecycle changes.

#### Scenario: TUI keyboard verification
- **WHEN** terminal input routing is changed
- **THEN** verification covers Vim or an equivalent TUI receiving Escape, Tab, Backspace, control-key navigation, printable input, and command-mode transitions in a focused terminal pane

#### Scenario: Physical keyboard and programmatic text stay separate
- **WHEN** terminal input routing is changed
- **THEN** verification proves printable physical keys can enter AppKit text interpretation for IME startup while Escape and Control keys remain terminal-owned
- **AND** verification proves committed printable physical input uses terminal key-event delivery
- **AND** static checks prevent `TerminalHostView.keyDown` from calling the programmatic text injection path

#### Scenario: Native command routing verification
- **WHEN** terminal keyboard routing is changed
- **THEN** verification covers app-reserved `Command` shortcuts and visible command-input keys so terminal input ownership does not break native macOS commands

#### Scenario: AppKit responder-chain verification
- **WHEN** terminal keyboard routing is changed
- **THEN** verification covers `performKeyEquivalent`/`doCommand` redispatch for Control or Command key equivalents
- **AND** verification covers Ghostty's special `Control-/` handling and focus-only split click/drag sequences that must not reach Vim mouse mode or terminal selection
- **AND** verification covers the terminal input router as the single owner of primary pointer sequence policy instead of only testing separate focus-click or pointer helpers

#### Scenario: GhosttyKit modulemap verification
- **WHEN** local GhosttyKit artifacts are prepared for the Apple client build
- **THEN** the setup script normalizes generated GhosttyKit module maps to use `header "ghostty.h"` instead of `umbrella header "ghostty.h"`
- **AND** shell contract checks reject cached GhosttyKit module maps that would cause Clang umbrella-header warnings for internal `ghostty/vt/*` headers

#### Scenario: New tab cwd verification
- **WHEN** terminal tab creation is changed
- **THEN** verification covers runtime cwd metadata, pane snapshot cwd fallback, explicit control-plane cwd, and default/home fallback

#### Scenario: Exit lifecycle verification
- **WHEN** child-exit handling is changed
- **THEN** verification covers `exit` from a split pane, `exit` from a single-pane tab, final-pane fallback behavior, direct surface close-request forwarding, and rejection of later text delivery to an exited runtime
