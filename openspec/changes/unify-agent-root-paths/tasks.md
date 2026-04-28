## 1. Runtime Path Model

- [ ] 1.1 Add a canonical `default` agent-name constant and helper semantics so omitted `agent_name` and explicit `default` select the same default agent.
- [ ] 1.2 Change Alan home path helpers so the global default agent root and config path resolve to `~/.alan/agents/default/`.
- [ ] 1.3 Change workspace path helpers so the workspace default agent root resolves to `<workspace>/.alan/agents/default/`.
- [ ] 1.4 Update `ResolvedAgentRoots` ordering so default sessions use global/workspace default roots and named sessions append global/workspace named roots after the default chain.
- [ ] 1.5 Ensure `agent_name = "default"` does not append a named `default` overlay on top of the default chain.
- [ ] 1.6 Rename or relabel root kinds and user-facing strings from base/default-agent split terminology to default/named terminology where practical.

## 2. Runtime Loading And Write Semantics

- [ ] 2.1 Update agent config overlay loading, `ALAN_CONFIG_PATH` fallback text, and global default config loading to use `~/.alan/agents/default/agent.toml`.
- [ ] 2.2 Update persona, policy, skill package, and skill override discovery to load default assets only from `.alan/agents/default/`.
- [ ] 2.3 Update writable default agent root selection so generated default persona, policy, config, and skill files are written under `.alan/agents/default/`.
- [ ] 2.4 Add negative tests proving `.alan/agent/agent.toml`, `.alan/agent/persona/`, `.alan/agent/skills/`, and `.alan/agent/policy.yaml` are ignored.
- [ ] 2.5 Keep package-local child-agent roots under skill-package `agents/<name>/` directories unchanged.

## 3. CLI, Daemon, And Client Surfaces

- [ ] 3.1 Update `alan init`, setup flows, connection commands, and config-path reporting to create/display `~/.alan/agents/default/agent.toml`.
- [ ] 3.2 Update daemon session, skill catalog, skill override, and workspace APIs that create or report default agent-root paths.
- [ ] 3.3 Update TUI setup/help/error text and tests that mention `~/.alan/agent/agent.toml`.
- [ ] 3.4 Accept `agent_name = "default"` through CLI and daemon validation as the default agent rather than a named specialization.
- [ ] 3.5 Update event, session, and API fixtures that include `.alan/agent/` paths.

## 4. Documentation And Repository Hygiene

- [ ] 4.1 Update `.gitignore` to allowlist `.alan/agents/**` and `.alan/models.toml` while no longer allowlisting `.alan/agent/**`.
- [ ] 4.2 Update `README.md`, `AGENTS.md`, architecture docs, governance docs, connection-profile docs, skill docs, and sub-agent docs to use `.alan/agents/default/`.
- [ ] 4.3 Add migration notes explaining that `.alan/agent/` was removed and users must move authored files to `.alan/agents/default/`.
- [ ] 4.4 Run a repo-wide search for `.alan/agent` and either update each reference or mark it as an explicit old-path negative test/migration note.

## 5. Verification

- [ ] 5.1 Run focused runtime tests for agent root resolution, config overlays, persona assembly, policy resolution, prompt cache, and skill discovery.
- [ ] 5.2 Run focused alan CLI/daemon tests for init/setup, skill catalog, skill override, session creation, and workspace routing.
- [ ] 5.3 Run focused TUI tests after setup/help/config path updates.
- [ ] 5.4 Run formatting, `cargo check -p alan-runtime -p alan`, `git diff --check`, and OpenSpec status/apply validation.
