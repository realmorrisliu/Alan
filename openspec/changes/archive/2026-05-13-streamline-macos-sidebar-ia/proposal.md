## Why

The current sidebar has moved toward the target shell model, but it still reads
partly as a compact dashboard: header copy, section labels, shortcut hints,
horizontal space dock, empty-state explanation, and repeated creation menus ask
the user to parse the UI instead of making the structure self-evident. Alan's
macOS shell should make spaces, tabs, and creation affordances obvious through
placement, shape, and interaction rather than explanatory text or ambient
notification dots.

## What Changes

- Keep the default sidebar as a restrained, narrow single vertical stack so the
  content aligns cleanly around the macOS traffic-light area.
- Replace section-like space presentation with a bottom, borderless space
  switcher made of compact icon buttons.
- Support left/right swipe gestures inside the sidebar with sidebar-local
  direct manipulation: sidebar content tracks the gesture near 1:1 across the
  full sidebar page width, previews the adjacent space without static side
  gaps, and commits or cancels on release while the workspace stays visually
  stable until commit.
- Show compact split topology indicators on split tab rows so users can see
  whether a tab contains multiple terminal panes and quickly focus a pane.
- Remove nonessential descriptive copy from the default sidebar, including
  product slogans, implementation-flavored empty-state text, and redundant
  section labels.
- Make creation affordances location-specific: compact space creation near the
  bottom switcher and tab creation in the active-space tab list.
- Keep Alan attachment inline, while preserving attention in accessibility and
  control surfaces instead of default sidebar notification dots.
- Preserve accessibility labels/tooltips even when visible text is reduced.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: replace the previous space-rail direction
  with a vertical Arc-like sidebar, bottom borderless space switcher,
  sidebar-local direct-manipulation swipe switching, split-aware tab rows, and
  minimal visible copy.
- `macos-shell-workspace-interactions`: require split tab indicators to route
  focus through the same pane-focus model as terminal split interactions, and
  require sidebar swipe to preview adjacent spaces in the sidebar without
  moving workspace content or mutating selection until commit.
- `macos-shell-build-test-contract`: require visual/manual verification for the
  streamlined sidebar reading order and accessibility-preserving copy removal.

## Impact

- Affected Apple client code:
  - `clients/apple/AlanNative/Views/Shell/ShellSidebarView.swift`
  - `clients/apple/AlanNative/Support/ShellDesignTokens.swift`
  - `clients/apple/AlanNative/Support/ShellSidebarSwipeMonitor.swift`
  - `clients/apple/AlanNative/MacShellRootView.swift`
- No shell runtime, daemon, protocol, or terminal rendering changes.
- May require small helper views for bottom space-switcher items, tab-list
  header controls, split indicators, empty states, gesture routing, and
  accessibility labels.
- Related reference docs:
  - `docs/spec/alan_macos_shell_ui_ux.md`
  - Apple Human Interface Guidelines, Sidebars:
    `https://developer.apple.com/design/human-interface-guidelines/sidebars`
