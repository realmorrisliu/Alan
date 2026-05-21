## ADDED Requirements

### Requirement: Quick Terminal Presentation Is Native And Lightweight
The quick terminal SHALL present as a detached, lightweight native macOS Peak
window that preserves Alan's terminal-first shell design and avoids dashboard or
floating-card composition.

#### Scenario: Quick terminal appears
- **WHEN** the quick terminal is summoned
- **THEN** Alan presents a focused terminal surface with restrained native
  material chrome, no inspector, no marketing-style header, and no duplicate
  sidebar

#### Scenario: Quick terminal appears outside the main window
- **WHEN** the user summons the quick terminal from any macOS Space
- **THEN** Alan presents the Peak on the current active display and Space
  without requiring, attaching to, or raising Alan's main window

#### Scenario: Quick terminal hides
- **WHEN** the quick terminal is dismissed without closing the session
- **THEN** Alan hides the presentation without changing regular shell sidebar,
  tab, split, or terminal geometry

#### Scenario: Terminal keys remain terminal keys
- **WHEN** the quick terminal owns focus and the user presses `Esc`
- **THEN** Alan sends the key to the terminal surface instead of hiding the Peak
  by default

#### Scenario: Focus changes outside the Peak
- **WHEN** the quick terminal loses focus because the user clicks another app or
  window
- **THEN** Alan keeps the Peak visible until the user invokes the quick-terminal
  toggle, hide command, close command, or promotion action

#### Scenario: Quick terminal can become a normal tab
- **WHEN** the user opens the Peak's `Open in Space` affordance
- **THEN** Alan offers Alan space destinations without duplicating sidebar
  chrome inside the Peak

#### Scenario: Quick terminal activity exists
- **WHEN** the hidden quick terminal has user-actionable activity
- **THEN** Alan surfaces that activity through the same compact activity and
  notification policy used for regular terminal panes
