# product-brand-identity Specification

## Purpose
TBD - created by archiving change normalize-alan-branding-and-macos-app-name. Update Purpose after archive.
## Requirements
### Requirement: Canonical product brand is lowercase alan
The product SHALL use `alan` as the canonical standalone brand name in
user-facing copy, app display metadata, docs headings, onboarding text,
accessibility labels, release notes, and visible command labels.

#### Scenario: Standalone brand is displayed
- **WHEN** a user-visible surface names the product without platform
  disambiguation
- **THEN** the surface renders the product name as `alan`
- **AND** the surface does not render `Alan`, `ALAN`, `AlanNative`, `alanterm`,
  or `alan shell` as the standalone product name

#### Scenario: Sentence or title starts with brand
- **WHEN** a sentence, title, menu item, or accessibility label starts with the
  product brand
- **THEN** the brand remains `alan` rather than being title-cased for grammar or
  platform convention

### Requirement: macOS platform label is alan for macOS
The native macOS app SHALL use `alan for macOS` as the platform variant label
when a surface needs to distinguish the macOS app from the CLI, runtime, docs,
or other future clients.

#### Scenario: Platform-specific copy is displayed
- **WHEN** a README, release note, download page, architecture doc, or support
  message distinguishes the native macOS app
- **THEN** it uses `alan for macOS` as the platform label
- **AND** it does not introduce a second app brand such as `AlanNative` or
  `alanterm`

#### Scenario: Product name is enough
- **WHEN** Dock, app menu, window title, or default app metadata only needs the
  product name
- **THEN** it uses `alan` instead of `alan for macOS`

### Requirement: Primary public domain is alanworks.app
The product SHALL use `alanworks.app` as the primary public domain for this
branding pass. Short domains such as `alan.now` MAY be reserved for future
action-oriented entry points, but they MUST NOT drive macOS app identifiers in
this change.

#### Scenario: macOS app identifier is derived
- **WHEN** the macOS app bundle identifier or local automation defaults need a
  reverse-DNS identity
- **THEN** they use the selected primary domain as `app.alanworks.macos`
- **AND** they do not use `dev.alan.macos`, `dev.alan.native`, or
  `com.realmorrisliu.AlanNative`

### Requirement: Terminal category is separate from shell command syntax
alan's user-facing product category SHALL describe the macOS app as a terminal
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
- **THEN** alan for macOS performs a best-effort migration or fallback read
  before writing future state only under the new canonical path

### Requirement: Brand validation is explicit and allowlisted
The repository SHALL include a focused brand validation step that rejects
non-allowlisted uses of obsolete or incorrectly-cased product names in active
surfaces.

#### Scenario: Obsolete brand name is introduced
- **WHEN** a change introduces `AlanNative`, `alanterm`, or `Alan Shell` as an
  active product/app name
- **THEN** brand validation fails with an actionable message naming the
  canonical replacement

#### Scenario: Compatibility-sensitive string is present
- **WHEN** a compatibility-sensitive surface contains `alan shell` as literal
  command syntax, an archive contains historical references, or Swift/Rust code
  uses idiomatic PascalCase identifiers that are not user-visible brand copy
- **THEN** brand validation allows the occurrence through an explicit allowlist
  rather than requiring unsafe global replacement

