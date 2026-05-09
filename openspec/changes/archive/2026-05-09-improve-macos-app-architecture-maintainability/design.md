## Context

The Apple client currently has useful architectural concepts but weak physical
organization. The macOS shell has a dedicated state model, host controller,
terminal runtime registry/service, surface controller, control plane, and
Ghostty bridge, but those concerns are still expressed through large flat Swift
files:

- `MacShellRootView.swift` mixes theme tokens, SwiftUI shell layout, AppKit
  material wrappers, window placement, sidebar, command tab, and voice command
  UI. The inspector product surface has been removed, but the remaining shell
  root still needs smaller view ownership.
- `AlanNativeApp.swift` mixes the app entry point, singleton handling, primary
  shell owner, window focusing, app delegate, and shell commands.
- `ContentView.swift` still combines legacy/mobile console models, daemon event
  reduction, polling, view model state, and complete UI composition.
- `TerminalHostView.swift` needs an AppKit `NSView`, but that view now owns
  layout, focus, window observation, input routing, overlay copy, runtime
  publication, and Ghostty attachment.
- `ShellControlPlane.swift` combines protocol models, local command execution,
  socket serving, file polling, state merging, persistence, and diagnostics.

This change is a planning and contract change. It should enable later apply
work to split source ownership without changing the shell's user-visible
behavior or terminal/control-plane contracts.

## Goals / Non-Goals

**Goals:**

- Establish a stable source-layout target for the Apple client that matches
  README guidance and Xcode project organization.
- Make SwiftUI scene roots readable as composition rather than view-controller
  substitutes.
- Keep AppKit bridges necessary for macOS terminal behavior, but narrow and name
  their boundaries.
- Separate terminal runtime, control-plane, model mutation, daemon API, and
  event-reducer responsibilities into files that can be tested and reviewed
  independently.
- Isolate legacy/mobile console code from the primary macOS shell path.
- Define validation so future architecture refactors can be checked without
  requiring full visual or behavioral retesting for every file move.

**Non-Goals:**

- Do not redesign the macOS shell UI, change Arc-like layout goals, or alter
  visual styling as part of this proposal.
- Do not change daemon HTTP APIs, shell control-plane wire formats, Ghostty
  integration behavior, terminal lifecycle semantics, or split interaction
  behavior.
- Do not require one giant mechanical move that rewrites every Swift file in a
  single implementation PR.
- Do not remove the existing iOS/mobile console; only isolate it from the
  primary macOS shell architecture.

## Decisions

### Decision: Introduce architecture folders by responsibility

Target source organization should use stable top-level folders under
`clients/apple/AlanNative`:

- `App/`: `AlanNativeApp`, app delegate, singleton startup, command definitions,
  window presentation/coordinator code.
- `Views/Shell/`: SwiftUI shell root, sidebar, workspace, command palette,
  pane title/search UI, and other shell-specific SwiftUI components.
- `Views/Console/` or `LegacyConsole/`: mobile/remote-control console UI and
  view model code that is not the primary macOS shell.
- `Models/`: API DTOs, shell snapshots, shell IDs, enums, value types, and
  current-format decoding.
- `Stores/` or `Controllers/`: observable app/shell controllers that own view
  state and delegate domain work to services.
- `Services/`: daemon API client, event stream reader/reducer, terminal runtime
  service, Ghostty bootstrap, shell control plane, socket server, persistence,
  and other process or IO code.
- `Support/`: design tokens, formatting helpers, window placement, AppKit
  adapters, and small utilities.

Alternative considered: keep the current flat directory and only split large
files. That lowers initial churn but leaves the same navigation and ownership
problem for future contributors.

### Decision: Split behavior-preserving slices

Implementation should proceed in small, behavior-preserving slices:

1. Move pure models and support utilities first.
2. Move app startup, commands, and window coordination.
3. Split SwiftUI shell view components by visible responsibility.
4. Isolate mobile/legacy console code.
5. Split terminal host bridge collaborators.
6. Split control-plane socket, file polling, local executor, and protocol DTOs.
7. Add or update focused structural checks after each slice.

Alternative considered: perform a one-shot reorganization. That would make the
final tree cleaner sooner, but it would conflict with active macOS shell work
and make review risky.

### Decision: Treat AppKit as a narrow bridge, not a smell

The terminal host, visual effect materials, window placement, first-responder
handling, hit testing, and local socket interactions legitimately require
AppKit or Darwin APIs. The goal is not to remove AppKit. The goal is to prevent
AppKit objects from becoming ambient dependencies across unrelated SwiftUI
views.

Alternative considered: force all UI into SwiftUI wrappers. That would fight the
terminal/event ownership requirements already captured in macOS shell specs.

### Decision: Keep existing specs as behavior owners

This change owns maintainability policy and validation. Existing specs remain
owners for product behavior:

- `macos-shell-ui-ux-conformance` owns visual and interaction presentation.
- `macos-shell-terminal-lifecycle` owns terminal runtime continuity and event
  ownership.
- `macos-shell-control-plane-reliability` owns IPC and authoritative command
  behavior.
- `macos-shell-build-test-contract` owns focused verification gates.

Alternative considered: fold architecture requirements into the UI conformance
spec. That would blur source-structure maintainability with user-visible UI
quality.

## Risks / Trade-offs

- [Risk] File moves can create noisy diffs and Xcode project churn. →
  Mitigation: move by responsibility in small PRs and keep behavior changes out
  of mechanical reorganization commits.
- [Risk] Active macOS UI changes may touch the same large files. → Mitigation:
  sequence implementation slices after active proposal branches are merged or
  rebase each slice before making behavior edits.
- [Risk] Over-splitting can make simple UI changes require too many jumps. →
  Mitigation: split by durable feature ownership, not by every private helper.
- [Risk] Structural checks can become brittle. → Mitigation: make checks focus
  on high-signal boundaries such as forbidden root-view ownership, file-size
  hotspots, and README/project layout drift, while leaving local naming details
  to review.
- [Risk] Moving legacy/mobile console code could accidentally change iOS build
  behavior. → Mitigation: keep iOS entry behavior intact and validate the Apple
  project after each console isolation slice.

## Migration Plan

1. Add or update architecture-focused scripts/checks that report current
   hotspots without requiring all code to be reorganized immediately.
2. Create the target folder structure and move pure model/support files with no
   behavior edits.
3. Update Xcode project membership and README structure in the same slice as
   each move.
4. Split SwiftUI shell views and app startup code after active UI polish changes
   are merged or rebased.
5. Split AppKit terminal/control-plane collaborators after focused shell
   contract tests are available for those boundaries.
6. Tighten architecture checks from warning/report mode to required gate once
   the largest hotspots have owners.

Rollback is straightforward for each slice: revert the file move or extraction
commit because behavior changes should be kept separate from structural moves.

## Open Questions

- Should the primary shell observable object continue to be named
  `ShellHostController`, or should future code distinguish `ShellStore`,
  `ShellCoordinator`, and `ShellRuntimeCoordinator`?
- Should legacy/mobile console code live under `Views/Console/` or a clearer
  `LegacyConsole/` namespace until the iOS product direction is revisited?
- Should the architecture check be implemented as a Swift script, shell script,
  or a small SwiftPM helper if it grows beyond simple path/size/import checks?
