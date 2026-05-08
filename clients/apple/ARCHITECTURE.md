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
| `AlanNativeApp.swift` | 31 | SwiftUI; macOS gates | Thin app entry and scene composition | `App/` |
| `App/AlanMacAppDelegate.swift` | 14 | AppKit; macOS gates | Reopen handling for the primary Alan window | `App/` |
| `App/AlanMacAppStartup.swift` | 19 | Darwin; macOS gates | Duplicate-instance startup and singleton guard handling | `App/` |
| `App/AlanMacPrimaryShellOwner.swift` | 20 | Foundation, SwiftUI; macOS gates | Primary `window_main` shell owner creation | `App/` |
| `App/AlanMacPrimaryWindowPresenter.swift` | 18 | AppKit; macOS gates | Primary Alan window focusing and activation | `App/` |
| `App/AlanMacShellCommands.swift` | 85 | SwiftUI; macOS gates | App menu and keyboard command definitions routed through shell workspace commands | `App/` |
| `AlanAppSingletonGuard.swift` | 141 | Foundation, AppKit, Darwin; macOS gates | OS-backed duplicate-instance guard | `App/` or `Support/Windowing/` |
| `Support/ShellDesignTokens.swift` | 211 | AppKit, SwiftUI; macOS gates | Shell palette, corner radii, and native material wrapper | `Support/` |
| `Support/ShellWindowPlacement.swift` | 202 | AppKit, SwiftUI; macOS gates | Hidden-titlebar placement, min-size, traffic-light metrics, and primary window activation | `Support/` |
| `MacShellRootView.swift` | 1787 | SwiftUI; macOS gates | Shell root layout, sidebar, command UI, and voice command UI | `Views/Shell/` |
| `TerminalPaneView.swift` | 806 | SwiftUI; macOS gates | Split-tree and pane leaf rendering | `Views/Shell/Terminal/` |
| `TerminalHostView.swift` | 1442 | AppKit, SwiftUI, QuartzCore, GhosttyKit; macOS gates | AppKit terminal host bridge, focus, input routing, overlays, runtime attachment | `Views/Shell/Terminal/` plus terminal collaborators |
| `GhosttyLiveHost.swift` | 896 | Foundation, AppKit, GhosttyKit; macOS/Ghostty gates | Ghostty canvas bridge and wakeup/occlusion integration | `Services/Terminal/` or `Support/TerminalBridge/` |
| `TerminalHostRuntime.swift` | 636 | Foundation; macOS gates | Terminal host runtime protocols and fallback runtime state | `Services/Terminal/` |
| `TerminalRuntimeRegistry.swift` | 172 | SwiftUI, AppKit; macOS gates | Pane-keyed terminal host/runtime registry | `Services/Terminal/` |
| `TerminalRuntimeService.swift` | 1054 | Foundation, AppKit, GhosttyKit; macOS/Ghostty gates | Window-scoped terminal runtime service and Ghostty bootstrap | `Services/Terminal/` |
| `TerminalSurfaceController.swift` | 1428 | Foundation, AppKit, GhosttyKit; macOS/Ghostty gates | Terminal input, pointer, scrollback, search, and surface adapters | `Services/Terminal/` |
| `ShellModel.swift` | 2145 | Foundation | Shell IDs, panes, tabs, spaces, split tree, snapshots, mutations, persistence shims | `Models/Shell/` |
| `ShellHostController.swift` | 1960 | Foundation, AppKit, SwiftUI; macOS gates | Observable shell controller, persistence, boot profiles, runtime projection, command routing | `Controllers/Shell/` plus service collaborators |
| `ShellControlPlane.swift` | 2105 | Foundation, Darwin; macOS gates | Protocol DTOs, socket server, file polling, local executor, state merging, persistence, diagnostics | `Services/ControlPlane/` plus `Models/ControlPlane/` |
| `AlanAPIClient.swift` | 764 | Foundation | Daemon API DTOs and HTTP client | `Services/Daemon/` plus `Models/API/` |
| `ContentView.swift` | 1960 | SwiftUI, AppKit; iOS/macOS gates | Legacy/mobile console UI, console view model state, daemon event polling and reduction | `Views/Console/`, `Models/Console/`, and `Services/Daemon/` |

## Target Layout

The accepted target under `clients/apple/AlanNative` is:

- `App/`: `AlanNativeApp`, app delegate, duplicate-instance startup, primary
  shell owner creation, app commands, and primary window coordination.
- `Views/Shell/`: the default macOS shell composition, sidebar, workspace,
  command palette, pane title/search UI, and shell-specific SwiftUI components.
- `Views/Console/`: mobile or legacy remote-control console screens and view
  models that are not the primary macOS shell path.
- `Models/`: API DTOs, shell snapshots, shell IDs, enums, value types, and
  compatibility decoding shims.
- `Controllers/`: observable app and shell controllers that own UI state and
  delegate IO or domain work to services.
- `Services/`: daemon API clients, event readers/reducers, terminal runtime
  services, Ghostty bootstrap, shell control plane, socket server, persistence,
  and other process or IO code.
- `Support/`: design tokens, formatting helpers, window placement, AppKit
  adapters, and small utilities.

## Apply Sequence Notes

- Start with report-mode checks and pure model/support moves.
- Keep behavior changes out of mechanical move commits.
- Rebase UI slices around `polish-macos-search-remove-inspector`,
  `add-macos-pane-title-bars`, and `normalize-macos-shell-corner-radii` before
  splitting `MacShellRootView.swift`.
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
