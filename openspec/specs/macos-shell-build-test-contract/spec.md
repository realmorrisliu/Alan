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

### Requirement: Runtime service ownership has focused tests
The Apple client SHALL include focused tests for process bootstrap, window
runtime service ownership, pane handle creation, reattachment, text delivery,
and teardown using fake Ghostty adapters where possible.

#### Scenario: Fake runtime reattaches view
- **WHEN** a test creates a pane handle, detaches the host view, and attaches a replacement host view
- **THEN** the test verifies that the pane handle identity and runtime metadata remain unchanged

#### Scenario: Fake runtime tears down once
- **WHEN** a test closes a pane, tab, or window through shell actions
- **THEN** the fake runtime observes exactly one teardown call per affected pane

### Requirement: Ghostty bootstrap is testable without launching the full app
The Apple client SHALL expose a bootstrap seam that lets tests verify Ghostty
dependency and initialization behavior without launching the full SwiftUI app or
requiring real terminal rendering.

#### Scenario: Bootstrap dependency missing
- **WHEN** a fake bootstrap reports missing Ghostty resources
- **THEN** tests verify that pane creation enters a non-ready state with an actionable error

#### Scenario: Bootstrap reused
- **WHEN** two window runtime services request terminal support in one test process
- **THEN** tests verify that the process bootstrap is invoked once and both services receive the same bootstrap result

### Requirement: Control-plane runtime tests use the service boundary
Control-plane tests SHALL exercise runtime-dependent mutations through the same
terminal runtime service boundary used by production code.

#### Scenario: Service accepts text
- **WHEN** a control-plane test sends text to a fake live pane runtime
- **THEN** the command response reports accepted bytes from the fake service and shell diagnostics remain clean

#### Scenario: Service reports runtime missing
- **WHEN** a control-plane test sends text to a pane whose service handle is absent
- **THEN** the command response reports a stable runtime-missing error

### Requirement: Terminal event ownership is contract-checked
The Apple client SHALL include focused shell contract checks that preserve the
terminal event ownership boundary between SwiftUI layout, AppKit terminal host
input, rendering canvases, and native window background dragging.

#### Scenario: SwiftUI terminal tap wrapper is reintroduced
- **WHEN** a code change wraps the terminal native view in a SwiftUI tap gesture for pane selection
- **THEN** the shell contract check fails with an error explaining that terminal-area selection belongs to the terminal host

#### Scenario: Activation delegate strongly retains controller state
- **WHEN** a code change stores terminal activation as a strong registry-owned closure
- **THEN** the shell contract check fails or the focused review checklist requires replacing it with the weak activation boundary

#### Scenario: Rendering canvas becomes interactive owner
- **WHEN** a code change lets Ghostty or fallback rendering canvas views receive terminal mouse-down hit tests as independent owners
- **THEN** the shell contract check fails or the focused review checklist requires routing those events through the terminal host

#### Scenario: Focused manual verification is performed
- **WHEN** event ownership implementation is ready for review
- **THEN** verification covers click-to-select, immediate typing, drag selection, right click, scrolling, and background window dragging in the running macOS app
