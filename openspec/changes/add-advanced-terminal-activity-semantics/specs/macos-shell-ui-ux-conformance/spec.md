## ADDED Requirements

### Requirement: Activity UI Is Compact And Terminal First
Terminal activity UI SHALL use compact pane title-bar accessories, sidebar tab
metadata, and accessibility values instead of dashboard panels, persistent
bottom status strips, or debug labels in the default shell.

#### Scenario: Pane reports progress
- **WHEN** a pane reports determinate, indeterminate, paused, or failed
  progress
- **THEN** Alan presents that state as a lightweight pane title-bar accessory or
  sidebar activity rail that does not resize the terminal canvas, toolbar
  content, or split dividers

#### Scenario: Pane reports agent status
- **WHEN** a pane running a supported CLI coding agent reports running, needs
  input, complete, or error state
- **THEN** Alan presents a compact user-facing status in the pane title-bar and
  sidebar row without exposing raw event names, hook payloads, session IDs, or
  debug implementation details

#### Scenario: No actionable activity exists
- **WHEN** a tab has no active, notable, or user-actionable terminal activity
- **THEN** the sidebar tab row shows worktree and branch context as its
  secondary line instead of reserving empty activity chrome or showing idle or
  success states

### Requirement: Sidebar Tab Rows Are Attention-Oriented Work Rows
Sidebar tab rows SHALL use a richer but restrained layout that helps users
identify a tab and decide whether it needs attention.

#### Scenario: Tab row default layout
- **WHEN** Alan renders a sidebar tab row
- **THEN** the row contains a leading topology or kind slot, a title line, one
  secondary line that shows activity or worktree/branch context, and an
  optional progress rail only when the displayed activity owns progress

#### Scenario: Close affordance appears
- **WHEN** a sidebar tab row is hovered, keyboard-focused, or otherwise
  interaction-active
- **THEN** the close affordance appears as a trailing overlay without reserving
  a permanent layout slot in the row content

#### Scenario: Close affordance is hidden
- **WHEN** a sidebar tab row is not interaction-active
- **THEN** title, secondary text, activity, and progress content may occupy the
  full row width without leaving a fixed empty close-button column

#### Scenario: Split tab leading slot
- **WHEN** a tab contains multiple visible panes
- **THEN** the leading slot shows split topology instead of the generic terminal
  icon, and activating that topology cycles focus through panes in a stable
  order

#### Scenario: Single-pane leading slot
- **WHEN** a tab contains one pane
- **THEN** the leading slot shows the tab kind or supported agent icon rather
  than a split topology indicator

#### Scenario: Activity takes precedence over context
- **WHEN** a tab has sidebar-worthy activity
- **THEN** the secondary line shows the source-first activity copy instead of
  worktree or branch context

#### Scenario: No activity fallback
- **WHEN** a tab has no sidebar-worthy activity
- **THEN** the secondary line uses worktree or repository leaf plus branch when
  available, with cwd leaf only as a fallback

### Requirement: Pane Title Bars Own Pane Detail
Pane title bars SHALL keep terminal title as the primary label and expose
pane-local context through accessories.

#### Scenario: Pane title bar with activity
- **WHEN** a pane has pane-local activity, cwd or worktree context, branch,
  process, or supported agent state
- **THEN** the pane title bar presents that detail as compact accessories while
  keeping the terminal title as the primary text

#### Scenario: Sidebar and pane title differ
- **WHEN** the sidebar tab row shows a tab-level activity from another pane
- **THEN** the focused pane title bar still shows only the focused pane's own
  local detail and does not mirror unrelated tab-level activity

### Requirement: Activity Notifications Are Low Noise
System and in-app notifications for terminal activity SHALL be reserved for
actionable, out-of-view, or user-configured events.

#### Scenario: Background agent needs input
- **WHEN** a background or unfocused pane's supported coding agent needs user
  input
- **THEN** Alan may notify the user and mark the owning tab without stealing
  focus or opening a new panel

#### Scenario: Foreground progress updates
- **WHEN** the focused visible pane emits progress updates
- **THEN** Alan updates visible activity UI without sending system
  notifications by default

#### Scenario: Long command completes while unfocused
- **WHEN** a long-running command completes in an unfocused pane and the event
  meets the notification policy
- **THEN** Alan may send a concise command-completion notification with success
  or failure state

#### Scenario: Foreground command succeeds
- **WHEN** a command succeeds in the focused visible pane
- **THEN** Alan does not send a system notification by default

#### Scenario: Agent fails in background
- **WHEN** a supported coding agent reports failure or error in a background or
  unfocused pane
- **THEN** Alan may send a concise notification and mark the owning tab

### Requirement: Quick Terminal Presentation Is Native And Lightweight
The quick terminal SHALL present as a detached, lightweight native macOS Peak
window that preserves Alan's terminal-first shell design and avoids dashboard or
floating-card composition.

#### Scenario: Quick terminal appears
- **WHEN** the quick terminal is summoned
- **THEN** Alan presents a focused terminal surface with restrained native
  material chrome, no inspector, no marketing-style header, and no duplicate
  sidebar

#### Scenario: Quick terminal appears outside the main window
- **WHEN** the user summons the quick terminal from any macOS Space
- **THEN** Alan presents the Peak on the current active display and Space
  without requiring, attaching to, or raising Alan's main window

#### Scenario: Quick terminal hides
- **WHEN** the quick terminal is dismissed without closing the session
- **THEN** Alan hides the presentation without changing regular shell sidebar,
  tab, split, or terminal geometry

#### Scenario: Terminal keys remain terminal keys
- **WHEN** the quick terminal owns focus and the user presses `Esc`
- **THEN** Alan sends the key to the terminal surface instead of hiding the Peak
  by default

#### Scenario: Focus changes outside the Peak
- **WHEN** the quick terminal loses focus because the user clicks another app or
  window
- **THEN** Alan keeps the Peak visible until the user invokes the quick-terminal
  toggle, hide command, close command, or promotion action

#### Scenario: Quick terminal can become a normal tab
- **WHEN** the user opens the Peak's `Open in Space` affordance
- **THEN** Alan offers Alan space destinations without duplicating sidebar
  chrome inside the Peak

#### Scenario: Quick terminal activity exists
- **WHEN** the hidden quick terminal has user-actionable activity
- **THEN** Alan surfaces that activity through the same compact activity and
  notification policy used for regular terminal panes
