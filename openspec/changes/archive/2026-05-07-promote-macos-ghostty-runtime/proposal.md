## Why

Alan's macOS shell currently links Ghostty through pane-host views, so terminal
runtime ownership is still tied too closely to SwiftUI/AppKit view attachment.
To reach Ghostty-class reliability, Alan needs a process-scoped terminal runtime
service that owns libghostty initialization, app lifetime, and pane surface
lifecycle independently from transient view rendering.

## What Changes

- Introduce a macOS terminal runtime service owned at the app/window runtime
  boundary rather than by each `AlanTerminalHostNSView`.
- Ensure libghostty is initialized once per process and that Ghostty app state is
  shared across all pane surfaces in a window-safe way.
- Move surface creation, teardown, reattachment, focus, occlusion, display, and
  metadata propagation behind stable pane IDs.
- Preserve Alan's window-scoped shell state, control plane, and agent-oriented
  pane routing while replacing view-owned terminal lifetimes.
- Make pane close, tab close, window close, and app termination perform
  deterministic surface/runtime cleanup with truthful final state.
- Fold the existing `converge-terminal-event-ownership` work into the same
  runtime boundary when implementation overlaps, rather than creating a second
  competing owner for terminal events.

## Capabilities

### New Capabilities
- `macos-terminal-runtime-foundation`: Defines the process/window/pane ownership
  model for Ghostty runtime initialization, multi-surface lifecycle, view
  reattachment, and terminal metadata projection.

### Modified Capabilities
- `macos-shell-terminal-lifecycle`: Terminal runtimes become service-owned and
  surface identity must remain stable across view recreation, selection changes,
  and close paths.
- `macos-shell-control-plane-reliability`: Runtime-dependent control responses
  must reflect service-level runtime state, not transient view availability.
- `macos-shell-build-test-contract`: The Apple client must test runtime service
  ownership and teardown without requiring all tests to launch the full app UI.

## Impact

- Apple client architecture: `AlanNativeApp.swift`, `MacShellRootView.swift`,
  `ShellHostController.swift`, `TerminalRuntimeRegistry.swift`,
  `TerminalHostView.swift`, `GhosttyLiveHost.swift`, and new service/controller
  files under `clients/apple/AlanNative`.
- Runtime behavior: background panes keep terminal processes alive, visible
  panes reattach to existing surfaces, and close paths tear down surfaces exactly
  once.
- Control plane: `pane.send_text`, pane focus, and metadata reads use the
  runtime service as the authoritative terminal runtime source.
- Tests: focused unit tests for runtime registry/service semantics plus manual
  app verification for multi-pane and multi-window behavior.
