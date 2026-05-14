## Why

The macOS sidebar currently feels unreliable and physically inconsistent: tab and space selection can flash back to the previously focused terminal, pinned-sidebar collapse is not a coordinated motion, and horizontal space swipes are modeled as a sidebar-only preview instead of a continuous space sequence.

This change formalizes a focused interaction pass so sidebar navigation, window chrome, and space switching share one authoritative focus and motion contract.

## What Changes

- Make sidebar tab and space selection authoritative focus transitions: selecting a tab or space updates the shell focused pane through the same shell-state path used by terminal activation.
- Replace pinned-sidebar insertion/removal with a continuous collapse and expand motion so the sidebar, terminal content inset, titlebar controls, and macOS traffic-light controls move together.
- Refine collapsed/floating sidebar chrome timing so traffic lights do not jump, appear ahead of the panel, or linger on the bare window corner.
- Replace the sidebar-only space swipe preview with a continuous pager model over the ordered space sequence, including edge preview, commit/cancel animation, and terminal focus handoff.
- Add focused tests and shell contract checks for selection persistence, sidebar/chrome animation invariants, and pager gesture behavior.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: clarify continuous sidebar chrome motion, pinned-sidebar collapse/expand behavior, and coordinated traffic-light/titlebar control movement.
- `macos-shell-workspace-interactions`: replace sidebar-only space swipe preview semantics with authoritative focus selection and continuous space pager switching.
- `macos-shell-build-test-contract`: require focused verification for selection/focus stability, space pager gestures, and coordinated sidebar/window-chrome behavior.

## Impact

- Apple client SwiftUI/AppKit shell UI:
  - `clients/apple/alan-macos/MacShellRootView.swift`
  - `clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift`
  - `clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift`
  - `clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift`
  - `clients/apple/alan-macos/Support/ShellWindowPlacement.swift`
- Shell state and runtime focus coordination:
  - `clients/apple/alan-macos/ShellHostController.swift`
  - `clients/apple/alan-macos/TerminalRuntimeRegistry.swift`
  - `clients/apple/alan-macos/TerminalHostView.swift`
- Focused Apple scripts and contract checks under `clients/apple/scripts/`.
