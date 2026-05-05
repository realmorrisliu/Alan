## ADDED Requirements

### Requirement: Runtime-Owned Request Control Resolution
Alan MUST resolve effective provider request controls through a runtime-owned
resolver before dispatching a model request. The resolver MUST combine turn
override, session/runtime override, agent config intent, model catalog default,
legacy provider budget intent, provider capabilities, and provider default in a
single typed result.

#### Scenario: Turn override has highest precedence
- **WHEN** a session has `model_reasoning_effort = "high"` and a turn requests `reasoning_effort = "low"`
- **THEN** the resolved request controls use `low` for that turn
- **AND** the session-level resolved controls remain unchanged for later turns

#### Scenario: Model default is applied once by the resolver
- **WHEN** no explicit reasoning effort or thinking budget is configured and the resolved model catalog entry declares default effort `medium`
- **THEN** the resolver returns reasoning effort `medium` with source `model_default`
- **AND** no other runtime, daemon, or provider-adapter layer recomputes that default independently

#### Scenario: Unknown model metadata uses provider default
- **WHEN** the selected provider/model has no model catalog metadata and no explicit request control is configured
- **THEN** the resolver returns no explicit reasoning effort or budget
- **AND** the source records that provider defaults will apply

### Requirement: Request Control Intent Is Separate From Resolved Controls
Alan MUST represent user/session/turn request-control intent separately from
resolved request controls. `Config` and transport DTOs MAY contain user intent,
but they MUST NOT be the authority for final effective request-control values.

#### Scenario: Config conflict remains local validation
- **WHEN** an agent config explicitly sets both `model_reasoning_effort` and `thinking_budget_tokens`
- **THEN** config loading rejects the config as ambiguous
- **AND** the resolver is not required to guess precedence between the two config fields

#### Scenario: RuntimeConfig does not duplicate independent effective truth
- **WHEN** a workspace overlay changes model metadata or a runtime launch applies a session override
- **THEN** Alan resolves request controls through the resolver
- **AND** `RuntimeConfig` does not require a separate sync helper to keep a duplicated effective reasoning field aligned with `Config`

### Requirement: Explicit Request Controls Are Validated Before Dispatch
Alan MUST validate explicit request controls against provider capability and
resolved model metadata before making a provider request. Explicit unsupported
controls MUST fail before dispatch instead of being silently dropped.

#### Scenario: Provider does not support effort control
- **WHEN** a turn explicitly requests reasoning effort and the selected provider declares no effort-control support
- **THEN** Alan rejects the turn before provider dispatch
- **AND** the error identifies the unsupported request control

#### Scenario: Model rejects unsupported effort
- **WHEN** the resolved model catalog entry supports only `low` and `high`
- **AND** a session or turn explicitly requests `xhigh`
- **THEN** Alan rejects the request before provider dispatch
- **AND** the error lists the supported efforts from the model metadata

### Requirement: Provider Adapters Only Project Normalized Controls
Provider adapters MUST consume normalized request controls from
`GenerationRequest` and project them to provider-specific payload fields.
Provider adapters MUST NOT own Alan-level override precedence, model default
selection, or config conflict resolution.

#### Scenario: Canonical effort overrides provider extra params
- **WHEN** a generation request contains normalized reasoning effort `low` and provider-specific extra params include `reasoning_effort = "high"`
- **THEN** the provider adapter sends `low`
- **AND** the provider-specific extra param does not create a competing effective value

#### Scenario: Legacy budget is normalized before projection
- **WHEN** `thinking_budget_tokens` is the only configured compatibility input
- **THEN** the runtime resolver produces a normalized budget control
- **AND** provider adapters project that budget according to provider rules without inferring Alan-level defaults

### Requirement: Daemon And Clients Mirror Resolver Metadata
Daemon session metadata, fork metadata, read responses, and client DTOs MUST
report request-control metadata produced by the runtime resolver. They MUST NOT
reconstruct reasoning-effort precedence independently.

#### Scenario: Create session response uses resolver output
- **WHEN** a session is created without explicit reasoning effort and the selected model default resolves to `medium`
- **THEN** the create-session response reports `reasoning_effort = "medium"`
- **AND** the value comes from runtime startup metadata

#### Scenario: Fork override uses resolver output
- **WHEN** a fork request explicitly overrides source-session reasoning effort
- **THEN** the forked session metadata reports the newly resolved effort
- **AND** daemon code does not merge source and override reasoning fields outside the runtime resolver

### Requirement: Request Control Tests Guard Layer Boundaries
Alan MUST include tests that cover resolver precedence, validation, provider
projection, and daemon metadata mirroring. Tests MUST make it hard to re-add
effective request-control logic in clients, daemon routes, provider adapters, or
runtime execution call sites.

#### Scenario: Resolver precedence matrix is tested
- **WHEN** resolver tests run
- **THEN** they cover turn override, session override, agent config, model default, legacy budget, and provider default cases

#### Scenario: Layering contract rejects duplicate resolution
- **WHEN** contract tests inspect request-control plumbing
- **THEN** they fail if `turn_executor` or daemon routes directly recompute effective reasoning effort instead of consuming resolver output
