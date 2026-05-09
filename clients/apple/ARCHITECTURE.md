# Apple Client Architecture Maintainability

This document records the current Apple client source ownership baseline and the
target layout for behavior-preserving refactor slices. It is intentionally about
maintainability, not product behavior.

## Current Inventory

The Apple client is being migrated out of the original flat
`clients/apple/AlanNative` source directory. Files listed with owner folders are
already split into the target layout and remain members of the `AlanNative`
Xcode target.

| File | Lines | Platform / bridge imports | Primary responsibility today | Target owner |
| --- | ---: | --- | --- | --- |
| `AlanNativeApp.swift` | 34 | SwiftUI; macOS gates | Thin app entry and scene composition | `App/` |
| `App/AlanMacAppDelegate.swift` | 13 | AppKit; macOS gates | Reopen handling for the primary Alan window | `App/` |
| `App/AlanMacAppStartup.swift` | 19 | Darwin; macOS gates | Duplicate-instance startup and singleton guard handling | `App/` |
| `App/AlanMacPrimaryShellOwner.swift` | 21 | Foundation, SwiftUI; macOS gates | Primary `window_main` shell owner creation | `App/` |
| `App/AlanMacPrimaryWindowPresenter.swift` | 20 | AppKit; macOS gates | Primary Alan window focusing and activation | `App/` |
| `App/AlanMacShellCommands.swift` | 91 | SwiftUI; macOS gates | App menu and keyboard command definitions routed through shell workspace commands | `App/` |
| `AlanAppSingletonGuard.swift` | 141 | Foundation, AppKit, Darwin; macOS gates | OS-backed duplicate-instance guard | `App/` or `Support/Windowing/` |
| `Support/ShellDesignTokens.swift` | 200 | AppKit, SwiftUI; macOS gates | Shell palette, corner radii, and native material wrapper | `Support/` |
| `Support/ShellWindowPlacement.swift` | 205 | AppKit, SwiftUI; macOS gates | Hidden-titlebar placement, min-size, traffic-light metrics, and primary window activation | `Support/` |
| `Support/ShellVoiceCommandController.swift` | 63 | AppKit, SwiftUI; macOS gates | Narrow speech-recognizer bridge for command palette voice actions | `Support/` |
| `MacShellRootView.swift` | 63 | SwiftUI; macOS gates | Thin primary shell composition root | `Views/Shell/` |
| `Views/Shell/ShellSidebarView.swift` | 538 | SwiftUI; macOS gates | Primary shell sidebar, tab rows, space dock, and sidebar state | `Views/Shell/` |
| `Views/Shell/ShellWorkspaceView.swift` | 46 | SwiftUI; macOS gates | Shell workspace composition and space keyboard shortcuts | `Views/Shell/` |
| `Views/Shell/ShellCommandTabView.swift` | 621 | SwiftUI; macOS gates | Command palette search, routing, attention, and action presentation | `Views/Shell/` |
| `TerminalPaneView.swift` | 1002 | SwiftUI; macOS gates | Split-tree and pane leaf rendering | `Views/Shell/Terminal/` |
| `TerminalHostView.swift` | 1376 | AppKit, SwiftUI, QuartzCore, GhosttyKit; macOS gates | AppKit terminal host bridge, focus, overlay composition, runtime attachment, and collaborator wiring | `Views/Shell/Terminal/` plus terminal collaborators |
| `GhosttyLiveHost.swift` | 896 | Foundation, AppKit, GhosttyKit; macOS/Ghostty gates | Ghostty canvas bridge and wakeup/occlusion integration | `Services/Terminal/` or `Support/TerminalBridge/` |
| `TerminalHostRuntime.swift` | 636 | Foundation; macOS gates | Terminal host runtime protocols and fallback runtime state | `Services/Terminal/` |
| `Services/Terminal/TerminalHostRuntimeReporter.swift` | 47 | Foundation; macOS gates | Runtime snapshot deduplication and main-queue publication for terminal host updates | `Services/Terminal/` |
| `Services/Terminal/TerminalHostWindowObserver.swift` | 55 | AppKit; macOS gates | Terminal host window key, screen, and occlusion notification ownership | `Services/Terminal/` |
| `TerminalRuntimeRegistry.swift` | 194 | SwiftUI, AppKit; macOS gates | Pane-keyed terminal host/runtime registry | `Services/Terminal/` |
| `TerminalRuntimeService.swift` | 1054 | Foundation, AppKit, GhosttyKit; macOS/Ghostty gates | Window-scoped terminal runtime service and Ghostty bootstrap | `Services/Terminal/` |
| `TerminalSurfaceController.swift` | 1424 | Foundation, AppKit, GhosttyKit; macOS/Ghostty gates | Terminal input, pointer, scrollback, search, and surface adapters | `Services/Terminal/` |
| `Models/Shell/ShellValueTypes.swift` | 210 | Foundation | Shell command enums, launch targets, process bindings, and context snapshots | `Models/Shell/` |
| `Models/Shell/ShellSnapshots.swift` | 517 | Foundation | Shell panes, tabs, spaces, split tree, state snapshots, and snapshot query helpers | `Models/Shell/` |
| `Models/Shell/ShellTreeMutations.swift` | 198 | Foundation | Split-tree resizing, equalization, split, removal, and attachment helpers | `Models/Shell/` |
| `Models/Shell/ShellStateMutations.swift` | 1034 | Foundation | Shell bootstrap defaults, state mutation result/error types, mutation helpers, and preview fixtures | `Models/Shell/` |
| `ShellModel.swift` | 169 | Foundation | Shell title, label, and status presentation helpers | `Models/Shell/` or `Support/ShellPresentation/` |
| `ShellHostController.swift` | 1632 | Foundation, AppKit, SwiftUI; macOS gates | Observable shell controller, runtime update intake, command routing, control-plane command handling | `Controllers/Shell/` plus service collaborators |
| `Services/Shell/ShellControlFilePoller.swift` | 182 | Foundation; macOS gates | File-backed command/result polling and Alan binding-file projection | `Services/Shell/` |
| `Services/Shell/ShellDiagnostics.swift` | 16 | Foundation; macOS gates | Shell service diagnostic routing | `Services/Shell/` |
| `Services/Shell/ShellEventStore.swift` | 298 | Foundation; macOS gates | Shell event buffering, diffing, `events.read`, and jsonl persistence | `Services/Shell/` |
| `Services/Shell/ShellLocalCommandExecutor.swift` | 706 | Foundation; macOS gates | Local shell control command execution against shell state | `Services/Shell/` |
| `Services/Shell/ShellPaneProjectionService.swift` | 266 | Foundation; macOS gates | Pane boot context, runtime metadata, viewport, attention, and Alan binding projection | `Services/Shell/` |
| `Services/Shell/ShellPublishedStateMerger.swift` | 158 | Foundation; macOS gates | Merge published shell state with authoritative runtime metadata | `Services/Shell/` |
| `Services/Shell/ShellSocketServer.swift` | 397 | Foundation, Darwin; macOS gates | Bounded local socket transport, request parsing, and client response handling | `Services/Shell/` |
| `Services/Shell/ShellStatePersistenceStore.swift` | 116 | Foundation; macOS gates | Shell state save/restore, persistence URL selection, and restored window context lookup | `Services/Shell/` |
| `ShellControlPlane.swift` | 253 | Foundation; macOS gates | Shell control-plane orchestration across socket, file polling, state publishing, pane support directories, event store, and diagnostics | `Services/Shell/` |
| `Models/API/DaemonAPIModels.swift` | 529 | Foundation | Daemon API response DTOs, operation payloads, JSON values, and API error type | `Models/API/` |
| `Models/Console/ConsoleModels.swift` | 148 | Foundation | Console chat messages, timeline entries, structured questions, and pending-yield value state | `Models/Console/` |
| `Services/Daemon/AlanAPIClient.swift` | 236 | Foundation | Daemon HTTP client, request construction, endpoint routing, and response validation | `Services/Daemon/` |
| `Services/Daemon/ConsoleEventReducer.swift` | 195 | Foundation | Console event page reader and event-to-message/timeline/pending-yield projection reducer | `Services/Daemon/` |
| `Views/Console/ContentView.swift` | 1708 | SwiftUI, AppKit; iOS/macOS gates | Legacy/mobile console UI, console view model state, and event pump coordination | `Views/Console/` and `Controllers/Console/` |

