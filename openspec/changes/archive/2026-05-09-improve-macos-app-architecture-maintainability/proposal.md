## Why

The macOS app has grown from a shell spike into a real native terminal surface,
but its source organization has not caught up with the architecture. SwiftUI
scene roots, AppKit bridges, terminal runtime ownership, control-plane IPC,
legacy console UI, and design tokens are still concentrated in a small number
of large flat files, which makes routine UI and terminal work harder to review,
test, and sequence safely.

Several active macOS shell changes now touch the same files and concepts. This
change establishes a maintainability contract before more UI polish, search,
split, voice, and remote-workspace work increases the cost of untangling those
boundaries.

## What Changes

- Introduce a native Apple client architecture contract for source layout,
  SwiftUI scene composition, AppKit bridge ownership, service/model boundaries,
  and legacy/mobile isolation.
- Require the Apple client README and project structure to describe the same
  organization developers actually edit.
- Require large SwiftUI roots to move toward small feature views and extracted
  command/window coordination instead of accumulating view, window, command,
  and debug logic in one file.
- Require AppKit escape hatches such as window placement and terminal host
  views to be narrow, named, and separated from unrelated SwiftUI layout.
- Require terminal runtime, control-plane, socket, model mutation, and API/event
  reducer responsibilities to have explicit owning files or services.
- Preserve current product behavior while making future refactors easier to
  review and verify in focused slices.

## Capabilities

### New Capabilities

- `macos-app-architecture-maintainability`: Defines maintainable native Apple
  client source organization, SwiftUI/AppKit boundaries, service/model
  ownership, and validation expectations for macOS app architecture changes.

### Modified Capabilities

- `macos-shell-build-test-contract`: Add a focused architecture-maintainability
  validation gate for Apple client structure changes, separate from UI visual
  conformance and shell behavior tests.

## Impact

- Affected source areas: `clients/apple/AlanNative`, `clients/apple/README.md`,
  `clients/apple/AlanNative.xcodeproj/project.pbxproj`, and Apple-focused
  script checks under `clients/apple/scripts`.
- Expected future implementation areas include `App/`, `Views/`, `Models/`,
  `Services/`, `Support/`, terminal host bridge files, control-plane service
  files, and mobile or legacy console folders.
- No user-facing behavior, daemon API, shell control-plane protocol, Ghostty
  runtime behavior, or visual design change is intended by this proposal alone.
