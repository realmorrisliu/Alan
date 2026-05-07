# macos-shell-build-test-contract Specification

## Purpose
Define the Apple client build, dependency, and focused test contract for the
macOS shell host.

## Requirements

### Requirement: Build requirements match documentation
The Apple client SHALL keep documented system requirements, deployment targets,
and project settings aligned.

#### Scenario: Deployment target changes
- **WHEN** the Xcode project deployment targets are changed
- **THEN** `clients/apple/README.md` and relevant specs are updated in the same change

#### Scenario: Documented build command
- **WHEN** a developer runs the documented macOS build command after preparing dependencies
- **THEN** the command succeeds or fails with documented, actionable dependency setup instructions

### Requirement: Ghostty dependency setup is explicit
The Apple project SHALL treat Ghostty framework, resources, and terminfo as an
explicit local dependency with a verifiable setup path.

#### Scenario: Dependencies are missing
- **WHEN** `GhosttyKit.xcframework`, `ghostty-resources`, or `ghostty-terminfo` are absent
- **THEN** the build or setup check reports the missing dependency and points to the supported preparation command

#### Scenario: Dependencies are present
- **WHEN** local Ghostty artifacts are prepared
- **THEN** the macOS app build links/copies them without module-map or umbrella-header warnings that obscure real failures

### Requirement: Shell model behavior has focused tests
The Apple client SHALL have focused automated tests for shell state mutation and
control-plane behavior that can run without launching the full app UI.

#### Scenario: State mutation tests
- **WHEN** shell spaces, tabs, and panes are created, split, moved, lifted, focused, and closed
- **THEN** tests verify focused IDs, pane trees, space membership, attention state, and failure cases

#### Scenario: Control-plane tests
- **WHEN** control-plane query and mutation commands are executed against a test host
- **THEN** tests verify successful responses, missing-target errors, event records, and text-delivery acknowledgement semantics

### Requirement: Terminal host boundary is testable
The terminal host SHALL expose a testable boundary for runtime attachment,
teardown, and text delivery without requiring the real Ghostty library in every
test.

#### Scenario: Mock runtime accepts text
- **WHEN** a test runtime is registered for a pane and `pane.send_text` is issued
- **THEN** the test verifies the text reaches the runtime and the control response reports accepted bytes

#### Scenario: Mock runtime unavailable
- **WHEN** no runtime is registered for a pane and `pane.send_text` is issued
- **THEN** the test verifies the response reports failure or durable queueing according to the delivery contract

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