## Target Layout

The accepted target under `clients/apple/AlanNative` is:

- `App/`: `AlanNativeApp`, app delegate, duplicate-instance startup, primary
  shell owner creation, app commands, and primary window coordination.
- `Views/Shell/`: the default macOS shell composition, sidebar, workspace,
  command palette, pane title/search UI, and shell-specific SwiftUI components.
- `Views/Console/`: mobile or legacy remote-control console screens and view
  models that are not the primary macOS shell path.
- `Models/`: API DTOs, shell snapshots, shell IDs, enums, value types, and
  current-format decoding.
- `Controllers/`: observable app and shell controllers that own UI state and
  delegate IO or domain work to services.
- `Services/`: daemon API clients, event readers/reducers, terminal runtime
  services, Ghostty bootstrap, shell projection services, shell control plane,
  socket server, persistence, and other process or IO code.
- `Support/`: design tokens, formatting helpers, window placement, AppKit
  adapters, and small utilities.

## Apply Sequence Notes

- Start with report-mode checks and pure model/support moves.
- Keep behavior changes out of mechanical move commits.
- `polish-macos-search-remove-inspector` and
  `normalize-macos-shell-corner-radii` were archived before the shell-root
  split. Keep future UI behavior work, such as `add-macos-pane-title-bars`,
  rebased on top of the current shell component files instead of burying
  behavior changes inside architecture-only slices.
