# Daemon API Contract

The daemon-owned API contract lives in
`crates/alan/src/daemon/api_contract.rs`. It is the canonical source for:

- endpoint ids, HTTP methods, route patterns, path parameters, and API areas;
- daemon route pattern constants used by Axum registration;
- Rust path builders used by daemon response URLs and CLI clients;
- remote-control scope metadata;
- relay forwarding, streaming/WebSocket exclusion, session binding, and response
  URL rewrite metadata;
- the TypeScript endpoint helper generated at
  `clients/tui/src/generated/api-contract.ts`.

Public `/api/v1/...` paths remain stable. Documentation may show concrete curl
examples, but production daemon, CLI, relay, remote-control, and TUI client code
must use the Rust contract or generated `apiPaths` helpers instead of embedding
raw route strings.

Regenerate client helper files with:

```bash
./scripts/generate-ts-types.sh
```

Run the raw-route guardrail with:

```bash
just guard-daemon-api-contract
```

Selected daemon payload types still live manually in `clients/tui/src/types.ts`.
They are guarded by `crates/alan/tests/daemon_payload_contract_test.rs`, which
serializes representative Rust response structs and checks that the mirrored TUI
interfaces expose the same top-level fields. Full structural TypeScript
generation for nested payloads remains a follow-up because the workspace does
not yet have a Rust-to-TypeScript schema generator in the build.
