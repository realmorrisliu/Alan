# macos-app-architecture-maintainability Specification

## Purpose
Define maintainable native Apple client source organization, SwiftUI/AppKit
boundaries, service/model ownership, and validation expectations for macOS app
architecture changes.
## Requirements
### Requirement: Apple client source layout mirrors architecture ownership
The native Apple client SHALL organize source files by durable responsibility so
developers can distinguish app startup, SwiftUI views, models, controllers,
services, and support code without reading every file in a flat directory. The
active macOS source root SHALL use the lowercase `clients/apple/alan-macos`
identity rather than the historical `clients/apple/AlanNative` identity.

#### Scenario: Source tree is inspected
- **WHEN** a developer inspects `clients/apple/alan-macos`
- **THEN** app entry code, shell views, console views, protocol models,
  observable controllers, service/IO code, and support utilities are grouped by
  responsibility or explicitly documented as migration debt rather than silently
  mixed in one flat source directory

#### Scenario: README documents source layout
- **WHEN** the Apple client README describes the directory structure
- **THEN** the documented folders match the source tree and Xcode project
  organization used by the current code

#### Scenario: Historical source root is referenced
- **WHEN** active docs, scripts, specs, or Xcode project groups refer to the
  Apple client source root
- **THEN** they use `clients/apple/alan-macos`
- **AND** they do not use `clients/apple/AlanNative` except in explicitly
  marked historical migration notes

### Requirement: SwiftUI scene roots compose focused feature views
SwiftUI scene and root view files SHALL primarily compose stable layout,
selection, and feature views. They MUST NOT accumulate unrelated design tokens,
window coordination, command routing, inspector/debug panels, service clients,
or platform bridge implementations.

#### Scenario: macOS shell root is edited
- **WHEN** a developer changes the default macOS shell layout
- **THEN** the root view remains a readable composition of sidebar, workspace,
  command, and optional utility surfaces, with feature-specific UI implemented
  in dedicated view files

#### Scenario: App commands are edited
- **WHEN** a developer changes menu or keyboard command ownership
- **THEN** command definitions and command routing live in app or shell command
  files rather than being buried in unrelated view body code

### Requirement: AppKit bridges are narrow and named
The Apple client SHALL isolate AppKit bridge code behind small, named wrappers
or coordinators for the specific desktop behavior they own, while keeping
unrelated SwiftUI views free of ambient `NSWindow`, `NSView`, `NSApp`, socket,
or process-management details.

#### Scenario: Window placement changes
- **WHEN** hidden-titlebar placement, minimum size, traffic-light metrics, or
  primary-window focusing behavior changes
- **THEN** the implementation is owned by an app/window support component rather
  than by the macOS shell root view file

#### Scenario: Material background changes
- **WHEN** a SwiftUI view needs native material rendering
- **THEN** the `NSVisualEffectView` bridge is isolated behind a reusable material
  wrapper or support component

#### Scenario: Terminal host bridge changes
- **WHEN** terminal first-responder, hit-testing, IME, pointer, keyboard, or
  Ghostty attachment behavior changes
- **THEN** the AppKit terminal host keeps those behaviors behind the terminal
  host boundary and does not leak AppKit ownership through unrelated SwiftUI
  views

### Requirement: Terminal host collaborators have explicit ownership
The terminal host implementation SHALL separate runtime attachment, overlay
presentation, input routing, window observation, metadata publishing, and
surface coordination into explicit collaborators when those responsibilities
become non-trivial.

#### Scenario: Terminal input routing changes
- **WHEN** keyboard, IME, paste, pointer, scroll, or terminal search routing is
  modified
- **THEN** the change is reviewable in terminal input/surface collaborators
  without requiring a full audit of overlay layout or window observation code

#### Scenario: Runtime snapshot publication changes
- **WHEN** terminal runtime metadata publication changes
- **THEN** the owning component clearly distinguishes snapshot construction from
  AppKit layout and visible overlay presentation

### Requirement: Control-plane implementation separates IPC, execution, and persistence
The shell control-plane implementation SHALL keep protocol DTOs, local command
execution, socket serving, file polling, state merging, event persistence, and
diagnostics in reviewable ownership units.

#### Scenario: Socket transport changes
- **WHEN** local socket request size, timeout, accept loop, or client response
  behavior changes
- **THEN** the change is isolated to the socket transport owner and does not
  require reviewing shell mutation semantics

#### Scenario: Local command execution changes
- **WHEN** a shell control command changes how it mutates shell state or applies
  side effects
- **THEN** the local command executor owns the behavior separately from socket
  read/write and file-polling code

#### Scenario: Persistence diagnostics change
- **WHEN** state, event, command, or binding persistence diagnostics change
- **THEN** the persistence/event owner can be reviewed independently from IPC
  request parsing

### Requirement: API clients and event reducers are not embedded in views
The Apple client SHALL keep daemon API clients, event polling or streaming
loops, protocol event reduction, and view model state projection outside
complete SwiftUI view files so each can be tested or reviewed without editing a
complete SwiftUI screen.

#### Scenario: Session event mapping changes
- **WHEN** daemon session events are mapped into chat messages, timeline rows,
  pending-yield state, or connection state
- **THEN** the reducer behavior is owned outside the SwiftUI view body and can
  be tested without rendering the full console UI

#### Scenario: API endpoint support changes
- **WHEN** daemon API request or response DTOs change
- **THEN** the API client and protocol models own that change separately from
  shell or console layout files

### Requirement: Mobile and legacy console surfaces are isolated from the primary macOS shell
The Apple client SHALL keep mobile or legacy remote-control console surfaces
separate from the primary macOS shell path so contributors can identify which UI
is active for macOS shell development.

