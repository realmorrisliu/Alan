## 1. Singleton Process Guard

- [ ] 1.1 Add a macOS-only `AlanAppSingletonGuard` that acquires an OS-backed exclusive lock under Application Support and holds the lock for the app process lifetime.
- [ ] 1.2 Wire the singleton guard into early macOS app startup before SwiftUI scenes, shell window contexts, control sockets, or terminal runtimes are created.
- [ ] 1.3 On duplicate-process startup, activate the existing Alan app by bundle identifier and terminate the duplicate process before it creates shell state.
- [ ] 1.4 Add focused Swift tests for first acquisition, rejected second acquisition, release, and owner-exit/stale-lock recovery.

## 2. Single Primary Window

- [ ] 2.1 Replace the macOS main `WindowGroup("Alan")` scene with a unique primary `Window("Alan", id: "main")` while keeping iOS scene behavior unchanged.
- [ ] 2.2 Move the primary `ShellWindowContext` and `ShellHostController` ownership to app-process scope and inject or reference that owner from `MacShellRootView`.
- [ ] 2.3 Replace the standard New Window command and `Command-N` path so it focuses or reopens the existing primary shell window instead of creating another window.
- [ ] 2.4 Handle Dock/application reopen and activation so a running Alan app presents or focuses one primary shell window.

## 3. Shell Control-Plane Alignment

- [ ] 3.1 Update control-plane contract checks and docs that describe each macOS window as an independent shell context.
- [ ] 3.2 Ensure duplicate window and duplicate process paths do not create additional `window_id`s, control directories, socket paths, persisted state files, event streams, or terminal runtime registries.
- [ ] 3.3 Keep singleton lifecycle diagnostics debug-facing and avoid adding default UI implementation jargon.

## 4. Verification

- [ ] 4.1 Run focused singleton guard tests.
- [ ] 4.2 Run `clients/apple/scripts/test-terminal-runtime-service.sh`.
- [ ] 4.3 Run `clients/apple/scripts/test-terminal-surface-controller.sh`.
- [ ] 4.4 Run `clients/apple/scripts/test-shell-runtime-metadata.sh`.
- [ ] 4.5 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 4.6 Build the macOS app with the documented `AlanNative` xcodebuild command.
- [ ] 4.7 Manually verify initial launch, repeated `open`, forced `open -n`, `Command-N`, Dock reopen, close/reopen, and `Command-Q` lock release.
- [ ] 4.8 Run `git diff --check`.
- [ ] 4.9 Run `openspec validate enforce-single-macos-app-instance --type change --strict --json`.

## 5. PR And Archive Readiness

- [ ] 5.1 Review the diff for accidental iOS behavior changes or duplicate shell owner creation.
- [ ] 5.2 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 5.3 Archive the OpenSpec change after implementation is merged.
