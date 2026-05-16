## Context

The current repository has two separate local workflows:

- `just install` builds the release Rust CLI and standalone TUI, then installs
  them under `~/.alan/bin`.
- `just app` runs `clients/apple/scripts/run-alan-debug-app.sh`, which builds a
  Debug `alan.app`, kills any running app process, waits for singleton release,
  and launches the new build.

That split is useful for early UI iteration, but it is the wrong shape for
distribution. The desired long-term model is app-first: users install
`alan.app`, and the CLI/TUI come from that same signed release package. Homebrew
cask can install the app and expose binaries from inside the app bundle, so the
release artifact does not need a separate formula or a runtime download step.

The project owner has a formal Developer ID signing identity, so this change
should not preserve ad-hoc signing as a supported local install fallback.

## Goals / Non-Goals

**Goals:**

- Make `alan.app` the primary macOS distribution artifact.
- Embed release `alan` and `alan-tui` executables in
  `alan.app/Contents/Resources/bin/`.
- Sign nested CLI/TUI binaries and the app bundle with Developer ID signing.
- Require notarization and stapling for published Homebrew/download artifacts.
- Make `just install` install the same release-shaped app/CLI/TUI package
  locally without killing or launching the app.
- Remove `~/.alan/bin` from the supported CLI/TUI distribution model.
- Remove `just app` entirely, without adding `just app-debug-run` or another
  debug-run replacement.
- Define the future Homebrew cask contract that installs the app and links the
  embedded CLI/TUI binaries.
- Provide an explicit direct-app command-line tools install action for users who
  download or drag-install `alan.app` without Homebrew.

**Non-Goals:**

- Implement auto-updates outside Homebrew.
- Ship a separate CLI formula or separate CLI/TUI release artifact.
- Add a runtime network downloader that fetches CLI/TUI from the app.
- Automatically mutate shell startup files or silently install command-line
  tools on app launch.
- Continue using `~/.alan/bin` as a PATH location, fallback, or compatibility
  install target.
- Preserve the Debug app runner as a documented local workflow.
- Change the daemon/session runtime behavior or the macOS shell UI itself.

## Decisions

1. **Use app-first packaging with embedded CLI/TUI.**

   The release assembly will build `target/release/alan`, the standalone
   `alan-tui`, and Release `alan.app`, then copy the CLI/TUI into
   `alan.app/Contents/Resources/bin/`.

   Alternative considered: publish separate app and CLI artifacts. That would
   force Homebrew users through a cask+formula coordination problem and create
   version skew between the app and CLI.

2. **Sign the assembled bundle, not just the Xcode product.**

   The assembly script should inject the CLI/TUI resources before signing.
   Nested binaries are signed first with the configured Developer ID
   Application identity, then the app bundle is signed with hardened runtime and
   timestamp options. Published artifacts are notarized and stapled after
   packaging.

   Alternative considered: rely on the Xcode project to sign the app before
   embedding resources. That produces an invalid final signature unless the
   nested resources are known to Xcode and signed in the correct order.

3. **Make signing configuration explicit and fail closed.**

   Local install and release packaging should require a Developer ID signing
   identity, supplied by a documented environment variable or project-local
   release config. If the identity is missing, the script fails with an
   actionable message. Ad-hoc signing is not a supported fallback for this
   change.

   The implementation should centralize local private release environment
   loading so `just install` and release assembly share the same inputs.
   Explicit environment variables take precedence; otherwise scripts may load
   only allowlisted signing/notarization assignments from `ALAN_RELEASE_ENV_FILE`
   or release-specific local env files such as `.env.release.local`,
   `.release.env.local`, `.env.local`, `.env`, or `~/.alan/release.env`. The
   env loader must not execute arbitrary shell code from those files.

   Notarization credentials can be optional for `just install` so local rebuilds
   do not upload every iteration to Apple, but they are mandatory for any
   publish/release artifact intended for Homebrew or direct download.

   The public release workflow should be one-command after `.env` is configured:
   `just release-check` validates and, when credentials are available, creates
   or refreshes the notary keychain profile; `just release` builds, signs,
   notarizes, staples, and archives the artifact. `just install` remains local
   and signed but does not notarize. The repository should include a tracked
   `.env.example` template while keeping real `.env` values ignored. The only
   supported automated notarization setup path is Apple ID app-specific password
   credentials stored into the notary keychain profile; direct App Store Connect
   API-key submission is intentionally out of scope.