#### Scenario: macOS shell contributor opens the project
- **WHEN** a developer needs to change the default macOS shell experience
- **THEN** primary shell files are distinguishable from iOS/mobile console files
  by folder, naming, or project grouping

#### Scenario: iOS console behavior changes
- **WHEN** a developer changes the mobile console layout or event handling
- **THEN** the change does not require editing primary macOS shell root files
  unless a shared model or service contract is intentionally changed

### Requirement: Large files have planned ownership boundaries
The Apple client SHALL avoid large multi-responsibility Swift files as the
stable end state. When a file remains large or in a transitional owner during
migration, the owning change SHALL document the intended split and avoid adding
unrelated responsibilities to that file.

#### Scenario: Large file receives new behavior
- **WHEN** a developer adds behavior to an existing large Apple client file
- **THEN** the change either places the behavior in the target owner file or
  documents why the temporary location is still compatible with the migration
  plan

#### Scenario: Refactor slice completes
- **WHEN** a behavior-preserving architecture refactor slice is completed
- **THEN** the resulting file ownership makes future changes narrower to review
  than the previous large-file organization

### Requirement: Architecture migration debt is explicit and bounded
The Apple client SHALL keep known architecture-maintainability warnings visible
as tracked migration debt until they are resolved by focused refactor slices.
Known debt MUST identify the affected owner or file, the intended boundary, and
whether the current architecture gate treats it as non-blocking.

#### Scenario: Architecture report has warnings
- **WHEN** `check-architecture-maintainability.sh` completes in report mode with
  warnings
- **THEN** `clients/apple/ARCHITECTURE.md` records the current warning classes
  and explains why they remain non-blocking migration debt

#### Scenario: New architecture warning appears
- **WHEN** a change introduces a new architecture-maintainability warning or
  broadens an existing one
- **THEN** the change either resolves the warning in the target owner or updates
  the migration debt record with a concrete follow-up boundary

#### Scenario: Migration debt is reduced
- **WHEN** a focused refactor slice resolves a tracked warning
- **THEN** the architecture debt record and validation expectations are updated
  in the same PR so the warning cannot silently reappear

### Requirement: Architecture warning debt is reduced by focused slices
The Apple client SHALL reduce tracked architecture-maintainability warnings
through focused, behavior-preserving refactor slices. Each slice MUST identify
the warning class it resolves, the owner boundary it clarifies, and the
verification commands that protect the moved behavior.

#### Scenario: Focused slice resolves a warning
- **WHEN** a refactor slice removes one or more warnings from
  `check-architecture-maintainability.sh`
- **THEN** the slice updates `clients/apple/ARCHITECTURE.md` with the new
  warning count and removes or narrows the corresponding debt entry

#### Scenario: Slice changes a terminal owner
- **WHEN** a slice moves code from a terminal runtime, host, or surface owner
- **THEN** focused terminal runtime or terminal surface scripts are run in
  addition to the architecture report

#### Scenario: Slice changes a shell controller owner
- **WHEN** a slice moves controller, store, projection, or command-routing code
  out of `ShellHostController.swift`
- **THEN** shell contract validation is run and the shared
  `ShellWorkspaceCommand` vocabulary remains the command boundary

#### Scenario: Slice changes console or mobile owners
- **WHEN** a slice moves code from `Views/Console/ContentView.swift`
- **THEN** the primary macOS shell path remains distinguishable from
  console/mobile surfaces by folder, naming, or project grouping

### Requirement: Architecture validation expectations track reduced debt
The architecture-maintainability gate SHALL keep current warning expectations
aligned with the tracked debt ledger. A PR that resolves a warning MUST update
the report expectations and documentation in the same change so the warning
cannot silently reappear.

#### Scenario: Warning count decreases
- **WHEN** `check-architecture-maintainability.sh` reports fewer warnings than
  the documented debt ledger
- **THEN** the implementation updates the ledger and any script expectations
  before the PR is considered complete

#### Scenario: Warning count does not decrease
- **WHEN** a refactor slice moves architecture code but does not reduce the
  warning count
- **THEN** the PR explains why the moved boundary is an intermediate step and
  leaves the debt ledger accurate

#### Scenario: New or broadened warning appears
- **WHEN** a change introduces a new architecture warning or broadens an
  existing warning while reducing another one
- **THEN** the change either resolves the new warning before merge or records a
  concrete follow-up boundary in the debt ledger

### Requirement: Apple client engineering identity is alan-macos
The Apple client SHALL use `alan-macos` as the active engineering identity for
the macOS app project, scheme, target-facing developer commands, source root,
architecture checks, and script path references.

#### Scenario: Developer builds the macOS app
- **WHEN** a developer reads or runs the documented macOS app build command
- **THEN** the command references `clients/apple/alan-macos.xcodeproj`
- **AND** the selected scheme is `alan-macos`
- **AND** the generated app product is `alan.app`

#### Scenario: Swift app entry is inspected
- **WHEN** a developer inspects the Swift app entry point
- **THEN** the type and file names do not contain `AlanNative`
- **AND** any Swift identifiers that include `alan` use Swift naming
  conventions rather than user-facing brand casing

#### Scenario: Architecture validation runs
- **WHEN** Apple architecture maintainability validation checks source paths,
  README path references, or Xcode project membership
- **THEN** it treats `clients/apple/alan-macos` and the renamed project/scheme
  as the canonical layout
- **AND** it reports active `AlanNative` project or source-root references as
  migration debt or validation failures
