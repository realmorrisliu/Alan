## Why

The native macOS app currently presents itself as `alan`, which matches the CLI
binary but looks out of place in Finder, Dock, menu bar, and installer contexts
where app names normally read as product names. The brand contract should
distinguish the user-visible macOS product name from lowercase command and
system identifiers.

## What Changes

- Use `Alan` as the canonical user-visible product brand in app metadata,
  menu/window copy, docs headings, onboarding text, release notes, and visible
  command labels.
- Use `Alan for macOS` when platform disambiguation is needed.
- Keep lowercase `alan` for CLI commands, embedded CLI/TUI executable names,
  dot directories, package names, storage namespaces, bundle identifiers, and
  other compatibility-sensitive machine identifiers.
- Update the generated macOS app product/display metadata from `alan.app` /
  `alan` to `Alan.app` / `Alan`.
- Update brand validation so uppercase `Alan` is required in user-visible brand
  surfaces and lowercase `alan` remains allowed only for command/system
  contexts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `product-brand-identity`: Change the canonical user-visible standalone brand
  from lowercase `alan` to `Alan`, while preserving lowercase machine
  identifiers.
- `macos-app-instance-lifecycle`: Change generated macOS app metadata to
  `Alan.app` and `CFBundleDisplayName = Alan`.
- `macos-shell-build-test-contract`: Update build and validation expectations
  for the capitalized macOS app bundle and display name.
- `macos-shell-ui-ux-conformance`: Update visible UI copy expectations so
  app chrome uses `Alan` and `Alan for macOS`.
- `macos-app-architecture-maintainability`: Update engineering identity
  validation so the generated product is `Alan.app` while the project/scheme
  remain `alan-macos`.

## Impact

- OpenSpec long-lived specs and active changes that reference `alan.app`,
  lowercase-only app metadata, or lowercase-only platform labels.
- Xcode project metadata for the macOS product name and bundle display name.
- Release, install, uninstall, validation, and Homebrew cask helper scripts that
  point at the built app bundle.
- Brand validation rules and focused tests for command-line tool installation.
