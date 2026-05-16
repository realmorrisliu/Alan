## MODIFIED Requirements

### Requirement: Branding and project identity checks run with Apple validation
Apple-client validation SHALL include focused checks that protect the canonical
`Alan` product brand, `Alan for macOS` platform label, and `alan-macos`
engineering identity.

#### Scenario: Brand scan runs
- **WHEN** Apple-client validation runs for a branding or project rename change
- **THEN** it scans active Apple source, scripts, docs, project metadata, and
  active OpenSpec changes for non-allowlisted `AlanNative`, `Alan Shell`,
  `alanterm`, `dev.alan.native`, `alan.app`, `alan for macOS`, and lowercase
  generated app metadata occurrences
- **AND** it reports the expected canonical replacement for each violation

#### Scenario: Renamed Xcode build runs
- **WHEN** implementation is ready for review
- **THEN** the documented Xcode build command uses
  `clients/apple/alan-macos.xcodeproj`, scheme `alan-macos`, configuration
  `Debug`, destination `platform=macOS`, and the shared derived-data path
- **AND** the build produces `Alan.app`

#### Scenario: Focused scripts are updated
- **WHEN** focused Apple shell scripts are run after the rename
- **THEN** they read source files from `clients/apple/alan-macos`
- **AND** script defaults such as bundle identifiers, capture helpers, and
  architecture checks use the current app identity instead of `AlanNative` or
  `dev.alan.native`
