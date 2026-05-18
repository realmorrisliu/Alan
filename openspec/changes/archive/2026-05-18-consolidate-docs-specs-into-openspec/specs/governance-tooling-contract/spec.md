## ADDED Requirements

### Requirement: Governance and tooling contracts live in OpenSpec
alan SHALL specify HITE governance, policy decisions, tool catalog identity,
runtime tool binding, capability routing, extension points, and workspace
routing in OpenSpec.

#### Scenario: Governance behavior changes
- **WHEN** a change modifies policy `allow`, `deny`, or `escalate` semantics,
  execution-backend boundaries, owner-boundary classes, audit requirements, or
  approval/resume behavior
- **THEN** the change updates this capability, `runtime-evidence-provenance`,
  `agent-capability-routing`, `human-visible-run-lifecycle`, or another active
  OpenSpec owner

#### Scenario: Tool binding behavior changes
- **WHEN** a change modifies tool catalog entries, runtime binding, locality,
  workspace scoping, child-runtime tool materialization, or extension routing
- **THEN** the behavior is specified in OpenSpec before it is documented as
  current guidance

### Requirement: Tool identity is separate from execution binding
alan SHALL keep stable tool catalog definitions separate from per-runtime
execution binding such as workspace root, current directory, profile exposure,
and policy decisions.

#### Scenario: Runtime exposes a tool
- **WHEN** a runtime registers or exposes a tool to an agent
- **THEN** the tool's identity, schema, and locality come from the catalog
- **AND** workspace-specific execution facts come from runtime context and
  policy

#### Scenario: Delegated capability is selected
- **WHEN** alan routes work to a delegated skill or child target
- **THEN** capability matching and mismatch recovery are observable through the
  OpenSpec-defined routing surface
