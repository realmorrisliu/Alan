## 1. Sidebar Structure

- [x] 1.1 Keep `ShellSidebarView.swift` as a single vertical sidebar column that aligns cleanly around the macOS traffic-light area.
- [x] 1.2 Replace section-like space presentation with a bottom borderless icon space switcher.
- [x] 1.3 Keep command entry and tab creation in the active-space tab-list flow.
- [x] 1.4 Replace trigger-style sidebar swipe with gesture-tracked, sidebar-local space transition preview that uses translation-first drag mapping across the full sidebar page width while the workspace remains stable until commit.
- [x] 1.5 Keep shell selection immutable during swipe preview and commit/cancel through the existing selection path on release.
- [x] 1.6 Preserve native vertical tab-list scrolling by locking horizontal intent before consuming sidebar scroll-wheel events, and disable tab-list vertical scrolling while a horizontal swipe is locked.

## 2. Copy And Interaction Cleanup

- [x] 2.1 Remove persistent sidebar section labels, shortcut labels, product slogans, and paragraph-style empty-state copy that are not needed for repeated use.
- [x] 2.2 Preserve or add accessibility labels, help text, and menu labels for controls whose visible copy is reduced.
- [x] 2.3 Ensure tab rows expose secondary actions through hover/focus/context menu without layout shifts.
- [x] 2.4 Remove sidebar notification dots while keeping Alan attachment inline and attention available to accessibility/control surfaces.
- [x] 2.5 Add compact split topology indicators to split tab rows, hiding them for single-pane tabs.
- [x] 2.6 Route split indicator activation through pane focus without mutating split layout or divider ratios.

## 3. Verification

- [x] 3.1 Run focused Apple build or shell UI checks affected by sidebar changes.
- [x] 3.2 Capture screenshot or manual review notes for default, selected, hover, no-notification-dot, and empty sidebar states.
- [ ] 3.3 Verify left/right sidebar swipe tracks gesture translation directly, commits on fast horizontal flicks, keeps the space title and tab list on the same full-width page motion, avoids static side padding gaps, holds position while fingers pause, honors zero-delta release, keeps horizontal and vertical movement axis-locked, previews sidebar content, keeps the workspace stable during drag, commits, cancels, resists edges, and leaves vertical tab-list scrolling intact.
- [x] 3.4 Verify split indicators for single-pane, two-pane horizontal, two-pane vertical, complex split, focus switching, and accessibility activation.
- [x] 3.5 Run `openspec validate --all --strict` before PR.
