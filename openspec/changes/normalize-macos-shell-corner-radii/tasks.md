## 1. Radius Inventory And Tokens

- [x] 1.1 Confirm which Apple client surfaces are active default macOS shell UI versus legacy or non-primary surfaces.
- [x] 1.2 Add a shared active-shell radius token surface near `ShellPalette` with `control = 6`, `row = 8`, `surface = 10`, and `overlay = 12`.
- [x] 1.3 Document allowed shape exceptions for semantic circles and any rare semantic pills.

## 2. Active Shell UI Normalization

- [x] 2.1 Replace hard-coded active-shell rounded rectangle radii in `MacShellRootView.swift` with the shared radius tokens.
- [x] 2.2 Replace hard-coded active-shell rounded rectangle radii in `TerminalPaneView.swift` with the shared radius tokens.
- [x] 2.3 Align normal-flow AppKit fallback radii in `TerminalHostView.swift` with the shared radius scale where those surfaces are visible in default shell flows.
- [x] 2.4 Replace decorative `Capsule` usage in default shell text chips, keycap hints, metadata chips, and command badges with restrained rounded rectangles unless documented as semantic pills.
- [x] 2.5 Ensure `add-macos-pane-title-bars` implementation uses the same radius tokens for pane title bars and close controls.

## 3. Guardrails

- [x] 3.1 Add or extend a focused shell contract check that flags active-shell `RoundedRectangle(cornerRadius: 14+)` and AppKit `cornerRadius >= 14` unless allowlisted.
- [x] 3.2 Add or extend a focused shell contract check or review checklist for new default-shell `Capsule` usage.
- [x] 3.3 Keep the check scoped to active shell files so legacy/non-primary surfaces are not rewritten implicitly.

## 4. Verification

- [x] 4.1 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [x] 4.2 Run `git diff --check`.
- [x] 4.3 Run `openspec validate normalize-macos-shell-corner-radii --type change --strict --json`.
- [x] 4.4 Run `openspec validate --all --strict --json`.
- [x] 4.5 Build the macOS app with the documented `AlanNative` Debug command.
- [x] 4.6 Capture or document light-mode screenshots for sidebar, single/split terminal, command palette, and remaining default-shell overlays after normalization.
- [x] 4.7 Manually verify the shell still feels native, readable, and not visually flat after reducing radii.

## 5. Archive Readiness

- [x] 5.1 Sync accepted radius requirements into `openspec/specs/` before archive.
- [x] 5.2 Record radius inventory, exceptions, validation commands, and visual evidence in the change before archive.
