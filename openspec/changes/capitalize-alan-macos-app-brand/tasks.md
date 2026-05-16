## 1. Brand Metadata And Distribution Paths

- [x] 1.1 Update Xcode macOS app metadata so the built app is `Alan.app` with `CFBundleDisplayName` and product name `Alan`.
- [x] 1.2 Update release assembly, install, uninstall, validation, and Homebrew cask helper scripts to use `Alan.app` while preserving embedded CLI paths under `Contents/Resources/bin/alan` and `alan-tui`.
- [x] 1.3 Update Apple command-line tool installer ownership checks and focused tests for the capitalized app bundle path.

## 2. OpenSpec And Docs Alignment

- [x] 2.1 Update active OpenSpec distribution artifacts and user-facing docs that still present the macOS app as `alan.app` or `alan for macOS`.
- [x] 2.2 Preserve lowercase `alan` in CLI syntax, embedded binary paths, package names, bundle identifiers, and storage paths.

## 3. Validation

- [x] 3.1 Update brand validation so it rejects obsolete identities and lowercase app/platform branding, while allowing lowercase command/system identifiers.
- [x] 3.2 Run focused validation for brand identity, release/homebrew scripts, command-line installer tests, and OpenSpec strict validation.
