## ADDED Requirements

### Requirement: Quick Terminal Summon And Dismiss Are Shell Commands
Quick terminal summon, dismiss, focus, and close operations SHALL route through
Alan's shared shell command/controller paths so keyboard shortcuts, menu
commands, command input, and control surfaces converge on the same behavior.
Alan SHALL expose a configurable global toggle shortcut for quick terminal; the
draft default shortcut is `Option+Space`.

#### Scenario: Quick terminal command opens
- **WHEN** the user invokes quick terminal from a keyboard shortcut, menu,
  command input, or supported control command
- **THEN** Alan summons the same quick terminal target through the shared shell
  controller path and focuses terminal input

#### Scenario: Quick terminal global shortcut toggles
- **WHEN** the quick terminal is visible and the user invokes the quick terminal
  toggle command again
- **THEN** Alan hides the quick terminal presentation without closing the
  underlying terminal runtime

#### Scenario: Quick terminal does not use Escape as hide
- **WHEN** the quick terminal owns focus and the user presses `Esc`
- **THEN** Alan treats the key as terminal input unless an Alan-owned nested
  quick-terminal menu or picker is currently open

#### Scenario: Quick terminal close is explicit
- **WHEN** the user invokes close while the quick terminal owns focus
- **THEN** Alan distinguishes hiding the quick terminal presentation from
  closing the underlying terminal session
