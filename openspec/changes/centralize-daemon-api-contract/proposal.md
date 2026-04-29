## Why

Daemon API changes currently fan out across manually maintained route strings,
response structs, relay path rewriting, remote-control scope checks, CLI clients,
TUI clients, tests, and documentation. The route and payload contract needs one
owned source so endpoint changes are deliberate instead of broad mechanical
search-and-edit work.

## What Changes

- Introduce a daemon-owned API contract layer for canonical endpoint identifiers,
  route patterns, URL builders, and response URL construction.
- Route Axum registration, daemon response URL fields, CLI calls, relay rewriting,
  and remote-control authorization through the same endpoint contract.
- Replace ad hoc frontend API strings with a client endpoint helper generated
  from, or checked against, the daemon contract.
- Replace the current hand-written "generated" TypeScript protocol/API file with
  a real generated or schema-checked contract for protocol events and selected
  daemon API payloads.
- Add drift checks that fail when server routes, remote-control scope rules,
  relay path handling, or TypeScript client endpoint/type surfaces diverge.
- Keep existing public endpoint paths stable in the first implementation unless
  an individual task explicitly marks a breaking route change.

## Capabilities

### New Capabilities

- `daemon-api-contract`: Defines canonical ownership, route construction,
  client-facing URL helpers, remote access scope metadata, relay rewrite rules,
  and generated/schema-checked type surfaces for the daemon HTTP/WebSocket API.

### Modified Capabilities

- None. No archived OpenSpec capability currently owns the daemon API contract.

## Impact

- `crates/alan/src/daemon/server.rs`, `routes.rs`, `websocket.rs`,
  `relay.rs`, and `remote_control.rs`.
- `crates/alan/src/cli/ask.rs` and any other Rust clients that construct daemon
  URLs.
- `clients/tui/src/client.ts`, `clients/tui/src/types.ts`,
  `clients/tui/src/generated/`, `clients/tui/src/client.test.ts`, and the TS type
  generation script.
- Protocol and daemon API serialization tests, especially event and session
  response compatibility tests.
- Documentation examples that show daemon API routes.
