## Context

The previous branding pass intentionally made `alan` the canonical product
brand everywhere, including the macOS bundle name and display metadata. That
kept CLI and app naming consistent, but it makes the native app look like a
command-line artifact in Finder, Dock, menu bar, and install flows.

The new distinction is:

- `Alan` is the user-visible product brand.
- `Alan for macOS` is the platform label when disambiguation is useful.
- `alan` remains the command/system identifier for CLI names, dot directories,
  package names, bundle identifiers, and embedded command-line binaries.

## Goals / Non-Goals

**Goals:**

- Make the generated macOS app bundle and display metadata read as `Alan.app`
  and `Alan`.
- Update release/install/Homebrew paths that refer to the app bundle while
  preserving embedded `Contents/Resources/bin/alan` and `alan-tui` CLI paths.
- Update OpenSpec contracts and active distribution artifacts so future work
  does not reintroduce lowercase app branding.
- Update brand validation to distinguish user-visible app casing from
  lowercase compatibility identifiers.

**Non-Goals:**

- Rename the Rust CLI binary, `alan-tui`, crate names, package names, or
  `~/.alan` data paths.
- Change the bundle identifier `app.alanworks.macos`.
- Rebrand historical archive records.
- Redesign app UI, app icon, release archive naming, or signing/notarization
  behavior beyond path updates required by `Alan.app`.

## Decisions

### User-visible product brand

Use `Alan` in app metadata, app menu, Dock/Finder names, headings, and visible
copy. This follows macOS convention for app names while retaining the simple
human-name brand. The lowercase form remains appropriate for shell commands and
machine identifiers where users expect command syntax.

Alternatives considered:

- Keep `alan` everywhere: internally consistent, but the app reads as a CLI
  artifact and feels less native in macOS app contexts.
- Use `Alan for macOS` as the bundle name: explicit, but too long for Dock/menu
  use and unnecessary when the product name alone is enough.

### Bundle and executable layout

Set the Xcode macOS product/display name to `Alan`, producing `Alan.app`. The
app bundle's internal macOS executable may follow Xcode product naming, while
the embedded CLI/TUI executables remain lowercase under
`Contents/Resources/bin/alan` and `Contents/Resources/bin/alan-tui`.

### Validation boundary

Brand validation should no longer reject every `Alan` occurrence. It should
reject obsolete identities (`AlanNative`, `Alan Shell`, `alanterm`,
`dev.alan.native`) and lowercase app/platform branding in active user-visible
surfaces, especially `alan.app`, `alan for macOS`, and lowercase generated app
metadata.

## Risks / Trade-offs

- App path churn breaks install scripts -> Update release assembly, install,
  uninstall, validation, Homebrew, and command-line link ownership checks
  together.
- Lowercase command identifiers get overcorrected -> Keep explicit tasks and
  validation text preserving embedded CLI paths and `~/.alan`.
- Active OpenSpec drift -> Update active distribution/change specs that still
  say `alan.app` before marking this change complete.
