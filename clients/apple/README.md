# Alan Native Client (SwiftUI)

`clients/apple` is Alan's native Apple client project, supporting macOS and iOS.

The macOS path is currently being reframed into Alan Shell: a real terminal app
whose shell is readable and operable by both humans and agents.

## System Requirements

- Xcode 16+
- macOS 15+ for development
- iOS 18+ simulator/device for iOS target

## Directory Structure

- `AlanNativeApp.swift`: app entry point
- `Views/`: UI views
- `State/`: app state and stores
- `Networking/`: daemon API and WebSocket client
- `Models/`: protocol data models
- `Resources/`: assets and app resources

## Quick Start

1. Open `clients/apple/AlanNative.xcodeproj` with Xcode
2. Select the `AlanNative` scheme
3. Select a run target: `My Mac` or an iOS simulator/device
4. Run the app

Default endpoint is `http://127.0.0.1:8090`; you can change it in the UI.

### Local Ghostty Prep

The macOS shell spike now includes a native AppKit terminal-host scaffold plus
a plain-shell-first boot contract. To prepare a local `GhosttyKit.xcframework`
for the next integration slice, run:

```bash
./clients/apple/scripts/setup-local-ghosttykit.sh
```

This follows the same boundary as `cmux`: Ghostty stays external, the script
syncs artifacts into a cache outside the repo, and then creates ignored local
links at `clients/apple/GhosttyKit.xcframework`,
`clients/apple/ghostty-resources`, and `clients/apple/ghostty-terminfo`.
It prefers explicit overrides first, then a local `~/Developer/ghostty`
checkout.

By default, the macOS app boots each new pane into your login shell. You can
override that boot contract with:

```bash
ALAN_SHELL_LOGIN_SHELL=/absolute/path/to/zsh
```

Or force a one-off startup command with:

```bash
ALAN_SHELL_BOOT_COMMAND='tmux attach || tmux new'
```

If you want an Alan-targeted surface to launch a specific Alan binary, set:

```bash
ALAN_SHELL_ALAN_PATH=/absolute/path/to/alan
```

Without that override, the macOS shell host resolves Alan in this order:

1. `ALAN_SHELL_ALAN_PATH`
2. worktree-local `target/debug/alan`
3. worktree-local `target/release/alan`
4. installed `~/.alan/bin/alan`
5. `alan` from the current `PATH`

### Window Capture Helper

For screenshot-driven UI iteration on the native macOS app, use:

```bash
zsh ./clients/apple/scripts/capture-alan-window.sh --list
zsh ./clients/apple/scripts/capture-alan-window.sh --output .artifacts/alan-window.png
```

You can also target a specific running process:

```bash
zsh ./clients/apple/scripts/capture-alan-window.sh --pid 12345 --output .artifacts/alan-window.png
```

The helper uses ScreenCaptureKit, so it may require Screen Recording permission
for your terminal on first use.

## Current Features (v0.1)

### Desktop (macOS)

- Alan Shell macOS root with Arc-like sidebar/workspace chrome
- Local typed shell snapshot preview
- Native AppKit terminal-host scaffold sized and focused by the shell host
- Plain-shell-first boot profile projection for the selected pane, with Alan as
  an explicit optional surface type
- Ghostty readiness discovery for local developer integration
- Live Ghostty-backed host path with runtime diagnostics, fallback config, and
  command-resolution inspection
- External Ghostty artifact cache plus ignored local links and app-bundled
  resources/terminfo
- Local file-backed shell control plane for `state`, `pane.focus`, and
  `pane.send_text`

### Mobile (iOS)

- Remote-control-first layout (Chat / Timeline dual panels)
- Same core controls as desktop:
  - connect to remote daemon
  - session switching and message submission
  - yield approval/input resume

## Protocol and Endpoints

The client uses the existing `/api/v1/sessions/*` compatibility layer:

- `POST /sessions`: create session
- `GET /sessions`: list sessions
- `POST /sessions/{id}/submit`: submit `Op`
- `GET /sessions/{id}/events/read`: incremental event polling
- `GET /sessions/{id}/read`: load session metadata + history
- `POST /sessions/{id}/fork`: fork session
- `POST /sessions/{id}/rollback`: rollback turns (in-memory only; non-durable)
- `POST /sessions/{id}/compact`: trigger compaction
- `DELETE /sessions/{id}`: delete session

## Command-Line Build

```bash
# macOS
xcodebuild \
  -project clients/apple/AlanNative.xcodeproj \
  -scheme AlanNative \
  -destination 'platform=macOS' build

# iOS
xcodebuild \
  -project clients/apple/AlanNative.xcodeproj \
  -scheme AlanNative \
  -destination 'platform=iOS Simulator,name=iPhone 16' build
```
