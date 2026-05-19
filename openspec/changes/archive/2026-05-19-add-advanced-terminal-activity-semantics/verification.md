# Verification Notes

Date: 2026-05-18

## A-Group Visual Review Evidence

This pass used focused projection tests and source-level UI review rather than a
saved running-app screenshot. The debug macOS app build passed, so the SwiftUI
surface compiles with the reviewed projections.

- Focused progress: `verifiesPaneTitleActivityAccessoryLabel` covers pane-local
  `Progress · 42%` title detail and freshness expiry without resizing title bars.
- Background agent needs input: `verifiesAgentActivityControlCommandProjectsOntoPane`,
  `verifiesTabSidebarActivityProjectionUsesHighestPriorityPane`, and
  `verifiesControllerRoutesActivityNotificationsOnce` cover Codex
  `Input needed` projection, tab-level selection, and notification routing.
- Command failure: `verifiesFocusedCommandFailureDemotesFromSidebarProjection`
  and `verifiesCommandFailureAcknowledgementSticksAfterFocus` cover failure
  visibility and focus acknowledgement.
- Cleared activity fallback: `verifiesClearingActivityRemovesPaneActivity` and
  `verifiesTabSidebarProjectionFallsBackToRepositoryBranch` cover returning from
  activity UI to worktree/branch context.
- Split leading topology: `verifiesSplitTabSelectionUsesStablePaneWithoutChangingLayout`,
  `bash clients/apple/scripts/test-shell-split-model.sh`, and the reviewed
  `ShellSidebarTabRow` split-summary leading slot cover the compact topology
  affordance and stable focus cycling.
- Hover close overlay: reviewed `ShellSidebarTabRow` hover state in
  `clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift`; the close
  control remains an overlay in the trailing row area and does not reserve a
  permanent text column.

## Commands Run

```bash
bash clients/apple/scripts/test-shell-runtime-metadata.sh
bash clients/apple/scripts/check-shell-contracts.sh
bash clients/apple/scripts/test-shell-sidebar-presentation.sh
bash clients/apple/scripts/test-shell-split-model.sh
xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan-macos -configuration Debug -destination platform=macOS -derivedDataPath /private/tmp/alan-xcode-derived-activity build
openspec validate add-advanced-terminal-activity-semantics --type change --strict --json
```

The first xcodebuild attempt used `target/xcode-derived-activity` and failed
before compilation because the local `target/` directory is owned by root. The
same build passed with `/private/tmp/alan-xcode-derived-activity`.
