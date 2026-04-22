# Spec Index

This directory contains Alan's target contracts and product specs.

It is not identical to "current behavior" documentation. Before using a spec as
the source of truth, check its `Status` line and cross-reference the current
implementation guides when needed.

Start here for shipped behavior:

- [Architecture](../architecture.md)
- [Current Governance Contract](../governance_current_contract.md)
- [Skills And Tools](../skills_and_tools.md)

## What Belongs In `docs/spec/`

A document belongs here when it defines a normative current or target contract
that code, tests, product work, or other docs can implement against.

That usually means the document defines:

- scope, goals, and non-goals
- stable vocabulary, invariants, or object/protocol boundaries
- normative behavior rules rather than brainstorming
- relationships to adjacent contracts
- acceptance criteria that can drive implementation or review

Product-layer docs can still be specs when they define stable UX or host
behavior rather than a one-off experiment.

## What Does Not Belong In `docs/spec/`

Move these somewhere else instead of keeping them in contract docs:

- implementation sequencing, rollout slices, refactor order, and issue mapping:
  `plans/`
- maintainer-only rollout notes or spike logistics: `docs/maintainer/`
- durable but still underdefined long-term directions: `docs/directions/`
- speculative ideas and hypotheses: `docs/ideas/`
- superseded duplicates that no longer carry unique contract value: merge or
  delete them

Short migration notes are tolerated inside a spec when they clarify compatibility
or rollout constraints, but the source of truth for execution order should live
in `plans/`.

## Recommended Spec Format

Specs do not need one rigid template, but the normal shape should be:

1. `# Title`
2. `> Status: ...`
3. `## Goal` or `## Purpose`, plus scope when useful
4. `## Non-Goals` when boundaries are not already obvious
5. normative sections for invariants, models, protocols, or behavior rules
6. `## Relationship ...` or `## Alignment ...` for adjacent contracts
7. `## Acceptance Criteria`

If a document is mostly issue references, implementation slices, or refactor
steps, it is probably a plan rather than a spec.

## Core Runtime And Execution

- [kernel_contract.md](./kernel_contract.md): stable kernel invariants.
- [execution_model.md](./execution_model.md): task/run/session/turn hierarchy.
- [compaction_contract.md](./compaction_contract.md): compaction semantics,
  triggers, and audit goals.
- [durable_run_contract.md](./durable_run_contract.md): checkpoint, replay, and
  side-effect recovery contract.
- [memory_architecture.md](./memory_architecture.md): long-lived memory model
  and retrieval boundaries.
- [pure_text_memory_contract.md](./pure_text_memory_contract.md): active target
  contract for pure-text working, episodic, and semantic memory.

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

- [hite_governance.md](./hite_governance.md): target HITE governance model
  and authorization semantics.
- [alan_coding_governance_contract.md](./alan_coding_governance_contract.md):
  workspace-aware steward and repo-worker coding governance.
- [governance_boundaries.md](./governance_boundaries.md): HITE boundary and
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
  app-server, relay, reconnect, and notification layering.
- [remote_control_security.md](./remote_control_security.md): trust boundaries
  and credential model.

## Product And UX Specs

- [alan_coding_steward_contract.md](./alan_coding_steward_contract.md): home-root
  coding steward and repo-scoped worker contract.
- [workspace_routing_contract.md](./workspace_routing_contract.md): routing
  local tasks into explicit target-workspace child runtimes.
- [alan_coding_eval_contract.md](./alan_coding_eval_contract.md): executable
  validation ladder for steward orchestration, repo-worker harness, and
  package-local benchmark scaffolding.
- [alan_tui_ui_ux.md](./alan_tui_ui_ux.md): terminal-native TUI operator
  console UX contract.
- [alan_shell_macos_contract.md](./alan_shell_macos_contract.md): macOS shell
  host product contract.
- [alan_macos_shell_ui_ux.md](./alan_macos_shell_ui_ux.md): native macOS shell
  UI/UX contract.

## Engineering And Repo Conventions

- [rust_test_placement_contract.md](./rust_test_placement_contract.md): target
  contract for where Rust tests belong across inline, extracted white-box, and
  crate-level integration layers.
