## Why

The macOS architecture-maintainability work is complete and archived, but the
current architecture report still records seven non-blocking migration warnings.
Those warnings should now be reduced through focused, reviewable refactor slices
instead of remaining as open-ended debt.

## What Changes

- Reduce the existing architecture-maintainability warning count through
  behavior-preserving refactors.
- Keep each refactor slice scoped to one owner or warning class so stacked PRs
  remain reviewable and easy to rebase.
- Update `clients/apple/ARCHITECTURE.md` and the architecture validation
  expectation whenever a warning is resolved.
- Preserve the current product behavior, shell command vocabulary, terminal
  runtime identity, and console/mobile separation while moving code.
- Do not introduce new UI behavior, daemon API changes, or terminal runtime
  semantics as part of debt cleanup.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `macos-app-architecture-maintainability`: define how existing tracked
  architecture warnings are reduced, validated, and removed from the debt
  record.

## Impact

- `clients/apple/AlanNative/ShellHostController.swift`
- `clients/apple/AlanNative/TerminalHostView.swift`
- `clients/apple/AlanNative/TerminalRuntimeRegistry.swift`
- `clients/apple/AlanNative/TerminalSurfaceController.swift`
- `clients/apple/AlanNative/Views/Console/ContentView.swift`
- `clients/apple/ARCHITECTURE.md`
- `clients/apple/scripts/check-architecture-maintainability.sh`
- Focused Apple shell/terminal scripts for touched owners
