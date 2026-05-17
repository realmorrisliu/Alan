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

### Requirement: Quick Terminal Has One Global Instance
The quick terminal SHALL have a deterministic relationship to Alan's spaces,
tabs, focus, persistence, and runtime identity. Alan SHALL model the MVP as one
global quick-terminal instance, not one instance per Alan space or one instance
per macOS Space.

#### Scenario: Existing quick terminal is reused globally
- **WHEN** Alan creates or restores the quick terminal
- **THEN** Alan reuses the single global quick-terminal runtime when it is live
  rather than creating another quick terminal for the current Alan space or
  macOS Space

#### Scenario: Quick terminal appears in the current macOS context
- **WHEN** the user summons the hidden global quick terminal from a different
  macOS Space or display
- **THEN** Alan presents the same quick-terminal instance on the current active
  display and Space without changing its runtime identity

#### Scenario: Regular workspace remains stable
- **WHEN** the quick terminal appears or disappears
- **THEN** selected space, selected tab, split tree, regular terminal focus, and
  pane runtime identities remain stable unless the user explicitly moves focus
  or closes a session

#### Scenario: Quick terminal cwd is deterministic
- **WHEN** Alan creates a new quick-terminal runtime because no live global
  quick-terminal instance exists
- **THEN** Alan chooses the working directory from the focused Alan pane when
  available, otherwise from the last quick-terminal cwd when available,
  otherwise from the user's home directory

#### Scenario: Quick terminal promotes into a Space
- **WHEN** the user chooses `Open in Space` for a target Alan space
- **THEN** Alan moves the quick-terminal runtime into that Space as a normal tab,
  hides the Peak presentation, and clears the global quick-terminal slot

#### Scenario: Quick terminal promotion is not copy or link
- **WHEN** Alan promotes the quick terminal into a normal tab
- **THEN** Alan does not copy the terminal process and does not keep the same
  runtime visible in both the Peak and the target tab
