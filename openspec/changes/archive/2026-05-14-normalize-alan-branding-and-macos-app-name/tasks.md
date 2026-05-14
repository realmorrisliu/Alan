## 1. Inventory And Classification

- [x] 1.1 Scan active source, docs, specs, scripts, project metadata, prompts, tests, and client copy for `Alan`, `AlanNative`, `Alan Shell`, `Ask Alan`, `Open in Alan`, `alanterm`, `dev.alan.native`, and `com.realmorrisliu.AlanNative`.
- [x] 1.2 Classify each match as user-facing brand copy, macOS platform copy, Apple engineering identity, Swift/Rust identifier, command syntax, runtime metadata, persisted local state, test fixture, active OpenSpec text, or historical archive.
- [x] 1.3 Define an allowlist for legitimate compatibility strings such as literal `alan shell ...` command syntax, archived OpenSpec history, external package names, and idiomatic non-user-facing type identifiers.

## 2. Apple Project And Source Rename

- [x] 2.1 Rename the Apple source root from `clients/apple/AlanNative` to `clients/apple/alan-macos` and update Xcode project file references and groups.
- [x] 2.2 Rename the Xcode project, scheme, and target-facing developer commands from `AlanNative` to `alan-macos`.
- [x] 2.3 Rename `AlanNativeApp.swift` and the `AlanNativeApp` entry type to app-owned names that do not contain `AlanNative`.
- [x] 2.4 Update Apple scripts, architecture checks, focused Swift test scripts, capture helpers, and local debug launch scripts to use the new source root, project file, scheme, product, and bundle identifier defaults.
- [x] 2.5 Update active OpenSpec changes and long-lived specs that mention the old `AlanNative` build command, source path, or architecture target.

## 3. macOS App Identity And State Migration

- [x] 3.1 Set the macOS app display name, product name, generated app bundle, and window/app metadata to `alan`.
- [x] 3.2 Change the default macOS bundle identifier from `dev.alan.native` to `app.alanworks.macos`.
- [x] 3.3 Update logging subsystem names, capture defaults, singleton lock naming, `TERM_PROGRAM`, and runtime diagnostics to use the new alan for macOS identity.
- [x] 3.4 Replace new App Support and persisted shell-state paths named `AlanNative` with the canonical alan for macOS path.
- [x] 3.5 Add a best-effort migration or fallback read for existing local state under the historical `AlanNative` support path before writing future state only to the new path.

## 4. Product Copy And Documentation

- [x] 4.1 Update visible macOS UI labels, menus, buttons, accessibility labels, and inline states from `Alan` branding to lowercase `alan`.
- [x] 4.2 Replace product/app references to `Alan Shell` with terminal emulator or terminal workspace language, while preserving literal `alan shell` command syntax where it names the CLI control namespace.
- [x] 4.3 Update README, CONTRIBUTING, AGENTS.md, Apple docs, narrative docs, and developer guides so standalone product references use `alan` and platform-specific references use `alan for macOS`.
- [x] 4.4 Update TUI onboarding copy, runtime prompts, persona templates, built-in skill copy, and tests that assert product identity text so the assistant/product identity is lowercase `alan`.
- [x] 4.5 Update command labels such as `Ask Alan...`, `Open in Alan`, `New Alan Tab`, `Alan Space`, and `Alan App` to lowercase `alan` wording or clearer terminal-workspace wording.

## 5. Validation And Regression Checks

- [x] 5.1 Add or update a focused brand/project scan that rejects non-allowlisted `AlanNative`, `Alan`, `Alan Shell`, `alanterm`, `dev.alan.native`, and `com.realmorrisliu.AlanNative` in active surfaces.
- [x] 5.2 Run `git diff --check`.
- [x] 5.3 Run the updated Apple architecture-maintainability check and shell contract check.
- [x] 5.4 Run the focused Apple Swift scripts affected by path, project, state, singleton, terminal runtime, and shell model renames.
- [x] 5.5 Build the macOS app with the renamed command: `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build`.
- [x] 5.6 Run focused Rust/TUI tests that cover updated prompt, onboarding, CLI help, and `alan shell` command syntax strings.
- [x] 5.7 Run `openspec validate normalize-alan-branding-and-macos-app-name --strict` and `openspec validate --all --strict`.

## 6. Review And Archive Readiness

- [x] 6.1 Review the final diff for accidental user-facing `Alan` title-case brand copy or unintended command namespace changes.
- [x] 6.2 Confirm old `AlanNative` names remain only in explicit historical migration notes, allowlisted compatibility tests, or archived OpenSpec history.
- [x] 6.3 Document any local migration behavior and rollback notes for users with existing alan for macOS state.
- [x] 6.4 After implementation merges, sync accepted delta specs into `openspec/specs/` and archive the completed OpenSpec change.
