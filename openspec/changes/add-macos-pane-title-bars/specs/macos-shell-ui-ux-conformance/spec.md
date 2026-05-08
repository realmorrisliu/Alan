## ADDED Requirements

### Requirement: Terminal panes expose narrow title bars
Each visible macOS terminal pane SHALL include a compact title bar at the top of
the pane that identifies the terminal and provides a pane-scoped close
affordance while keeping terminal content visually dominant.

#### Scenario: Single pane title visible
- **WHEN** a terminal tab contains one visible pane
- **THEN** the pane shows a narrow title bar above the terminal canvas with a user-facing terminal title and one close button

#### Scenario: Split pane titles visible
- **WHEN** a terminal tab contains multiple visible panes
- **THEN** every pane leaf shows its own title bar and close button without adding a pane selector strip, card grid, or debug labels

#### Scenario: Long title fits
- **WHEN** a pane title is long or changes while the pane is visible
- **THEN** the title truncates within a stable fixed-height title bar without resizing split dividers, sidebar rows, toolbar content, or sibling panes

### Requirement: Pane title bars consume terminal metadata
Pane title bars SHALL consume the current terminal title already projected into
pane metadata, and SHALL use existing user-facing fallback labels only when the
terminal title is unavailable.

#### Scenario: Terminal title exists
- **WHEN** a pane has a non-empty `viewport.title`
- **THEN** the title bar shows the normalized terminal title rather than raw pane IDs, cwd-first labels, runtime phases, or debug event text

#### Scenario: Terminal title missing
- **WHEN** a pane has no usable terminal title
- **THEN** the title bar falls back to working-directory name, cwd leaf, launch target, process name, or `Terminal` using user-facing copy

#### Scenario: Debug terms suppressed
- **WHEN** terminal metadata contains implementation-oriented summaries such as `title updated`, `window attached`, or raw runtime state
- **THEN** the title bar does not expose those terms outside the explicit inspector debug surface

### Requirement: Pane close button targets its pane
The pane title bar close button SHALL close the pane represented by that title
bar through the shared shell controller mutation path.

#### Scenario: Inactive split pane closed
- **WHEN** a user clicks the close button on a non-selected visible split pane
- **THEN** Alan closes that pane, repairs the split tree, and keeps the remaining pane runtimes alive without closing a different selected pane

#### Scenario: Single pane tab closed
- **WHEN** a user clicks the close button for the only pane in a tab and other tabs remain
- **THEN** Alan applies the existing tab-close semantics for that tab and focuses a remaining terminal pane

#### Scenario: Last remaining pane protected
- **WHEN** a close button targets the only pane in the only remaining tab
- **THEN** Alan keeps the shell state valid and does not remove the final workspace surface

### Requirement: Pane title bars preserve terminal input ownership
Pane title bars SHALL own only their explicit title and button controls, and
SHALL not intercept terminal input, selection, mouse reporting, scrollback, or
renderer hit-testing inside the terminal canvas.

#### Scenario: Terminal canvas clicked below title bar
- **WHEN** a user clicks, drags, scrolls, or right-clicks inside the terminal canvas below a pane title bar
- **THEN** the terminal host receives the event according to the terminal event ownership contract

#### Scenario: Close button clicked
- **WHEN** a user clicks the close button in the pane title bar
- **THEN** the button handles the pane close action without routing that click through terminal text input

#### Scenario: Title area clicked
- **WHEN** a user clicks the non-button title area
- **THEN** Alan may focus the pane, but it does not send text, mouse reports, or scroll events to the terminal application
