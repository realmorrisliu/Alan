## Context

Alan's macOS app can host Ghostty-backed terminal panes, but the current
ownership boundary is still too view-shaped. `AlanTerminalHostNSView` and
`AlanGhosttyLiveHost` participate in creating Ghostty app/surface state, while
SwiftUI selection and split rendering decide which host view is alive. Ghostty's
macOS app takes the opposite shape: process/app initialization is centralized,
terminal surfaces have durable runtime identity, and AppKit views attach to
those surfaces.

Alan also has requirements Ghostty does not have: an agent-readable control
plane, stable pane IDs, window-scoped shell state, and future Alan-specific
attachments. The goal is to move terminal lifetime below the shell model without
copying Ghostty's whole window architecture.

## Goals / Non-Goals

**Goals:**

- Create a process/window/pane runtime foundation that initializes libghostty
  once and owns pane surfaces independently from transient view attachment.
- Make terminal runtime state addressable by Alan pane IDs for UI, control
  plane, and future automation.
- Make pane, tab, window, and app close paths deterministic and testable.
- Provide a mockable boundary so most Apple-client tests do not require real
  Ghostty artifacts.
- Align or absorb overlapping terminal event ownership work from
  `converge-terminal-event-ownership`.

**Non-Goals:**

- Reimplement Ghostty's full terminal surface behavior in this change. That is
  covered by `complete-macos-terminal-surface`.
- Replace Alan's Arc-like spaces/tabs shell with Ghostty's window UI.
- Add App Intents or UI smoke coverage. That is covered by
  `add-macos-shell-automation-tests`.

## Decisions

1. Introduce `AlanTerminalRuntimeService` as the ownership boundary.

   The service is created from app/window startup code and exposed to shell
   controllers through dependency injection. It owns Ghostty initialization,
   window-scoped runtime configuration, pane surface handles, and metadata
   projection.

   Alternative considered: keep extending `TerminalRuntimeRegistry` and
   `AlanTerminalHostNSView`. That preserves fewer files but leaves lifetime
   coupled to view creation and makes background pane delivery unreliable.

2. Split process, window, and pane responsibilities explicitly.

   A process-level bootstrap initializes libghostty resources, terminfo, logging,
   and global configuration once. A window-level service owns the Ghostty app
   handle and pane table for one shell window. Pane-level `TerminalSurfaceHandle`
   instances own the surface, pending delivery buffer, lifecycle phase, and
   metadata snapshot.

   Alternative considered: one global service for all windows. Alan's control
   plane and persisted shell state are already window-scoped, so a single
   cross-window service would make routing and teardown harder to reason about.

3. Make AppKit terminal views adapters.

   `AlanTerminalHostNSView` becomes responsible for focus, backing scale/display
   updates, occlusion, frame changes, and event forwarding into an existing pane
   surface. It must not create a new Ghostty app or become the source of truth
   for pane readiness.

   Alternative considered: let each view recreate and rebind surfaces on demand.
   That loses scrollback/process continuity and makes `pane.send_text` depend on
   the selected tab.

4. Keep control-plane responses derived from runtime service state.

   Runtime-dependent commands such as `pane.send_text`, focus, resize, and close
   query the service after the mutation is accepted or rejected. The registry may
   expose snapshots, but snapshots are not the authority for delivery.

   Alternative considered: keep returning optimistic responses from shell model
   mutations. That creates false success for background or failed terminals.

5. Prefer test doubles over hidden compile-time Ghostty assumptions.

   The runtime service exposes protocols for surface creation, text delivery,
   metadata events, and teardown. Production code uses the Ghostty adapter; tests
   use fakes.

   Alternative considered: run all tests against real Ghostty artifacts. That is
   valuable for an integration lane but too heavy for everyday model and control
   plane tests.

## Risks / Trade-offs

- Runtime ownership migration can duplicate terminal owners temporarily -> Keep
  a single feature branch/change sequence and remove view-owned Ghostty app
  creation before marking runtime service tasks complete.
- Ghostty APIs may require main-thread affinity for some calls -> Encode actor
  isolation in service protocols and add tests for off-main control calls being
  marshaled correctly.
- Process-global initialization can make tests order-dependent -> Add an
  injectable bootstrap interface and make fake bootstrap the default in unit
  tests.
- Multi-window service teardown can race with control-plane clients -> Make
  close operations transition panes through closing/closed phases before
  releasing handles and return stable errors after closure.
- Existing `converge-terminal-event-ownership` may overlap -> Treat that change
  as a dependency source and move accepted event-boundary tasks into this
  service where they share ownership.

## Migration Plan

1. Add service protocols and fake implementations behind the current code path.
2. Move Ghostty process/app initialization into the service while keeping the
   existing host view API stable.
3. Move surface creation and teardown into pane handles and make host views
   attach to handles.
4. Route control-plane delivery and metadata reads through service snapshots.
5. Remove view-owned Ghostty app/surface creation and redundant event ownership
   paths.
6. Add unit/contract tests, then manually verify multi-tab, split, and
   multi-window terminal continuity.

Rollback is straightforward before removing the old host path: keep the service
adapter behind one creation site and restore the previous host-owned creation.
After archive, rollback requires reverting the service migration as a normal
code change.

## Open Questions

- Should the Ghostty app handle be process-scoped with window IDs, or
  window-scoped beneath a process bootstrap? The design assumes window-scoped
  service ownership until Ghostty API constraints prove otherwise.
- Which Ghostty callbacks must run on the main actor versus a terminal runtime
  queue?
- How much of `converge-terminal-event-ownership` should be archived into this
  change versus kept as a small prerequisite PR?
