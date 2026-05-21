# macos-shell-terminal-lifecycle Specification

## Purpose
Define the native macOS shell terminal lifecycle contract for pane-owned
terminal runtimes, truthful text delivery, stable runtime metadata, and
user-safe fallback states.
## Requirements
### Requirement: Terminal runtimes survive view selection changes
The macOS shell host SHALL keep a tab's terminal process, renderer surface,
runtime metadata, and buffered control state owned by the shell model or a
dedicated runtime registry rather than by the transient SwiftUI/AppKit view that
happens to be visible. Runtime continuity applies while the Tab remains part of
the current shell state; explicit close operations and workspace lifecycle
retirement of inactive unpinned Tabs SHALL finalize the affected terminal
runtimes through the runtime service boundary.

#### Scenario: Switching away from a tab
- **WHEN** a user switches from one tab to another and the first tab is no longer rendered
- **THEN** the first tab's terminal process and runtime record remain alive unless the tab or pane is explicitly closed or the Tab is later retired by the workspace lifecycle contract

#### Scenario: Switching back to a tab
- **WHEN** a user returns to a previously selected tab
- **THEN** the host reattaches the visible view to the existing terminal runtime instead of booting a new shell process

#### Scenario: Closing a tab
- **WHEN** a tab is explicitly closed
- **THEN** all terminal runtimes owned by that tab are torn down exactly once and their final state is reflected in shell state

#### Scenario: Retiring an inactive unpinned Tab
- **WHEN** workspace lifecycle pruning retires an inactive unpinned Tab
- **THEN** terminal runtimes owned by that Tab are finalized through the same runtime service ownership boundary used by explicit close operations

#### Scenario: Restoring a Tab after app restart
- **WHEN** alan restores a Pinned Tab or retained Unpinned Tab from the workspace manifest after app restart
- **THEN** alan creates new terminal runtimes from the restore snapshot instead of claiming continuity with processes from the prior app instance

### Requirement: Pane text delivery is truthful
The macOS shell host SHALL only acknowledge `pane.send_text` as applied when the
target pane runtime accepts the text or queues it in a durable pane-specific
delivery buffer that will be flushed when the runtime is attached.

#### Scenario: Visible pane accepts text
- **WHEN** `pane.send_text` targets a visible pane with a ready terminal runtime
- **THEN** the response reports `applied: true` and includes the accepted byte count

#### Scenario: Background pane accepts text
- **WHEN** `pane.send_text` targets a background pane with an existing terminal runtime
- **THEN** the text is delivered to that runtime without requiring the tab to become visible

#### Scenario: Target pane cannot accept text
- **WHEN** `pane.send_text` targets a missing, closed, or not-yet-bootable pane
- **THEN** the response reports `applied: false` with a specific error code and does not claim accepted bytes

### Requirement: Focus and metadata follow runtime identity
The macOS shell host SHALL associate focus, cwd, title, process status,
attention, renderer phase, and last-command metadata with stable pane IDs.

#### Scenario: Runtime metadata arrives for a background pane
- **WHEN** a background pane reports cwd, title, process, or attention changes
- **THEN** the shell state for that pane updates without changing the user's selected tab

#### Scenario: Visible focus changes
- **WHEN** the user focuses a visible pane
- **THEN** shell state updates the focused pane while preserving the runtime records for all other panes

### Requirement: Host fallback state is user-safe
The macOS shell host SHALL make unavailable Ghostty or failed terminal runtime
states explicit and actionable without presenting a fake usable terminal.

#### Scenario: Ghostty is unavailable
- **WHEN** the app launches without a linked or loadable Ghostty runtime
- **THEN** the affected pane reports a non-ready terminal state and the UI provides setup/debug information without accepting terminal input as if it succeeded

#### Scenario: Surface creation fails
- **WHEN** a terminal surface cannot be created for a pane
- **THEN** the pane records the failure reason and control-plane mutations against that pane fail or queue according to the delivery contract

