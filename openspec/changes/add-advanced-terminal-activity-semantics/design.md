## Context

Alan's macOS shell already has the important foundations for this work:
terminal panes have stable runtime handles, pane metadata projects title/cwd/
attention/active-task state into shell state, pane title bars can host
lightweight metadata accessories, and the sidebar is designed around compact
space/tab scanning. The gap is that activity is still fragmented. Ghostty
progress reports and command-finished actions arrive through the host, Alan
runtime activity has its own state, and third-party CLI agent status is not yet
modeled as terminal activity.

The research target spans three related groups:

- A: activity and attention, including Ghostty progress, command completion,
  CLI coding-agent status, tab metadata, and notification policy.
- B: semantic terminal behavior, including prompt/command boundaries, command
  output ranges, prompt navigation, copy-last-output, and search/browse flows.
- Quick terminal: a detached global Peak terminal that can be summoned from any
  macOS Space while using the same shell and runtime contracts instead of a
  separate terminal model.

This change is intentionally a draft owner for all three groups. The first
implementation-ready detail pass should focus on A.

## Goals / Non-Goals

**Goals:**

- Define a durable terminal activity model that can ingest Ghostty progress,
  command completion, bell/attention, Alan session activity, and supported CLI
  coding-agent lifecycle events.
- Keep activity display terminal-first: pane title-bar accessories, sidebar row
  metadata, accessibility, and optional notifications rather than dashboards or
  bottom status strips.
- Make sidebar tab rows denser and more useful as tab-level attention/identity
  surfaces while keeping pane title bars responsible for pane-local detail.
- Define semantic command-boundary behavior that can later power prompt
  navigation, copy-last-output, and command-output browsing without adopting
  full Warp-style blocks.
- Define quick terminal behavior as a presentation and workspace interaction
  over the existing terminal runtime service.
- Split future work into phases so A can be refined and implemented before B
  and quick terminal.

**Non-Goals:**

- Implement Warp blocks, Warp Agent View, code review panels, file trees, or
  changes sidebars.
- Solve remote SSH/mux/cloud terminal persistence in this change.
- Require provider-specific CLI agent plugins for the first draft. Structured
  plugin or hook support can be added per agent after the Alan-side contract is
  stable.
- Replace existing terminal title/cwd/search/scrollback contracts.

## Decisions

### 1. Use one normalized activity model above source-specific adapters

Alan should introduce a normalized terminal activity snapshot owned by the
macOS shell/runtime boundary. Source adapters translate Ghostty progress,
command-finished actions, bells, Alan session state, and CLI agent events into
that snapshot. UI and control-plane consumers read the normalized state, not
source-specific structs.

The model should carry enough normalized state to produce both pane-local and
tab-level display projections:

- source kind: Codex, Claude, OpenCode, Alan, shell, progress, command, process,
  or unknown
- status: needs input, failed, paused, progress, running, bell, exited, idle,
  done, or stale
- priority/attention: passive, active, notable, or awaiting user
- progress value: absent, indeterminate, percent, paused, or failed
- command metadata: exit code, duration when available, command boundary when
  known, and last update time
- agent metadata: agent kind, session id when available, project/cwd when
  available, and status detail when safe to show
- display labels: reliable source label when known, fallback source label,
  state label, detail label, optional pane hint, and progress display
- freshness: updated-at timestamp plus optional expiry/stale deadlines

The normalized model should expose two display projections rather than making
every UI surface repeat merge logic:

- `paneActivityDisplay`: the pane-local activity and detail for pane title bars,
  accessibility, and control surfaces
- `tabActivityDisplay`: the highest-priority sidebar-worthy activity selected
  across the tab's panes for sidebar rows and tab-level attention

Alternative considered: add separate UI state for Ghostty progress, command
completion, and agent notifications. That is quicker initially but would repeat
the same sidebar/titlebar/notification decisions for every source.

### 2. Treat A as the first implementation phase

The first phase should focus on activity ingestion, priority, freshness,
projection, and low-noise notification policy. B and quick terminal stay in the
draft spec so the direction is not forgotten, but they should not block the A
phase design.

Alternative considered: implement semantic command boundaries first. That would
help command-finished quality, but Alan already receives enough Ghostty and
agent signals to build useful activity UI without waiting for full prompt
semantics.

### 3. Prefer structured CLI agent signals, with conservative fallback

Alan should prefer structured agent events or documented notification hooks for
Codex, Claude Code, OpenCode, and similar CLIs. The first implementation can
start with the highest-confidence signals and an unknown-agent fallback. It
should not depend on brittle screen scraping of agent TUI text.

For unsupported or partially supported agents, Alan may show process-level
activity such as foreground command running or command finished, but it should
not claim precise "needs input" or "done" states without a reliable signal.

Alternative considered: parse common TUI output for prompts and status. That is
fragile across agent releases, localization, themes, terminal widths, and
alternate-screen rendering.

