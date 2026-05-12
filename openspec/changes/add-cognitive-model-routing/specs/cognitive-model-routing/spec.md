## ADDED Requirements

### Requirement: Cognitive System Configuration
Alan SHALL allow agent configuration to declare System 1 and System 2 cognitive
model bindings that resolve through available provider, credential, and model
configuration with optional reasoning-effort intent.

#### Scenario: Cognition config declares two model bindings
- **WHEN** `agent.toml` declares System 1 and System 2 cognition entries
- **THEN** Alan resolves each entry to a concrete provider, credential scope,
  model, and request-control intent before provider dispatch

#### Scenario: Cognition config omits a required system
- **WHEN** cognition routing is enabled but System 1 or System 2 is not
  configured
- **THEN** Alan rejects startup or session creation with a diagnostic that names
  the missing cognitive system

#### Scenario: Cognition config is absent
- **WHEN** an agent has no cognition configuration
- **THEN** Alan preserves existing single-profile behavior using the resolved
  `connection_profile`

#### Scenario: Invalid cognitive model binding is rejected
- **WHEN** cognition config references a missing provider, credential scope, or
  model binding
- **THEN** Alan rejects startup or session creation with a diagnostic that names
  the missing binding component

### Requirement: Provider Availability And Cognitive Binding Separation
Alan SHALL keep provider and credential availability separate from System
1/System 2 cognitive-role assignment.

#### Scenario: Provider availability is role-neutral
- **WHEN** Alan loads configured AI providers and available models
- **THEN** those provider/model entries do not themselves imply System 1 or
  System 2 behavior

#### Scenario: Cognitive binding selects from availability
- **WHEN** Alan resolves System 1 or System 2
- **THEN** the cognitive binding selects from available provider/model entries
  rather than duplicating provider credentials inside the cognition block

### Requirement: Runtime-Owned Cognitive Routing
Alan SHALL select the cognitive system in runtime before provider dispatch by
applying explicit overrides, deterministic safety gates, configured defaults,
System 1 fallback, and System 1 self-escalation. Deterministic safety gates
SHALL supersede any explicit System 1 routing intent.

#### Scenario: Explicit System 2 override wins
- **WHEN** a session or turn explicitly requests System 2
- **THEN** Alan routes the turn to System 2 regardless of the default routing
  mode

#### Scenario: Deterministic gate forces System 2
- **WHEN** runtime detects a configured high-risk or high-complexity condition
  that requires deep reasoning
- **THEN** Alan routes the turn to System 2 before generating a fast draft

#### Scenario: System 1 override is superseded by gate
- **WHEN** a session or turn explicitly requests System 1
- **AND** runtime detects a configured high-risk or high-complexity condition
  that requires deep reasoning
- **THEN** Alan ignores or rejects the forced System 1 intent and routes the
  turn to System 2 before generating a fast draft
- **AND** the routing metadata records that the deterministic gate superseded
  the override

#### Scenario: Configured default route is honored
- **WHEN** no override or deterministic gate applies
- **AND** the resolved cognition config declares a default cognitive system
- **THEN** Alan routes the turn to the configured default cognitive system

#### Scenario: Missing default falls back to System 1
- **WHEN** no override, deterministic gate, or configured default applies
- **THEN** Alan starts the turn on System 1

### Requirement: System 1 Self-Escalation
Alan SHALL provide an internal-only escalation action that lets System 1 request
a System 2 rerun with a bounded reason and needed-context summary. Alan SHALL
withhold side-effecting tools from unaccepted System 1 attempts until runtime
accepts the System 1 route for execution or routes the turn to System 2.
Runtime acceptance of a System 1 route is an internal commit point and SHALL NOT
itself require user confirmation unless the active governance or tool policy
requires confirmation.

#### Scenario: System 1 escalates
- **WHEN** System 1 emits the internal escalation action
- **THEN** runtime does not accept the System 1 draft as user-visible output
- **AND** runtime reruns the original task on System 2 with bounded triage notes

#### Scenario: Escalation action is not a user tool
- **WHEN** Alan exposes tool definitions to user-governed tools or client
  dynamic tools
- **THEN** the internal escalation action is not exposed as a normal external
  side-effecting tool

#### Scenario: Escalation before external side effects
- **WHEN** System 1 determines that a task needs System 2 before any
  side-effecting tool has executed
- **THEN** runtime reruns the original logical turn on System 2 and includes the
  bounded System 1 triage notes

#### Scenario: Read-only context before escalation
- **WHEN** System 1 used read-only tools before emitting the internal escalation
  action
- **THEN** runtime provides the read-only tool results to System 2 as observed
  context instead of discarding them

#### Scenario: Speculative System 1 thinking and observation is allowed
- **WHEN** runtime starts an automatic System 1 attempt
- **THEN** System 1 can perform model-internal reasoning, calculation, planning,
  unaccepted draft generation, and read-only tool use before route acceptance
- **AND** runtime does not treat that speculative thinking or read-only
  observation as an external side effect

#### Scenario: Side-effecting tool is blocked before System 1 acceptance
- **WHEN** runtime starts an automatic System 1 attempt
- **AND** System 1 requests a side-effecting tool before runtime has accepted
  the System 1 route for execution
- **THEN** runtime does not execute the side-effecting tool in the unaccepted
  System 1 phase
- **AND** runtime routes to System 2 or defers the side effect until the System
  1 route is accepted

#### Scenario: Autonomous System 1 route can be accepted without user yield
- **WHEN** runtime starts an automatic System 1 attempt under governance that
  allows autonomous execution
- **AND** no deterministic gate or policy rule requires System 2 or user
  confirmation
- **THEN** runtime can accept the System 1 route for execution without emitting
  a user-confirmation yield

#### Scenario: Accepted side effect already happened before escalation
- **WHEN** a side-effecting tool has already completed after runtime accepted
  the System 1 execution phase or after an external client changed state
- **THEN** runtime treats the side effect as part of the current session state
  and System 2 continues from the observed post-side-effect state rather than
  replaying the original task as if no side effect occurred

### Requirement: Cognitive Routing Observability
Alan SHALL record cognitive routing metadata for each routed turn without
exposing hidden reasoning content.

#### Scenario: Routed turn records metadata
- **WHEN** Alan dispatches a model request through cognitive routing
- **THEN** turn metadata records selected cognitive system, routing source,
  model binding id, provider, model, effective reasoning effort, and a bounded
  routing reason

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

### Requirement: Provider-Native Continuation Partitioning
Alan SHALL treat provider-native continuation state as an optimization scoped to
compatible cognitive model bindings, while preserving tape-level continuation
across System 1 and System 2.

#### Scenario: Compatible binding reuses native continuation
- **WHEN** the selected cognitive model binding has the same provider family,
  credential scope, model, and continuation-affecting settings as the current
  provider-native continuation state
- **THEN** runtime can reuse that provider-native continuation state

#### Scenario: Incompatible binding clears native continuation
- **WHEN** cognitive routing selects a model binding with a different provider
  family, credential scope, model, or continuation-affecting setting
- **THEN** runtime clears or isolates provider-native continuation and projects
  the accepted tape into the selected provider request instead

#### Scenario: Tape continuation remains authoritative
- **WHEN** provider-native continuation cannot be reused after a cognitive
  system switch
- **THEN** Alan still continues from the accepted runtime tape rather than
  losing conversation state
