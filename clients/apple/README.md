# Alan Native Client (SwiftUI)

`clients/apple` is Alan's native Apple client project, supporting macOS and iOS.

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

## Current Features (v0.1)

### Desktop (macOS)

- Sidebar workspace layout (connection panel + session list + new session config)
- Session management: create, switch, refresh, fork, delete
- Runtime controls: interrupt / compact / rollback
- Chat and steering: `Op::Turn` and `Op::Input`
- Event timeline: turn/tool/yield/warning/error
- Yield interactions:
  - confirmation (approve/reject/modify)
  - structured_input (form response)
  - custom/dynamic (manual JSON resume)
- History recovery: reads `/read` on switch and keeps incremental sync via `/events/read`

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
- `POST /sessions/{id}/rollback`: rollback turns
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
