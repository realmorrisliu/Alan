## ADDED Requirements

### Requirement: Native scrollback is synchronized
The macOS terminal surface SHALL provide native scrollback and scrollbar
synchronization for normal terminal buffers while respecting alternate-screen
and terminal mouse modes.

#### Scenario: User scrolls shell output
- **WHEN** a terminal pane has scrollback and the user scrolls with a trackpad, wheel, or scrollbar
- **THEN** the visible terminal buffer and scrollbar position update together without changing shell focus

#### Scenario: Terminal enters alternate screen
- **WHEN** a full-screen terminal application or alternate-screen mode is active
- **THEN** the surface routes scroll input according to terminal mode and does not expose stale normal-buffer scrollback as the active viewport

#### Scenario: Program updates scrollback
- **WHEN** terminal output adds or removes scrollback
- **THEN** native scrollbar range and thumb position update from terminal surface metrics

### Requirement: Keyboard and IME input is complete
The macOS terminal surface SHALL translate keyboard, modifier, key-equivalent,
text input, IME/preedit, marked text, and secure-input state into terminal
surface operations through one input adapter.

#### Scenario: Printable input
- **WHEN** a focused terminal pane receives printable keyboard input
- **THEN** the adapter delivers the corresponding terminal input to the surface and preserves pane focus

#### Scenario: Command key equivalent
- **WHEN** the user presses a command-key shortcut that belongs to the app menu or command surface
- **THEN** the shortcut is routed to the native command handler instead of being incorrectly inserted into the terminal

#### Scenario: IME composition
- **WHEN** the user composes text with an input method
- **THEN** marked text, preedit updates, commit, and cancellation are reflected in the terminal surface according to Ghostty input semantics

#### Scenario: Secure input active
- **WHEN** terminal secure input is active
- **THEN** the surface records secure-input state and avoids exposing sensitive typed content through debug or automation summaries

### Requirement: Mouse and pointer input matches terminal modes
The macOS terminal surface SHALL handle primary, secondary, other-button,
movement, drag, pressure, scroll, hover, and URL pointer states according to
terminal mouse mode and native AppKit behavior.

#### Scenario: Mouse application active
- **WHEN** a terminal application enables mouse reporting
- **THEN** clicks, movement, drag, and scroll events are delivered to the terminal surface with the expected button and modifier state

#### Scenario: Text selection mode
- **WHEN** the terminal is not consuming mouse events for an application
- **THEN** drag gestures select terminal text and do not accidentally drag the window background

#### Scenario: URL hover
- **WHEN** the pointer hovers over a terminal URL recognized by the surface
- **THEN** the UI exposes a native pointer or context affordance without showing raw detection metadata

### Requirement: Clipboard and selection are native
The macOS terminal surface SHALL support native selection, copy, paste, bracketed
paste behavior when available, context menus, and paste failure reporting.

#### Scenario: Copy selection
- **WHEN** the user selects terminal text and invokes Copy
- **THEN** the selected terminal text is written to the system pasteboard and the selection state remains coherent

#### Scenario: Paste into focused pane
- **WHEN** the user invokes Paste in a ready terminal pane
- **THEN** pasteboard text is delivered to the terminal surface through the paste path appropriate for terminal mode

#### Scenario: Paste cannot be delivered
- **WHEN** paste targets a closed, readonly, failed, or not-ready terminal surface
- **THEN** the surface reports a user-visible non-delivery state and records diagnostics

### Requirement: Search is pane scoped
The macOS terminal surface SHALL provide pane-scoped terminal search with native
keyboard entry, next/previous navigation, match count when available, and clean
dismissal.

#### Scenario: Search current pane
- **WHEN** the user opens search while a terminal pane is focused and enters a query
- **THEN** matches are highlighted only within that pane and focus can return to the terminal after dismissal

#### Scenario: Navigate matches
- **WHEN** the user invokes next or previous match
- **THEN** the surface navigates within the pane's terminal buffer and updates match status

### Requirement: Terminal state overlays are truthful
The terminal surface SHALL expose user-facing overlays or inline state for child
exit, renderer failure, readonly mode, input-not-ready, bell/attention, and
recoverable fallback states.

#### Scenario: Child process exits
- **WHEN** the terminal child process exits
- **THEN** the pane shows a terminal-specific exit state and control-plane text delivery no longer reports false success

#### Scenario: Renderer fails
- **WHEN** the renderer reports failure or cannot draw the surface
- **THEN** the pane presents an actionable non-ready state instead of a blank fake terminal

#### Scenario: Bell occurs in background pane
- **WHEN** a background pane emits a bell or attention event
- **THEN** Alan records pane attention and surfaces it through sidebar/status affordances without stealing focus
