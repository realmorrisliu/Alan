## Context

The public naming direction is now clear:

- `alan` is the canonical product brand and is intentionally lowercase.
- `alan for macOS` is the native macOS platform variant label.
- The app category is a terminal emulator or terminal workspace, not a shell.

The current repository predates that distinction. User-facing docs and UI copy
still use `Alan`, `Alan Shell`, `Ask Alan`, and `Open in Alan`; the Apple client
also carries `AlanNative` through the Xcode project, target, scheme, source
root, generated app reference, Swift app entry type, script paths, logging
subsystem, singleton support paths, and App Support storage. The rename is
therefore not a safe global search/replace: the implementation must distinguish
brand copy, compatibility-sensitive command syntax, Swift/Xcode identifiers,
and persisted local state.

## Goals / Non-Goals

**Goals:**

- Make `alan` the only standalone public product name.
- Use `alan for macOS` wherever platform disambiguation is needed.
- Remove `AlanNative` from active source, Xcode project metadata, scripts,
  current docs, active OpenSpec changes, and generated macOS app identity.
- Rename the Apple client engineering surface to a lowercase macOS-specific
  identity, with `clients/apple/alan-macos`, `alan-macos.xcodeproj`, and an
  `alan-macos` scheme as the target direction.
- Keep the shipped app product/display name as `alan` and align the default
  bundle identifier to the selected primary domain, with
  `app.alanworks.macos` as the target bundle id.
- Treat `alanworks.app` as the primary public domain for this change. `alan.now`
  may be reserved later as a short action-oriented entry point, but it is not
  the root for app identifiers.
- Add validation so future changes do not reintroduce `AlanNative`, `Alan`
  brand copy, or `Alan Shell` product language in active surfaces.

**Non-Goals:**

- Rename the Rust CLI binary from `alan`.
- Remove or redesign the existing `alan shell ...` CLI namespace in this change.
  That namespace remains allowed when it is literal command syntax or an IPC
  control-plane surface.
- Rewrite archived OpenSpec history only to remove historical references.
- Force non-user-facing Swift/Rust identifiers to be lowercase when language
  conventions require PascalCase or other casing. The ban is on `AlanNative`
  and on user-facing brand casing, not on idiomatic type names such as
  `AlanApp`.
- Redesign terminal UX, command behavior, or agent/session semantics.

## Decisions

### Brand and platform labels

Use `alan` for standalone product references, including app display name, menu
bar name, window title, onboarding copy, docs headlines, accessibility labels,
and visible command labels. Use `alan for macOS` for download, platform,
README, release, and architecture copy that needs to distinguish the native app
from the CLI/runtime.

Alternatives considered:

- `alanterm`: clearer terminal category, but it creates a second product name
  and makes the product sound like a narrow terminal utility.
- `alan shell`: matches existing internal vocabulary, but developer users will
  read it as a command shell like `zsh` or `fish`, which is inaccurate for a
  terminal emulator.
- `Alan`: already present, but conflicts with the desired lowercase brand.

### Apple engineering identity

Use `alan-macos` for the Apple client project path, project file, scheme, and
target-facing developer commands. This keeps the engineering name lowercase and
platform-specific without inventing a second public brand.

Swift symbols should move away from `AlanNative*` to concise app-owned names
such as `AlanApp`, `AlanMacAppDelegate`, or existing owner-specific names. This
preserves Swift naming conventions while removing the legacy `Native` concept.

### Product, bundle, and persisted identity

The built product should be `alan.app`, with `CFBundleDisplayName` and macOS
product name set to `alan`. The bundle identifier should move from
`dev.alan.native` to `app.alanworks.macos` so local automation, singleton
detection, and capture scripts use the selected `alanworks.app` domain rather
than the obsolete `native` label.

Local support paths that currently use `AlanNative` should migrate to `alan` or
`alan-macos` owned paths. If old state exists, the implementation should attempt
a one-time best-effort migration or fallback read before writing to the new
location, then continue using only the new path.

### Compatibility exceptions

The literal command namespace `alan shell` remains allowed in command examples,
CLI help, agent-facing skills, scripts, and protocol docs when it names the CLI
subcommand or control-plane API. It must not be presented as the product name or
app name. The implementation should use allowlisted scans rather than a blind
ban on every `alan shell` string.

Historical archive directories may retain old names as immutable history. Active
source, active OpenSpec changes, long-lived specs, top-level docs, and generated
macOS product metadata should not.

## Risks / Trade-offs

- **Xcode rename churn** -> Keep the change scoped to one Apple project rename
  pass and verify project membership, scripts, and README together.
- **Persisted local state loss** -> Add a migration/fallback path for old
  `AlanNative` App Support and singleton state before writing only new paths.
- **Overbroad lowercase replacement** -> Use a classification pass and allowlist
  language identifiers, command syntax, historical archives, and proper nouns in
  external package names.
- **Active OpenSpec drift** -> Update long-lived specs and active change tasks
  that refer to the old build command so future implementation work uses the
  new project/scheme names.
- **CLI namespace confusion remains** -> Keep `alan shell` allowed only as
  command syntax and make docs describe the macOS product as a terminal emulator
  or terminal workspace.

## Migration Plan

1. Classify all active `Alan`, `AlanNative`, `Alan Shell`, `Ask Alan`,
   `Open in Alan`, `dev.alan.native`, and `com.realmorrisliu.AlanNative`
   occurrences by surface: user-visible copy, Apple project identity, runtime
   metadata, script path, command syntax, test fixture, or historical archive.
2. Rename the Apple source/project surface from `AlanNative` to `alan-macos`,
   including Xcode project path, group path, scheme/target references, build
   scripts, architecture checks, README commands, and active OpenSpec build
   tasks.
3. Update the product metadata to emit `alan.app`, display name `alan`, and
   bundle id `app.alanworks.macos`.
4. Replace public copy with `alan` / `alan for macOS` language and describe the
   app category as terminal emulator or terminal workspace.
5. Add or update brand/project scans so active surfaces reject `AlanNative` and
   non-allowlisted uppercase brand copy.
6. Verify with brand scans, Apple shell contract checks, architecture checks,
   focused Swift scripts, and the renamed Xcode build command.

## Open Questions

- Should `TERM_PROGRAM` report `alan` or a platform-specific value such as
  `alan-macos`? The default recommendation is `alan`, because terminal programs
  usually expose product identity rather than platform labels.
- Should the old bundle identifier keep a temporary compatibility alias for
  automation scripts? The default recommendation is no for active defaults, with
  script flags available for historical captures if needed.
