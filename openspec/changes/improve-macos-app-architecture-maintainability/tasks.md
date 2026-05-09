## 1. Architecture Inventory And Guardrails

- [x] 1.1 Inventory current Apple client Swift files by primary responsibility, line count, platform scope, AppKit imports, and Xcode project membership.
- [x] 1.2 Define the accepted target folder layout under `clients/apple/AlanNative` and record which current files should move to each owner.
- [x] 1.3 Add a focused architecture-maintainability check that reports flat-directory drift, README/source layout mismatch, oversized multi-responsibility files, and AppKit bridge leakage.
- [x] 1.4 Keep the first architecture check in report or narrowly failing mode so it protects new regressions without requiring the full migration in one commit.

## 2. App Entry And Window Ownership

- [x] 2.1 Move macOS app singleton startup and duplicate-instance activation out of `AlanNativeApp.swift` into app-owned files.
- [x] 2.2 Move primary shell owner creation and stable `window_main` setup into an app or shell startup owner.
- [x] 2.3 Move shell menu/keyboard command definitions into a dedicated command owner that still routes through the shared shell workspace command API.
- [x] 2.4 Move hidden-titlebar placement, traffic-light metrics, min-size, tabbing, and primary-window focusing out of `MacShellRootView.swift` into a window support/coordinator boundary.

## 3. SwiftUI Shell View Structure

- [x] 3.1 Split `MacShellRootView.swift` into focused shell root, sidebar, workspace, command palette, material/theme, and support files without changing behavior.
- [x] 3.2 Ensure the shell root reads as stable composition and no longer owns feature-specific sidebar rows, command search internals, inspector/debug panels, or voice command implementation.
- [x] 3.3 Move shell visual tokens and material wrappers into support/design-token files that can be reused without importing unrelated shell layout.
- [x] 3.4 Coordinate with `polish-macos-search-remove-inspector` so inspector removal and Find bar work are not buried inside the architecture split.

## 4. Console, API, And Event Reduction Boundaries

- [x] 4.1 Move mobile or legacy console UI from `ContentView.swift` into a clearly named console/mobile folder while preserving the non-macOS app entry path.
- [x] 4.2 Move chat/timeline/pending-yield value types into model files or console-specific model files.
- [x] 4.3 Split daemon API DTOs and `AlanAPIClient` ownership from console view model and SwiftUI screens.
- [x] 4.4 Extract daemon event polling/reading and event-to-UI-state reduction from `AlanConsoleViewModel` so event mapping is testable without rendering the full console UI.

## 5. Shell Model And Controller Ownership

- [ ] 5.1 Split pure shell enum/value types, pane/tree/tab/space snapshots, obsolete legacy decoding removal, bootstrap defaults, and mutation helpers out of the current monolithic `ShellModel.swift`.
- [ ] 5.2 Keep shell mutation behavior covered by the existing focused shell model script tests after each split.
- [ ] 5.3 Review `ShellHostController` for controller/store/service boundaries and move persistence, boot-profile projection, attention projection, and runtime update projection where they have clear owners.
- [ ] 5.4 Preserve `ShellWorkspaceCommand` as the shared command vocabulary used by menu, command palette, keyboard, and control-plane paths.

## 6. Terminal Host And Runtime Service Boundaries

- [ ] 6.1 Split `TerminalHostView.swift` so `AlanTerminalHostNSView` remains the AppKit bridge while input routing, overlay presentation, window observation, runtime snapshot publication, and Ghostty attachment have explicit collaborators.
- [ ] 6.2 Keep terminal-area event ownership routed through the AppKit terminal host and preserve the weak activation boundary during extraction.
- [ ] 6.3 Keep terminal runtime identity service-backed and pane-keyed while moving files or collaborators.
- [ ] 6.4 Run focused terminal surface/runtime scripts after terminal-host or runtime-service extractions.

## 7. Control Plane And IPC Boundaries

- [ ] 7.1 Split shell control-plane protocol DTOs from socket server, file-polling control plane, local command executor, state merger, event persistence, and diagnostics.
- [ ] 7.2 Keep socket transport behavior bounded by request size, request timeout, response timeout, and concurrency limits during extraction.
- [ ] 7.3 Keep local command execution authoritative against shell/runtime state and separate from transport read/write code.
- [ ] 7.4 Run focused shell control-plane contract scripts after IPC or local executor extraction.

## 8. Documentation And Project Organization

- [x] 8.1 Update `clients/apple/README.md` so documented source layout matches the actual folders and current macOS/iOS entry paths.
- [x] 8.2 Update the Xcode project file to keep groups, file references, and target membership aligned with the new source layout.
- [x] 8.3 Document which active macOS OpenSpec changes may conflict with architecture slices and how implementation should sequence or rebase around them.
- [x] 8.4 Confirm no behavior-only OpenSpec requirement is moved into this architecture-maintainability capability.

## 9. Verification

- [x] 9.1 Run the new Apple architecture-maintainability check and document current remaining hotspots.
- [x] 9.2 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 9.3 Run focused Apple shell scripts affected by moved model, runtime, terminal surface, or control-plane files.
- [x] 9.4 Run `git diff --check`.
- [x] 9.5 Run `openspec validate improve-macos-app-architecture-maintainability --type change --strict --json`.
- [x] 9.6 Run `openspec validate --all --strict --json`.
- [x] 9.7 Build the macOS app with `xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build`.

## 10. PR Review And Archive Readiness

- [x] 10.1 Keep mechanical file moves separate from behavior edits in commits or PR descriptions.
- [x] 10.2 Ask reviewers to evaluate whether source ownership, AppKit bridge boundaries, and validation gates make future work easier to maintain.
- [ ] 10.3 Before archive, sync the accepted `macos-app-architecture-maintainability` requirements into `openspec/specs/`.
- [ ] 10.4 Before archive, sync the accepted build/test contract delta into `openspec/specs/macos-shell-build-test-contract/spec.md`.
- [ ] 10.5 Record implementation verification evidence and remaining architecture debt before archiving the change.
