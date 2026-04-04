# Alan Shell macOS Spike Plan

> Status: maintainer note for `#199` substrate spike.

## Current Starting Point

The current Apple client is a remote-control-first SwiftUI app:

1. `clients/apple/AlanNative/AlanNativeApp.swift` mounts a single `ContentView`.
2. `clients/apple/AlanNative/ContentView.swift` is a session/timeline/chat controller for `/api/v1/sessions/*`.
3. `clients/apple/AlanNative/AlanAPIClient.swift` speaks the daemon compatibility API, not a terminal-host control plane.

This means the macOS shell spike should not start by modifying the current chat
surface into a terminal emulator. It should introduce a new macOS-first host
path.

## Spike Goal

Prove that a native macOS host can:

1. own a terminal surface,
2. boot directly into `alan-tui`,
3. produce a structured shell snapshot locally.

The spike does not need to prove full sidebar UX, persistence, or voice.

## Recommended Entry Point

1. Keep the current cross-platform SwiftUI remote-control client intact for now.
2. Add a macOS-only root path that can host a future terminal shell scene.
3. Introduce shell-model types before integrating `libghostty`.
4. Start with one window, one surface, one pane, and a fake or stub shell-state
   query if needed.
5. Replace the fake pane host with the `libghostty` host once the window and
   lifecycle shape are stable.

Current progress in the spike branch:

1. macOS now mounts a dedicated shell root instead of the old remote-control UI,
2. shell object-model types are in place and can emit the canonical snapshot,
3. the pane host is now a real AppKit bridge that reports focus and sizing
   lifecycle back to the shell host,
4. selected panes now materialize an explicit `alan-tui` boot profile.

## Suggested Implementation Slices

### Slice 1: macOS shell root

Add a macOS-specific app/root split so the macOS target can mount a dedicated
shell host without disrupting the current iOS client.

Exit criteria:

1. The macOS target can boot into a shell-host root.
2. iOS behavior remains unchanged.

### Slice 2: shell object-model types

Add local types for:

1. `ShellWindow`
2. `ShellSpace`
3. `ShellSurface`
4. `ShellPaneTree`
5. `ShellPane`
6. `AttentionState`

Exit criteria:

1. The app can materialize the minimal shell snapshot from local state.

### Slice 3: pane host container

Add a macOS pane container that can later be backed by `libghostty`.

Exit criteria:

1. One pane host is visible in a native macOS window.
2. Focus and resize lifecycle are under app control.

### Slice 4: `alan-tui` boot path

Wire the initial pane host to boot into `alan-tui`.

Exit criteria:

1. The macOS host opens directly into a terminal surface running `alan-tui`.

Maintainer note:

1. Use `clients/apple/scripts/setup-local-ghosttykit.sh` to stage or build a
   local `GhosttyKit.xcframework`, cache it outside the repo, and create the
   ignored `clients/apple/*` links before attempting the live `libghostty`
   embed.

### Slice 5: local shell snapshot

Expose a local debug or developer path that returns the canonical shell snapshot
from `docs/spec/alan_shell_macos_contract.md`.

Exit criteria:

1. One command or debug action can print the current shell snapshot as JSON.

## Recommended File Direction

The exact names can change, but the first spike will likely need a structure
closer to:

1. `clients/apple/AlanNative/AlanNativeApp.swift`
2. `clients/apple/AlanNative/MacShellRootView.swift`
3. `clients/apple/AlanNative/ShellModel.swift`
4. `clients/apple/AlanNative/ShellHostController.swift`
5. `clients/apple/AlanNative/TerminalPaneView.swift`

## Explicit Non-Goals For The Spike

1. Do not rebuild the full sidebar first.
2. Do not mix the shell control-plane contract into the existing session API
   types.
3. Do not block on browser surfaces, voice, or iOS companion work.
4. Do not promise full restore or process resurrection.