### Requirement: Surface readiness is lifecycle metadata
The macOS shell host SHALL track surface readiness, input readiness, renderer
health, child process status, readonly state, and terminal mode as runtime
metadata associated with stable pane IDs.

#### Scenario: Surface becomes input ready
- **WHEN** a pane surface finishes creation and can accept terminal input
- **THEN** pane lifecycle metadata records input-ready state and pending delivery may flush according to the delivery contract

#### Scenario: Renderer becomes unhealthy
- **WHEN** a terminal renderer reports degraded or failed health
- **THEN** pane lifecycle metadata records that state and terminal input/delivery responses remain truthful

#### Scenario: Child exits
- **WHEN** the terminal child process exits
- **THEN** pane lifecycle metadata records exit status and later text delivery does not claim success unless a new runtime is explicitly started

### Requirement: Terminal mode changes survive view changes
The macOS shell host SHALL keep terminal mode metadata such as alternate screen,
mouse reporting, search state, and readonly state with the runtime identity
rather than with transient host views.

#### Scenario: View recreated during alternate screen
- **WHEN** a pane view is recreated while an alternate-screen application is active
- **THEN** the replacement view reflects the current terminal mode rather than reverting to normal-buffer assumptions

#### Scenario: Background pane exits readonly mode
- **WHEN** a background pane changes readonly or input readiness state
- **THEN** the pane metadata updates without selecting that tab

### Requirement: Terminal lifecycle ownership is service backed
The macOS shell host SHALL route terminal process, renderer surface, runtime
metadata, pending delivery buffer, and teardown ownership through the terminal
runtime service rather than through transient host views.

#### Scenario: Runtime survives SwiftUI reconstruction
- **WHEN** SwiftUI reconstructs the shell content view while a pane remains part of shell state
- **THEN** the terminal runtime service keeps the pane surface alive and the new view attaches to the same runtime identity

#### Scenario: Runtime no longer exists
- **WHEN** shell state references a pane whose terminal runtime has irrecoverably failed or closed
- **THEN** lifecycle metadata reports the non-ready state and the UI/control plane do not treat the pane as a ready terminal

### Requirement: Pane close finalizes runtime identity
The macOS shell host SHALL make pane, tab, and window close operations call the
runtime service finalizer for each affected pane before the pane is removed from
authoritative runtime state.

#### Scenario: Closing a split pane
- **WHEN** a user closes one pane in a split tab
- **THEN** the runtime service finalizes only that pane's surface and the remaining panes keep their runtime identities

#### Scenario: Closing a window
- **WHEN** a shell window closes
- **THEN** every pane runtime owned by that window transitions to closing or closed state before the window control identity is released

### Requirement: Reattachment preserves terminal continuity
Visible terminal views SHALL reattach to existing runtime handles and MUST NOT
restart shell processes, clear scrollback, or reset pane metadata solely because
selection, split layout, or window visibility changed.

#### Scenario: Tab selection changes repeatedly
- **WHEN** a user switches between terminal tabs several times
- **THEN** each tab keeps its existing terminal process, scrollback, title, cwd, and runtime phase

#### Scenario: Split layout changes
- **WHEN** a pane is moved, resized, or temporarily hidden by split zoom
- **THEN** its runtime handle remains continuous and reattaches when visible again

### Requirement: Terminal-area events are owned by the terminal host
The macOS shell host SHALL route mouse events that occur inside terminal pixels
through the pane's AppKit terminal host rather than through SwiftUI tap gesture
wrappers around the terminal view.

#### Scenario: First click activates and reaches the terminal
- **WHEN** a user clicks a visible terminal pane that is not currently selected
- **THEN** the shell selects that pane, makes its terminal host first responder, and forwards the same mouse-down event to the terminal renderer

#### Scenario: Terminal text selection starts on first drag
- **WHEN** a user begins a drag inside a visible terminal pane
- **THEN** the drag is handled by the terminal host and can start terminal text selection without requiring a prior selection-only click

