## Why

Alan's live Ghostty host can boot and draw a terminal, but it does not yet expose
the full native terminal surface behavior that makes Ghostty reliable for daily
macOS use. The next step is to complete the terminal surface adapter: scrollback,
native scrollbars, input edge cases, clipboard/search, renderer health, and
user-facing terminal state.

## What Changes

- Add a first-class Alan terminal surface adapter around Ghostty surfaces instead
  of continuing to grow ad hoc input forwarding in `TerminalHostView`.
- Implement native scrollback and scrollbar synchronization equivalent in shape
  to Ghostty's `SurfaceScrollView` model.
- Complete keyboard, IME/preedit, key-equivalent, mouse, pressure, paste, copy,
  selection, and right-click behavior against Ghostty surface APIs.
- Surface terminal title, cwd, bell, progress, child-exit, renderer health,
  cursor, search, readonly state, and supported input readiness into Alan's pane
  metadata and UI.
- Document deferred Ghostty parity gaps for live secure-input callbacks, URL
  hover callbacks, and a dedicated bracketed-paste API instead of treating them
  as accepted requirements.
- Keep implementation details hidden from the default UI while exposing
  diagnostics in the inspector debug layer.
- Define failure and fallback states so Alan never presents a fake usable
  terminal when Ghostty surface creation, rendering, or input delivery fails.

## Capabilities

### New Capabilities
- `macos-terminal-surface-parity`: Defines required native terminal surface
  behavior for scrollback, input, clipboard, search, renderer health, terminal
  state overlays, and debug diagnostics.

### Modified Capabilities
- `macos-shell-terminal-lifecycle`: Runtime metadata must include the surface
  states needed for title, cwd, bell, process exit, renderer health, and input
  readiness.
- `macos-shell-ui-ux-conformance`: Default UI must remain terminal-first while
  terminal overlays and failures are presented as user-facing terminal state,
  not implementation jargon.
- `macos-shell-build-test-contract`: The Apple client must add focused tests or
  documented manual verification for terminal surface input and scrollback
  behavior.

## Impact

- Apple client terminal host: `TerminalHostView.swift`, `GhosttyLiveHost.swift`,
  `TerminalHostRuntime.swift`, `TerminalPaneView.swift`, and likely new
  `TerminalSurface*` support files.
- UI behavior: users can scroll, copy/paste, search, interact with terminal
  mouse applications, and understand terminal exit/failure states without
  opening debug views.
- Runtime metadata: pane/title/cwd/attention/process fields become driven by
  richer terminal surface events.
- Verification: requires app-level manual tests plus focused fake-surface tests
  for IME, selection, scrollback, right-click, search, process exit, and
  renderer failure/fallback states.
