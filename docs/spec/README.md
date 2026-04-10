# Spec Index

This directory contains Alan's target contracts and product specs.

It is not identical to "current behavior" documentation. Before using a spec as
the source of truth, check its `Status` line and cross-reference the current
implementation guides when needed.

Start here for shipped behavior:

- [Architecture](../architecture.md)
- [Current Governance Contract](../governance_current_contract.md)
- [Skills And Tools](../skills_and_tools.md)

## Core Runtime And Execution

- [kernel_contract.md](./kernel_contract.md): stable kernel invariants.
- [execution_model.md](./execution_model.md): task/run/session/turn hierarchy.
- [compaction_contract.md](./compaction_contract.md): compaction semantics,
  triggers, and audit goals.
- [durable_run_contract.md](./durable_run_contract.md): checkpoint, replay, and
  side-effect recovery contract.
- [memory_architecture.md](./memory_architecture.md): long-lived memory model
  and retrieval boundaries.

## Interaction And Protocol

- [app_server_protocol.md](./app_server_protocol.md): multi-client app-server
  protocol target.
- [connection_profile_contract.md](./connection_profile_contract.md): unified
  operator-facing connection, credential, and profile management.
- [provider_capability_contract.md](./provider_capability_contract.md):
  adapter-level capability boundaries, parity targets, and degradation rules
  across provider families.
- [interaction_inbox_contract.md](./interaction_inbox_contract.md): `steer`,
  `follow_up`, and `next_turn`.
- [provider_auth_contract.md](./provider_auth_contract.md): provider selection
  and auth-layer boundaries.
- [scheduler_contract.md](./scheduler_contract.md): schedule/sleep/wake source
  of truth semantics.

## Governance, Skills, And Extensibility

- [governance_boundaries.md](./governance_boundaries.md): commit-boundary and
  exception-plane contract.
- [skill_system_contract.md](./skill_system_contract.md): authoritative
  skill-system contract.
- [capability_router.md](./capability_router.md): capability routing across
  builtin, bridge, and extension providers.
- [extension_contract.md](./extension_contract.md): extension lifecycle and
  integration contract.

## Remote, Distributed, And Reliability

- [harness_bridge.md](./harness_bridge.md): bridge contract across Alan nodes.
- [remote_control_architecture.md](./remote_control_architecture.md): node,
  app-server, and relay layering.
- [remote_control_security.md](./remote_control_security.md): trust boundaries
  and credential model.
- [mobile_reliability_contract.md](./mobile_reliability_contract.md): reconnect
  and notification contract for mobile clients.

## Product And UX Specs

- [alan_tui_ui_ux.md](./alan_tui_ui_ux.md): terminal-native TUI operator
  console UX contract.
- [reference_coding_agent.md](./reference_coding_agent.md): reference coding
  product layer on top of the runtime.
- [alan_shell_macos_contract.md](./alan_shell_macos_contract.md): macOS shell
  host product contract.
- [alan_macos_shell_ui_ux.md](./alan_macos_shell_ui_ux.md): native macOS shell
  UI/UX contract.