#### Scenario: Terminal host lifetime remains pane-keyed
- **WHEN** SwiftUI recreates the terminal leaf view for an existing pane
- **THEN** the registry reuses the pane-keyed terminal host and refreshes its weak activation boundary without transferring terminal event ownership to the SwiftUI view

### Requirement: Terminal activation does not retain shell controllers
Registry-owned terminal host views SHALL use a weak activation boundary when
requesting pane selection from the shell controller.

#### Scenario: Host requests activation
- **WHEN** a terminal host receives a mouse-down event for a pane with a stable pane ID
- **THEN** it calls the weak activation boundary for that pane before requesting terminal focus

#### Scenario: Activation boundary is unavailable
- **WHEN** a terminal host has no activation delegate available
- **THEN** terminal input handling remains local to the host and the host does not keep a strong closure that can retain the shell controller

### Requirement: Split workspace mutations preserve live runtimes
The macOS shell host SHALL preserve pane runtime identity across split resize,
equalize, focus, pane lift, and cross-tab pane move operations unless the
operation explicitly closes the pane or tab.

#### Scenario: Resize split
- **WHEN** the user resizes a split divider
- **THEN** all panes in the tab keep their existing runtime handles and metadata

#### Scenario: Equalize splits
- **WHEN** the user equalizes splits in a tab
- **THEN** all panes in the tab keep their existing runtime handles and metadata

#### Scenario: Lift pane
- **WHEN** the user lifts a pane to its own tab
- **THEN** the pane keeps its runtime handle, scrollback, title, cwd, and pending delivery state

#### Scenario: Move pane to another tab
- **WHEN** the user moves a pane to another tab within the same window
- **THEN** the pane keeps its runtime handle, scrollback, title, cwd, and pending delivery state

### Requirement: Zoom preserves sibling runtimes
The macOS shell host SHALL implement split zoom as view state that does not
close, recreate, or detach sibling terminal ContentInstance runtimes unnecessarily.

#### Scenario: Zoom hides siblings
- **WHEN** a PaneSlot with terminal content is zoomed
- **THEN** sibling terminal ContentInstances remain registered in the terminal runtime service and keep their scrollback, title, cwd, and pending delivery state

#### Scenario: Unzoom reattaches siblings
- **WHEN** the user exits zoom
- **THEN** sibling PaneSlots reappear by reattaching terminal views to their existing terminal ContentInstance runtime handles

### Requirement: Pane movement preserves runtime continuity
In-tab pane movement and drag/drop-backed movement SHALL move PaneSlot placement
without replacing the mounted ContentInstance identity or any terminal ContentInstance
runtime identity.

#### Scenario: In-tab movement
- **WHEN** a PaneSlot moves to another split position in the same tab
- **THEN** the PaneSlot keeps its mounted ContentInstance
- **AND** terminal content keeps its runtime handle, scrollback, title, cwd, and pending delivery state

#### Scenario: Drag/drop movement
- **WHEN** a PaneSlot moves through an enabled drag/drop affordance
- **THEN** the PaneSlot and mounted ContentInstance keep the same identities as the equivalent explicit move command

### Requirement: Terminal commands target the runtime owner
Copy, paste, and terminal search SHALL resolve the focused PaneSlot to mounted
terminal content and deliver to that terminal ContentInstance runtime or host
surface rather than to transient shell chrome.

#### Scenario: Copy terminal selection
- **WHEN** Copy is invoked and the focused terminal host owns a selection
- **THEN** the terminal host handles the copy operation without changing terminal ContentInstance runtime state

#### Scenario: Paste terminal input
- **WHEN** Paste is invoked for a focused PaneSlot that mounts terminal content
- **THEN** the paste operation is delivered through that terminal ContentInstance input path

#### Scenario: Search terminal content
- **WHEN** terminal search is invoked for a focused PaneSlot that mounts terminal content
- **THEN** search state follows that terminal ContentInstance runtime identity across view reconstruction

