# Alan macOS Shell UI / UX Contract

> Status: VNext design contract for the native macOS Alan app.

## Purpose

Define the normative user-facing information architecture, chrome, terminology,
and interaction model for the macOS Alan app.

This document exists so the macOS client can be refactored toward a coherent
native product instead of evolving through local UI experiments.

It complements `docs/spec/alan_shell_macos_contract.md`:

1. `alan_shell_macos_contract.md` defines the shell model and control-plane contract.
2. This document defines how that model must appear and behave for human operators.

When the current prototype conflicts with this document, the prototype should be
refactored to match this contract.

## Product Naming

1. The product name shown to users is `Alan`.
2. `Alan Shell` is an internal architecture label only.
3. `Alan Shell` must not appear in window titles, sidebar branding, toolbar copy,
   onboarding copy, or empty states.

## Quality Bar

The quality bar is:

1. Apple for native fit and finish,
2. Arc for sidebar organization and vertical tab behavior,
3. Linear for restraint, hierarchy, and product sharpness.

This does not mean visually cloning Arc or Linear.

It means the app must feel:

1. native rather than web-wrapped,
2. calm rather than status-heavy,
3. precise rather than dashboard-like,
4. confident enough to remove unnecessary chrome.

## Core Product Reading

At first glance, a user should read the interface in this order:

1. current tab and its terminal content,
2. the current space and its tab list,
3. only then any secondary state such as attention, Alan attachment, or inspector detail.

The terminal must remain the visual and functional center of gravity.

## User-Facing Model

### Window

The native macOS window.

Rules:

1. A window owns one visible sidebar context and one active content surface.
2. The titlebar and toolbar must feel like native window chrome, not like a web page header.

### Space

`Space` is a user-visible concept and must remain visible in the product.

It is intentionally aligned with Arc's concept of a space:

1. a durable top-level work scope,
2. a container that organizes tabs,
3. a unit that can carry its own title, identity, and future defaults.

Rules:

1. A space is not an internal implementation detail.
2. A space must be represented in the sidebar as a stable, directly selectable object.
3. A space exists to scope and organize tabs.
4. The user should not need to understand pane trees or internal surface models to understand spaces.

### Tab

`Tab` is the primary user-facing work object inside a space.

Internally this maps to the current shell `Surface` object, but the word
`surface` must not be the default product language.

Rules:

1. The sidebar's main list is a tab list for the active space.
2. A tab may represent a terminal tab, an Alan tab, a log tab, or future kinds.
3. The user should primarily think in terms of tabs, not surfaces.

### Pane

`Pane` is a split leaf inside a tab.

Rules:

1. Pane exists as a user-facing concept only when split layout is relevant.
2. Pane identity is useful for routing and control, but should not dominate the default UI.
3. Internal pane IDs such as `pane_3` are debug data, not default chrome.

### Alan Session

Alan is optional content attached to a pane, not the primary layout model.

Rules:

1. The app must remain useful when no Alan session exists.
2. Alan status should appear as a recessed tab or pane attribute, not as the main product structure.
3. The app should never imply that a tab only exists if Alan is running.

## Information Architecture

The primary app layout is:

```text
| Space Rail | Active-Space Tab List | Main Terminal Content | Inspector? |
```

Rules:

1. The sidebar is the organizing layer.
2. The content area is the work layer.
3. The inspector is optional secondary detail.
4. Attention is a state system layered onto spaces and tabs, not a separate primary column.

## Sidebar Contract

### Sidebar Structure

The sidebar must be restructured into two distinct layers:

1. `Space rail`
2. `Active-space tab list`

The current prototype pattern of rendering `Spaces`, `Tabs`, and `Attention`
as sibling sections is not acceptable as the target IA.

### Space Rail

The space rail is a narrow vertical strip used only for switching spaces.

Target behavior:

