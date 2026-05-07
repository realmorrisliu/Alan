## ADDED Requirements

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