### Requirement: Split close operations define runtime finalization
The macOS shell host SHALL define explicit terminal runtime finalization
semantics for close pane, close tab, close window, pane lift, and pane move
operations that empty containers.

#### Scenario: Close focused pane
- **WHEN** the user invokes close pane
- **THEN** alan finalizes exactly that pane runtime and repairs the split tree around the removed leaf

#### Scenario: Close tab after moving last pane
- **WHEN** a move operation leaves the source tab empty and alan closes that tab
- **THEN** alan does not finalize the moved pane runtime as part of source tab cleanup

### Requirement: Terminal keyboard input is terminal-host owned
The macOS shell host SHALL route keyboard events for the focused terminal pane
through the terminal host unless a visible alan command surface or an explicit
app-reserved `Command` shortcut owns that key.

#### Scenario: Vim control key reaches terminal
- **WHEN** a focused terminal pane is running a TUI such as Vim and no alan command surface is visible
- **THEN** non-`Command` terminal keys such as Escape, Tab, Backspace, `Control-[`, `Control-W`, `Control-F`, and `Control-B` are delivered to the terminal runtime
- **AND** the shell workspace command router does not consume those keys as pane, tab, or command-input actions

#### Scenario: Printable physical keyboard input uses Ghostty key events
- **WHEN** a focused terminal pane receives printable physical keyboard input such as `a` or `:`
- **THEN** alan first lets AppKit text interpretation process the key so IME composition can start
- **AND** alan delivers committed printable input through the Ghostty key-event path
- **AND** alan does not bypass Ghostty's key encoder by sending that physical key through programmatic text injection

#### Scenario: IME composition can start from printable input
- **WHEN** a focused terminal pane uses a Chinese/Japanese/Korean input method
- **AND** the user types the first printable key of a composition
- **THEN** alan lets AppKit `interpretKeyEvents` create or update marked text
- **AND** alan updates Ghostty preedit state from the resulting marked text

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

#### Scenario: AppKit key equivalent is re-dispatched to terminal
- **WHEN** AppKit routes a focused terminal Control or Command key through `performKeyEquivalent`
- **AND** the key is not a visible command-surface key or explicit app/workspace shortcut
- **THEN** alan preserves Ghostty's key-equivalent state machine and allows AppKit to continue to `doCommand`
- **AND** `doCommand` re-dispatches the same event back through the terminal host
- **AND** the re-dispatched event is delivered to the terminal runtime exactly once

#### Scenario: Control slash is encoded like Ghostty
- **WHEN** a focused terminal pane receives `Control-/`
- **THEN** alan converts the key-equivalent text to `Control-_` before terminal delivery
- **AND** the event does not become an AppKit beep or an unresolved native command

#### Scenario: Focus-only split click is not injected into Vim
- **WHEN** the app and window are already active
- **AND** the user clicks a terminal split pane that is selected in the shell model but is not the AppKit first responder
- **THEN** alan focuses that terminal host and consumes the focus-transfer mouse down
- **AND** matching left mouse drags are suppressed until the focus-transfer mouse up
- **AND** the matching left mouse up is suppressed
- **AND** Vim mouse mode does not receive a stray click or selection drag from the focus transfer

#### Scenario: Terminal input router owns primary pointer sequence policy
- **WHEN** terminal pointer routing is evaluated for a focused or focus-transfer terminal pane
- **THEN** the macOS terminal surface controller owns the sequence policy for focus-only primary button events, normal-buffer selection drags, alternate-screen mouse delivery, mouse-reporting delivery, and unready-surface ignores
- **AND** the AppKit host view only normalizes events and executes the returned focus, deliver, consume, or fallthrough decision
- **AND** focus-transfer suppression state MUST NOT be split between separate host-view drag guards and surface pointer routing

#### Scenario: Modifier changes follow Ghostty semantics
- **WHEN** modifier keys change while IME marked text is active
- **THEN** alan does not forward the modifier transition to the terminal runtime
- **AND** outside IME composition alan preserves caps-lock and right-side modifier bits when building Ghostty key events

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