1. Width should feel rail-like, approximately `52-60pt`.
2. Each space should be represented by a compact avatar, glyph, or initial.
3. The active space should be obvious through selection tone and shape, not through heavy cards.
4. Creating a new space belongs here as a compact affordance, not as a full-width hero button.

### Active-Space Tab List

The tab list is the main sidebar surface.

Target behavior:

1. Width should feel list-like, approximately `220-260pt`.
2. The tab list must only show tabs for the active space.
3. Every row must be readable as a tab row, not a card.
4. The top of the tab list should host a compact `Go to or Command...` entry point.
5. New-tab creation should be contextual to the tab list, not conflated with space creation.

### Tab Row Contract

Each tab row should express:

1. icon or kind marker,
2. primary title,
3. compact secondary context such as cwd or branch,
4. optional low-emphasis status indicator,
5. hover actions for close, move, or more.

Rules:

1. Tabs must feel lightweight and skimmable.
2. A tab row must not look like a self-contained card.
3. Most state should be conveyed through spacing, tint, badges, and selection, not repeated pills.
4. Attention belongs on the row as a dot, badge, or subtle status mark.

## Toolbar and Titlebar Contract

The window should use a unified, native-feeling toolbar.

The toolbar must not feel like a page header rendered inside the app.

Target composition:

1. current tab title,
2. optional secondary path or repo context,
3. one primary command entry point,
4. a small number of high-frequency actions,
5. optional inspector toggle.

Rules:

1. The command entry point should read more like `Go to or Command...` than `Command Surface`.
2. Toolbar actions must be few and quiet.
3. `Open Alan` may exist, but should not compete with the base terminal workflow.
4. Attention should not appear as a large standalone pill in the toolbar.

The toolbar should prefer:

1. one compact command field,
2. one new-tab affordance,
3. one split/menu affordance,
4. one inspector toggle.

## Main Content Contract

The main content region must privilege the terminal canvas above all other chrome.

Rules:

1. The terminal should read like the surface itself, not like a card placed inside a page.
2. In a single-pane tab, the terminal should be nearly full-bleed within the content region.
3. The app should avoid multiple nested rounded panels around terminal content.
4. White space and depth should separate navigation from content, not repeatedly frame the canvas.

The current prototype's panelized terminal composition is not the target design.

## Split and Pane Contract

When a tab contains multiple panes, pane chrome must remain restrained.

Rules:

1. A split pane may show a lightweight title or process hint.
2. Per-pane controls should be contextual, hover-revealed, or menu-driven where possible.
3. `Focused` should not be a primary visible button.
4. Engineering labels such as `window attached` or internal pane IDs must not appear in default pane chrome.
5. Focus should be expressed with selection tone, subtle borders, and terminal cursor behavior before explicit labels.

Single-pane tabs should not show a pane selector strip by default.

## Attention Contract

Attention is a routing and urgency system, but it must be expressed as product
state rather than as a permanent side feed.

Rules:

1. Attention should primarily appear on space and tab rows.
2. Waiting-for-user state should be visible with subtle but clear emphasis.
3. An optional aggregated attention jump may exist in the toolbar or command flow.
4. A persistent `Attention` content block in the sidebar is not the target IA.

Accepted expressions of attention:

1. tab-row badges,
2. space-level dots or counts,
3. inspector overview summaries,
4. command-bar jump targets.

## Alan Affordance Contract

Alan-specific state should feel integrated, not bolted on.

Rules:

1. An Alan-backed tab is still a tab.
2. Alan attachment should read as a tab or pane attribute, not a separate UI subsystem.
3. Alan should use one restrained marker or status treatment instead of a large dedicated card.
4. The app must still feel complete when every tab is a plain shell tab.

Preferred product language:

1. `Open in Alan`
2. `Alan tab`
3. `Ask Alan`

Avoid exposing low-level terms like:

1. `binding`
2. `surface`
3. `window attached`
4. `title updated`

outside explicit debug surfaces.

## Inspector Contract

The inspector must follow progressive disclosure.

