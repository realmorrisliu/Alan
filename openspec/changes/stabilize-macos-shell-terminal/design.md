## Context

The native Apple client currently mixes three responsibilities that need clearer
boundaries before the macOS shell can behave like a dependable terminal:

- SwiftUI scene and selection state live in `MacShellRootView`, `TerminalPaneView`,
  `ShellModel`, and `ShellHostController`.
- The actual terminal host is bridged through `TerminalHostView`,
  `GhosttyLiveHost`, and `TerminalHostRuntime`.
- Agents and scripts mutate/read shell state through `ShellControlPlane`,
  file-command fallbacks, persisted shell snapshots, and event files.

The review found that terminal processes and Ghostty surfaces are effectively
owned by the currently rendered view. Switching tab selection can therefore
tear down a background terminal. The control plane also reports some mutations
as successful before the runtime has actually accepted them, uses shared
window/state paths across a `WindowGroup`, and has a serial socket loop with no
clear request size or duration bounds.

The UI has drifted from the documented macOS shell direction in
`docs/spec/alan_macos_shell_ui_ux.md`: it reads more like a diagnostic dashboard
than a terminal-first workspace with a space rail, active-space tab list,
restrained toolbar, and optional inspector. Build metadata is also inconsistent:
documentation advertises older OS requirements while the Xcode project targets
newer platforms, and local Ghostty artifacts are referenced in ways that make a
clean dependency setup hard to verify.

## Goals / Non-Goals

**Goals:**

- Preserve terminal runtimes, process state, renderer state, focus, and metadata
  across SwiftUI view creation/destruction and tab selection changes.
- Make runtime-dependent control-plane mutations return authoritative results:
  accepted, queued, or rejected with stable error codes.
- Give each macOS window its own shell identity, socket path, persisted state,
  event stream, and control directory.
- Bound local IPC request size, client lifetime, and main-actor command duration.
- Make persistence, event, and file-command failures visible to developers and
  control clients.
- Bring the default macOS UI back to the documented shell information
  architecture without removing useful debug data.
- Align build documentation, deployment targets, Ghostty dependency setup, and
  focused automated tests.

**Non-Goals:**

- Do not redesign Alan daemon/runtime protocol semantics as part of this change.
- Do not replace Ghostty or implement a custom terminal emulator.
- Do not build a complete cross-platform Apple UI rewrite; the scope is the
  macOS shell experience and testable model/control boundaries.
- Do not add remote network shell control. This change hardens the local control
  surface that already exists.

## Decisions

### Decision: Own terminal runtimes outside transient views

Introduce a window-scoped terminal runtime registry owned by the shell host/model
layer, keyed by stable pane IDs. The registry should expose a narrow runtime
handle protocol for attach, detach, focus, resize, send text, metadata updates,
and teardown. `TerminalHostView` should attach an AppKit/Ghostty surface to an
existing pane runtime when rendered and detach from it when removed, but view
deallocation must not imply process teardown.

Rationale: SwiftUI views are a projection of state, not the correct owner for
terminal processes. A registry keeps terminal identity aligned with pane
identity and lets background tabs keep running.

Alternative considered: keep the current view-owned host and cache invisible
views. That would fight SwiftUI lifecycle behavior, make memory use harder to
reason about, and still leave control-plane delivery tied to rendering.

### Decision: Replace notification-only text delivery with runtime results

Route `pane.send_text` and future runtime-dependent mutations through the
runtime registry. The response must distinguish:

- accepted by a live runtime, with accepted byte count;
- queued in a pane-specific buffer with explicit durability/flush semantics; or
- rejected with a stable error code.

NotificationCenter may still be used internally for UI observation, but it must
not be the source of truth for whether a terminal accepted input.

Rationale: Agents need to know whether input reached the target shell. A
fire-and-forget notification can disappear when the target pane is not rendered.

Alternative considered: keep returning `applied: true` after posting a
notification and add logs for missed observers. Logs would help debugging but
would preserve misleading control responses.

### Decision: Generate a per-window shell context at scene creation

Create a per-scene shell context object containing `window_id`, control
directory, socket path, persisted state path, event path, and runtime registry.
Each `WindowGroup` instance should construct or restore one context rather than
using fixed identifiers such as `window_main`. Window restoration may reuse a
previous context only when the restored scene identity explicitly matches.

Rationale: A macOS `WindowGroup` implies multiple independent windows. Shared
state paths make one window's agents, tabs, panes, and file commands bleed into
another.

Alternative considered: force the app to a single window. That would avoid some
bugs but conflict with native macOS expectations and leave the implementation
less general than the declared scene model.

### Decision: Make the control socket bounded and concurrent per request

Keep the local socket protocol simple, but enforce a maximum request byte size,
a request read deadline, a command execution deadline, and bounded per-client
work. Accepting new connections should not be blocked by one client waiting on
newline input or a slow main-actor mutation. Responses should use a stable
success/error envelope that includes machine-readable codes.

Rationale: The control plane is a local automation boundary. It should be
predictable under malformed clients, slow handlers, and missing targets.

Alternative considered: retain the serial accept/read/handle loop and rely on
well-behaved clients. That is insufficient for an agent-facing surface because
failed tool calls can leave stale clients or partial requests behind.

### Decision: Surface persistence and file-command failures

