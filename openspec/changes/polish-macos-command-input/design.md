## Context

`ShellCommandTabView` currently owns a broad command palette:

- action enum and matching logic,
- best-match intent,
- attention candidates,
- routing candidates,
- voice controller affordance,
- rows grouped under visible sections.

The user direction is narrower: `Command-K` should summon a beautiful Liquid
Glass input box and should not show the candidate actions underneath. That makes
this change mostly a surface simplification and interaction reset, not a command
vocabulary expansion.

## Goals / Non-Goals

**Goals:**

- Make `Command-K` feel like a native, premium floating input rather than a
  dashboard-like command palette.
- Remove default candidate/action/routing/attention lists from the overlay.
- Keep keyboard and focus behavior crisp: open, type, submit, dismiss, restore
  terminal focus.
- Share material roles with the material-system direction where possible.

**Non-Goals:**

- Do not build a full fuzzy command launcher with ranked visible suggestions in
  this change.
- Do not expand command vocabulary beyond the existing workspace actions needed
  for typed submission.
- Do not add voice input to `Command-K`; voice MVP work owns voice interaction.
- Do not change terminal `Command-F` Find behavior.

## Decisions

1. Replace the palette body with an input-only overlay.

   The overlay should contain the search/command icon, text field, optional
   compact key hint, and a slim close affordance if needed. It should not render
   `Actions`, `Routing`, `Attention`, `Best match`, or command rows beneath the
   field.

   Alternative considered: keep suggestions but visually hide them until query
   text exists. The user specifically asked to remove the candidate actions, so
   the default surface should not become a conditional suggestion list.

2. Keep typed execution minimal and deterministic.

   Return may execute an exact or well-known typed command using existing shell
   workspace command handlers. If the typed command cannot be resolved, the input
   should remain open with a subtle non-row state such as shake, tint, or inline
   placeholder/status. It should not open a candidate list.

3. Make material and geometry do the product work.

   The input should use a Liquid Glass-style material role, soft depth, and
   restrained rounded geometry consistent with the shell radius scale. It should
   look rich through native material behavior and focus treatment, not through
   decorative gradients or a large panel.

4. Preserve terminal focus ownership.

   Opening the input temporarily captures keyboard focus. Dismissing it via
   Escape, click-away, successful submit, or close control should return focus to
   the previously focused terminal pane when available.

## Risks / Trade-offs

- Removing visible candidates reduces discoverability -> Mitigate through
  sidebar/menu affordances, existing native menu items, and future optional typed
  help if needed outside this default surface.
- Exact typed routing may feel too limited -> Keep the implementation modular so
  future fuzzy command resolution can improve submission without adding visible
  candidate lists back by default.
- Liquid material can become visually busy -> Share material roles with
  `optimize-macos-material-system` and verify readability over the active shell.
