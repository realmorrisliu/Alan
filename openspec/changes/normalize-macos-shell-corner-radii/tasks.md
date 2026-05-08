## 1. Radius Inventory And Tokens

- [ ] 1.1 Confirm which Apple client surfaces are active default macOS shell UI versus legacy or non-primary surfaces.
- [ ] 1.2 Add a shared active-shell radius token surface near `ShellPalette` with `control = 6`, `row = 8`, `surface = 10`, and `overlay = 12`.
- [ ] 1.3 Document allowed shape exceptions for semantic circles and any rare semantic pills.

## 2. Active Shell UI Normalization

- [ ] 2.1 Replace hard-coded active-shell rounded rectangle radii in `MacShellRootView.swift` with the shared radius tokens.
- [ ] 2.2 Replace hard-coded active-shell rounded rectangle radii in `TerminalPaneView.swift` with the shared radius tokens.
- [ ] 2.3 Align normal-flow AppKit fallback radii in `TerminalHostView.swift` with the shared radius scale where those surfaces are visible in default shell flows.
- [ ] 2.4 Replace decorative `Capsule` usage in default shell text chips, keycap hints, metadata chips, and command badges with restrained rounded rectangles unless documented as semantic pills.
- [ ] 2.5 Ensure `add-macos-pane-title-bars` implementation uses the same radius tokens for pane title bars and close controls.

## 3. Guardrails

- [ ] 3.1 Add or extend a focused shell contract check that flags active-shell `RoundedRectangle(cornerRadius: 14+)` and AppKit `cornerRadius >= 14` unless allowlisted.
- [ ] 3.2 Add or extend a focused shell contract check or review checklist for new default-shell `Capsule` usage.
- [ ] 3.3 Keep the check scoped to active shell files so legacy/non-primary surfaces are not rewritten implicitly.

## 4. Verification

- [ ] 4.1 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 4.2 Run `git diff --check`.
- [ ] 4.3 Run `openspec validate normalize-macos-shell-corner-radii --type change --strict --json`.
- [ ] 4.4 Run `openspec validate --all --strict --json`.
- [ ] 4.5 Build the macOS app with the documented `AlanNative` Debug command.
- [ ] 4.6 Capture or document light-mode screenshots for sidebar, single/split terminal, command palette, and inspector after normalization.
- [ ] 4.7 Manually verify the shell still feels native, readable, and not visually flat after reducing radii.

## 5. Archive Readiness

- [ ] 5.1 Sync accepted radius requirements into `openspec/specs/` before archive.
- [ ] 5.2 Record radius inventory, exceptions, validation commands, and visual evidence in the change before archive.
