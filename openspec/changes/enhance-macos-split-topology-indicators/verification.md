## Verification

- `./clients/apple/scripts/test-shell-split-model.sh`
  - Passed.
  - Covers two-pane directions, three columns, three rows, main-stack variants,
    four columns, 2 by 2 grid, focused-pane mapping, and complex-count fallback.

- `bash clients/apple/scripts/check-shell-contracts.sh`
  - Passed.
  - Confirms the topology projection lives in the testable model layer, the
    split-model test covers it, and complex indicators use `complexCountOverlay`
    instead of the previous side-by-side icon/count pattern.

- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination generic/platform=macOS -derivedDataPath /private/tmp/alan-xcode-derived-split-topology build`
  - Passed.
  - The documented `target/xcode-derived` path is currently root-owned in this
    checkout, so the build was rerun with a writable temporary derived-data path.

- `openspec validate "enhance-macos-split-topology-indicators" --type change --strict --json`
  - Passed.

- `openspec validate --all --strict --json`
  - Passed.
  - Validated 45 items: 14 changes and 31 specs.

- `git diff --check -- clients/apple/alan-macos/ShellModel.swift clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift clients/apple/scripts/test-shell-split-model.swift clients/apple/scripts/check-shell-contracts.sh openspec/changes/enhance-macos-split-topology-indicators`
  - Passed.

## Visual Notes

- Selected and unselected indicators keep the existing 22 by 18 point footprint
  and reuse the existing sidebar material container.
- Three-column and four-column layouts render as equal compact vertical segments.
- Three-row and four-row layouts render as equal compact horizontal segments.
- Main-stack layouts keep the main pane as the larger segment and render the
  opposite stack as two compact sibling segments.
- Grid layouts render as a 2 by 2 compact segment grid using the root split
  direction to preserve pane order.
- Complex layouts render as a single-pane-shaped base with the pane count
  overlaid inside the shape, not beside it.

`clients/apple/scripts/capture-alan-window.sh --list` could not capture running
window evidence in this shell because ScreenCaptureKit did not have Screen
Recording permission for the terminal session.
