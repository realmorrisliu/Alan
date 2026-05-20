## ADDED Requirements

### Requirement: Tab Organization Mutations Report Authoritative Results
The macOS shell control plane and local command paths SHALL return results
derived from authoritative shell state after Tab reorder, pin/unpin, and Move to
Space mutations are accepted or rejected.

#### Scenario: Reorder succeeds
- **WHEN** a Tab reorder mutation is accepted
- **THEN** the result reports `applied: true` with the Tab ID, Space ID, section,
  and resulting index

#### Scenario: Pin succeeds
- **WHEN** a Tab pin mutation is accepted
- **THEN** the result reports `applied: true`, the Tab's pinned state, and the
  resulting section/index

#### Scenario: Move to Space succeeds
- **WHEN** a Tab moves to another Space
- **THEN** the result reports `applied: true`, source Space, target Space,
  section, resulting index, and resulting focused Space/Tab when focus changes

#### Scenario: Mutation is invalid
- **WHEN** a Tab organization mutation references a missing Tab, missing Space,
  invalid section, or invalid index
- **THEN** the result reports `applied: false` with a stable error code and
  leaves shell state unchanged

### Requirement: Tab Organization Events Are Observable
Tab organization mutations SHALL emit shell events with enough detail for
diagnostics and agents to observe ordering, pin state, Space movement, and focus
outcomes.

#### Scenario: Tab reordered
- **WHEN** a Tab order changes
- **THEN** the shell event stream records the Tab ID, Space ID, previous section
  and index, and current section and index

#### Scenario: Tab moved to another Space
- **WHEN** a Tab moves to another Space
- **THEN** the shell event stream records the previous and current Space,
  section, index, and focus outcome
