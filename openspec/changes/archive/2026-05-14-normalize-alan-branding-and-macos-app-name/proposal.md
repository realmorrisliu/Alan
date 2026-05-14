## Why

alan currently mixes the public product name, macOS platform label, shell
terminology, and the historical `AlanNative` engineering name across docs,
Swift/Xcode metadata, scripts, runtime strings, and OpenSpec contracts. This
creates unclear branding and makes the macOS app look like an implementation
prototype rather than the native `alan` product.

## What Changes

- Establish `alan` as the canonical lowercase product brand.
- Establish `alan for macOS` as the platform variant label for the native macOS
  app and related marketing/developer docs.
- Define `terminal emulator` / `terminal workspace` as the user-facing product
  category and avoid using `alan shell` as a product or app name.
- **BREAKING**: Remove the `AlanNative` Apple client identity from project,
  scheme, target, source-root, generated app bundle, logs, scripts, docs, and
  persisted support paths.
- Keep compatibility-sensitive CLI namespaces such as `alan shell` only where
  they are command syntax or protocol-facing control-plane names, not brand
  copy.
- Preserve language-specific identifier conventions where required, such as
  PascalCase Swift type names, while avoiding `AlanNative` as an identifier.

## Capabilities

### New Capabilities

- `product-brand-identity`: Defines canonical product names, platform labels,
  capitalization rules, prohibited aliases, and scanning expectations for
  user-facing and compatibility-sensitive surfaces.

### Modified Capabilities

- `macos-app-architecture-maintainability`: Update the accepted Apple client
  source/project identity so the source root, Xcode project/scheme/target, and
  app-entry naming no longer use `AlanNative`.
- `macos-shell-build-test-contract`: Update macOS build/test/documentation
  expectations and validation scripts to use the renamed project, scheme,
  product, and source paths.
- `macos-shell-ui-ux-conformance`: Update default UI language expectations so
  visible copy uses lowercase `alan` branding and does not present `alan shell`
  as the app/product name.
- `macos-app-instance-lifecycle`: Update app bundle, singleton, launch, and
  persisted app-support naming expectations to align with the `alan for macOS`
  app identity.

## Impact

- Apple client project files, source paths, Swift entry types, bundle/product
  metadata, scripts, and docs under `clients/apple/`.
- User-facing app text such as menu items, command labels, accessibility labels,
  onboarding, README copy, and macOS window/app names.
- Runtime-visible metadata and diagnostics such as `TERM_PROGRAM`, logging
  subsystem names, App Support subdirectories, singleton lock naming, and shell
  control error text.
- OpenSpec long-lived specs and active changes that currently reference
  `AlanNative`, `Alan`, `Alan Shell`, `Ask Alan`, `Open in Alan`, or old
  `AlanNative` build commands.
- Potential local migration for old app support state under the historical
  `AlanNative` path.
