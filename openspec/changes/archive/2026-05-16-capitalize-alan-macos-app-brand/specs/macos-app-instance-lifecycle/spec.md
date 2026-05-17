## MODIFIED Requirements

### Requirement: Native macOS app identity uses alan for macOS naming
The native macOS app SHALL align bundle, display, singleton, logging, capture,
and persisted support identities with the `Alan` product brand and
`Alan for macOS` platform label, while preserving lowercase command and machine
identifiers where required for compatibility.

#### Scenario: App metadata is generated
- **WHEN** the macOS app bundle is built
- **THEN** the generated app product is `Alan.app`
- **AND** `CFBundleDisplayName` and macOS product name are `Alan`
- **AND** the default bundle identifier is `app.alanworks.macos`

#### Scenario: Singleton and support paths are created
- **WHEN** singleton lock files or App Support persistence paths are created
- **THEN** they use the current Alan for macOS identity
- **AND** they do not create new paths named `AlanNative`

#### Scenario: Logs and capture helpers identify the app
- **WHEN** maintainers inspect logs, run capture helpers, or filter running app
  instances by bundle identifier
- **THEN** defaults use `app.alanworks.macos` and compatibility-safe lowercase
  command/system identifiers where paths or process names require them
- **AND** `com.realmorrisliu.AlanNative` and `dev.alan.native` are not active
  defaults
