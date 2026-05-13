## Context

The current shell material layer is concentrated in `ShellDesignTokens.swift`.
It uses `NSVisualEffectView.Material.sidebar` for `ShellMaterialBackgroundView`,
then overlays project-specific color washes. Other active shell surfaces still
use direct fills such as `Color.white.opacity(...)`, `ShellPalette.window`, or
opaque sidebar control colors.

Apple's current guidance frames Liquid Glass as the top functional layer for
controls and navigation, while standard materials provide structure inside the
content layer. That maps well to Alan's desired product reading order:
navigation and command entry float above a terminal-first content area, while
the terminal itself remains stable, high contrast, and readable.

## Goals / Non-Goals

**Goals:**

- Establish semantic material roles instead of choosing materials by apparent
  color.
- Make sidebar, command entry, compact buttons, and floating overlays feel native
  and current without overwhelming terminal content.
- Keep the terminal canvas visually dominant and legible across light mode,
  reduce-transparency, and increased-contrast settings.
- Centralize material bridge code so SwiftUI feature views request roles instead
  of instantiating `NSVisualEffectView` details directly.

**Non-Goals:**

- Do not redesign sidebar information architecture; that is owned by
  `streamline-macos-sidebar-ia`.
- Do not redesign command input behavior; that is owned by
  `polish-macos-command-input`.
- Do not introduce dark-mode completeness as part of this pass.
- Do not change shell runtime, terminal protocol, or command routing behavior.

## Decisions

1. Use semantic material roles.

   Add or refine a small set of roles such as `windowBackdrop`, `sidebarGlass`,
   `workspaceBackdrop`, `terminalSurround`, `floatingOverlay`, and
   `controlGlass`. Views should request the role, not `NSVisualEffectView`
   constants directly. This keeps the design vocabulary stable if AppKit or
   SwiftUI exposes newer Liquid Glass APIs differently by SDK.

   Alternative considered: directly swap all current fills to `.ultraThinMaterial`.
   That would be fast, but it would blur the distinction between navigation,
   content, and terminal surfaces.

2. Reserve Liquid Glass-style treatment for navigation and controls.

   Sidebar rail/list backgrounds, command entry points, floating command input,
   and compact icon controls may use glass-like material treatment. Workspace and
   terminal content backgrounds should use standard materials, tonal surfaces, or
   subdued vibrancy to preserve legibility.

   Alternative considered: make the entire window a continuous glass layer. This
   conflicts with Apple's content-layer guidance and would make terminal text
   harder to scan.

3. Prefer system foreground styles over custom low-contrast grays on material.

   Text and symbols over material should use `.primary`, `.secondary`,
   `.tertiary`, accent, or other system-vibrant styles unless a specific token is
   needed. Custom RGB colors stay available for Alan accent/attention, but not as
   the default way to solve legibility.

4. Material polish must degrade gracefully.

   Implementation should explicitly review reduced transparency and increased
   contrast. If a material role becomes too transparent or busy, the role should
   fall back to a more solid standard material or tonal fill without changing the
   view hierarchy.

5. Treat radius and shadow as surface semantics, not decorative effects.

   The current terminal surface establishes the baseline for a primary content
   surface: continuous 12pt corners, a focused contact shadow, and a restrained
   rim/highlight treatment. Other active shell elements should derive from a
   smaller semantic scale: micro geometry for split indicators, 6-10pt compact
   controls and inputs, 12pt selected/content surfaces, 16pt collapsed floating
   panels, and capsules only for semantic pill inputs such as `Ask Alan...` and
   the command palette. Shadow should appear only where elevation changes the
   interaction model: selected navigation rows, floating inputs, the terminal
   surface, collapsed sidebar panels, and modal command entry. Static sidebar
   controls and titlebar ghost buttons should rely on tint, stroke, hover, and
   highlight instead of default shadows.

   Alternative considered: give every translucent control a small shadow to
   make the glass visible. That made light mode feel dirty because dark ambient
   shadow mixed with the sidebar material. Focused adaptive contact shadows are
   clearer and closer to native macOS hierarchy.

## Risks / Trade-offs

- Visual regressions can be subjective -> Mitigate with screenshot/manual review
  tasks that compare sidebar, terminal, controls, and overlays in the same run.
- New SDK material APIs may not be available on every build host -> Keep roles
  behind compatibility wrappers and use current AppKit/SwiftUI material APIs as
  fallback.
- Too much glass can reduce terminal readability -> Keep terminal canvas and
  content layer out of Liquid Glass and verify contrast manually.
