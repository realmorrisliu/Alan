## ADDED Requirements

### Requirement: New terminal tabs inherit focused pane cwd
The macOS shell SHALL create user-requested terminal tabs in the focused pane's
current working directory unless the caller supplies an explicit working
directory or no valid focused-pane cwd exists.

#### Scenario: Runtime cwd is current
- **WHEN** the user invokes New Terminal Tab from a focused pane whose runtime metadata reports cwd `/repo/app`
- **THEN** the new tab's initial pane starts in `/repo/app`
- **AND** the new tab's pane snapshot records `/repo/app` as its cwd

#### Scenario: Snapshot cwd is fallback
- **WHEN** the focused pane has no runtime cwd metadata but its shell pane snapshot records cwd `/repo/app`
- **THEN** the new terminal tab starts in `/repo/app`

#### Scenario: Explicit cwd wins
- **WHEN** a control-plane or command path opens a new terminal tab with an explicit cwd `/tmp/work`
- **THEN** alan starts the new tab in `/tmp/work` even if the focused pane has a different cwd

#### Scenario: Missing focused cwd falls back
- **WHEN** a new terminal tab is requested and alan cannot resolve a valid cwd from the focused pane or request
- **THEN** alan falls back to the workspace default working directory or the user's home directory

#### Scenario: Split and tab creation agree
- **WHEN** a user creates a split pane and a new terminal tab from the same focused pane
- **THEN** both new terminal runtimes use the same cwd resolution order
