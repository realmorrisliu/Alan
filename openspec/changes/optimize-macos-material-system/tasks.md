## 1. Material Roles

- [x] 1.1 Inventory active shell material, background, button, hover, and overlay fills in the Apple client.
- [x] 1.2 Define semantic material/control roles in the shell support layer for window, sidebar, workspace, terminal surround, floating overlay, and compact controls.
- [x] 1.3 Keep AppKit visual-effect bridge code isolated behind reusable support wrappers.

## 2. Active Shell Application

- [x] 2.1 Apply the semantic roles to `MacShellRootView.swift` and workspace backgrounds.
- [x] 2.2 Apply the sidebar/control roles to `ShellSidebarView.swift` without changing sidebar information architecture.
- [x] 2.3 Apply terminal surround roles to active terminal shell chrome without reducing terminal text contrast.
- [x] 2.4 Apply overlay/control roles to active floating shell surfaces without redesigning command input behavior.
- [x] 2.5 Align active shell radius and shadow usage to the semantic elevation scale, keeping the approved terminal surface treatment as the baseline.

## 3. Verification

- [x] 3.1 Run focused Apple build or shell UI checks affected by the changed files.
- [x] 3.2 Capture screenshot or manual review notes for light-mode sidebar, terminal, command entry, controls, and overlays.
- [x] 3.3 Review reduced-transparency or increased-contrast behavior, or document why local verification was unavailable.
- [x] 3.4 Run `openspec validate --all --strict` before PR.
- [x] 3.5 Re-run focused shell/OpenSpec/build verification after the elevation-scale polish pass.

### Verification Notes

- 2026-05-12: Built `AlanNative` Debug for macOS after the material-role changes.
- 2026-05-12: Captured light-mode default shell review screenshot at `/tmp/alan-ui-polish-material-default.png`; sidebar, terminal content area, compact controls, and the unified window backdrop were reviewed together.
- 2026-05-12: Captured command input review screenshot at `/tmp/alan-ui-polish-command-input-command-p-retry.png`; command entry, floating overlay material, and terminal scrim treatment were reviewed together.
- 2026-05-12: Reduced-transparency behavior is handled in the shared material wrappers by disabling AppKit visual-effect material when `accessibilityReduceTransparency` is active and falling back to solid semantic fills. Increased-contrast behavior is handled by `NSWorkspace.shared.accessibilityDisplayShouldIncreaseContrast`, which strengthens fills and strokes for control, overlay, panel, and terminal chrome roles. I did not toggle global macOS accessibility settings during the run.
- 2026-05-12: Checked active shell files for escaped one-off `.ultraThinMaterial`, `Color.white.opacity(...)`, or direct shell background/control fills; the remaining direct palette match is the space-rail attention-dot stroke, not a material or translucent background fill.
- 2026-05-13: Extended the material-system spec with semantic elevation requirements. The approved terminal surface treatment now anchors the hierarchy; selected navigation, floating inputs, floating panels, and command palette shadows use shared adaptive elevation tokens while static titlebar and command launcher controls stay shadowless.
- 2026-05-13: Verified the elevation polish with `git diff --check`, `openspec validate optimize-macos-material-system --type change --strict`, `bash clients/apple/scripts/check-shell-contracts.sh`, and `xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -derivedDataPath target/xcode-derived build`. The Xcode build passed with the existing CoreSimulator version warning.
