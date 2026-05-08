## Verification Evidence

- `clients/apple/scripts/test-terminal-surface-controller.sh` passed.
- `clients/apple/scripts/test-shell-runtime-metadata.sh` passed.
- `clients/apple/scripts/test-shell-split-model.sh` passed.
- `bash clients/apple/scripts/check-shell-contracts.sh` passed.
- `git diff --check` passed.
- `openspec validate polish-macos-search-remove-inspector --type change --strict --json` passed.
- `openspec validate --all --strict --json` passed.
- `xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build` passed.

## Manual Notes

- Launched `target/xcode-derived/Build/Products/Debug/Alan.app` in light mode.
- Confirmed the default shell no longer shows the right-side inspector or inspector controls.
- Confirmed `Command-F` opens the pane-scoped SwiftUI Find bar, focuses the query field, query text highlights terminal matches, and Escape dismisses the bar back to terminal focus.
- Confirmed split-pane Find opens on the selected pane and no passive terminal search overlay appears.

## Build Warnings

- Xcode reported a CoreSimulator version warning: current `1051.49.0`, build `1051.50.0`. The macOS Debug build still succeeded.
