## Context

The current macOS terminal pane has one physical interaction area but several
logical event owners:

- SwiftUI `ShellTerminalLeafView` wraps the native terminal view with
  `.onTapGesture(perform: onSelect)` for pane selection.
- `AlanTerminalHostNSView` owns terminal focus, key input, IME, mouse forwarding,
  scroll, paste, selection readback, and Ghostty synchronization.
- The full-size Ghostty or fallback canvas is a child view of the host and
  participates in AppKit hit-testing even though the host is the object that
  forwards terminal input.
- The hidden-titlebar shell window uses background dragging, while terminal
  host/canvas views opt out through `mouseDownCanMoveWindow`.

That layering makes terminal interaction fragile. A pane click is selection in
SwiftUI, terminal input in AppKit, and potentially a window drag depending on
which layer receives the event. The code should converge on one owner for
terminal-area events without changing the broader shell state model.

## Goals / Non-Goals

**Goals:**

- Make `AlanTerminalHostNSView` the single owner of mouse events that occur
  inside terminal pixels.
- Keep SwiftUI responsible for rendering layout, reading shell state, and
  owning explicit controls such as pane selector buttons.
- Ensure a terminal click selects the pane, focuses the terminal host, and
  forwards the same event to Ghostty without requiring a second click.
- Keep drawing subviews, including Ghostty canvas and fallback canvas, from
  stealing terminal events from the host.
- Keep non-interactive shell background draggable while terminal panes remain
  non-draggable.
- Avoid retain cycles caused by registry-owned AppKit views storing strong
  SwiftUI/controller closures.

**Non-Goals:**

- Do not change Ghostty API semantics, occlusion handling, or renderer startup.
- Do not redesign split-pane layout, sidebar layout, or the visual treatment of
  terminal corners.
- Do not change control-plane protocol payloads or shell persistence format.
- Do not implement a new terminal emulator or replace Ghostty selection logic.

## Decisions

### Decision: Terminal host owns terminal-area events

Remove the SwiftUI `.onTapGesture(perform: onSelect)` wrapper from the terminal
leaf. The native host should activate its pane from AppKit mouse handlers before
it forwards terminal input to Ghostty.

Rationale: terminal mouse events already need AppKit for first responder,
selection, scroll, IME, paste, and Ghostty coordinate conversion. Keeping
selection in SwiftUI creates a competing recognizer outside the terminal input
pipeline.

Alternative considered: keep the SwiftUI tap gesture and tune gesture priority.
That would preserve two event owners and still risk stealing the first terminal
click from Ghostty or fighting AppKit responder behavior.

### Decision: Use a weak activation delegate instead of stored closures

Introduce a narrow main-actor delegate:

```swift
@MainActor
protocol TerminalHostActivationDelegate: AnyObject {
    func terminalHostDidRequestActivation(paneID: String)
}
```

`ShellHostController` should conform and call `focus(paneID:)`. `TerminalHostView`
passes the delegate through `TerminalRuntimeRegistry` into
`AlanTerminalHostNSView`. The host stores it as `weak`.

Rationale: terminal host views are pane-keyed and registry-owned. Storing a
strong `onSelect` closure on those views can create a long-lived cycle from the
controller to the registry to the host view back to the controller. A weak
delegate keeps the imperative AppKit bridge explicit and bounded.

Alternative considered: pass `onSelect` closures through `NSViewRepresentable`.
That is convenient for transient SwiftUI views but wrong for registry-owned
AppKit objects with longer lifetimes.

### Decision: Make canvases drawing-only in hit-testing

`AlanGhosttyCanvasView` and `AlanTerminalFallbackCanvasView` should return `nil`
from `hitTest(_:)` so AppKit delivers terminal mouse events to
`AlanTerminalHostNSView`. Passive placeholder/diagnostic overlay views inside
the host should also be non-interactive unless they intentionally contain real
controls.

Rationale: the canvas is the rendering attachment passed to Ghostty; the host is
the event adapter. This keeps coordinate conversion, pane activation, focus, and
Ghostty forwarding in one class.

Alternative considered: override `hitTest(_:)` on the host and forcibly return
the host for all descendants. That is stronger than necessary and would make it
harder to add future interactive controls inside the terminal host.

### Decision: Activate before forwarding mouse-down events

For primary, secondary, and other mouse-down events, the host should request
activation for its pane, make itself first responder, then forward position and
button state to Ghostty. Mouse-up, drag, motion, scroll, pressure, and key paths
remain host-owned and should not depend on SwiftUI tap state.

Rationale: selecting a non-focused pane and sending the first click to the
terminal are one user action. Activation must not consume the event that starts
terminal selection or clicks a terminal application.

Alternative considered: activate only on mouse-up. That delays focus and can
misroute the down/up pair that terminal applications expect.

### Decision: Encode the boundary in shell contract checks

Extend `check-shell-contracts.sh` to reject the SwiftUI terminal leaf
`.onTapGesture(perform: onSelect)` path and require the weak delegate plus
drawing-only canvas hit-testing.

Rationale: this class of regression is easy to reintroduce during visual chrome
changes. A lightweight textual contract check is appropriate because the
boundary is structural.

Alternative considered: rely only on manual app testing. Manual testing remains
necessary for terminal interaction, but it should not be the first place this
architectural boundary is noticed.

## Risks / Trade-offs

- Weak delegate is missing or stale -> the terminal still receives input but a
  non-selected pane may not update shell selection; mitigate by configuring the
  delegate every `updateNSView` and adding a contract check.
- Drawing-only canvas hit-testing hides future canvas controls -> only use
  transparent hit-testing for rendering/passive views; future real controls must
  be separate AppKit or SwiftUI controls with explicit ownership.
- Activation changes event ordering -> activate before Ghostty mouse-down and
  verify first click, drag selection, right click, scroll, and typing after pane
  switching in the running app.
- Contract checks become too implementation-specific -> check only durable
  boundary markers: no SwiftUI terminal tap wrapper, weak delegate, and
  transparent rendering canvases.

## Migration Plan

1. Add the activation delegate protocol and `ShellHostController` conformance.
2. Thread the weak delegate through `TerminalHostView`,
   `TerminalRuntimeRegistry.hostView(...)`, and
   `AlanTerminalHostNSView.configure(...)`.
3. Move terminal-pane activation into host mouse-down handling and remove
   `onSelect` from `ShellTerminalLeafView`.
4. Make Ghostty, fallback, and passive overlay rendering views transparent to
   hit-testing while preserving `mouseDownCanMoveWindow == false`.
5. Add shell contract checks for the event ownership boundary.
6. Verify with `check-shell-contracts.sh`, the macOS Xcode build, and manual app
   interaction covering pane click-to-focus, immediate typing, text selection,
   right click, scroll, and background window dragging.

Rollback is straightforward: keep the delegate path isolated to terminal
activation, so it can be reverted without changing runtime ownership,
control-plane delivery, or Ghostty lifecycle code.

## Open Questions

- Should scroll on a non-selected pane activate that pane, or should it continue
  to scroll without changing selection? The first implementation should preserve
  the current scroll semantics unless manual testing shows a native expectation
  mismatch.
- Should the fallback diagnostic overlay ever contain interactive setup actions?
  If so, those controls need explicit ownership rather than inheriting the
  drawing-only overlay behavior.
