## ADDED Requirements

### Requirement: Zoom preserves sibling runtimes
The macOS shell host SHALL implement split zoom as view state that does not
close, recreate, or detach sibling pane runtimes unnecessarily.

#### Scenario: Zoom hides siblings
- **WHEN** a pane is zoomed
- **THEN** sibling panes remain registered in the terminal runtime service and keep their scrollback, title, cwd, and pending delivery state

#### Scenario: Unzoom reattaches siblings
- **WHEN** the user exits zoom
- **THEN** sibling panes reappear by reattaching to their existing runtime handles

### Requirement: Pane movement preserves runtime continuity
In-tab pane movement and drag/drop-backed movement SHALL move pane placement
without replacing the pane runtime identity.

#### Scenario: In-tab movement
- **WHEN** a pane moves to another split position in the same tab
- **THEN** the pane keeps its runtime handle, scrollback, title, cwd, and pending delivery state

#### Scenario: Drag/drop movement
- **WHEN** a pane moves through an enabled drag/drop affordance
- **THEN** the pane keeps the same runtime identity as the equivalent explicit move command

### Requirement: Terminal commands target the runtime owner
Copy, paste, and terminal search SHALL be delivered to the focused pane's
terminal runtime or host surface rather than to transient shell chrome.

#### Scenario: Copy terminal selection
- **WHEN** Copy is invoked and the focused terminal host owns a selection
- **THEN** the terminal host handles the copy operation without changing pane runtime state

#### Scenario: Paste terminal input
- **WHEN** Paste is invoked for the focused terminal pane
- **THEN** the paste operation is delivered through that pane's terminal input path

#### Scenario: Search terminal content
- **WHEN** terminal search is invoked for the focused pane
- **THEN** search state follows that pane's runtime identity across view reconstruction
