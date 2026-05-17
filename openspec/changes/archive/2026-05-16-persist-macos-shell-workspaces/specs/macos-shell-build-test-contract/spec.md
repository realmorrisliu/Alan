## ADDED Requirements

### Requirement: Workspace persistence verification covers Tab lifecycle
Changes to macOS shell workspace persistence SHALL include focused verification for manifest startup, Space retention, Pinned Tab restore snapshots, Unpinned Tab TTL retirement, and active-task retirement protection.

#### Scenario: Manifest startup behavior is tested
- **WHEN** workspace persistence changes are implemented
- **THEN** focused tests cover missing manifest default creation and corrupt manifest quarantine with fresh default startup

#### Scenario: Space retention is tested
- **WHEN** tab close or lifecycle retirement can leave a Space without Tabs
- **THEN** focused tests or manual notes verify the Space remains visible and selected with an empty workspace state

#### Scenario: Pinned Tab restore is tested
- **WHEN** Pinned Tab persistence is implemented
- **THEN** focused tests cover single-pane cwd restoration, split layout restoration, and the fact that post-pin transient split/cwd changes do not update the pin snapshot without an explicit update-pin action

#### Scenario: Unpinned Tab TTL is tested
- **WHEN** Unpinned Tab lifecycle pruning is implemented
- **THEN** focused tests cover retained Tabs inside the 12 hour TTL, retired inactive Tabs after the TTL, and selection repair when the selected Tab is retired

#### Scenario: Active tasks are tested
- **WHEN** terminal-aware active-task metadata is used for pruning
- **THEN** focused tests cover foreground command protection, alan pending/yield protection, and idle shell eligibility for retirement