Change shell state/event/command/binding file operations to return or record
`Result` values instead of ignoring errors. Use `os.Logger` for developer
diagnostics and keep a small inspectable diagnostic surface in shell state or
debug inspector data for recent control-plane failures.

Rationale: Silent IO failure turns state divergence into guesswork. The app
does not need to stop on every failure, but developers and control clients need
evidence when state publication or command ingestion fails.

Alternative considered: only add logging around the highest-level publish call.
That would miss lower-level decode/delete/write failures that currently erase
the best debugging evidence.

### Decision: Reframe the UI around spaces, tabs, terminal, and inspector

Keep the existing shell model concepts, but render them with the documented
information architecture:

- compact space rail;
- active-space tab list;
- terminal-first content region with minimal pane chrome;
- restrained native toolbar with command entry and frequent actions;
- optional inspector split into Overview and Debug layers.

Raw identifiers, runtime phases, socket paths, binding paths, and JSON snapshots
should move behind the Debug inspector rather than appearing in the default
workflow.

Rationale: The app is a terminal product surface, not primarily a runtime
debugger. Debug information is valuable, but it should not dominate the
default interaction model.

Alternative considered: keep the dashboard layout and polish spacing/copy. That
would improve presentation but would not satisfy the existing macOS shell UI/UX
contract.

### Decision: Treat impeccable design guidance as the UI acceptance layer

The impeccable design guidance now serves as the concrete acceptance layer for
the UI portion of this change. It does not create a second redesign track; it
sharpens how the existing `macos-shell-ui-ux-conformance` requirements should be
implemented and reviewed.

The implementation should optimize for:

- terminal-first composition, where the active terminal tab remains the clear
  center of gravity;
- Arc-like organization, with a material space rail and active-space tab list
  instead of dashboard sections;
- quiet native macOS light-mode surfaces, using material and subtle separators
  rather than hard-coded themed panels;
- compact, scan-friendly rows and controls rather than cards, pills, or
  explanatory chrome;
- progressive disclosure, where runtime IDs, bindings, socket paths, JSON, and
  phases remain available only in the Debug inspector.

This decision makes UI work reviewable before it is considered complete. A task
that changes the macOS shell chrome should be checked against the documented
layout contract, the OpenSpec scenarios, and a running app screenshot in the
default light appearance.

Rationale: the product direction is already documented, but implementation can
still drift toward a polished diagnostic dashboard if the acceptance criteria are
too abstract. Binding the impeccable guidance to the OpenSpec change gives the
implementation pass a specific visual and interaction target without expanding
scope beyond the macOS shell experience.

### Decision: Add a testable terminal-host boundary and dependency checks

Extract enough of the terminal host behind protocols or small adapter types that
model/control-plane tests can run with a mock runtime instead of real Ghostty.
Add a dependency check for required local Ghostty artifacts and keep deployment
target documentation synchronized with Xcode project settings.

Rationale: The highest-risk behavior is state/routing/lifecycle logic, not the
real terminal renderer itself. Tests need a deterministic runtime double, while
the build needs explicit failure modes when local artifacts are missing.

Alternative considered: test only by launching the full app. That would catch
some integration bugs but would be slow, brittle, and unlikely to cover
background-pane and socket edge cases.

## Risks / Trade-offs

- Runtime registry leaks processes if close paths are incomplete -> Centralize
  tab/pane close teardown through the registry and add tests for exactly-once
  teardown.
- Durable input queue semantics become ambiguous -> Either implement a real
  per-pane queue with flush tests or reject unavailable runtimes explicitly.
  Do not claim queued delivery without a verifiable flush path.
- Per-window persistence changes break existing local files -> Treat legacy
  fixed-path state as best-effort readable for one migration release, but publish
  new state under window-scoped paths.
- Socket timeouts reject legitimate long-running commands -> Keep local
  commands small and return accepted async work only when there is a separate
  status/event path to observe completion.
- UI refactor risks hiding diagnostics developers use today -> Preserve debug
  data in an explicit inspector debug layer and keep copy/export affordances for
  paths and JSON snapshots.
- Ghostty dependency checks may add setup friction -> Make the error actionable
  and keep the documented preparation command close to the failing build step.

## Migration Plan

1. Introduce the runtime handle protocol and mock runtime without changing the
   visible UI.
2. Add the window-scoped shell context and move socket/state/event paths off
   fixed `window_main`-style identifiers.
3. Route `pane.send_text` through the runtime registry and update control
   responses to report accepted, queued, or rejected outcomes.
4. Move `TerminalHostView` attach/detach behavior onto existing runtime handles
   and reserve teardown for explicit pane/tab/window close.
5. Add bounded IPC handling, stable error codes, and observable persistence/file
   command diagnostics.
6. Refactor the sidebar, toolbar, terminal content, and inspector to match the
   macOS shell UI/UX contract while preserving debug affordances.
7. Align Apple README/spec build requirements with Xcode deployment targets and
   add Ghostty dependency setup checks.
8. Add focused tests for shell model mutation, control-plane commands, runtime
   delivery semantics, window isolation, and IPC bounds.

## Open Questions

- Should background pane text delivery prefer immediate rejection when a runtime
  is not booted, or should the first implementation include a durable per-pane
  queue?
- What is the intended migration behavior for existing local shell state files
  written under the fixed `window_main` identity?
- Should the runtime registry live directly in `ShellHostController`, or should
  it be a separate `@MainActor` service injected into the controller/model?
