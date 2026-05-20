## ADDED Requirements

### Requirement: Semantic Command Boundaries Are Pane Scoped
Alan SHALL model shell prompt marks, command start, command end, command text,
output range, exit status, and timestamps as semantic metadata owned by the
terminal pane when those signals are available.

#### Scenario: Shell integration reports command boundary
- **WHEN** shell integration or terminal protocol events identify a command
  start and command end
- **THEN** Alan records the command boundary and output range for the owning
  pane without changing terminal rendering or scrollback ownership

#### Scenario: Semantic data is unavailable
- **WHEN** prompt marks or command boundaries are unavailable because of shell,
  tmux, SSH, or application mode limitations
- **THEN** Alan keeps normal terminal behavior and disables semantic-only
  actions instead of guessing output ranges from screen text

### Requirement: Semantic Terminal Actions Are Focused And Reversible
Alan SHALL expose prompt navigation, copy-last-command-output, and
command-output search actions through pane-scoped command paths that do not
mutate terminal process state.

#### Scenario: User jumps between prompts
- **WHEN** semantic prompt marks exist for the focused pane and the user invokes
  previous or next prompt
- **THEN** Alan scrolls the pane viewport to the target prompt without changing
  shell focus, split layout, or command history

#### Scenario: User copies last command output
- **WHEN** a focused pane has a known last command output range and the user
  invokes copy last output
- **THEN** Alan copies that output to the system pasteboard without sending text
  into the terminal application

#### Scenario: User searches command output
- **WHEN** a focused pane has command output ranges and the user invokes
  command-output search
- **THEN** Alan scopes the interaction to that pane and returns focus to the
  terminal after dismissal

#### Scenario: Command-aware actions are command-menu actions
- **WHEN** a focused pane has reliable command boundary metadata
- **THEN** Alan exposes command-aware prompt navigation, copy-last-output, and
  search-last-output actions through `Go to or Command...` without adding
  persistent terminal chrome

#### Scenario: Command boundaries are unavailable
- **WHEN** a focused pane has no reliable command boundary metadata
- **THEN** Alan falls back to ordinary scrollback search, selection copy, and
  normal scrollback navigation rather than guessing command ranges from screen
  text

#### Scenario: No command browser in MVP
- **WHEN** semantic terminal actions are available
- **THEN** Alan does not render a command browser, visible command blocks, or
  persistent command-output segmentation for the MVP
