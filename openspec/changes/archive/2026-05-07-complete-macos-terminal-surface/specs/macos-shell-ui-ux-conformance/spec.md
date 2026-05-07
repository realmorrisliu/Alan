## ADDED Requirements

### Requirement: Terminal overlays use user-facing language
The macOS terminal UI SHALL present terminal search, child-exit, renderer
failure, readonly, input-not-ready, and clipboard states with concise
terminal-user language in the canvas area or inspector overview, while raw
runtime details remain debug-only.

#### Scenario: Renderer failure visible
- **WHEN** a focused terminal pane cannot render
- **THEN** the default UI explains that the terminal cannot draw and offers an actionable next step without showing raw Ghostty callback names or pane IDs

#### Scenario: Child exit visible
- **WHEN** a terminal child process exits
- **THEN** the pane shows a compact terminal exit state rather than debug event names

#### Scenario: Debug layer opened
- **WHEN** the user opens the inspector debug layer
- **THEN** renderer diagnostics, surface identifiers, input mode details, and raw event payloads may be inspected with debug framing

### Requirement: Terminal search does not displace workspace structure
Terminal search UI SHALL be compact, pane scoped, and layered over the terminal
workflow without turning the shell into a dashboard or page layout.

#### Scenario: Search opens
- **WHEN** the user invokes terminal search
- **THEN** the search control appears as a compact terminal tool for the focused pane and the sidebar, toolbar, and split layout keep stable dimensions

#### Scenario: Search closes
- **WHEN** the user dismisses terminal search
- **THEN** keyboard focus returns to the terminal pane that owned the search interaction
