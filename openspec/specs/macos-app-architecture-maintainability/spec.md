# macos-app-architecture-maintainability Specification

## Purpose
Define maintainable native Apple client source organization, SwiftUI/AppKit
boundaries, service/model ownership, and validation expectations for macOS app
architecture changes.

## Requirements
### Requirement: Apple client source layout mirrors architecture ownership
The native Apple client SHALL organize source files by durable responsibility so
developers can distinguish app startup, SwiftUI views, models, controllers,
services, and support code without reading every file in a flat directory.

#### Scenario: Source tree is inspected
- **WHEN** a developer inspects `clients/apple/AlanNative`
- **THEN** app entry code, shell views, console views, protocol models,
  observable controllers, service/IO code, and support utilities are grouped by
  responsibility rather than mixed in one flat source directory

#### Scenario: README documents source layout
- **WHEN** the Apple client README describes the directory structure
- **THEN** the documented folders match the source tree and Xcode project
  organization used by the current code

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
stable end state. When a file remains large during migration, the owning change
SHALL document the intended split and avoid adding unrelated responsibilities to
that file.

#### Scenario: Large file receives new behavior
- **WHEN** a developer adds behavior to an existing large Apple client file
- **THEN** the change either places the behavior in the target owner file or
  documents why the temporary location is still compatible with the migration
  plan

#### Scenario: Refactor slice completes
- **WHEN** a behavior-preserving architecture refactor slice is completed
- **THEN** the resulting file ownership makes future changes narrower to review
  than the previous large-file organization
