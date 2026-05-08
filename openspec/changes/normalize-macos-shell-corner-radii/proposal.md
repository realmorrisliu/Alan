## Why

The macOS shell UI currently mixes many one-off corner radii across the default
surface: 8, 10, 11, 12, 13, 14, 16, 18, 20, 22, 24, plus several `Capsule`
shapes. This makes Alan feel softer and less precise than the intended calm,
Arc-like native shell.

## What Changes

- Introduce a small Alan shell corner-radius scale for active macOS shell UI.
- Reduce large rounded rectangles in sidebar, command UI, inspector, terminal
  surrounds, and debug/info surfaces.
- Replace decorative `Capsule` usage in text controls and chips with modest
  rounded rectangles unless the shape is a true status dot or system-like
  circular control.
- Keep terminal panes visually continuous: split panes still share one terminal
  surround, but the outer terminal corners become smaller and more precise.
- Treat legacy/non-primary Apple surfaces separately; do not expand this pass
  into unrelated UI rewrites.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: Adds the default shell corner-radius scale,
  shape exceptions, and visual-review requirements for smaller, more precise
  radius usage.
- `macos-shell-build-test-contract`: Adds focused verification that UI changes
  do not reintroduce large ad hoc radii or capsule-heavy default shell chrome.

## Impact

- Apple client UI: primarily `clients/apple/AlanNative/MacShellRootView.swift`
  and `clients/apple/AlanNative/TerminalPaneView.swift`.
- Apple client AppKit host fallback: `TerminalHostView.swift` only where its
  placeholder/fallback panels remain visible in normal shell flows.
- Active OpenSpec UI polish: coordinate with
  `add-macos-pane-title-bars` so pane title bars use the same radius scale.
- Visual QA: running-app screenshots for sidebar, terminal, command UI, and
  inspector in light mode.
