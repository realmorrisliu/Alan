## ADDED Requirements

### Requirement: Release installation replaces the debug app runner
The Apple client build/test contract SHALL treat release-shaped installation as
the supported local app workflow. The repository MUST NOT require or preserve a
`just app` workflow that force-kills and relaunches the macOS app.

#### Scenario: Local app workflow is validated
- **WHEN** Apple client workflow checks inspect the justfile and app scripts
- **THEN** they verify `just install` is the documented local app installation path
- **AND** they verify the justfile does not expose a recipe named `app`
- **AND** they verify the justfile does not expose a replacement debug app runner recipe for the same force-rebuild-and-launch workflow

#### Scenario: Legacy debug runner is removed
- **WHEN** Apple client contract checks inspect app runner scripts
- **THEN** they do not require `clients/apple/scripts/run-alan-debug-app.sh` as the supported app workflow
- **AND** they fail if a default local app workflow kills a running `alan.app` process and immediately relaunches it

### Requirement: Release packaging has focused validation
The Apple client SHALL provide focused validation for the release app package,
embedded CLI/TUI binaries, Developer ID signatures, and publication readiness
when distribution packaging changes.

#### Scenario: Release app layout is checked
- **WHEN** release packaging implementation is ready for review
- **THEN** focused checks verify `alan.app` was built in Release configuration
- **AND** focused checks verify embedded `Contents/Resources/bin/alan` and `Contents/Resources/bin/alan-tui` exist and are executable
- **AND** focused checks verify the embedded binaries are the release binaries from the current build

#### Scenario: Signatures are checked
- **WHEN** release packaging implementation is ready for review
- **THEN** focused checks verify the embedded CLI and TUI are signed with the configured Developer ID Application identity
- **AND** focused checks verify the embedded TUI includes the hardened-runtime entitlement required by its standalone runtime
- **AND** focused checks verify the app bundle is signed after embedded binaries are in place
- **AND** focused checks fail if ad-hoc signatures are used for local install or release artifacts

#### Scenario: Publication readiness is checked
- **WHEN** an artifact is intended for Homebrew cask or direct public download
- **THEN** focused checks verify notarization and stapling completed successfully
- **AND** focused checks verify the cask metadata links the embedded CLI and TUI from the installed app bundle
