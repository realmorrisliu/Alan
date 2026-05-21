## Why

Alan's terminal shell has stable pane runtimes and compact activity surfaces,
but it does not yet provide a globally summonable terminal for quick work across
macOS Spaces. Quick terminal needs a separate change so the completed activity
contracts can archive without claiming an unimplemented Peak surface.

## What Changes

- Add a detached native macOS Peak quick terminal that can be summoned from any
  macOS Space without raising Alan's main window.
- Model the MVP as one global quick-terminal instance with a normal terminal
  runtime, not one instance per Alan space or macOS Space.
- Route quick terminal show, hide, focus, close, and promote operations through
  the shared shell command/controller paths.
- Preserve runtime state across hide/show; treat `Esc` as terminal input by
  default; avoid focus-loss auto-hide.
- Add `Open in Space` promotion that moves the existing quick-terminal runtime
  into a normal Alan tab without copying or linking the terminal process.

## Capabilities

### New Capabilities

- `macos-quick-terminal-peak`: Owns global quick-terminal identity, lifecycle,
  runtime ownership, summon/dismiss behavior, cwd rules, and promotion.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: Adds native lightweight Peak presentation
  constraints for the quick terminal.
- `macos-shell-workspace-interactions`: Adds shared command routing,
  global-toggle behavior, focus restoration, and promotion semantics.

## Impact

- Apple shell model/controller paths for quick-terminal slot, show/hide/focus,
  close, and promotion commands.
- AppKit/SwiftUI window ownership for a detached Peak that does not depend on
  the main window.
- Terminal runtime service ownership, cwd selection, activity notification, and
  hidden-session lifecycle behavior.
- Keyboard shortcut/menu/command input surfaces and focused Apple tests.
