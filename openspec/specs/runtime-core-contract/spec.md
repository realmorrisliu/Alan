# runtime-core-contract Specification

## Purpose
Defines durable runtime-core contracts for sessions, turns, tape, rollout,
operations, emitted events, compaction, scheduling, rollback, fork, and recovery
semantics.

## Requirements
### Requirement: Runtime core contracts live in OpenSpec
alan SHALL keep durable runtime, kernel, execution, compaction, scheduler,
interaction-inbox, durable-run, and app-server protocol requirements in
OpenSpec rather than in `docs/spec/` contract pages.

#### Scenario: Runtime behavior changes
- **WHEN** a change modifies session, turn, tape, rollout, compaction,
  scheduling, rollback, fork, app-server protocol, or interaction input-mode
  behavior
- **THEN** the requirement is added to this capability, an existing runtime
  capability, or an active OpenSpec change
- **AND** no long-form replacement contract is authored under `docs/spec/`

#### Scenario: Legacy runtime contract is referenced
- **WHEN** active documentation still links to a legacy runtime contract under
  `docs/spec/`
- **THEN** that file is a short bridge to this capability, `daemon-api-contract`,
  `runtime-memory-surfaces`, `child-run-lifecycle`, or another named OpenSpec
  owner
- **AND** the bridge does not restate the full legacy contract

### Requirement: Runtime object boundaries remain explicit
alan SHALL preserve explicit boundaries among host configuration, resolved
agent definitions, workspaces, agent instances, sessions, turns, tape, rollout
records, operations, and emitted events.

#### Scenario: Runtime-owned object model is extended
- **WHEN** a new runtime object or state transition is introduced
- **THEN** the OpenSpec delta identifies which layer owns it
- **AND** the delta states how the object is observed by daemon clients or
  persisted in rollout/session state when applicable

#### Scenario: User input advances execution
- **WHEN** a client submits `turn`, `input`, `resume`, `interrupt`, `compact`,
  or `rollback` operations
- **THEN** the operation semantics are specified in OpenSpec before client
  behavior depends on them

### Requirement: Runtime durability and recovery stay auditable
alan SHALL specify durable run, scheduler, compaction, rollback, replay, and
recovery behavior with auditable state transitions and explicit degradation
semantics.

#### Scenario: Durable state is written or replayed
- **WHEN** runtime execution persists rollout records, checkpoints, scheduled
  wakeups, effect records, or recovery metadata
- **THEN** the OpenSpec requirement identifies the durability scope,
  idempotency expectation, and user-visible failure mode

#### Scenario: Compaction or recovery degrades
- **WHEN** compaction, memory flush, scheduler wake, replay, or recovery cannot
  complete normally
- **THEN** alan records the limitation and continues only through the
  degradation path specified by OpenSpec
