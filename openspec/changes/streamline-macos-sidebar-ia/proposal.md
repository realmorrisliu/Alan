## Why

The current sidebar has moved toward the target shell model, but it still reads
partly as a compact dashboard: header copy, section labels, shortcut hints,
horizontal space dock, empty-state explanation, and repeated creation menus ask
the user to parse the UI instead of making the structure self-evident. Alan's
macOS shell should make spaces, tabs, attention, and creation affordances obvious
through placement, shape, and interaction rather than explanatory text.

## What Changes

- Keep the default sidebar as a single vertical stack so the content aligns
  cleanly around the macOS traffic-light area.
- Replace section-like space presentation with a bottom, borderless space
  switcher made of compact icon buttons.
- Support left/right swipe gestures inside the sidebar to switch spaces.
- Show compact split topology indicators on split tab rows so users can see
  whether a tab contains multiple terminal panes and quickly focus a pane.
- Remove nonessential descriptive copy from the default sidebar, including
  product slogans, implementation-flavored empty-state text, and redundant
  section labels.
- Make creation affordances location-specific: compact space creation near the
  bottom switcher and tab creation in the active-space tab list.
- Represent attention and Alan attachment through tab-row and space-switcher
  marks instead of separate explanatory blocks.
- Preserve accessibility labels/tooltips even when visible text is reduced.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: replace the previous space-rail direction
  with a vertical Arc-like sidebar, bottom borderless space switcher, swipe
  switching, split-aware tab rows, and minimal visible copy.
- `macos-shell-workspace-interactions`: require split tab indicators to route
  focus through the same pane-focus model as terminal split interactions.
- `macos-shell-build-test-contract`: require visual/manual verification for the
  streamlined sidebar reading order and accessibility-preserving copy removal.

## Impact

- Affected Apple client code:
  - `clients/apple/AlanNative/Views/Shell/ShellSidebarView.swift`
  - `clients/apple/AlanNative/Support/ShellDesignTokens.swift`
  - `clients/apple/AlanNative/MacShellRootView.swift`
- No shell runtime, daemon, protocol, or terminal rendering changes.
- May require small helper views for bottom space-switcher items, tab-list
  header controls, split indicators, empty states, gesture routing, and
  accessibility labels.
- Related reference docs:
  - `docs/spec/alan_macos_shell_ui_ux.md`
  - Apple Human Interface Guidelines, Sidebars:
    `https://developer.apple.com/design/human-interface-guidelines/sidebars`
