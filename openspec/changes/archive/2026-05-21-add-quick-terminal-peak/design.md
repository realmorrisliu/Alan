## Context

`add-advanced-terminal-activity-semantics` captured the quick-terminal direction
while implementing terminal activity A-group work. That direction should now
live in a separate implementation change because quick terminal requires
windowing, global shortcut, runtime slot, and workspace-promotion decisions that
are larger than activity UI.

The product target is a detached, globally summonable macOS Peak terminal that
feels like part of Alan's native shell, not a dashboard, panel-heavy assistant,
or duplicate mini app.

## Goals / Non-Goals

**Goals:**

- Present a single global quick terminal from any macOS Space and active
  display.
- Use the existing terminal runtime service and shell command/controller paths.
- Preserve terminal scrollback and process state across hide/show.
- Keep `Esc` routed to the terminal by default.
- Avoid focus-loss auto-hide.
- Support explicit close and `Open in Space` promotion.
- Keep hidden quick-terminal activity visible through the same compact
  activity/notification policy used for ordinary panes.

**Non-Goals:**

- No separate terminal runtime owner for the Peak.
- No per-space quick-terminal instances in the MVP.
- No duplicate sidebar, inspector, marketing header, or dashboard composition
  inside the Peak.
- No remote SSH/mux/cloud terminal persistence.
- No semantic command-output actions; those are owned by
  `add-semantic-terminal-actions`.

## Decisions

### 1. Use one global quick-terminal runtime slot

The MVP should model one global quick terminal. Summoning it from another Alan
space, macOS Space, or display moves the presentation of the same runtime rather
than creating another terminal session.

This keeps user expectations simple and avoids hard-to-debug lifecycle
duplication.

### 2. Present a detached native Peak over normal runtime ownership

The quick terminal should be a detached native macOS window/panel that hosts a
normal terminal surface backed by the existing terminal runtime service. It
should not be a popover child of Alan's main window and should not require the
main window to come forward.

Using normal runtime ownership preserves activity, clipboard, input, scrollback,
and lifecycle behavior.

### 3. Make hide, close, and promote distinct

Hide removes the Peak presentation but preserves the runtime. Close tears down
the terminal through the same lifecycle used by ordinary panes. Promote moves
the runtime into a selected Alan space as a normal tab, hides the Peak, and
clears the global quick slot.

Alan must not copy the terminal process or show the same runtime in both the
Peak and a normal tab.

### 4. Route commands through the shared shell controller

Keyboard shortcuts, menu commands, command input, and control surfaces should
all converge on shared shell command/controller paths. The draft global toggle
is configurable, with `Option+Space` as the first proposed default.

This keeps automation and user actions aligned and avoids one-off quick-terminal
behavior.

### 5. Preserve terminal-first key semantics

`Esc` remains terminal input unless an Alan-owned nested quick-terminal picker
or menu is open. Losing focus does not hide the Peak; explicit toggle/hide/close
commands control presentation.