### 4. Keep semantic terminal behavior lighter than blocks

Alan should model prompt marks, command start/end, command text, output ranges,
exit status, and search ranges as semantic terminal metadata. It should expose
focused actions such as jump previous/next prompt, copy last command output,
and browse/search command output. It should not turn every command into a
visible card or block by default.

This preserves Alan's terminal-first UI while still making terminal output more
usable by humans and agents.

Alternative considered: copy Warp's command block model. That would make
command-level interactions explicit, but it conflicts with Alan's current
Arc-like shell direction and risks making the terminal feel like an IDE panel.

The B-group MVP should be action-only. When reliable prompt or command boundary
metadata exists, `Go to or Command...` can expose command-aware actions:

- jump to previous prompt
- jump to next prompt
- copy last command output
- search last command output

When reliable boundary metadata is absent, Alan falls back to ordinary terminal
behavior such as scrollback search, selection copy, and normal scrollback
navigation. It must not keep showing precise "last command output" actions while
guessing from screen text. The MVP does not add a command browser, visible
command blocks, or terminal output segmentation. It may store recent
`CommandSegment` metadata to support the actions, but that storage is pane-local
runtime metadata for the current terminal session rather than a long-term
command history database.

### 5. Implement quick terminal as a detached global Peak over normal runtimes

Quick terminal should be a detached, globally summonable macOS Peak window, not
a child popover of Alan's main window. It hosts one global quick-terminal
pane/runtime and uses the same lifecycle, metadata, activity, clipboard, search,
and command-routing contracts as regular panes.

The draft default shortcut is a configurable global toggle, initially
`Option+Space`. When the quick terminal is hidden, invoking the toggle presents
the same global instance on the current macOS Space and active display without
forcing Alan's main window to the front. When it is visible, invoking the same
toggle hides it. `Esc` must remain terminal input and must not dismiss the Peak
by default. Losing focus also does not hide the Peak; hide is an explicit toggle
or command.

Hide and show preserve scrollback and process state unless the user explicitly
closes the quick terminal. If the global quick instance already exists, Alan
restores that instance and its cwd. If Alan must create a new quick terminal, it
chooses the cwd from the currently focused Alan pane when available, otherwise
the last quick-terminal cwd, otherwise the user's home directory.

The Peak can be promoted into a normal Alan Space through an `Open in Space`
affordance. Promotion moves the current quick-terminal runtime into the selected
Space as a normal tab, hides the Peak, and clears the global quick slot. Alan
does not copy the terminal process or display the same runtime simultaneously in
the Peak and a tab.

Alternative considered: create a separate AppKit terminal panel with its own
runtime owner. That would ship faster but would duplicate lifecycle, focus, and
metadata behavior.

### 6. Sidebar tab rows are tab-level attention and identity surfaces

Sidebar tab rows should become richer work rows, but their job is still quick
location and attention triage, not pane inspection. A tab row shows:

- a leading visual slot
- a title line
- one secondary line that shows activity when activity exists, otherwise
  worktree/branch context
- an optional progress rail only when the displayed activity owns progress

The leading slot expresses how to enter the tab. Single-pane tabs use the
semantic kind or agent icon. Split tabs replace that icon with split topology;
clicking the topology cycles focus through visible panes in order. Agent or Alan
identity inside a split tab appears in title/activity metadata, not by mixing
another icon into the topology slot.

Tab titles initially follow the focused or primary pane. This keeps the first
implementation simple and makes focus cycling visible in the row. The
tab-level activity line, however, is selected from the highest-priority activity
across the entire tab. If that activity belongs to a non-focused pane, the row
uses a short pane hint such as `Pane 2`.

When no sidebar-worthy activity exists, the secondary line falls back to
worktree/repository leaf and branch rather than full cwd. This keeps the tab
recognizable without turning the sidebar into a pane inspector.

Alternative considered: keep the existing icon/title/subtitle row and only
replace the subtitle with activity. That would be smaller, but it preserves the
current low-density feel and keeps forcing status, cwd, branch, and process to
compete for one field.

### 7. Pane title bars provide pane-local detail

Pane title bars keep terminal title as the primary label and use accessories
for pane-local activity, worktree/cwd, branch, process, and agent/Alan state.
This separates the surfaces: the sidebar answers "which tab needs attention?",
while the pane title bar answers "what is this pane specifically doing?".

Pane-local accessories can show more detail than the sidebar, but they still
avoid raw IDs, hook payloads, event names, and redundant labels. If activity
already says `Codex · Running`, a separate Codex marker should not repeat the
same information.

Alternative considered: move cwd/branch/process into the sidebar as permanent
context. That is useful for identification, but it duplicates common terminal
titles and makes the tab row feel like a compact inspector instead of an
attention surface.

### 8. Activity priority is action-first, with progress above generic running