4. **Use Homebrew cask binary links from the app bundle.**

   The cask should install `alan.app` and expose:

   ```ruby
   app "alan.app"
   binary "#{appdir}/alan.app/Contents/Resources/bin/alan", target: "alan"
   binary "#{appdir}/alan.app/Contents/Resources/bin/alan-tui", target: "alan-tui"
   ```

   Documentation should use `brew install --cask alan` as the canonical command.
   `brew install alan` may be documented only when the selected tap has no
   formula/cask token ambiguity.

5. **Keep direct app installs useful without Homebrew.**

   The app bundle should include an explicit command-line tools install action,
   such as a menu command, that creates `alan` and `alan-tui` symlinks from the
   embedded resources into a user-visible command directory. The default direct
   install target should be a conventional PATH directory such as
   `/usr/local/bin`, with an override for local development or nonstandard
   machines. The app must not silently install tools on launch, mutate shell
   startup files, or use `~/.alan/bin`.

   For Homebrew installs, the cask-managed binary links are authoritative. The
   direct-app installer must not write into Homebrew's prefix and must not create
   alternate links in a different PATH directory when an existing Homebrew prefix
   already exposes alan command-line links.

6. **Turn `just install` into the local release install path.**

   `just install` should call the release assembly/install script, install the
   signed app into a user-level app directory such as `~/Applications/alan.app`,
   and install or refresh CLI/TUI symlinks in a configurable PATH directory. It
   must not write to `~/.alan/bin`, kill, restart, or open the running app. If
   the app is running, it may warn that the new version takes effect after the
   user restarts it.

   Alternative considered: keep `just app` as a separate debug command. That
   preserves the current force-restart behavior and keeps developers on a path
   that diverges from real distribution, so this change removes it instead.

## Risks / Trade-offs

- **Risk: Developer ID signing slows local iteration.** -> Keep `just install`
  release-shaped and signed, while normal code/test loops still use focused
  build/test commands that do not install.
- **Risk: notarization makes every local install slow.** -> Require notarization
  for publish artifacts, but allow local `just install` to skip notarization
  while still requiring Developer ID signing.
- **Risk: direct app command-line install conflicts with Homebrew-managed
  binaries.** -> Treat Homebrew prefix links as externally managed, require an
  explicit install action, avoid writing into Homebrew's prefix from the app,
  and skip direct-install symlink creation when Homebrew-managed links already
  exist in another prefix.
- **Risk: `/usr/local/bin` is not writable on every machine.** -> The installer
  reports the exact target and provides an override or manual command instead
  of falling back to `~/.alan/bin`.
- **Risk: replacing a running app bundle surprises users.** -> The install
  script never kills or launches the app and emits an explicit restart-needed
  warning when a running instance is detected.
- **Risk: the app and CLI versions drift.** -> The package should include
  version metadata or a manifest generated during assembly and used by
  verification to prove the app, CLI, and TUI came from the same source revision
  and version, with manifest SHA-256 values recorded after embedded binary
  signing and compared to the delivered embedded binaries.

## Migration Plan

1. Add a release assembly script that builds CLI/TUI/app, embeds CLI/TUI, signs
   nested binaries and app, and can optionally notarize/staple for publication.
2. Update `scripts/install.sh` and `just install` to use the release assembly
   path and install without killing or launching `alan.app`.
3. Remove the `just app` recipe and the debug-runner script from the supported
   workflow. Update contract checks and docs so it is not reintroduced.
4. Add app-side explicit CLI/TUI command-line tools install behavior for direct
   app installs.
5. Add Homebrew cask metadata or a template that installs `alan.app` and links
   the embedded binaries.
6. Validate signing, nested binary layout, local install behavior, Homebrew cask
   structure, and docs/contract checks before implementation is considered
   complete.

Rollback is straightforward before publishing: restore the previous
`scripts/install.sh` and justfile behavior. After a public cask release, rollback
requires publishing a corrected versioned artifact and cask checksum rather than
mutating an already-published download.

## Open Questions

None.
