## ADDED Requirements

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
