## Context

Alan's current terminal host can launch and render a Ghostty-backed surface, but
daily terminal quality depends on much more than drawing bytes. Ghostty's macOS
surface stack coordinates native scrollback, IME, key equivalents, mouse modes,
selection, clipboard, search, title/cwd updates, renderer health, secure input,
and failure presentation through a coherent AppKit surface controller.

Alan needs the same class of terminal behavior while preserving its own shell
model, Arc-like sidebar, and agent control plane. This change completes the
surface adapter after runtime ownership is moved behind a stable service.

## Goals / Non-Goals

**Goals:**

- Add a first-class terminal surface controller/view adapter instead of growing
  ad hoc forwarding inside SwiftUI wrappers.
- Implement native scrollback and scrollbar synchronization.
- Complete keyboard, key-equivalent, modifier, IME/preedit, mouse, scroll,
  pressure, paste, copy, selection, and context-menu behavior.
- Project terminal title, cwd, bell, progress, child exit, renderer health,
  search, readonly, and input readiness into pane state where Ghostty exposes
  the needed surface callbacks.
- Document unsupported live secure-input callbacks, URL hover callbacks, and
  dedicated bracketed-paste delivery as deferred parity gaps.
- Present terminal failures and overlays as user-facing terminal state, with raw
  diagnostics only in the inspector debug layer.

**Non-Goals:**

- Move Ghostty runtime ownership. That belongs to
  `promote-macos-ghostty-runtime`.
- Rebuild split layout or command/menu routing. That belongs to
  `upgrade-macos-shell-splits-commands`.
- Guarantee every Ghostty terminal feature on the first implementation pass.
  Missing API coverage must be explicit and testable.

## Decisions

1. Create `AlanTerminalSurfaceController` as the surface behavior owner.

   The controller binds a service-owned surface handle to AppKit views,
   scrollback, input adapters, selection/clipboard, search, and metadata events.
   SwiftUI remains responsible for layout and composition.

   Alternative considered: continue putting event handlers in
   `AlanTerminalHostNSView`. That makes parity hard to audit and spreads input,
   scrollback, and metadata across unrelated files.

2. Use an AppKit `NSScrollView` adapter for terminal scrollback.

   The scroll view owns native scrollbar behavior, scroll elasticity decisions,
   and synchronization with Ghostty surface scrollback metrics. Terminal
   alternate-screen and application mouse modes can disable or reinterpret native
   scrolling through the controller.

   Alternative considered: implement scrollbars in SwiftUI. That would fight
   AppKit event routing and make Ghostty-style terminal scrollback less native.

3. Normalize input through dedicated adapters.

   Keyboard, key equivalents, flags, text input/IME, mouse buttons, drag,
   movement, pressure, and scroll are translated in one place before reaching
   Ghostty. The adapter records unsupported cases and exposes diagnostics.

   Alternative considered: forward raw AppKit events from several views. That
   hides gaps and makes test coverage brittle.

4. Treat terminal overlays as terminal state, not debug UI.

   Search, copy/selection feedback, child-exit, renderer failure, readonly, and
   input-readiness states are visible when they affect terminal use. Raw pane
   IDs, callback names, runtime phases, and Ghostty internals stay in Debug.

   Alternative considered: put all terminal state in the inspector. That keeps
   the canvas clean but leaves users without actionable feedback when terminal
   input or rendering is unavailable.

5. Require a mixed verification strategy.

   Unit tests can cover controller state transitions, fake input translation,
   scrollback metrics, and metadata projection. Some behavior, especially IME,
   native selection, and renderer health, needs manual or UI-smoke evidence until
   a robust app automation lane exists.

## Risks / Trade-offs

- Ghostty APIs may not expose every desired state cleanly -> Mark unsupported
  states explicitly and add diagnostics instead of silently faking behavior.
- Native scrollback can conflict with terminal application mouse mode -> Route
  mode changes through the surface controller and add manual verification for
  shell scrollback and full-screen terminal apps.
- IME/preedit behavior is difficult to unit test -> Add focused adapter tests
  where possible and document manual checks for common input methods.
- Search and overlays can clutter the terminal-first UI -> Keep overlays compact
  and contextual, and keep inspector debug details out of the default canvas.
- This change depends on stable runtime handles -> Implement after or alongside
  `promote-macos-ghostty-runtime`, not before it.

## Migration Plan

1. Add surface controller, scroll view, and input adapter types behind current
   terminal host creation.
2. Move existing event forwarding into the controller without changing behavior.
3. Add scrollback, selection/clipboard, search, metadata, and overlay paths
   incrementally.
4. Replace old ad hoc handlers once parity checks pass.
5. Add unit tests and manual verification notes for IME, scrollback, mouse apps,
   search, paste, and failure states.

## Implementation Notes

### 2026-05-06 Surface Controller Pass

