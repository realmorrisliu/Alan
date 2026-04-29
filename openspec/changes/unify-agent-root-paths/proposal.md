## Why

The current split between `.alan/agent/` for the default agent and `.alan/agents/<name>/`
for named agents makes the on-disk model harder to explain and harder to map to
`agent_name`. Moving the default agent into `.alan/agents/default/` makes every
agent definition addressable through the same directory shape.

## What Changes

- **BREAKING** Remove `.alan/agent/` as an agent definition root.
- **BREAKING** Resolve the default agent from `.alan/agents/default/` in both Alan home
  and workspace scopes.
- Keep named agents under `.alan/agents/<name>/`, with `default` reserved for the
  default agent.
- Resolve named agents by layering `.alan/agents/default/` first, then
  `.alan/agents/<name>/`.
- Update config, policy, persona, skill, CLI, daemon, TUI, docs, and tests that
  currently point at `.alan/agent/`.
- Do not add a compatibility fallback or automatic merge from `.alan/agent/`; users
  must move authored files to `.alan/agents/default/`.

## Capabilities

### New Capabilities

- `agent-root-layout`: Defines the canonical on-disk layout and resolution order for
  default and named agent roots.

### Modified Capabilities

- None. There are no archived OpenSpec capabilities for agent root layout yet.

## Impact

- Runtime agent root resolution in `crates/runtime/src/agent_root.rs`,
  `agent_definition.rs`, `paths.rs`, prompt/persona loading, skill discovery, policy
  resolution, and config loading.
- CLI/daemon setup and reporting surfaces that create or display default agent config
  paths.
- TUI setup/help text and tests that mention `~/.alan/agent/agent.toml`.
- Repository hygiene and documentation for source-controlled `.alan` roots.
- Existing local installs and workspaces using `.alan/agent/` will need manual
  migration to `.alan/agents/default/`.
