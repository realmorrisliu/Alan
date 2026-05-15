# Manual Verification

Date: 2026-05-15

## Automated Verification

- `clients/apple/scripts/test-shell-window-placement.sh` passed with focused coverage for collapsed-sidebar pointer retention across the adjacent left resize frame.
- `clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh` passed.
- `bash clients/apple/scripts/check-brand-identity.sh` passed after the brand scan was run from the repository root so the existing relative exclusions apply.
- `bash clients/apple/scripts/check-shell-contracts.sh` passed.
- `openspec validate refine-macos-sidebar-interactions --strict` passed.
- `openspec validate --all --strict` passed with `29 passed, 0 failed`.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination 'platform=macOS' build` failed before compilation because the sandbox could not write the default DerivedData folder under `~/Library/Developer/Xcode/DerivedData`.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination 'platform=macOS' -derivedDataPath target/xcode-derived build` passed. Xcode still printed CoreSimulator availability warnings, but the macOS build completed successfully.
- `git diff --check` passed.

## Manual Visual Verification Status

- Pinned sidebar collapse/expand: not performed in this run.
- Floating sidebar reveal/hide: not performed in this run.
- Tab click persistence: not performed in this run.
- Space click persistence: not performed in this run.
- Sidebar-local space swipe pager motion: not performed in this run.
- Visible-frame-zoomed collapsed-sidebar left-edge retention: covered by focused AppKit geometry tests; live visual verification not performed in this run.

The remaining human acceptance work is to launch the built app and visually verify those five interactions in the macOS shell. In particular, confirm that sidebar-local swipe motion moves only the active-space header and tab list while the command launcher, bottom space dock, sidebar chrome, traffic lights, and terminal workspace remain fixed.

## Archive Readiness

Before archiving, sync the active delta into the long-lived specs with the new sidebar-local swipe semantics. The archived contract should describe `ShellSidebarSwipeMonitor` as the sidebar input adapter, `ShellSidebarSpaceContentPagerState` as the local pager state, `ShellSidebarView` as the owner of pager rendering, and `MacShellRootView` as a stable sidebar/workspace layout without full-window space paging.
