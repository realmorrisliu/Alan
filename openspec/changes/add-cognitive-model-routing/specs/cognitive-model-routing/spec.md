## ADDED Requirements

### Requirement: Cognitive System Configuration
Alan SHALL allow agent configuration to declare System 1 and System 2 cognitive
profiles using existing connection profile identifiers and optional
reasoning-effort intent.

#### Scenario: Cognition config declares two systems
- **WHEN** `agent.toml` declares System 1 and System 2 cognition entries
- **THEN** Alan resolves each entry through the normal connection profile and
  request-control machinery

#### Scenario: Cognition config is absent
- **WHEN** an agent has no cognition configuration
- **THEN** Alan preserves existing single-profile behavior using the resolved
  `connection_profile`

#### Scenario: Invalid cognitive profile is rejected
- **WHEN** cognition config references a missing connection profile
- **THEN** Alan rejects startup or session creation with a diagnostic that names
  the missing profile

### Requirement: Runtime-Owned Cognitive Routing
Alan SHALL select the cognitive system in runtime before provider dispatch by
applying explicit overrides, deterministic gates, and System 1 self-escalation.

#### Scenario: Explicit override wins
- **WHEN** a session or turn explicitly requests System 2
- **THEN** Alan routes the turn to System 2 regardless of the default routing
  mode

#### Scenario: Deterministic gate forces System 2
- **WHEN** runtime detects a configured high-risk or high-complexity condition
  that requires deep reasoning
- **THEN** Alan routes the turn to System 2 before generating a fast draft

#### Scenario: Default route uses System 1
- **WHEN** no override or deterministic gate applies
- **THEN** Alan starts the turn on System 1

### Requirement: System 1 Self-Escalation
Alan SHALL provide an internal-only escalation action that lets System 1 request
a System 2 rerun with a bounded reason and needed-context summary.

#### Scenario: System 1 escalates
- **WHEN** System 1 emits the internal escalation action
- **THEN** runtime does not accept the System 1 draft as user-visible output
- **AND** runtime reruns the original task on System 2 with bounded triage notes

#### Scenario: Escalation action is not a user tool
- **WHEN** Alan exposes tool definitions to user-governed tools or client
  dynamic tools
- **THEN** the internal escalation action is not exposed as a normal external
  side-effecting tool

### Requirement: Cognitive Routing Observability
Alan SHALL record cognitive routing metadata for each routed turn without
exposing hidden reasoning content.

#### Scenario: Routed turn records metadata
- **WHEN** Alan dispatches a model request through cognitive routing
- **THEN** turn metadata records selected cognitive system, routing source,
  profile id, model, effective reasoning effort, and a bounded routing reason

#### Scenario: Escalated turn records both phases
- **WHEN** System 1 escalates to System 2
- **THEN** rollout metadata records that System 1 requested escalation and that
  System 2 produced the accepted draft

### Requirement: Single Runtime First Implementation
Alan SHALL implement cognitive routing inside the existing runtime turn loop
without making System 1 and System 2 separate child agents or default parallel
model executions.

#### Scenario: System 2 rerun remains same logical turn
- **WHEN** a System 1 attempt escalates to System 2
- **THEN** runtime preserves one logical user turn and records the escalation as
  routing metadata rather than spawning a separate child-agent session
