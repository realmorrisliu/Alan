## Context

`improve-macos-app-architecture-maintainability` established the Apple client
source layout, validation report, and migration-debt ledger. The current report
still completes with seven non-blocking warnings for large files or AppKit
imports in transitional owners:

- `ShellHostController.swift` remains large and imports AppKit.
- `TerminalHostView.swift` remains large.
- `TerminalRuntimeRegistry.swift` imports AppKit.
- `TerminalSurfaceController.swift` remains large.
- `Views/Console/ContentView.swift` remains large and imports AppKit.

This change turns those warnings into a debt-reduction work queue. It should
produce small stacked PRs that can be reviewed and merged independently while
keeping the architecture report and debt ledger current.

## Goals / Non-Goals

**Goals:**
- Reduce the architecture-maintainability warning count through focused,
  behavior-preserving refactor slices.
- Keep each slice tied to one owner boundary or warning class.
- Update `clients/apple/ARCHITECTURE.md` and validation expectations in the
  same slice that resolves a warning.
- Preserve terminal runtime identity, terminal event ownership, shell command
  vocabulary, and console/mobile isolation.

**Non-Goals:**
- Redesign the macOS shell UI.
- Change daemon API routes, payloads, or runtime semantics.
- Complete all terminal, shell, and console decomposition in one PR.
- Make report mode fail on every known warning before the debt is reduced.

## Decisions

1. Address the smallest AppKit leak first.

   `TerminalRuntimeRegistry.swift` should be the first implementation slice
   because it likely has the narrowest behavioral surface. Removing or
   relocating its AppKit dependency can reduce one warning before larger owner
   splits begin.

2. Split by owner, not by line count alone.

   Large files should only shrink when code moves into a durable owner such as
   a controller, service, bridge, adapter, or view component. Mechanical line
   moves that leave responsibilities ambiguous do not count as resolving debt.

3. Keep the warning count as an explicit validation signal.

   Each slice should run `check-architecture-maintainability.sh` and document
   the expected warning count. If the count changes, the script expectation and
   `clients/apple/ARCHITECTURE.md` must be updated together.

4. Use stacked PRs with a low-conflict order.

   The preferred order is:
   - remove the `TerminalRuntimeRegistry.swift` AppKit warning;
   - reduce `ShellHostController.swift` controller/store/service debt;
   - reduce `TerminalSurfaceController.swift` surface adapter debt;
   - reduce `TerminalHostView.swift` terminal-host collaborator debt;
   - reduce `Views/Console/ContentView.swift` console/mobile debt.

   If discovery shows a later slice is lower risk or a dependency is inverted,
   the task order may be adjusted, but each PR should still explain the
   warning it resolves.

## Risks / Trade-offs

- Behavior-preserving moves can still alter object lifetimes or activation
  paths. Mitigation: run focused shell, terminal runtime, and terminal surface
  scripts for touched owners.
- Stacked PRs can conflict when adjacent files are edited in parallel.
  Mitigation: keep each slice narrow and restack after every merge.
- Strictly chasing line count can create shallow abstractions. Mitigation: only
  count a warning as resolved when the moved code has a named owner and clearer
  review boundary.
- Console cleanup touches legacy/mobile surfaces that are not the primary
  macOS shell path. Mitigation: defer console decomposition until smaller shell
  and terminal debt slices have landed.
