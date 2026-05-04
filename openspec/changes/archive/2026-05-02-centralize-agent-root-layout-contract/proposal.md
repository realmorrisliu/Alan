## Why

The `.alan/agent/` to `.alan/agents/default/` change exposed that agent-root
layout is not owned by one module: runtime, daemon, CLI, TUI, docs, tests, and
fixtures all had local path construction or duplicated canonical strings. That
turns a layout-contract change into broad mechanical churn and makes missed
writers/readers easy.

## What Changes

- Introduce a runtime-owned typed agent-root layout API that is the canonical
  source for default and named agent paths.
- Move `agent.toml`, `persona/`, `skills/`, and `policy.yaml` path construction
  behind helper methods instead of local string joins.
- Update CLI and daemon writers/readers to use runtime layout helpers rather than
  duplicating `.alan/agents/default/` knowledge.
- Add repository checks or focused tests that reject new raw canonical agent-root
  layout strings in Rust production code outside the layout module.
- Keep TypeScript/TUI behavior aligned through daemon-provided paths or a small
  explicit client contract, rather than scattering path strings through UI code.
- Do not change the public on-disk layout introduced by `unify-agent-root-paths`;
  this is an ownership and coupling reduction change.

## Capabilities

### New Capabilities

- `agent-root-layout-contract`: Defines ownership, typed APIs, allowed consumers,
  and guardrails for canonical agent-root layout construction.

### Modified Capabilities

- None. No archived OpenSpec capabilities exist for agent-root layout yet.

## Impact

- `crates/runtime/src/agent_root.rs`, `paths.rs`, and related config/persona/policy
  resolution code.
- `crates/alan` CLI/daemon setup, connection pinning, workspace creation, skill
  catalog, and skill override write paths.
- TUI setup/config path detection if canonical path data remains duplicated in
  TypeScript.
- Tests and fixtures that currently assert literal `.alan/agents/default/...`
  paths.
- Documentation may be lightly updated to point to the canonical contract, but
  this change should primarily reduce code coupling rather than alter user-facing
  behavior.
