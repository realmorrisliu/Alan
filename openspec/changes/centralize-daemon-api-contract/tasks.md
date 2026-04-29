## 1. Endpoint Contract Inventory

- [ ] 1.1 Add a daemon API contract module with endpoint ids, methods, route patterns, path parameters, API areas, and public metadata types.
- [ ] 1.2 Describe every current health, connection, session, skill, WebSocket, events, and relay endpoint in the endpoint registry.
- [ ] 1.3 Add endpoint path builder helpers for concrete client-facing paths, including encoded path parameters where needed.
- [ ] 1.4 Add tests that fail when a route registered by the daemon is missing endpoint metadata.

## 2. Server And Rust Client Consumers

- [ ] 2.1 Convert daemon route registration to use contract route-pattern constants or helpers while preserving existing paths.
- [ ] 2.2 Convert create-session and fork-session response URL construction to use endpoint contract builders.
- [ ] 2.3 Convert Rust daemon clients such as `alan ask` to use endpoint contract URL/path helpers instead of hand-written API strings.
- [ ] 2.4 Add compatibility tests for representative existing route paths and session response URL fields.

## 3. Remote Access And Relay

- [ ] 3.1 Add explicit remote `SessionScope` metadata for all endpoints that participate in remote access.
- [ ] 3.2 Convert remote-control required-scope resolution to match endpoint metadata rather than prefix/suffix rules.
- [ ] 3.3 Add explicit relay forwarding, streaming, WebSocket, lifecycle, and response-rewrite metadata to the endpoint contract.
- [ ] 3.4 Convert relay forwarding validation, session id extraction, lifecycle detection, and response URL rewriting to contract helpers.
- [ ] 3.5 Preserve existing remote-control and relay behavior with table-driven tests for session, connection, skill, relay, unknown, streaming, and WebSocket paths.

## 4. TypeScript Endpoint Surface

- [ ] 4.1 Add a deterministic Rust-emitted daemon API contract manifest suitable for TypeScript generation or drift checks.
- [ ] 4.2 Generate `clients/tui/src/generated/api-contract.ts` endpoint ids and path builders from the daemon manifest.
- [ ] 4.3 Convert `clients/tui/src/client.ts` to use generated endpoint helpers for supported API calls.
- [ ] 4.4 Update TUI client tests to assert behavior through generated helpers instead of duplicating raw paths where practical.

## 5. Protocol And Payload Drift Checks

- [ ] 5.1 Replace the static heredoc-only `scripts/generate-ts-types.sh` flow with real generation or Rust-backed schema/snapshot checks.
- [ ] 5.2 Cover protocol event type serialization drift, including runtime heartbeat, compaction observed, and memory flush observed events.
- [ ] 5.3 Cover selected daemon payload drift for session creation/list/read responses, child-run responses, and connection profile responses.
- [ ] 5.4 Document any payload types intentionally left manual and add follow-up TODOs only where there is a concrete blocker.

## 6. Guardrails And Documentation

- [ ] 6.1 Add a raw route string guardrail for production daemon, CLI, relay, remote-control, and TUI client code outside approved contract/generated files.
- [ ] 6.2 Update daemon API documentation examples to reference the canonical contract and keep public path examples stable.
- [ ] 6.3 Run a repo-wide search for raw `/api/v1/` strings and convert production occurrences or justify allowlist entries.

## 7. Verification

- [ ] 7.1 Run focused Rust tests for daemon route registration, route matching, response URL builders, remote-control scope resolution, and relay rewriting.
- [ ] 7.2 Run focused TUI tests for generated endpoint helpers and daemon client API calls.
- [ ] 7.3 Run the TypeScript generation or drift-check script and verify no generated files are stale.
- [ ] 7.4 Run formatting, `cargo check -p alan`, relevant `cargo test -p alan` filters, relevant TUI tests, guardrail checks, and OpenSpec validation.
