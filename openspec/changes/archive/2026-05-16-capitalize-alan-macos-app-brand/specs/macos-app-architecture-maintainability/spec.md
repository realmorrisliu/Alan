## MODIFIED Requirements

### Requirement: Apple client engineering identity is alan-macos
The Apple client SHALL use `alan-macos` as the active engineering identity for
the macOS app project, scheme, target-facing developer commands, source root,
architecture checks, and script path references.

#### Scenario: Developer builds the macOS app
- **WHEN** a developer reads or runs the documented macOS app build command
- **THEN** the command references `clients/apple/alan-macos.xcodeproj`
- **AND** the selected scheme is `alan-macos`
- **AND** the generated app product is `Alan.app`

#### Scenario: Swift app entry is inspected
- **WHEN** a developer inspects the Swift app entry point
- **THEN** the type and file names do not contain `AlanNative`
- **AND** any Swift identifiers that include `Alan` use Swift naming
  conventions rather than command-facing lowercase casing

#### Scenario: Architecture validation runs
- **WHEN** Apple-client architecture validation scans source layout,
  README/build commands, scripts, project metadata, and active OpenSpec work
- **THEN** it recognizes `alan-macos` as the engineering identity and `Alan` as
  the user-visible product brand
- **AND** it rejects reintroduced `AlanNative` project, path, scheme, or target
  identity unless the occurrence is an explicit compatibility or migration
  fixture
