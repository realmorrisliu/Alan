## 1. Runtime Layout API

- [ ] 1.1 Add a runtime-owned typed layout API for global/workspace default roots, named roots, and launch roots where applicable.
- [ ] 1.2 Add semantic methods for `agent.toml`, `persona/`, `skills/`, and `policy.yaml` paths under an agent root.
- [ ] 1.3 Centralize omitted/default/named agent-name normalization and single-component validation in the runtime layout module.
- [ ] 1.4 Keep existing public path helper functions as delegates or update call sites in a compatibility-preserving order.

## 2. Runtime Consumers

- [ ] 2.1 Convert agent definition overlay resolution to use the typed layout API.
- [ ] 2.2 Convert config overlay, persona assembly, prompt cache, policy resolution, and skill discovery tests/helpers to semantic layout helpers where practical.
- [ ] 2.3 Preserve negative tests proving legacy `.alan/agent/` roots are not read or written.
- [ ] 2.4 Verify package-local child-agent roots under skill-package `agents/<name>/` remain outside the workspace agent-root layout API.

## 3. CLI And Daemon Consumers

- [ ] 3.1 Convert `alan init` workspace structure creation to runtime layout helpers.
- [ ] 3.2 Convert connection pin/default config path validation and writes to runtime layout helpers.
- [ ] 3.3 Convert daemon workspace resolver, session creation, runtime manager, skill catalog, and skill override path construction to runtime layout helpers.
- [ ] 3.4 Update route and integration tests to assert behavior through semantic helpers except where literal paths are the user-facing contract.

## 4. TUI And Client Contract

- [ ] 4.1 Keep TUI offline setup path construction isolated in `config-path.ts` and document it as a mirror of the runtime contract.
- [ ] 4.2 Prefer daemon-returned canonical paths in TUI online flows and avoid recomputing equivalent paths in UI code.
- [ ] 4.3 Update TUI tests to cover the isolated mirror and daemon-returned path display behavior.

## 5. Guardrails And Documentation

- [ ] 5.1 Add a raw-layout-string guardrail for Rust production code with an explicit allowlist for the runtime layout owner, tests, docs, and OpenSpec artifacts.
- [ ] 5.2 Document the layout API ownership boundary in architecture or spec docs.
- [ ] 5.3 Run a repo-wide search for `.alan/agents/default`, `agents/default`, and `.join("agents").join("default")`; convert production occurrences or justify allowlist entries.

## 6. Verification

- [ ] 6.1 Run focused runtime tests for agent root resolution, config overlays, persona assembly, policy resolution, prompt cache, and skill discovery.
- [ ] 6.2 Run focused alan CLI/daemon tests for init, connection pinning, workspace routing, skill catalog, skill override, and session creation.
- [ ] 6.3 Run focused TUI tests for setup/config path behavior.
- [ ] 6.4 Run formatting, `cargo check -p alan-runtime -p alan`, `bun run lint` in `clients/tui`, guardrail checks, and OpenSpec validation.
