## Verification

- `clients/apple/scripts/test-shell-runtime-metadata.sh` passed.
- `bash clients/apple/scripts/check-shell-contracts.sh` passed.
- `git diff --check -- clients/apple/alan-macos/TerminalPaneView.swift clients/apple/scripts/check-shell-contracts.sh clients/apple/scripts/test-shell-runtime-metadata.swift openspec/changes/polish-macos-pane-title-bar-readability` passed.
- `openspec validate polish-macos-pane-title-bar-readability --type change --strict --json` passed.
- `openspec validate --all --strict --json` passed.
- `xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS -derivedDataPath debug/xcode-derived-pane-title build` passed. The DerivedData path is repo-local and not under `/tmp` or `/private/tmp`.

## Notes

- `/private/tmp` no longer contains top-level `alan*` build directories after cleanup.
- Visual verification for the light-mode focused title, split-pane integration, and narrow accessory fallback was completed by user acceptance on 2026-05-19 after relaunch/restart. Temporary debug launches plus Computer Use were not treated as valid evidence for this UI change.
- Review follow-up restored the title bar's full-width background and focus hit area while keeping fit-content responsive content inside the row.
