# Sidebar-local Space Swipe Design

## Context

`refine-macos-sidebar-interactions` intended to improve the existing sidebar
space swipe interaction. The correct product model is not a full-window space
pager. The existing macOS sidebar already supports left/right swipe, but the
interaction is not continuous enough: the user should be able to drag the
active space content directly, then have the UI settle to the source or adjacent
target space based on distance, velocity, momentum, and edge constraints.

The earlier `refine-macos-sidebar-interactions` spec incorrectly moved the
abstraction from sidebar-local space navigation to full shell paging. That made
the right-side terminal workspace participate in the swipe, which breaks the
terminal-first layout and causes visible artifacts at the terminal surface.

## Goals

- Keep space swipe scoped to the sidebar.
- Make the swipe continuous and finger-tracked.
- Move only the active-space content inside the sidebar: the space title/header
  and the active space tab list.
- Keep the command input, bottom space switcher, sidebar material surface,
  sidebar chrome, traffic lights, and right-side terminal workspace fixed during
  the drag.
- Commit shell selection and terminal focus only after the gesture settles to a
  target space.

## Non-goals

- Do not make the whole app window or terminal workspace a horizontal pager.
- Do not redesign the sidebar information architecture.
- Do not change terminal runtime identity, terminal input ownership, split
  layout, or pane lifecycle semantics.
- Do not make the bottom space switcher part of the moving page; it is a fixed
  navigation control.

## Architecture

`MacShellRootView` should own the stable shell layout:

```text
MacShellRootView
  -> pinned/floating sidebar surface
  -> ShellWorkspaceView
```

It should not wrap the whole shell in a space pager. `ShellWorkspaceView`
renders the currently committed selected space only.

`ShellSidebarView` should own the sidebar-local swipe presentation. Inside the
sidebar, introduce a focused content pager such as
`ShellSidebarSpaceContentPager`. That pager wraps only:

- active space title/header
- active space tab list

The following remain outside the pager and visually fixed:

- command input
- bottom space switcher
- sidebar background/material
- pinned and floating sidebar chrome controls
- macOS traffic-light placement
- right-side terminal workspace

## Responsibilities

`ShellSidebarSwipeMonitor` should be an input adapter. It converts macOS
trackpad/scroll events into gesture signals:

- horizontal/vertical intent lock
- accumulated horizontal translation
- latest effective horizontal velocity
- phaseful release, cancellation, momentum handoff, and phaseless idle release

It should not know which UI elements move, and it should not directly call shell
selection APIs.

`ShellSidebarSpaceContentPager` should be the physical/visual state machine. It
tracks:

- source space index
- adjacent target space index, when one exists
- direct drag offset from finger translation
- sidebar content page width
- settlement phase: dragging, settling to source, or settling to target

`ShellHostController` remains the owner of committed shell selection and focus.
The pager calls the controller selection/focus path only when the release should
commit to a target space.

## Gesture Semantics

During drag:

- horizontal movement maps directly to the sidebar content offset;
- the current and adjacent space content pages move from the same translation;
- edge swipes apply resistance instead of wrapping or showing nonexistent
  spaces;
- vertical tab-list scrolling is disabled only after horizontal intent locks;
- terminal workspace, terminal focus, selected space, selected tab, split tree,
  and divider ratios remain unchanged.

On release:

- if distance or velocity crosses the commit threshold and an adjacent target
  exists, the pager settles to the target and commits shell selection through
  the controller path;
- if the threshold is not met, the pager settles back to the source and leaves
  shell selection unchanged;
- release handling should use the last effective finger velocity rather than a
  zero-delta ended event;
- phaseless input may use a short idle timeout to avoid leaving the pager stuck.

The core boundary is:

```text
preview = sidebar presentation state
selection = committed shell state
```

## Spec Correction

`refine-macos-sidebar-interactions` should restore the sidebar-local contract
from the accepted `streamline-macos-sidebar-ia` direction, while keeping the
improved gesture physics and authoritative commit path.

The spec should remove language that says a space page includes the terminal
workspace surface. It should instead require that the workspace terminal surface
stays visually stable during the swipe and updates only after the switch
commits.

## Testing

Focused tests should cover:

- horizontal intent lock and direct translation mapping;
- vertical scroll pass-through before horizontal lock;
- no vertical tab-list movement after horizontal lock;
- edge resistance at the first and last spaces;
- commit by distance threshold;
- commit by release velocity or momentum handoff;
- cancel below threshold;
- phaseful release using the last effective velocity;
- phaseless idle settlement;
- root layout stability: terminal workspace and fixed sidebar regions do not
  move during sidebar swipe.

Manual verification should include a running macOS app check confirming that:

- only the space title/header and active tab list move with the gesture;
- command input and bottom space switcher remain fixed;
- the right-side terminal surface does not slide, duplicate, or expose top-edge
  artifacts;
- after commit, terminal focus follows the selected pane for the committed
  space.
