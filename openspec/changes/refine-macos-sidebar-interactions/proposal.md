## Why

The macOS sidebar currently feels unreliable and physically inconsistent: tab and space selection can flash back to the previously focused terminal, pinned-sidebar collapse is not a coordinated motion, and horizontal space swipes need a continuous sidebar-local content pager instead of discontinuous source/target preview behavior.

This change formalizes a focused interaction pass so sidebar navigation, window chrome, and space switching share one authoritative focus and motion contract.

## What Changes

- Make sidebar tab and space selection authoritative focus transitions: selecting a tab or space updates the shell focused pane through the same shell-state path used by terminal activation.
- Replace pinned-sidebar insertion/removal with a continuous collapse and expand motion so the sidebar, terminal content inset, titlebar controls, and macOS traffic-light controls move together.
- Refine collapsed/floating sidebar chrome timing so traffic lights do not jump, appear ahead of the panel, or linger on the bare window corner.
- Promote collapsed-sidebar reveal retention from view-local SwiftUI hover to a window-level pointer judgment so a revealed floating sidebar does not hide when the pointer crosses the left resize frame after visible-frame zoom.
- Replace discontinuous sidebar space swipe behavior with a continuous, sidebar-local content pager over the ordered `ShellSpace` sequence, including edge preview, commit/cancel animation, and terminal focus handoff only after commit.
- Add focused tests and shell contract checks for selection persistence, sidebar/chrome animation invariants, and pager gesture behavior.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: clarify continuous sidebar chrome motion, pinned-sidebar collapse/expand behavior, and coordinated traffic-light/titlebar control movement.
- `macos-shell-workspace-interactions`: refine sidebar space swipe semantics with authoritative focus selection and a continuous sidebar-local content pager.
- `macos-shell-build-test-contract`: require focused verification for selection/focus stability, sidebar-local space pager gestures, window-level collapsed-sidebar reveal retention, and coordinated sidebar/window-chrome behavior.

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
