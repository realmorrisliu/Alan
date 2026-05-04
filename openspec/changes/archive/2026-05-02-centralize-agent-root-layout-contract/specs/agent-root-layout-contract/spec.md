## ADDED Requirements

### Requirement: Runtime-owned agent-root layout
Alan SHALL expose a runtime-owned typed API for canonical agent-root layout
construction. Production Rust code outside the layout owner SHALL use that API for
default roots, named roots, and standard agent-root asset paths.

#### Scenario: Default root paths are requested semantically
- **WHEN** a caller needs the global or workspace default agent root
- **THEN** the caller can request the default root through the runtime layout API
- **AND** the caller does not need to join literal `agents/default` path segments

#### Scenario: Standard asset paths are requested semantically
- **WHEN** a caller needs `agent.toml`, `persona/`, `skills/`, or `policy.yaml` under an agent root
- **THEN** the caller can request the asset path through the runtime layout API
- **AND** the returned path uses the canonical agent-root layout

### Requirement: Centralized agent-name semantics
Alan SHALL centralize agent-name normalization, validation, and `default` reservation
semantics in the runtime layout contract.

#### Scenario: Explicit default name is normalized once
- **WHEN** a CLI, daemon, or runtime caller receives `agent_name = "default"`
- **THEN** the caller uses the runtime normalization API
- **AND** the result selects the default root chain rather than a named overlay

#### Scenario: Named agent validation is shared
- **WHEN** a caller receives a named agent value
- **THEN** the runtime layout contract validates that it is a safe single path component
- **AND** callers do not duplicate path traversal checks for agent-root layout decisions

### Requirement: Writers and readers use the same layout contract
Alan SHALL use the same runtime layout contract for agent-root reads and writes. Setup
and mutation flows MUST NOT construct default agent-root paths independently from the
runtime resolver.

#### Scenario: Setup writes a loadable default config
- **WHEN** `alan init`, connection pinning, or a setup flow writes a default `agent.toml`
- **THEN** it writes to the path returned by the runtime layout contract
- **AND** the runtime default config loader can read that same path without extra mapping

#### Scenario: Workspace APIs write loadable assets
- **WHEN** daemon workspace APIs write persona, policy, skill packages, or skill overrides
- **THEN** they write to paths returned by the runtime layout contract
- **AND** runtime discovery can load those assets from the same roots

### Requirement: Client path mirrors are constrained
Alan SHALL keep non-Rust mirrors of canonical setup paths isolated and explicitly
tested. Daemon responses that already include canonical paths SHALL be preferred over
client-side recomputation for online flows.

#### Scenario: TUI setup needs an offline default config path
- **WHEN** the TUI runs setup before a daemon is available
- **THEN** any local default config path construction is isolated in a small helper
- **AND** tests assert that helper matches the canonical user-facing path

#### Scenario: TUI displays daemon-provided paths
- **WHEN** a daemon API response includes a canonical config or agent-root path
- **THEN** the TUI displays that returned path
- **AND** the TUI does not recompute an equivalent path from duplicated layout rules

### Requirement: Raw layout-string guardrail
Alan SHALL provide a mechanical guardrail that detects new raw canonical agent-root
layout strings in Rust production code outside approved layout-owner locations.

#### Scenario: Production code adds a raw default-root string
- **WHEN** production Rust code outside the runtime layout owner introduces a raw string such as `.alan/agents/default`
- **THEN** the guardrail reports the occurrence
- **AND** the fix is to use the runtime layout contract or add an explicit allowlist entry with justification

#### Scenario: Tests and documentation use literal paths
- **WHEN** tests, documentation, or OpenSpec artifacts use literal canonical paths to describe the external contract
- **THEN** the guardrail allows those paths through an explicit scope or allowlist
- **AND** the allowed usage does not become a production-code layout owner