- Added `AlanTerminalSurfaceController` plus input, scrollback metrics,
  selection/clipboard, search-state, and metadata/overlay adapters.
- Moved the existing AppKit key, mouse, scroll, selection, clipboard, and IME
  forwarding paths through the controller while preserving the Ghostty host as
  the renderer owner.
- Added surface readiness state to host runtime snapshots so the inspector can
  distinguish renderer health, input readiness, terminal mode, and process
  state without exposing those details in the default terminal canvas.
- Added compact user-facing overlays for search, missing surface, startup,
  renderer failure, child exit, and read-only states.
- Wired terminal search to Ghostty binding actions so query changes and
  next/previous navigation reach the terminal search engine, with total and
  selected-match state flowing back through Ghostty action callbacks.
- Added control-plane delivery rejection for child-exit and renderer-failure
  states so remote text delivery does not report false success.

### 2026-05-06 Native Scroll And Clipboard Pass

- Added `AlanTerminalNativeScrollViewAdapter`, modeled after Ghostty's
  `SurfaceScrollView` shape: an `NSScrollView` owns native scrollbar behavior
  while the terminal canvas remains the visible renderer.
- Wired Ghostty `GHOSTTY_ACTION_SCROLLBAR` callbacks into pane-scoped
  `AlanTerminalScrollbackMetrics`, and route normal-buffer native scrolls back
  to Ghostty with `scroll_to_row:<row>`.
- Kept alternate-screen and terminal mouse-reporting scroll input routed to the
  terminal surface instead of exposing stale normal-buffer scrollbar state.
- Added controller-owned copy and paste paths. Copy reads terminal selection
  through a selection engine and writes through a pasteboard writer seam; paste
  uses the existing failure-aware surface delivery path so closed or not-ready
  panes do not report success.
- Added focused fake-surface tests for scrollback callback routing, native row
  scroll action dispatch, alternate-screen scroll routing, selection copy, and
  paste non-delivery.

### 2026-05-06 Terminal Status Metadata Pass

- Extended pane context metadata with renderer health, surface readiness, input
  readiness, readonly state, and terminal mode so runtime state survives view
  refreshes and can be consumed outside the debug inspector.
- Updated runtime projection so terminal title, cwd, bell/attention, process
  exit, and renderer failure update `ShellPane` viewport/context/attention
  state, which drives sidebar rows and the compact pane status strip.
- Added user-facing terminal status summaries that prioritize process exit and
  renderer failure over generic cwd or activity text while keeping raw renderer
  details in the existing debug surface.
- Added focused shell runtime metadata tests for context projection, sidebar
  status priority, and space attention propagation.

### 2026-05-06 Mouse And Pointer Normalization Pass

- Added `AlanTerminalPointerAdapter` so primary, secondary, other-button,
  movement, drag, hover, pressure, and scroll routing are modeled outside direct
  AppKit event handlers.
- Matched Ghostty's AppKit other-button normalization, including back/forward
  button mapping, so higher mouse buttons no longer collapse to middle click.
- Split normal-buffer drags/buttons into terminal-selection routing while
  alternate-screen and terminal mouse-reporting modes continue to deliver input
  to terminal applications.
- Kept right-click responder-chain fallback: if Ghostty does not consume a
  secondary button event, the host calls AppKit `super` so native context
  behavior remains available.
- Added focused fake-surface tests for pointer mode routing, pressure delivery,
  unready-surface suppression, Ghostty-compatible button mapping, and
  mouse-reporting scroll forwarding.

Remaining unsupported Ghostty parity after this pass:

- Alternate-screen and application mouse-mode detection are not yet sourced from
  live Ghostty callbacks; the controller honors the modeled mode, but production
  mode updates still need a Ghostty callback source.
- Paste uses the service-owned control-text delivery path, which Ghostty treats
  as paste-like input, but a dedicated bracketed-paste API is not exposed
  separately yet.
- URL hover, secure input, and terminal application mode diagnostics remain
  placeholders until the needed Ghostty surface callbacks are available.

Manual verification notes:

- Full notes are captured in
  `openspec/changes/complete-macos-terminal-surface/manual-verification.md`.
- Built and launched the Debug `Alan.app`.
- Verified a real shell prompt accepts input and prints command output.
- Verified `Command-F` opens the terminal search overlay.
- Verified typing while search is active updates the overlay instead of sending
  text into the shell.
- Verified `Escape` dismisses search and terminal input resumes.
- Verified the inspector Debug view remains the place for raw shell/runtime
  JSON, while the search overlay uses user-facing terminal language.

## Open Questions

- Which Ghostty surface APIs expose search, URL hover, and renderer health
  directly versus requiring callback plumbing?
- Should paste confirmation be policy-driven in Alan or delegated entirely to
  Ghostty terminal behavior?
- How should secure input be represented in Alan's inspector without leaking
  sensitive terminal context?
