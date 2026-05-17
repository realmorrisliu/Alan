## ADDED Requirements

### Requirement: Canonical product brand is Alan
The product SHALL use `Alan` as the canonical standalone user-visible brand
name in app display metadata, docs headings, onboarding text, accessibility
labels, release notes, and visible command labels. Lowercase `alan` SHALL remain
available for CLI commands, package identifiers, dot directories, bundle
identifiers, storage namespaces, and other compatibility-sensitive machine
identifiers.

#### Scenario: Standalone brand is displayed
- **WHEN** a user-visible surface names the product without platform
  disambiguation
- **THEN** the surface renders the product name as `Alan`
- **AND** the surface does not render `alan`, `ALAN`, `AlanNative`, `alanterm`,
  or `alan shell` as the standalone product name

#### Scenario: Command or system identifier is displayed
- **WHEN** docs, help text, scripts, tests, paths, package metadata, or terminal
  output refer to literal command syntax or machine identifiers
- **THEN** they may use lowercase identifiers such as `alan`, `alan-tui`,
  `~/.alan`, `app.alanworks.macos`, and `alan-macos`
- **AND** they do not imply those lowercase identifiers are the app's
  user-visible brand spelling

### Requirement: macOS platform label is Alan for macOS
The native macOS app SHALL use `Alan for macOS` as the platform variant label
when a surface needs to distinguish the macOS app from the CLI, runtime, docs,
or other future clients.

#### Scenario: Platform-specific copy is displayed
- **WHEN** a README, release note, download page, architecture doc, or support
  message distinguishes the native macOS app
- **THEN** it uses `Alan for macOS` as the platform label
- **AND** it does not introduce a second app brand such as `AlanNative` or
  `alanterm`

#### Scenario: Product name is enough
- **WHEN** Dock, app menu, window title, or default app metadata only needs the
  product name
- **THEN** it uses `Alan` instead of `Alan for macOS`

## MODIFIED Requirements

### Requirement: Terminal category is separate from shell command syntax
Alan's user-facing product category SHALL describe the macOS app as a terminal
emulator or terminal workspace. The phrase `alan shell` MUST NOT be used as the
product name, app name, or product category.

#### Scenario: macOS app is described
- **WHEN** docs or UI explain what the native app is
- **THEN** they describe it as a terminal emulator or terminal workspace
- **AND** they do not describe the app as `alan shell`

#### Scenario: CLI syntax is documented
- **WHEN** docs, help text, skills, scripts, or tests refer to the literal
  `alan shell ...` command namespace
- **THEN** that command syntax remains allowed
- **AND** the surrounding copy makes clear it is a command/control namespace,
  not the product or app name

### Requirement: Historical AlanNative identity is removed from active surfaces
The active repository MUST remove `AlanNative` as a product, project, target,
source-root, bundle, logging, or storage identity across source, docs, specs,
project metadata, scripts, generated app metadata, logs, persisted support
paths, and tests.

#### Scenario: Active repository is scanned
- **WHEN** the active repository excluding archived OpenSpec history is scanned
  for `AlanNative`
- **THEN** only explicitly allowlisted historical migration notes or test
  fixtures may match
- **AND** no current path, project file, build command, generated product name,
  source type, log subsystem, or app-support path depends on `AlanNative`

#### Scenario: Local state from old app exists
- **WHEN** local macOS state exists under the historical `AlanNative` support
  path
- **THEN** Alan for macOS performs a best-effort migration or fallback read
  before writing future state only under the new canonical path

### Requirement: Brand validation is explicit and allowlisted
The repository SHALL include a focused brand validation step that rejects
non-allowlisted uses of obsolete product names and incorrectly-cased
user-visible app branding in active surfaces.

#### Scenario: Obsolete brand name is introduced
- **WHEN** a change introduces `AlanNative`, `alanterm`, or `Alan Shell` as an
  active product/app name
- **THEN** brand validation fails with an actionable message naming the
  canonical replacement

#### Scenario: Lowercase app brand is introduced
- **WHEN** a change introduces `alan.app`, `alan for macOS`, or lowercase
  generated app display metadata in an active user-visible app surface
- **THEN** brand validation fails with an actionable message naming `Alan.app`,
  `Alan for macOS`, or `Alan` as the canonical replacement

#### Scenario: Compatibility-sensitive string is present
- **WHEN** a compatibility-sensitive surface contains `alan shell` as literal
  command syntax, lowercase `alan` as a CLI or path identifier, an archive
  contains historical references, or Swift/Rust code uses idiomatic PascalCase
  identifiers that are not user-visible brand copy
- **THEN** brand validation allows the occurrence through an explicit allowlist
  rather than requiring unsafe global replacement

## REMOVED Requirements

### Requirement: Canonical product brand is lowercase alan
**Reason**: The macOS app should follow product-name casing in user-visible app
contexts instead of presenting the CLI command spelling as the brand.
**Migration**: Use `Alan` for user-visible brand surfaces and lowercase `alan`
only for command/system identifiers.

### Requirement: macOS platform label is alan for macOS
**Reason**: The platform label should use the updated user-visible product
brand.
**Migration**: Use `Alan for macOS` in platform-specific copy.
