## ADDED Requirements

### Requirement: Semantic Command Output Actions Are Pane Scoped
The macOS terminal surface SHALL support semantic command-output actions only
for the owning pane and only when command boundary metadata is available.

#### Scenario: Copy known command output
- **WHEN** the focused terminal pane has a known command output range and the
  user invokes copy last command output
- **THEN** the terminal surface copies that range from the pane buffer to the
  pasteboard without sending printable input to the terminal process

#### Scenario: Latest command output is empty
- **WHEN** the focused terminal pane has a reliable latest command output range
  that contains no rows
- **THEN** Alan treats that empty range as the latest command output instead of
  copying or searching output from an older command

#### Scenario: Command output range is unknown
- **WHEN** the focused terminal pane does not have a reliable command output
  range
- **THEN** Alan falls back to ordinary terminal selection, visible-range copy,
  or scrollback behavior where appropriate without guessing a last-command
  output range from visible screen text

### Requirement: Prompt Navigation Respects Terminal Modes
Prompt navigation SHALL operate on normal-buffer semantic prompt marks and
SHALL avoid conflicting with alternate-screen or terminal mouse modes.

#### Scenario: Normal buffer prompt navigation
- **WHEN** the focused pane is in normal-buffer mode and semantic prompt marks
  are available
- **THEN** previous or next prompt navigation scrolls the terminal viewport to
  the selected prompt mark

#### Scenario: Alternate screen is active
- **WHEN** an alternate-screen application owns the focused pane
- **THEN** prompt navigation does not expose stale normal-buffer prompt marks as
  active application state

### Requirement: Command Output Search Reuses Search Ownership
Command-output search flows SHALL reuse pane-scoped terminal search ownership
instead of introducing a global search panel.

#### Scenario: Search command output
- **WHEN** the user opens command-output search for a focused pane
- **THEN** Alan scopes the query, match navigation, and dismissal behavior to
  that pane's terminal surface

#### Scenario: Search dismissed
- **WHEN** the user dismisses command-output search
- **THEN** keyboard focus returns to the terminal pane that owned the
  interaction when available

#### Scenario: Search last output unavailable
- **WHEN** reliable command output range metadata is unavailable
- **THEN** search-last-output falls back to pane-scoped scrollback search rather
  than presenting a fake command-scoped search range
