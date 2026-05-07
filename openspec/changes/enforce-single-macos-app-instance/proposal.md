## Why

Alan's macOS app currently uses a multi-window SwiftUI scene shape, so users can
create duplicate Alan windows and forced launches can start more than one app
process. The native shell experience should behave like one focused workspace:
one running Alan app instance and one primary shell window.

## What Changes

- Add a macOS app lifecycle contract that guarantees a single running Alan app
  instance for the native bundle.
- Replace the current multi-window main scene policy with one primary shell
  window that is focused or reopened rather than duplicated.
- Disable or replace user-facing window creation paths such as New Window and
  `Command-N` for the primary Alan shell surface.
- Define forced second-launch behavior: the second process activates the
  existing app and exits before creating windows or shell runtime state.
- Add lifecycle verification for repeated launch, forced launch, Dock/app
  activation, New Window commands, normal quit, and startup-lock release.

## Capabilities

### New Capabilities
- `macos-app-instance-lifecycle`: Defines the native macOS app singleton,
  primary window, activation, reopen, quit, and verification requirements.

### Modified Capabilities
- `macos-shell-control-plane-reliability`: The existing multi-window isolation
  requirement is replaced with a single primary-window control-plane ownership
  requirement so duplicate windows/processes cannot create competing shell
  state.

## Impact

- Apple client app entry point and scene declaration:
  `clients/apple/AlanNative/AlanNativeApp.swift`.
- New macOS-only lifecycle support for startup locking, existing-instance
  activation, and menu/window command policy.
- Possible focused Apple-client tests or verification scripts for singleton lock
  behavior and window-count checks.
- Manual verification notes for `open Alan.app`, `open -n Alan.app`,
  `Command-N`, Dock reopen, close/reopen, and `Command-Q`.
