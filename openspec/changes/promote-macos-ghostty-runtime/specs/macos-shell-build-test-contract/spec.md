## ADDED Requirements

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
