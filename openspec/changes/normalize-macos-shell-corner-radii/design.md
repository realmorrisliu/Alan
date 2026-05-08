## Context

The active macOS shell surface uses `MacShellRootView.swift`,
`TerminalPaneView.swift`, and `TerminalHostView.swift` for the default
terminal-first app. A quick inventory shows the active shell uses hard-coded
rounded rectangles at 8, 10, 11, 12, 13, 14, 16, 18, 20, 22, and 24 points,
plus `Capsule` shapes for chips and keycap-like controls. The largest values
show up in the command palette outer shell (`24`), inspector cards (`20`),
command search field (`18`), tab rail cards (`18`), and terminal info cards
(`22`).

This conflicts with the current product direction: Arc-like native material,
compact rows, terminal-first focus, and calm precision. The UI should feel
native and deliberate, not pill-heavy or card-heavy. The goal is not to remove
all curvature; it is to make radius choices scarce, named, smaller, and tied to
component roles.

## Goals / Non-Goals

**Goals:**

- Define a small radius scale for the active Alan macOS shell.
- Reduce large rounded rectangles and pill/capsule shapes in the default shell
  UI.
- Make future pane title bars, command rows, sidebar rows, inspector cards, and
  terminal surrounds use the same scale.
- Keep true circular elements only where the shape is semantic: status dots,
  traffic-light-like indicators, or icon-only circular affordances that are
  intentionally system-like.
- Add verification so new ad hoc large radii do not reappear immediately.

**Non-Goals:**

- Redesign spacing, typography, colors, shadows, or information architecture
  except where radius changes make them visibly inconsistent.
- Rewrite legacy/non-primary `ContentView.swift` surfaces in this pass.
- Remove all rounded corners or make Alan feel boxy.
- Change split behavior, terminal runtime ownership, AppKit hit-testing, or
  shell model state.

## Decisions

1. Introduce a role-based radius scale, not a free numeric palette.

   Proposed active-shell scale:

   - `none = 0`: full-height material panels, continuous sidebars, separators,
     and background bands.
   - `control = 6`: icon buttons, compact keycaps, small title-bar controls,
     small inline controls.
   - `row = 8`: sidebar rows, command rows, attention rows, chips, inspector
     sub-rows, and pane title bars.
   - `surface = 10`: terminal surround, search fields, inline panels, and
     small grouped tool surfaces.
   - `overlay = 12`: command palette outer shell, inspector cards, fallback
     overlay cards, and rare modal-like surfaces.

   Values above `12` are not part of the default shell scale. They require a
   documented exception in code or a spec update. This keeps Alan precise while
   leaving enough curvature for native material surfaces.

   Alternative considered: make everything `8`. That would be simple but too
   blunt; overlays and terminal surrounds need slightly different optical
   treatment from small rows. Alternative considered: preserve Apple's very soft
   continuous style at `18+`. That keeps current feel but does not answer the
   user's concern that the app is too round and under-specified.

2. Add central radius tokens near `ShellPalette`.

   A small `ShellRadii` enum or similar helper should live with the shell visual
   tokens, and active-shell SwiftUI views should reference it instead of
   hard-coded radius values. AppKit-only surfaces can mirror those constants
   explicitly where Swift types cannot be shared cleanly.

   Alternative considered: leave numeric values inline but document the scale.
   That is easy to drift from and makes review harder.

3. Replace decorative `Capsule` use with modest rounded rectangles.

   `Capsule` makes compact controls read as pills. In Alan's default shell,
   text chips, keycap hints, metadata chips, and command badges should usually
   use `control` or `row` radii instead. Keep `Circle` for status dots and
   system-like round indicators.

   Alternative considered: allow capsules for every chip because they are common
   in SwiftUI. That is exactly what makes the UI feel soft and inconsistent.

4. Scope the implementation to the primary macOS shell first.

   The first pass should update `MacShellRootView.swift`,
   `TerminalPaneView.swift`, and normal-flow AppKit fallback cards in
   `TerminalHostView.swift`. `ContentView.swift` appears to contain older or
   separate console surfaces; it should be inventoried but not rewritten unless
   it is confirmed to be part of the active default shell.

5. Verify visually and mechanically.

   The useful mechanical guard is a focused script or check-shell-contract rule
   that flags new active-shell `RoundedRectangle(cornerRadius: 14+)`,
   `layer?.cornerRadius = 14+`, and default-shell `Capsule` usage except for an
   allowlist. Visual review must still compare single-pane, split-pane,
   command-palette, and inspector screenshots because small radius changes can
   affect the perceived density and hierarchy.

## Risks / Trade-offs

- Smaller radii can make the app feel too austere -> Keep material, opacity,
  spacing, and selected-state subtleties intact; do not pair the radius pass
  with a broad flattening pass.
- Hard caps can block legitimate modal styling -> Allow `overlay = 12` and make
  larger radii require explicit documentation rather than silently banning all
  exceptions.
- Replacing capsules may reduce affordance clarity -> Preserve padding,
  contrast, hover/focus state, and icon alignment when changing shapes.
- Existing pane-title-bar work may introduce new radii -> Make that change use
  the same `ShellRadii` scale before implementation.
- Mechanical grep checks can be brittle -> Keep them focused on active shell
  files and allowlisted semantic circles/capsules.

## Migration Plan

No state migration is required. Implement the scale behind shared shell visual
tokens, replace active-shell hard-coded radii in small batches, then run visual
verification. Archive should sync the accepted radius requirements into
`openspec/specs/` after implementation lands.
