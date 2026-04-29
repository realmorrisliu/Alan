## Why

The native macOS app is meant to be a real terminal host for both humans and
agents, but the current prototype still has lifecycle, control-plane, build, and
UI-contract risks that can make tabs unreliable or misleading. Recording these
issues as an OpenSpec change gives the next implementation pass a concrete
contract instead of leaving the review as informal notes.

## What Changes

- Stabilize terminal runtime ownership so tabs and panes keep their process and
  Ghostty surface lifecycles independently of whether SwiftUI is currently
  rendering them.
- Make `pane.send_text` and related control-plane mutations report delivery
  truthfully, including failure when the target pane is not reachable.
- Make each macOS window use isolated shell state, persistence, socket paths, and
  event streams.
- Harden the local shell control socket and file-command fallback against stalled
  clients, oversized requests, and silent persistence failures.
- Refactor the default macOS UI toward the existing `Alan` product contract:
  space rail, active-space tab list, terminal-first content, restrained toolbar,
  and progressive inspector.
- Align the Apple project build contract with documented system requirements and
  make Ghostty dependencies explicit and verifiable.
- Add focused tests for shell state mutation, control-plane behavior, terminal
  lifecycle routing, and UI contract-sensitive model behavior.

## Capabilities

### New Capabilities

- `macos-shell-terminal-lifecycle`: Terminal tabs and panes preserve process,
  renderer, focus, metadata, and text-delivery semantics across selection and
  view lifecycle changes.
- `macos-shell-control-plane-reliability`: The local shell control surface uses
  stable window-scoped identities, bounded IPC behavior, explicit mutation
  acknowledgements, and inspectable failure states.
- `macos-shell-ui-ux-conformance`: The macOS app default UI conforms to the
  documented Alan shell UI/UX contract and keeps terminal content as the visual
  and functional center.
- `macos-shell-build-test-contract`: The Apple client has a documented,
  reproducible build/dependency contract and focused test coverage for shell
  model and control-plane risks.

### Modified Capabilities

- None.

## Impact

- Apple client: `AlanNativeApp.swift`, `MacShellRootView.swift`,
  `TerminalPaneView.swift`, `TerminalHostView.swift`, `GhosttyLiveHost.swift`,
  `TerminalHostRuntime.swift`, `ShellHostController.swift`,
  `ShellControlPlane.swift`, `ShellModel.swift`, and the Xcode project.
- Product specs: `docs/spec/alan_shell_macos_contract.md` and
  `docs/spec/alan_macos_shell_ui_ux.md` remain the narrative source of truth;
  this change adds implementation-ready OpenSpec requirements.
- Build/dependencies: local Ghostty artifact setup, deployment targets,
  resources, framework linking, and warning policy.
- Tests: shell model mutation tests, control-plane IPC/file command tests,
  terminal lifecycle/delivery tests with a mock host boundary, and project build
  verification.
