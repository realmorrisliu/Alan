## MODIFIED Requirements

### Requirement: Windows have isolated shell identities
The Alan macOS shell control plane SHALL have one active primary shell window
identity per running native app instance. Duplicate window or duplicate process
launch paths MUST NOT create competing shell control directories, socket paths,
event streams, persisted shell state files, or terminal runtime registries.

#### Scenario: Opening a second window
- **WHEN** the user invokes a second-window path such as New Window or `Command-N`
- **THEN** Alan focuses or reopens the existing primary shell window and does not create another `window_id`, socket path, control directory, or persisted state file

#### Scenario: Reading window state
- **WHEN** an agent queries the primary window's shell state
- **THEN** the response contains only spaces, tabs, panes, events, and focus state for the singleton primary shell window

#### Scenario: Forced duplicate process
- **WHEN** a forced second app process starts while the primary app instance owns the shell control plane
- **THEN** the second process exits without publishing a second socket, state file, event stream, or terminal runtime registry

#### Scenario: Reopening primary window
- **WHEN** the existing app process reopens the primary shell window after it was closed
- **THEN** the reopened window uses the app instance's singleton shell identity instead of allocating an independent window-scoped control plane
