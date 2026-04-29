## Context

Alan's daemon API is consumed by several in-repo clients and transport layers:
the Axum daemon, CLI commands, the TUI client, relay proxying, remote-control
scope enforcement, WebSocket handling, tests, and docs. Today those layers share
the API contract mostly by convention:

- `server.rs` registers literal route patterns.
- `routes.rs` builds response URL fields with literal strings.
- `remote_control.rs` infers required scopes with path prefix/suffix checks.
- `relay.rs` parses forwarded paths and rewrites response URLs with local string
  logic.
- `clients/tui/src/client.ts` builds endpoint URLs manually.
- `scripts/generate-ts-types.sh` writes fixed TypeScript text while claiming to
  generate it from Rust protocol definitions.

This is a high-amplification surface because a path, route capability, response
shape, or event type change can silently drift in one consumer while compiling
elsewhere.

## Goals / Non-Goals

**Goals:**

- Make daemon endpoint identity, route patterns, URL construction, remote scope,
  and relay behavior explicit in one Rust-owned contract.
- Keep first-pass public routes stable while replacing local string construction.
- Give Rust clients and TypeScript clients a generated or checked endpoint
  surface instead of manually reconstructing paths.
- Add drift checks that catch missing endpoint metadata, stale generated client
  helpers, stale protocol event types, and incomplete remote/relay coverage.
- Keep the implementation incremental so route registration can migrate without a
  daemon rewrite.

**Non-Goals:**

- Do not introduce `/api/v2` or rename endpoints as part of this change.
- Do not require every daemon response/request payload to have complete OpenAPI
  coverage before the path and scope contract lands.
- Do not split large daemon/TUI modules for its own sake.
- Do not change runtime protocol semantics.

## Decisions

### Decision: Add a Rust-first daemon API contract module

Create a daemon module such as `crates/alan/src/daemon/api_contract.rs` that owns
endpoint IDs, methods, Axum route patterns, concrete path builders, client URL
builders, remote scope metadata, relay forwarding policy, and response URL field
metadata.

Rationale: The daemon is the server authority and already owns handler wiring.
Keeping the source in Rust lets server registration, Rust clients, relay, and
remote-control checks share a typed surface without a separate schema compiler
becoming the source of truth.

Alternative considered: make TypeScript or OpenAPI the source of truth. That
would make the TUI comfortable but invert ownership away from the daemon and add
more migration work before the existing Rust path consumers can benefit.

### Decision: Keep route registration explicit, but driven by constants/helpers

Axum route wiring can remain readable in `server.rs`, but path strings must come
from the contract module. For example, a session create/list endpoint can expose
a `SESSIONS` route pattern and concrete builders for `/api/v1/sessions`, while
session-specific endpoints expose typed builders for `{id}` paths.

Rationale: A full declarative router table would force all handler registration
through a larger abstraction. The first implementation only needs to remove
duplicated route knowledge, not hide Axum.

Alternative considered: generate the Axum router from endpoint descriptors. This
could be attractive later, but it is a larger refactor and makes handler-specific
method composition harder to review.

### Decision: Store remote scope and relay policy with endpoint metadata

Each endpoint that participates in remote access must have explicit metadata for
the required `SessionScope` and relay behavior. `remote_control.rs` should match
requests against the contract rather than relying on broad suffix rules such as
`ends_with("/submit")` or `starts_with("/api/v1/connections")`. `relay.rs` should
use the same contract to decide which forwarded paths are allowed, which paths
are streaming/WebSocket, and which response fields need relay URL rewriting.

Rationale: Security and transport semantics are part of the API contract. They
should not be inferred independently from path spelling.

Alternative considered: keep path-based matching and add more tests. Tests help,
but they still require every endpoint author to remember parallel logic in
server, relay, and auth code.

### Decision: Generate TypeScript endpoint helpers from a daemon manifest

Expose a deterministic API contract manifest from Rust, either through a small
binary/test helper or a scriptable command, and generate
`clients/tui/src/generated/api-contract.ts` from it. The generated helper should
provide endpoint IDs and URL builder functions, and TUI client code should call
those helpers instead of embedding `/api/v1/...` strings.

Rationale: Endpoint helper generation removes the biggest TUI string-drift
surface without requiring full payload schema generation to be solved first.

Alternative considered: manually mirror endpoint constants in TypeScript. That
would improve local organization but preserve the cross-language drift problem.

### Decision: Convert protocol/API type generation into a checked surface

Replace the current hard-coded `generate-ts-types.sh` behavior with one of:

1. real Rust-to-TypeScript generation for protocol and selected daemon payloads,
   if a lightweight dependency fits the workspace; or
2. schema/snapshot checks that compare Rust-emitted event/payload metadata
   against the TypeScript generated files.

The implementation must stop presenting static heredocs as generated from Rust.

Rationale: A complete OpenAPI effort is not required to remove immediate drift,
but the current script gives false confidence. A generated or checked surface is
needed before protocol and client evolution can be trusted.

Alternative considered: leave payload types manual and only generate endpoint
paths. That would solve route churn but leave known event/type drift unresolved.

### Decision: Start with session, connection, skill, relay, and health endpoints

The initial contract must cover every endpoint registered by the daemon today:
health, connections, sessions, skills, WebSocket, events, and relay routes. The
first migration should avoid adding new endpoints until existing ones are
described.

Rationale: Partial route registries are easy to ignore. The value comes from a
testable inventory that fails when a new route is added without metadata.

Alternative considered: only cover session endpoints first. That would reduce
scope, but remote-control and relay risks cross sessions/connections/relay.

## Risks / Trade-offs

- Rust contract manifest grows too large -> Split descriptors by area
  (`sessions`, `connections`, `skills`, `relay`) behind one public registry.
- Route matching becomes subtly different from Axum matching -> Add table-driven
  tests that assert representative concrete paths resolve to the same endpoint
  metadata and handler expectations.
- TypeScript generation adds build friction -> Keep generation as an explicit
  script/test first, and fail drift in CI rather than during normal `cargo build`.
- Payload type generation is harder than endpoint generation -> Land endpoint
  helpers and event/type drift checks first, then expand payload coverage in
  focused tasks.
- Remote access authorization changes accidentally -> Preserve existing scopes
  with golden tests before moving logic behind the contract.

## Migration Plan

1. Add the contract module with descriptors for all current daemon endpoints and
   tests that the inventory contains every route pattern from `server.rs`.
2. Replace response URL construction in daemon route responses with contract
   builders.
3. Replace remote-control scope inference and relay route handling with contract
   matching while preserving existing behavior.
4. Generate TypeScript endpoint helpers from the manifest and migrate TUI client
   calls to them.
5. Replace or constrain the TypeScript type generation script so protocol event
   and selected daemon payload types are generated or checked against Rust.
6. Update docs/tests to reference contract helpers or generated surfaces instead
   of raw endpoint strings where practical.

## Open Questions

- Which Rust-to-TS tool, if any, best fits the workspace without adding heavy
  compile-time dependencies?
- Should the daemon expose the API contract manifest through an internal command
  only, or also as a debug endpoint?
- Should Apple client helpers be generated in the same phase or covered by a
  follow-up once the TUI path is proven?
