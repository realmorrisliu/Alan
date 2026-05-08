## Verification Evidence

- `bash clients/apple/scripts/check-shell-contracts.sh` passed.
- `git diff --check` passed.
- `openspec validate normalize-macos-shell-corner-radii --type change --strict --json` passed.
- `openspec validate --all --strict --json` passed.
- `xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build` passed.

## Manual Notes

- Launched `target/xcode-derived/Build/Products/Debug/Alan.app` in light mode.
- Confirmed sidebar rows, command launcher, terminal surround, command palette, pane title bars, and Find bar use the smaller restrained radius scale.
- Confirmed active default shell files no longer contain large ad hoc numeric rounded rectangle radii or decorative `Capsule` chrome.
- Confirmed semantic `Circle` use remains limited to status/attention indicators.
- Confirmed the shell remains readable and native rather than visually flat after the radius reduction.

## Build Warnings

- Xcode reported a CoreSimulator version warning: current `1051.49.0`, build `1051.50.0`. The macOS Debug build still succeeded.
