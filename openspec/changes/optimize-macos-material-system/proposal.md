## Why

Alan's macOS shell already uses native material wrappers, but the visual system is
still partly color-token driven and does not clearly separate navigation glass,
content backgrounds, terminal canvas treatment, and button/control materials.
Apple's current guidance distinguishes Liquid Glass as a functional layer for
controls and navigation from standard materials used in the content layer; Alan
should adopt that distinction before more UI polish accumulates local one-off
effects.

## What Changes

- Define a semantic macOS shell material system for window background, sidebar,
  terminal surround, overlay, and compact controls.
- Treat Liquid Glass as a restrained functional layer for navigation, command
  entry, floating controls, and transient interactive affordances.
- Keep terminal content and workspace backgrounds on standard materials, tonal
  surfaces, and legible contrast rather than applying Liquid Glass everywhere.
- Replace hard-coded white/opaque fills in active shell chrome with reusable
  material roles and system-vibrant foreground colors where appropriate.
- Add visual verification expectations for material hierarchy, readability, and
  reduced-transparency/high-contrast behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `macos-shell-ui-ux-conformance`: tighten the native material contract around
  functional Liquid Glass versus content-layer standard materials.
- `macos-shell-build-test-contract`: require focused verification for material
  hierarchy and accessibility-related material settings.

## Impact

- Affected Apple client code:
  - `clients/apple/AlanNative/Support/ShellDesignTokens.swift`
  - `clients/apple/AlanNative/MacShellRootView.swift`
  - `clients/apple/AlanNative/Views/Shell/ShellSidebarView.swift`
  - `clients/apple/AlanNative/Views/Shell/ShellCommandTabView.swift`
  - `clients/apple/AlanNative/TerminalPaneView.swift`
  - `clients/apple/AlanNative/TerminalHostView.swift`
- No runtime protocol or daemon API changes.
- May add small reusable SwiftUI/AppKit wrappers for semantic material roles, but
  must keep AppKit bridge ownership inside `Support/` or another approved bridge
  boundary.
- Research inputs:
  - Apple Human Interface Guidelines, Materials:
    `https://developer.apple.com/design/human-interface-guidelines/materials`
  - Apple Technology Overview, Liquid Glass:
    `https://developer.apple.com/documentation/TechnologyOverviews/liquid-glass`
  - Apple Technology Overview, Adopting Liquid Glass:
    `https://developer.apple.com/documentation/TechnologyOverviews/adopting-liquid-glass`
