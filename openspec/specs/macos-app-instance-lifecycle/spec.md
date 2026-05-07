# macos-app-instance-lifecycle Specification

## Purpose
Define the native macOS app singleton and primary shell window lifecycle so
launch, reopen, duplicate-process, and New Window paths preserve one Alan app
instance and one shell control plane per user session.

## Requirements
### Requirement: Native macOS launches use one app instance
The Alan macOS app bundle SHALL allow only one running Alan app instance for the
current user and bundle identifier.

#### Scenario: Initial launch
- **WHEN** no Alan macOS app instance is running and the user launches the app
- **THEN** one Alan app process starts and acquires the singleton app lock

#### Scenario: Repeated normal launch
- **WHEN** an Alan macOS app instance is already running and the user launches the app through normal Finder, Dock, Spotlight, or `open` behavior
- **THEN** the existing app instance is activated and no additional Alan app process remains running

#### Scenario: Forced duplicate launch
- **WHEN** an Alan macOS app instance is already running and a second app process is forced with `open -n` or direct executable launch
- **THEN** the second process activates the existing app and terminates before creating a SwiftUI scene, shell window context, control socket, or terminal runtime

#### Scenario: Quit releases singleton ownership
- **WHEN** the running Alan app quits normally
- **THEN** the singleton app lock is released so the next launch can become the owner

#### Scenario: Crashed owner does not block relaunch
- **WHEN** a prior Alan app process exits without a clean quit
- **THEN** stale singleton state does not prevent a later launch from acquiring ownership

### Requirement: Native macOS presents one primary shell window
The Alan macOS app SHALL present at most one primary shell window, and all
launch, reopen, activation, and New Window paths SHALL focus or reopen that
window instead of creating another primary shell window.

#### Scenario: First owned launch creates primary window
- **WHEN** the owned Alan app instance completes launch
- **THEN** exactly one primary Alan shell window is presented

#### Scenario: New Window command
- **WHEN** the user invokes the New Window menu item or presses `Command-N`
- **THEN** no additional primary Alan shell window is created and the existing primary window is focused or reopened

#### Scenario: Activation while primary window is visible
- **WHEN** the user activates Alan while the primary shell window is already visible
- **THEN** the existing primary shell window becomes key without allocating another shell window

#### Scenario: Reopen after closing primary window
- **WHEN** the Alan app is still running after the primary shell window has been closed
- **THEN** Dock or application reopen presents one primary shell window and does not create more than one shell window

### Requirement: Primary shell owner is process scoped
The macOS app SHALL own the primary shell context at app-process scope so scene
or root-view recreation does not allocate competing shell hosts while the app
process remains running.

#### Scenario: Root view recreated
- **WHEN** SwiftUI recreates the primary shell root view for the same running app process
- **THEN** the view reuses the process-scoped shell owner instead of creating a fresh shell window identity

#### Scenario: Primary scene reopened
- **WHEN** the primary window scene is reopened in the existing app process
- **THEN** the shell owner remains singular and no additional terminal runtime registry is created for a duplicate window

#### Scenario: Duplicate process exits early
- **WHEN** a second app process fails singleton lock acquisition
- **THEN** it exits without creating shell persistence files, control-plane sockets, or runtime registries

### Requirement: Singleton behavior has focused verification
The Apple client SHALL include focused automated checks or documented manual
verification for macOS process singleton, primary-window singleton, command
routing, reopen, and lock-release behavior.

#### Scenario: Lock behavior tested
- **WHEN** singleton lock code changes
- **THEN** tests verify first acquisition, rejected second acquisition, release, and owner-exit recovery

#### Scenario: Window singleton verified
- **WHEN** macOS scene or command behavior changes
- **THEN** tests, local scripts, or manual notes verify initial launch, `Command-N`, Dock reopen, close/reopen, repeated `open`, and forced `open -n`

#### Scenario: Documentation updated
- **WHEN** singleton behavior changes the shell window lifecycle contract
- **THEN** Apple-client README or related developer docs no longer describe multiple independent macOS windows as the supported default model
