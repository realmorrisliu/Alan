## MODIFIED Requirements

### Requirement: Native macOS launches use one app instance
The alan for macOS app bundle SHALL allow only one running alan app instance for
the current user and bundle identifier.

#### Scenario: Initial launch
- **WHEN** no alan for macOS app instance is running and the user launches the
  app
- **THEN** one alan app process starts and acquires the singleton app lock

#### Scenario: Repeated normal launch
- **WHEN** an alan for macOS app instance is already running and the user
  launches the app through normal Finder, Dock, Spotlight, or `open` behavior
- **THEN** the existing app instance is activated and no additional alan app
  process remains running

#### Scenario: Forced duplicate launch
- **WHEN** an alan for macOS app instance is already running and a second app
  process is forced with `open -n` or direct executable launch
- **THEN** the second process activates the existing app and terminates before
  creating a SwiftUI scene, shell window context, control socket, or terminal
  runtime

#### Scenario: Quit releases singleton ownership
- **WHEN** the running alan app quits normally
- **THEN** the singleton app lock is released so the next launch can become the
  owner

#### Scenario: Crashed owner does not block relaunch
- **WHEN** a prior alan app process exits without a clean quit
- **THEN** stale singleton state does not prevent a later launch from acquiring
  ownership

### Requirement: Native macOS presents one primary shell window
The alan for macOS app SHALL present at most one primary terminal workspace
window, and all launch, reopen, activation, and New Window paths SHALL focus or
reopen that window instead of creating another primary terminal workspace
window.

#### Scenario: First owned launch creates primary window
- **WHEN** the owned alan app instance completes launch
- **THEN** exactly one primary terminal workspace window is presented without
  requiring a Dock icon click, application reopen, or other secondary
  activation step

#### Scenario: New Window command
- **WHEN** the user invokes the New Window menu item or presses `Command-N`
- **THEN** no additional primary terminal workspace window is created and the
  existing primary window is focused or reopened

#### Scenario: Activation while primary window is visible
- **WHEN** the user activates alan while the primary terminal workspace window
  is already visible
- **THEN** the existing primary terminal workspace window becomes key without
  allocating another shell window

#### Scenario: Reopen after closing primary window
- **WHEN** the alan app is still running after the primary terminal workspace
  window has been closed
- **THEN** Dock or application reopen presents one primary terminal workspace
  window and does not create more than one terminal workspace window

## ADDED Requirements

### Requirement: Native macOS app identity uses alan for macOS naming
The native macOS app SHALL align bundle, display, singleton, logging, capture,
and persisted support identities with the `alan` product brand and
`alan for macOS` platform label.

#### Scenario: App metadata is generated
- **WHEN** the macOS app bundle is built
- **THEN** the generated app product is `alan.app`
- **AND** `CFBundleDisplayName` and macOS product name are `alan`
- **AND** the default bundle identifier is `app.alanworks.macos`

#### Scenario: Singleton and support paths are created
- **WHEN** singleton lock files or App Support persistence paths are created
- **THEN** they use the new alan for macOS identity
- **AND** they do not create new paths named `AlanNative`

#### Scenario: Logs and capture helpers identify the app
- **WHEN** maintainers inspect logs, run capture helpers, or filter running app
  instances by bundle identifier
- **THEN** defaults use `app.alanworks.macos` and lowercase alan naming
- **AND** `com.realmorrisliu.AlanNative` and `dev.alan.native` are not active
  defaults
