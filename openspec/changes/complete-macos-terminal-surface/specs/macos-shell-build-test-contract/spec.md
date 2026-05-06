## ADDED Requirements

### Requirement: Surface behavior has focused verification
The Apple client SHALL add focused tests or documented manual verification for
terminal scrollback, input translation, IME/preedit, selection, clipboard,
search, terminal mode changes, renderer health, and child-exit behavior.

#### Scenario: Scrollback verification
- **WHEN** terminal surface work changes scrollback or scrollbar behavior
- **THEN** tests or manual notes verify normal-buffer scrolling, alternate-screen behavior, and scrollbar synchronization

#### Scenario: Input verification
- **WHEN** terminal input adapter behavior changes
- **THEN** tests or manual notes verify printable input, command-key routing, modifiers, IME composition, paste, and terminal mouse mode

#### Scenario: Failure-state verification
- **WHEN** renderer health, child-exit, or fallback UI changes
- **THEN** tests or manual notes verify that the default UI is truthful and debug details remain inspector-only

### Requirement: Surface adapters are unit-testable with fakes
Terminal surface controllers and input/scrollback adapters SHALL be testable
with fake surface handles for state transitions and event translation that do
not require a live Ghostty renderer.

#### Scenario: Fake scroll metrics
- **WHEN** a fake surface publishes scrollback metrics
- **THEN** adapter tests verify native scrollbar range and visible viewport updates

#### Scenario: Fake input events
- **WHEN** adapter tests send keyboard, mouse, paste, and search commands through fake events
- **THEN** the fake surface receives normalized terminal operations or command-routing decisions
