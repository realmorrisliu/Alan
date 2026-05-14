## MODIFIED Requirements

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

## ADDED Requirements

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
