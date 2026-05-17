## MODIFIED Requirements

### Requirement: Runtime metadata is projected by pane identity
The runtime service SHALL project terminal title, cwd, process status,
attention, renderer phase, readiness, delivery diagnostics, terminal activity,
semantic command state, and CLI coding-agent status into alan shell state using
stable pane IDs.

#### Scenario: Metadata event from background pane
- **WHEN** a background pane emits a title, cwd, process, attention,
  renderer-state, activity, semantic command, or agent-status event
- **THEN** shell state updates the matching pane record without changing user
  focus

#### Scenario: Progress event from background pane
- **WHEN** a background pane emits terminal progress or command-completion
  activity
- **THEN** the runtime service projects that activity by pane ID so sidebar,
  titlebar, accessibility, and control surfaces can read the same state

#### Scenario: Agent event from background pane
- **WHEN** a supported CLI coding agent emits a lifecycle event from a terminal
  pane
- **THEN** the runtime service associates the normalized agent activity with the
  stable pane ID rather than with a transient host view

#### Scenario: Metadata event after pane close
- **WHEN** a terminal callback arrives after its pane has reached closed state
- **THEN** the runtime service ignores or records it as late diagnostics without
  resurrecting the pane
