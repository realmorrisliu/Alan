## ADDED Requirements

### Requirement: Workspace manifest is the restore authority
The macOS shell SHALL use a versioned workspace manifest as the authoritative source for restoring Spaces, Tabs, pin snapshots, Tab lifecycle metadata, and the last selected Space/Tab across app restarts.

#### Scenario: Manifest is present
- **WHEN** Alan for macOS starts and a valid workspace manifest exists for `window_main`
- **THEN** alan loads Spaces, Tabs, pin snapshots, lifecycle metadata, and the last selected Space/Tab from that manifest
- **AND** alan materializes the current shell state from the manifest rather than bootstrapping a fresh default state

#### Scenario: Manifest is missing
- **WHEN** Alan for macOS starts and no workspace manifest exists for `window_main`
- **THEN** alan creates a default manifest with one default Space and one default unpinned terminal Tab
- **AND** alan uses that manifest as the restore authority for the launched shell state

#### Scenario: Legacy shell state exists without manifest
- **WHEN** `shell-state-window_main.json` exists but no workspace manifest exists
- **THEN** alan does not migrate that legacy shell state into the workspace manifest
- **AND** alan creates a default manifest instead

### Requirement: Corrupt workspace manifests fail open safely
The macOS shell SHALL preserve evidence of a malformed workspace manifest and start with a default workspace rather than failing to launch or silently overwriting the only copy.

#### Scenario: Manifest cannot be decoded
- **WHEN** Alan for macOS starts and the workspace manifest cannot be decoded
- **THEN** alan preserves the bad manifest as a timestamped corrupt file
- **AND** alan creates a fresh default manifest
- **AND** alan starts with the default workspace

#### Scenario: Fresh manifest is written after corruption
- **WHEN** alan creates a default manifest after detecting corruption
- **THEN** future workspace mutations write to the fresh manifest path
- **AND** the corrupt file remains available for diagnostics

### Requirement: Spaces persist until explicit deletion
The macOS shell SHALL treat Spaces as durable user-created containers that remain visible until the user explicitly deletes the Space.

#### Scenario: Empty Space remains visible
- **WHEN** the last Tab in a Space is closed or retired
- **THEN** alan keeps the Space in the workspace manifest
- **AND** the sidebar continues to show that Space
- **AND** selecting that Space shows an empty workspace state instead of deleting the Space

#### Scenario: Tab retirement does not delete Space
- **WHEN** automatic Tab lifecycle retirement removes every Tab from a Space
- **THEN** alan keeps the Space record and its ordering in the workspace manifest

#### Scenario: Space is explicitly deleted
- **WHEN** the user invokes a delete-space action for a Space
- **THEN** alan removes that Space and its Tabs from the workspace manifest
- **AND** alan chooses a remaining Space or creates a default Space if no Spaces remain

### Requirement: Pinned Tabs restore from explicit snapshots
The macOS shell SHALL persist Pinned Tabs by saving an explicit restore snapshot at pin or update-pin time, and SHALL restore from that snapshot rather than from later transient Tab mutations.

#### Scenario: Single-pane Tab is pinned
- **WHEN** the user pins a Tab that contains one terminal pane
- **THEN** alan saves a pin snapshot with that pane's cwd, launch target, title, and Tab identity
- **AND** future app launches restore that Pinned Tab as a new terminal pane at the pinned cwd

#### Scenario: Split Tab is pinned
- **WHEN** the user pins a Tab that contains a split layout
- **THEN** alan saves the split tree and each leaf pane's cwd and launch target in the pin snapshot
- **AND** future app launches restore the Pinned Tab with that split layout and pane cwd mapping

#### Scenario: Pinned Tab changes after pinning
- **WHEN** a Pinned Tab is split, moved, resized, or cd'd after the pin snapshot was saved
- **THEN** alan does not update the pin snapshot automatically
- **AND** future app launches restore the Tab from the saved pin snapshot

#### Scenario: User updates the pin snapshot
- **WHEN** the user explicitly updates or re-applies pinning for an already Pinned Tab
- **THEN** alan replaces the prior pin snapshot with the Tab's current restorable layout and cwd state

### Requirement: Unpinned Tabs restore until inactive TTL expiry
The macOS shell SHALL retain Unpinned Tabs across app restarts until they are inactive and older than the configured lifecycle TTL.

#### Scenario: Unpinned Tab is inside TTL
- **WHEN** an Unpinned Tab has `max(lastActivatedAt, lastActivityAt)` within 12 hours
- **THEN** alan keeps that Tab in the workspace manifest
- **AND** app restart restores it as a new terminal runtime at its latest restorable cwd or layout

#### Scenario: Unpinned Tab expires while inactive
- **WHEN** an Unpinned Tab is not pinned
- **AND** it has no active task
- **AND** `now - max(lastActivatedAt, lastActivityAt)` is greater than 12 hours
- **THEN** alan retires that Tab from the workspace manifest during lifecycle pruning

#### Scenario: Selected Tab expires
- **WHEN** the selected Tab is retired during startup pruning
- **THEN** alan selects the first remaining Tab in the selected Space
- **AND** if the selected Space has no remaining Tabs, alan keeps the Space selected with no selected Tab

### Requirement: Active tasks prevent unpinned Tab retirement
The macOS shell SHALL protect Unpinned Tabs from lifecycle retirement when terminal-aware metadata indicates that user work is actively running or waiting for input.

#### Scenario: Foreground command is running
- **WHEN** an Unpinned Tab contains a terminal pane with an active foreground command
- **THEN** alan treats that Tab as having an active task
- **AND** lifecycle pruning does not retire it solely because its TTL anchor is older than 12 hours

#### Scenario: alan session is active
- **WHEN** an Unpinned Tab contains an alan session that is running, waiting for input, or pending yield
- **THEN** alan treats that Tab as having an active task
- **AND** lifecycle pruning does not retire it solely because its TTL anchor is older than 12 hours

#### Scenario: Shell is idle
- **WHEN** an Unpinned Tab contains only an idle shell prompt
- **THEN** alan does not treat `processExited == false` by itself as an active task
- **AND** the Tab can be retired after TTL expiry

### Requirement: Shell state remains a runtime snapshot
The macOS shell SHALL keep `ShellStateSnapshot` as the current UI/control-plane/runtime projection while using the workspace manifest as the durable restore authority.

#### Scenario: Runtime metadata changes
- **WHEN** terminal title, cwd, renderer state, attention, or alan binding metadata changes
- **THEN** alan updates current shell state for UI and control-plane publication
- **AND** alan writes only restorable workspace intent and lifecycle metadata back to the manifest

#### Scenario: App restarts
- **WHEN** Alan for macOS restarts after publishing a shell state file in the previous process
- **THEN** alan restores Spaces and Tabs from the workspace manifest
- **AND** terminal runtimes are newly created rather than restored from the old shell state process snapshot
