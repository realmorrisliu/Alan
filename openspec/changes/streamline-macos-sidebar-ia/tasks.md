## 1. Sidebar Structure

- [ ] 1.1 Keep `ShellSidebarView.swift` as a single vertical sidebar column that aligns cleanly around the macOS traffic-light area.
- [ ] 1.2 Replace section-like space presentation with a bottom borderless icon space switcher.
- [ ] 1.3 Keep command entry and tab creation in the active-space tab-list flow.
- [ ] 1.4 Add left/right sidebar swipe handling for previous/next space switching without breaking vertical tab-list scrolling.

## 2. Copy And Interaction Cleanup

- [ ] 2.1 Remove persistent sidebar section labels, shortcut labels, product slogans, and paragraph-style empty-state copy that are not needed for repeated use.
- [ ] 2.2 Preserve or add accessibility labels, help text, and menu labels for controls whose visible copy is reduced.
- [ ] 2.3 Ensure tab rows expose secondary actions through hover/focus/context menu without layout shifts.
- [ ] 2.4 Keep attention and Alan attachment expressed as compact row/switcher marks rather than separate sidebar blocks.
- [ ] 2.5 Add compact split topology indicators to split tab rows, hiding them for single-pane tabs.
- [ ] 2.6 Route split indicator activation through pane focus without mutating split layout or divider ratios.

## 3. Verification

- [ ] 3.1 Run focused Apple build or shell UI checks affected by sidebar changes.
- [ ] 3.2 Capture screenshot or manual review notes for default, selected, hover, attention, and empty sidebar states.
- [ ] 3.3 Verify left/right sidebar swipe switches spaces and vertical tab-list scrolling still works.
- [ ] 3.4 Verify split indicators for single-pane, two-pane horizontal, two-pane vertical, complex split, focus switching, and accessibility activation.
- [ ] 3.5 Run `openspec validate --all --strict` before PR.
