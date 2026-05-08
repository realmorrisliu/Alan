## Verification Evidence

- `clients/apple/scripts/test-shell-split-model.sh` passed.
- `clients/apple/scripts/test-shell-runtime-metadata.sh` passed.
- `clients/apple/scripts/test-terminal-surface-controller.sh` passed.
- `bash clients/apple/scripts/check-shell-contracts.sh` passed.
- `git diff --check` passed.
- `openspec validate add-macos-pane-title-bars --type change --strict --json` passed.
- `openspec validate --all --strict --json` passed.
- `xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build` passed.

## Manual Notes

- Launched `target/xcode-derived/Build/Products/Debug/Alan.app` in light mode.
- Confirmed single-pane tabs show a compact title bar above the terminal canvas.
- Confirmed split panes show one title bar per visible pane, with the title bar outside the terminal host canvas.
- Confirmed the title-bar close affordance is a slim plain `xmark` with no drawn border or background.
- Confirmed clicking a pane title area focuses that pane without sending terminal text.

## Build Warnings

- Xcode reported a CoreSimulator version warning: current `1051.49.0`, build `1051.50.0`. The macOS Debug build still succeeded.