Sidebar activity should be source-first in its copy, such as `Codex · Input
needed`, `Build · 42%`, `Progress · 42%`, or `Shell · Command failed 1`.
Reliable source labels are used when available; otherwise progress falls back
to `Progress` rather than guessing from output text or cwd.

The initial sidebar priority is:

1. input needed, approval, or blocked
2. failed, error, or command failed
3. paused
4. active progress
5. running agent or long foreground command
6. bell or exited
7. no sidebar-worthy activity, so show worktree/branch context

Successful commands and idle states do not become sidebar activity. They may be
pane-local transient detail or disappear immediately. Progress rail is shown
only when the displayed activity itself owns progress, so the row never shows a
progress bar for one pane while the copy describes another pane.

Initial freshness rules are intentionally simple:

- Progress clears or becomes stale after 15 seconds without an update or clear.
- Command failure remains sidebar-worthy until the user focuses the tab or for
  about 30 seconds, whichever comes first.
- Bell is brief, roughly 5-10 seconds, unless another source upgrades it.
- Process exited remains visible until the user closes or restarts the pane.
- Running agent remains while a reliable agent/process signal says it is active.
- Needs-input, approval, blocked, failed, and error states remain until a newer
  reliable state replaces them.

These windows are defaults, not user-visible promises. They should be easy to
tune after visual review.

Alternative considered: generic running above progress. That emphasizes which
agent is active, but it hides reliable progress and makes the progress rail less
useful.

### 9. UI projection stays compact and progressive

Activity should surface through pane title-bar accessories, sidebar secondary
line/rail, accessibility values, and optional notifications. The default UI
must not add persistent bottom strips, debug labels, large attention buttons,
notification dots for every background change, or dashboard sections.

Close controls in sidebar tab rows should be hover/focus overlays that do not
reserve a permanent trailing layout slot. The row content may use a subtle fade
or material treatment under the overlay so text does not collide visually with
the close button.

Notifications should be reserved for actionable or out-of-view activity:
agent needs input, agent error, long command finished while unfocused, or
explicit user opt-in. Foreground progress should usually stay visual only.

The first source adapters should be Ghostty progress, Ghostty command
finished/failed, Alan binding/session state, and Codex. That slice covers the
highest-confidence local signals and the initial CLI-agent notification use
case before broadening to Claude Code or OpenCode.

Default notification policy should notify for agent needs input, agent
failed/error, long command failed or completed in background, and unexpected
process exit. It should not notify for foreground progress, command success,
idle, or generic running.

Alternative considered: use a notification center panel as the primary surface.
That can be useful later, but it would be the wrong default center of gravity
for a terminal-first shell.

## Risks / Trade-offs

- Stale progress can linger after interrupted programs -> Activity snapshots
  need source-specific freshness policies and timeout/clear semantics.
- Notifications can become noisy -> Default policy must gate on focus,
  severity, duration, and explicit user intent.
- Richer sidebar rows can become mini dashboards -> Keep only title,
  activity-or-context, optional rail, and leading topology/kind slot in the
  default row.
- Hover close overlays can obscure text -> Apply fade/material treatment under
  the overlay and keep the row accessible through keyboard focus.
- CLI agent protocols can change -> Keep per-agent adapters small and let
  unsupported agents fall back to generic terminal activity.
- Semantic prompt data can be missing inside tmux, ssh, or unsupported shells ->
  Semantic actions must degrade to normal scrollback/search behavior.
- Quick terminal can fight normal window focus -> It must not bring Alan's main
  window forward on summon, must stay visible across focus changes until the
  user toggles or closes it, and must avoid stealing input while hidden.
- A broad draft can become too large to implement -> Tasks are split into A,
  B, and quick-terminal phases, with A as the first refinement target.

## Migration Plan

1. Add the normalized activity types and projection paths without changing the
   visible UI.
2. Map existing Ghostty progress, command-finished, bell, child-exit, and active
   task signals into the normalized activity model.
3. Refactor sidebar tab row projection to the richer title plus
   activity-or-context layout with hover-only close overlay and leading
   topology/kind slot.
4. Update pane title-bar projection to read pane-local normalized activity while
   preserving terminal title as the primary label.
5. Add CLI agent adapters incrementally behind focused tests.
6. Add semantic command-boundary storage and actions after A is stable.
7. Add the detached global quick terminal Peak after the shared
   runtime/activity behavior is proven in regular panes.

## Open Questions

- Which CLI agent should be the first structured adapter: Codex, Claude Code,
  or OpenCode? The current draft defaults to Codex unless implementation
  evidence shows another adapter is materially lower risk.
- What exact notification defaults should Alan use for foreground vs
  background panes? The current draft defaults to notifying only actionable or
  out-of-view states.
- What exact visual size/placement should the Peak use on small displays and
  external monitors? The current draft assumes current active display placement
  with size clamped to the visible rect.
