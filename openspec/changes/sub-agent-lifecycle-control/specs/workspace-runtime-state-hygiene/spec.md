## ADDED Requirements

### Requirement: Generated Workspace State Ignore Rules
Repository ignore rules SHALL ignore generated workspace `.alan` runtime state by default while allowing authored agent definitions to remain trackable.

#### Scenario: Generated sessions and memory exist
- **WHEN** a workspace contains generated `.alan` runtime state such as `.alan/sessions/` or `.alan/memory/`
- **THEN** normal repository status does not show those generated files as untracked source changes

#### Scenario: Authored agent definitions exist
- **WHEN** a workspace contains `.alan/agents/default/`, `.alan/agents/<name>/`, `.alan/models.toml`, policies, or authored skill packages intended for source control
- **THEN** repository ignore rules do not prevent those authored files from being tracked
- **AND** documentation explains that `.alan/agents/default/` is the workspace default agent definition root while `.alan/agents/<name>/` is the workspace named-agent definition root

### Requirement: Alan Home Workspace State
The system SHALL prevent Alan home from being treated as a normal workspace that creates nested `~/.alan/.alan/` runtime state.

#### Scenario: Alan home is selected as workspace
- **WHEN** the resolved workspace root is Alan home
- **THEN** runtime state paths resolve to the canonical Alan home layout rather than appending another `.alan`

#### Scenario: Legacy nested state exists
- **WHEN** legacy nested Alan-home runtime state is detected
- **THEN** the system reports the condition safely without deleting data implicitly

### Requirement: Canonical Workspace Identity
The system SHALL compare workspace identities using canonical paths where available.

#### Scenario: Same workspace uses path casing variants
- **WHEN** two paths refer to the same workspace on a case-insensitive filesystem
- **THEN** runtime manager and registry identity checks resolve them to one workspace identity

#### Scenario: Path cannot be canonicalized
- **WHEN** a workspace path cannot be canonicalized because it does not exist yet
- **THEN** the system uses a deterministic normalized fallback and canonicalizes after creation where practical

### Requirement: Generated State Documentation
The documentation SHALL explain which `.alan` paths are generated runtime state and which paths may be source-controlled.

#### Scenario: Developer reads workspace state docs
- **WHEN** a developer checks the repository documentation for `.alan` workspace state
- **THEN** the docs identify generated sessions/memory paths separately from authored agent roots, policies, and skills
