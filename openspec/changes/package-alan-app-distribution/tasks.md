## 1. Release Assembly Pipeline

- [x] 1.1 Add a release assembly script that builds `cargo build --release -p alan`, builds standalone `alan-tui`, and builds Release `Alan.app` into the shared derived-data path.
- [x] 1.2 Copy the release CLI and standalone TUI into `Alan.app/Contents/Resources/bin/alan` and `Alan.app/Contents/Resources/bin/alan-tui`.
- [x] 1.3 Generate or record package metadata that proves the app, CLI, and TUI were assembled from the same source revision or release version.
- [x] 1.4 Require a configured Developer ID Application signing identity and fail with an actionable message when it is missing.
- [x] 1.5 Sign embedded `alan` and `alan-tui` first, then sign `Alan.app` with hardened runtime and timestamp options.
- [x] 1.6 Add release publication mode that notarizes and staples artifacts intended for Homebrew cask or direct public download.
- [x] 1.7 Add a shared release env loader for allowlisted local signing/notarization settings, including `ALAN_DEVELOPER_ID_APPLICATION` and Apple ID app-specific-password notarization variables.
- [x] 1.8 Add one-command public release helpers: `just release-check` and `just release`.
- [x] 1.9 Add automatic notary keychain profile creation or refresh when `.env` contains complete Apple ID app-specific-password credentials.

## 2. Local Install Workflow

- [x] 2.1 Update `scripts/install.sh` so `just install` uses the release assembly pipeline and installs `Alan.app` into a user-level app directory.
- [x] 2.2 Install or refresh CLI/TUI symlinks in a configurable PATH directory from the same release app bundle, without using `~/.alan/bin`.
- [x] 2.3 Ensure `just install` does not kill, launch, or relaunch `Alan.app`.
- [x] 2.4 Detect a running `Alan.app` during install and print a restart-needed message without terminating the app.
- [x] 2.5 Remove the `just app` recipe from the justfile.
- [x] 2.6 Remove `clients/apple/scripts/run-alan-debug-app.sh` from the supported workflow, and do not add a replacement debug app runner recipe.
- [x] 2.7 Update uninstall behavior to account for the installed app and PATH symlinks without deleting user data under `~/.alan`.

## 3. Direct App Command-line Tool Install

- [x] 3.1 Add app-side command-line tool installer logic that locates embedded CLI/TUI resources in `Contents/Resources/bin`.
- [x] 3.2 Add an explicit macOS app command or menu action that installs or refreshes PATH-visible `alan` and `alan-tui` symlinks when Homebrew has not already provided authoritative binary links.
- [x] 3.3 Refuse to overwrite non-alan-owned files at the target CLI/TUI paths and surface actionable skipped-path diagnostics.
- [x] 3.4 Ensure the direct app installer never writes into Homebrew's prefix, never creates alternate PATH links when Homebrew-managed links already exist, never writes into `~/.alan/bin`, and does not run silently on app launch.

## 4. Homebrew Cask Distribution

- [x] 4.1 Add a Homebrew cask template or tap metadata that installs `Alan.app` from the signed and notarized release artifact.
- [x] 4.2 Link embedded `Alan.app/Contents/Resources/bin/alan` as `alan` through the cask `binary` artifact.
- [x] 4.3 Link embedded `Alan.app/Contents/Resources/bin/alan-tui` as `alan-tui` through the cask `binary` artifact.
- [x] 4.4 Document `brew install --cask alan` as the canonical Homebrew command and only mention `brew install alan` when token ambiguity is not present.
- [x] 4.5 Record architecture and checksum handling for the release artifact so future cask updates are deterministic.

## 5. Documentation And Contract Checks

- [x] 5.1 Update `README.md`, `AGENTS.md`, `clients/tui/README.md`, and `clients/apple/README.md` to describe the app-first install and remove `just app` guidance.
- [x] 5.2 Update `clients/apple/scripts/check-shell-contracts.sh` so it rejects reintroducing `just app` or a default force-kill app runner.
- [x] 5.3 Add focused package-layout validation for Release `Alan.app`, embedded executable paths, executable bits, post-signing manifest SHA-256 checksums, and stale binary detection.
- [x] 5.4 Add focused signature validation that fails on ad-hoc signatures and verifies nested binaries and app bundle signing order outcomes.
- [x] 5.5 Add publication validation for notarization/stapling and Homebrew cask binary links.
- [x] 5.6 Document the private release env loader and its supported local signing/notarization variables.
- [x] 5.7 Document the one-command `just release-check` / `just release` workflow.
- [x] 5.8 Add a tracked `.env.example` for canonical release signing/notarization variables while keeping real `.env` ignored.
- [x] 5.9 Sign the standalone `alan-tui` with the hardened-runtime entitlement required for Bun standalone startup, and validate that entitlement in focused release checks.

## 6. Verification

Local note: full signed assembly/install verification is pending because the
current keychain reports no valid Developer ID signing identities.

- [x] 6.1 Run `just --list` and verify it includes `install` but not `app` or `app-debug-run`.
- [ ] 6.1a Run `just release-check` with complete local signing and notarization credentials.
- [ ] 6.2 Run the release assembly validation with a Developer ID signing identity configured.
- [ ] 6.3 Run `just install` and verify it installs the signed app plus PATH symlinks without killing or launching `Alan.app`.
- [x] 6.4 Verify direct app command-line tool install behavior for missing CLI/TUI entries, existing Homebrew links in any detected prefix, non-alan-owned target files, and no `~/.alan/bin` writes.
- [x] 6.5 Run focused Apple checks including `clients/apple/scripts/check-shell-contracts.sh`, the direct installer test, and a macOS target build.
- [x] 6.6 Validate Homebrew cask metadata locally against the generated release artifact or template.
- [ ] 6.7 Run release app package/signature validation against a signed assembled app.
- [x] 6.8 Run `openspec validate package-alan-app-distribution --strict` after spec or implementation changes.
- [x] 6.9 Run `openspec validate --all --strict` before opening or updating a PR.

## 7. Review And Archive Readiness

- [x] 7.1 Keep `proposal.md`, `design.md`, `tasks.md`, and delta specs aligned if implementation discoveries change the distribution contract.
- [x] 7.2 Prepare PR notes that call out the breaking removal of `just app` and the Developer ID signing requirement.
- [ ] 7.3 Before archiving, sync accepted `alan-app-distribution` requirements into `openspec/specs/alan-app-distribution/spec.md`.
- [ ] 7.4 Before archiving, sync accepted `macos-shell-build-test-contract` delta requirements into `openspec/specs/macos-shell-build-test-contract/spec.md`.
- [ ] 7.5 Before archiving, validate the full OpenSpec tree and confirm the release packaging docs match the implemented scripts.
