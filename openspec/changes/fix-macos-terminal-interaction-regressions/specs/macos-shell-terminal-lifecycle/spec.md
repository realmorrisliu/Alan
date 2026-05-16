## ADDED Requirements

### Requirement: Terminal keyboard input is terminal-host owned
The macOS shell host SHALL route keyboard events for the focused terminal pane
through the terminal host unless a visible alan command surface or an explicit
app-reserved `Command` shortcut owns that key.

#### Scenario: Vim control key reaches terminal
- **WHEN** a focused terminal pane is running a TUI such as Vim and no alan command surface is visible
- **THEN** non-`Command` terminal keys such as Escape, Tab, Backspace, `Control-[`, `Control-W`, `Control-F`, and `Control-B` are delivered to the terminal runtime
- **AND** the shell workspace command router does not consume those keys as pane, tab, or command-input actions

#### Scenario: IME marked text owns composing backspace
- **WHEN** a focused terminal pane has active AppKit `NSTextInputClient` marked text from a Chinese/Japanese/Korean input method
- **AND** the user presses Backspace or an equivalent composing control key
- **THEN** alan lets AppKit `interpretKeyEvents` update or clear the marked text before terminal delivery
- **AND** alan updates Ghostty preedit state from the resulting marked text
- **AND** alan MUST NOT forward the composing control character to the terminal as a deletion of already-committed terminal input

#### Scenario: Ghostty binding wins for focused terminal
- **WHEN** Ghostty reports that a focused terminal key event is a terminal binding
- **THEN** alan sends the key event to the terminal runtime instead of treating it as an unresolved native command

#### Scenario: App-reserved command shortcut remains native
- **WHEN** a focused terminal pane receives an explicit app or workspace `Command` shortcut such as New Terminal Tab or Close Tab
- **THEN** alan executes the native workspace command and does not send that shortcut as terminal text

#### Scenario: Visible command surface owns its own keys
- **WHEN** alan's command input is visible while a terminal pane is focused
- **THEN** command-input keys such as submit, dismiss, and command-input toggle are handled by that surface before terminal delivery

### Requirement: Shell child exit drives pane lifecycle
The macOS shell host SHALL treat terminal child-process exit as a lifecycle
event for the owning pane rather than as a request to clear, refresh, or
implicitly restart the terminal runtime.

#### Scenario: Exit closes split pane
- **WHEN** a pane in a split tab receives a normal shell child-exit signal from user input such as `exit`
- **THEN** alan closes only that pane through the normal pane-close path
- **AND** sibling panes keep their terminal runtime identities, scrollback, cwd, and focus eligibility

#### Scenario: Exit closes single-pane tab
- **WHEN** the only pane in a tab receives a normal shell child-exit signal and the tab can be closed
- **THEN** alan closes that tab through the normal tab-close path
- **AND** focus moves to the shell model's next valid tab or empty-space state

#### Scenario: Close-surface after child exit preserves exited metadata
- **WHEN** Ghostty reports a close-surface callback for a terminal surface whose child process is no longer alive
- **THEN** alan forwards a non-confirming close request from the surface host to the shell owner
- **AND** the shell owner closes the owning pane or tab through the normal close path
- **AND** alan preserves exited runtime metadata long enough for observers to see the terminal lifecycle transition
- **AND** releasing the Ghostty surface MUST NOT rewrite the pane metadata back to a running state before the controller observes the exit

#### Scenario: Final pane cannot close safely
- **WHEN** the final visible terminal pane receives a shell child-exit signal and closing it would leave the shell in an unsupported state
- **THEN** alan keeps an explicit exited pane state with terminal input disabled
- **AND** alan does not create a replacement shell runtime unless the user explicitly starts one

#### Scenario: Final pane closes into empty space
- **WHEN** the final visible terminal pane receives a shell child-exit signal and the shell model supports an empty focused space
- **THEN** alan closes the owning pane and tab through the normal close path
- **AND** the focused space remains available without creating a replacement terminal runtime

#### Scenario: Text delivery after exit is rejected
- **WHEN** text delivery targets a pane whose child process has exited and no replacement runtime was explicitly started
- **THEN** the runtime response reports failure with a stable child-exited reason
