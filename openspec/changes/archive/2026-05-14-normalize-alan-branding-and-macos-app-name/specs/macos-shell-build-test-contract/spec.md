## MODIFIED Requirements

### Requirement: Build requirements match documentation
The Apple client SHALL keep documented system requirements, deployment targets,
project settings, project naming, scheme naming, and build commands aligned
with the active `alan for macOS` engineering identity.

#### Scenario: Deployment target changes
- **WHEN** the Xcode project deployment targets are changed
- **THEN** `clients/apple/README.md` and relevant specs are updated in the same
  change

#### Scenario: Documented build command
- **WHEN** a developer runs the documented macOS build command after preparing
  dependencies
- **THEN** the command succeeds or fails with documented, actionable dependency
  setup instructions

#### Scenario: Project or scheme name changes
- **WHEN** the Apple project, source root, target, scheme, or generated product
  name changes
- **THEN** `clients/apple/README.md`, architecture docs, focused scripts, active
  OpenSpec tasks, and Xcode project settings are updated in the same change
- **AND** no active documented build command references `AlanNative`

## ADDED Requirements

### Requirement: Branding and project identity checks run with Apple validation
Apple-client validation SHALL include focused checks that protect the canonical
`alan` product brand, `alan for macOS` platform label, and `alan-macos`
engineering identity.

#### Scenario: Brand scan runs
- **WHEN** Apple-client validation runs for a branding or project rename change
- **THEN** it scans active Apple source, scripts, docs, project metadata, and
  active OpenSpec changes for non-allowlisted `Alan`, `AlanNative`,
  `Alan Shell`, `alanterm`, and `dev.alan.native` occurrences
- **AND** it reports the expected canonical replacement for each violation

#### Scenario: Renamed Xcode build runs
- **WHEN** implementation is ready for review
- **THEN** the documented Xcode build command uses
  `clients/apple/alan-macos.xcodeproj`, scheme `alan-macos`, configuration
  `Debug`, destination `platform=macOS`, and the shared derived-data path
- **AND** the build produces `alan.app`

#### Scenario: Focused scripts are updated
- **WHEN** focused Apple shell scripts are run after the rename
- **THEN** they read source files from `clients/apple/alan-macos`
- **AND** script defaults such as bundle identifiers, capture helpers, and
  architecture checks use the new app identity instead of `AlanNative` or
  `dev.alan.native`
