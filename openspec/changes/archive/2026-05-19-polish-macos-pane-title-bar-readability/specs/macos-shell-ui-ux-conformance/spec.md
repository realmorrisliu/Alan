## MODIFIED Requirements

### Requirement: Terminal panes expose narrow title bars
Each visible macOS terminal pane SHALL include a compact title bar at the top of
the pane that identifies the terminal and provides a pane-scoped close
affordance while keeping terminal content visually dominant. The title bar SHALL
read as part of the terminal surface rather than as a separate selected chrome
overlay.

#### Scenario: Single pane title visible
- **WHEN** a terminal tab contains one visible pane
- **THEN** the pane shows a narrow title bar above the terminal canvas with a user-facing terminal title and one slim close button
- **AND** the title bar uses the same terminal-surface background as the canvas rather than a selected/unselected material wash above it

#### Scenario: Split pane titles visible
- **WHEN** a terminal tab contains multiple visible panes
- **THEN** every pane leaf shows its own title bar and close button without adding a pane selector strip, card grid, or debug labels
- **AND** title-bar backgrounds do not create per-pane cards, opaque overlays, or separate toolbar bands above each terminal pane

#### Scenario: Focused title remains readable
- **WHEN** a pane becomes the focused terminal pane
- **THEN** the pane title remains visible as text with sufficient foreground contrast against the terminal surface background
- **AND** focused state does not hide the title, blend it into the title-bar background, or replace it with an icon-only representation

#### Scenario: Long title fits
- **WHEN** a pane title is long or changes while the pane is visible
- **THEN** the title truncates within a stable fixed-height title bar without resizing split dividers, sidebar rows, toolbar content, or sibling panes

#### Scenario: Narrow title bar degrades predictably
- **WHEN** a pane title bar does not have enough width to show all detail
- **THEN** lower-priority accessories degrade from text plus icon to icon-only or hidden before the title text or close affordance disappear
- **AND** the title remains text with truncation rather than degrading to icon-only content

### Requirement: Pane title bars consume terminal metadata
Pane title bars SHALL consume the current terminal title already projected into
pane metadata, and SHALL use existing user-facing fallback labels only when the
terminal title is unavailable. Pane title-bar detail SHALL be presented in
left-to-right semantic priority order using fit-content item widths where space
allows.

#### Scenario: Terminal title exists
- **WHEN** a pane has a non-empty `viewport.title`
- **THEN** the title bar shows the normalized terminal title rather than raw pane IDs, cwd-first labels, runtime phases, or debug event text

#### Scenario: Terminal title missing
- **WHEN** a pane has no usable terminal title
- **THEN** the title bar falls back to cwd leaf, working-directory name, launch target, process name, or `Terminal` using user-facing copy

#### Scenario: Debug terms suppressed
- **WHEN** terminal metadata contains implementation-oriented summaries such as `title updated`, `window attached`, or raw runtime state
- **THEN** the title bar does not expose those terms outside explicit developer/debug-only surfaces

#### Scenario: Metadata stays in title chrome
- **WHEN** terminal status, branch, attention, or alan binding metadata is useful in the default pane UI
- **THEN** alan presents it as lightweight pane-title-bar accessories rather than as a persistent bottom status strip below the terminal canvas

#### Scenario: Detail order is semantic
- **WHEN** a pane title bar has title, activity, status, cwd or worktree, branch, process, alan state, and close detail available
- **THEN** alan orders visible content from left to right as title, activity/status, cwd or worktree, branch, process or alan state, and close
- **AND** each visible item uses fit-content width rather than reserving a fixed-width column for every accessory

#### Scenario: Detail fallback preserves priority
- **WHEN** available title-bar width cannot fit all detail labels
- **THEN** activity and status detail outrank cwd, worktree, branch, process, and alan detail
- **AND** cwd, worktree, branch, process, and alan detail can collapse to icon-only or hide before the title text collapses