It should be off by default and should not compete with the main content area.

The inspector should have two layers:

1. `Overview`
2. `Debug`

### Overview

Overview should contain only user-relevant secondary state:

1. focused tab or pane summary,
2. cwd or repo context,
3. Alan attachment summary,
4. attention summary,
5. a minimal process or terminal status summary.

### Debug

Debug may contain:

1. snapshot JSON,
2. runtime phase,
3. Ghostty host data,
4. control-path data,
5. Alan binding details.

Rules:

1. Debug detail must never dominate the default layout.
2. Default UI should not require reading implementation state to use the app.

## Copy and Terminology Contract

The product should use a small, stable vocabulary.

Preferred user-facing terms:

1. `Space`
2. `Tab`
3. `Split`
4. `Inspector`
5. `Go to or Command...`
6. `Open in Alan`

Internal-only terms by default:

1. `Surface`
2. `Binding`
3. `Viewport snapshot`
4. `Window attached`
5. raw pane IDs

Copy rules:

1. Use fewer labels.
2. Prefer concrete action language over system explanation.
3. Do not restate architecture in the sidebar.
4. Keep brand copy minimal.

## Motion and Feedback Contract

Motion should feel native, calm, and informative.

Target uses:

1. space switching,
2. tab selection,
3. inspector reveal,
4. tab insertion or close,
5. subtle attention-state transitions.

Rules:

1. Motion must support orientation, not spectacle.
2. Avoid gadgety pulses, large spring animations, or decorative flourish.
3. Selection and hierarchy changes should be legible even with reduced motion.

## Explicit Anti-Goals

The target product must avoid:

1. dashboard composition with many equal-weight cards,
2. separate sibling sections for spaces, tabs, and attention,
3. repeated status pills for every visible state,
4. page-like headers inside the main window,
5. terminal panes wrapped in multiple decorative containers,
6. debug and runtime data permanently occupying the main canvas,
7. over-explaining the object model in the default UI.

## Refactor Mapping

This contract should drive refactors in these files first:

1. `clients/apple/AlanNative/MacShellRootView.swift`
   - restructure sidebar into `space rail + active-space tab list`
   - reduce toolbar actions and rename command entry point
   - move attention from feed to row-level and aggregate state
   - make inspector overview/debug explicit
2. `clients/apple/AlanNative/TerminalPaneView.swift`
   - remove card-like terminal framing
   - suppress engineering labels in default chrome
   - collapse pane metadata and selector chrome
   - make single-pane tabs visually quieter
3. `clients/apple/AlanNative/TerminalHostView.swift`
   - keep the host view focused on terminal rendering and necessary interaction hooks
   - avoid decorative UI that belongs in the shell chrome
4. `clients/apple/AlanNative/ShellModel.swift`
   - preserve `Space -> Surface -> Pane` internally
   - treat `Tab` as the default UI name for `Surface`
   - keep attention and Alan state compatible with row-level rendering

## Acceptance Criteria

The refactor is only acceptable when all of the following are true:

1. A user can immediately tell that spaces organize tabs.
2. The sidebar reads as `space switcher + tab list`, not as three unrelated sections.
3. The terminal occupies the clear majority of visual attention in the main window.
4. A single-pane tab does not expose unnecessary pane chrome.
5. Attention is expressed without a permanent sidebar feed.
6. The default UI does not expose `surface`, `binding`, raw pane IDs, or runtime event jargon.
7. Alan appears as an optional capability layered onto the terminal, not as the product's structural center.
8. The app feels native on macOS and no longer reads like a web dashboard wrapped in a desktop shell.

## Refactor Sequence

Recommended order:

1. Sidebar IA and terminology.
2. Toolbar simplification and command-entry rename.
3. Attention migration from feed to tab and space status.
4. Terminal canvas chrome reduction.
5. Split-pane chrome reduction.
6. Inspector split into overview and debug.
7. Motion and polish pass.