- Split terminal host and control-plane collaborators only with focused runtime
  or IPC script checks in the same slice.

## Validation

Run the architecture report directly:

```bash
bash clients/apple/scripts/check-architecture-maintainability.sh
```

The default mode reports known migration debt and fails only on narrow
regressions such as new root-level Swift files or Xcode project membership drift.
Use `--strict` when intentionally tightening the architecture gate.

## Implementation Evidence

The architecture-maintainability implementation was completed as behavior-
preserving PR slices. The final validation pass before syncing this spec ran:

- `bash clients/apple/scripts/test-terminal-runtime-service.sh`
- `bash clients/apple/scripts/test-terminal-surface-controller.sh`
- `bash clients/apple/scripts/test-shell-runtime-metadata.sh`
- `bash clients/apple/scripts/check-shell-contracts.sh`
- `bash clients/apple/scripts/check-architecture-maintainability.sh`
- `git diff --check`
- `openspec validate improve-macos-app-architecture-maintainability --type change --strict --json`
- `openspec validate --all --strict --json`
- `xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build`

The macOS build succeeded. Local Xcode continued to print the existing
CoreSimulator version warning while building for `platform=macOS`; simulator
device support was not required for this validation.

## Remaining Architecture Debt

`check-architecture-maintainability.sh` currently completes in report mode with
eight known warnings:

- `ShellHostController.swift` remains large and still imports AppKit outside a
  final narrow controller/service split.
- `TerminalHostView.swift` remains large pending additional terminal-host
  collaborator extraction.
- `TerminalRuntimeRegistry.swift` still imports AppKit before it is moved under
  a terminal service or bridge owner.
- `TerminalSurfaceController.swift` remains large pending deeper terminal
  surface adapter splits.
- `Views/Console/ContentView.swift` remains large and imports AppKit because the
  legacy/mobile console path is isolated but not fully decomposed.
- `Controllers/` is documented as the target owner for observable controllers
  but has not been introduced yet.

The current architecture gate intentionally keeps those warnings non-blocking
while failing narrower regressions such as new root-level Swift files, project
membership drift, or reintroduced control-plane ownership in the wrong file.
