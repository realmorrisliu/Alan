# daemon-api-contract Specification

## Purpose
Define Alan daemon API route contract requirements: endpoint metadata, shared URL
construction, remote access scope, relay policy, generated client helpers,
payload drift checks, public route compatibility, and raw route-string
guardrails.
## Requirements
### Requirement: Canonical Endpoint Registry
The daemon SHALL define a canonical endpoint registry that lists every supported
HTTP and WebSocket endpoint with a stable endpoint id, HTTP method, route pattern,
path parameters, and API area.

#### Scenario: all registered daemon routes have endpoint metadata
- **WHEN** the daemon router is built
- **THEN** every public route registered by the daemon has a matching endpoint
  registry entry

#### Scenario: adding a route without metadata fails verification
- **WHEN** a developer adds a public daemon route without adding a registry entry
- **THEN** the route contract verification fails

### Requirement: Shared URL Construction
The daemon SHALL use endpoint contract helpers to construct session response URLs
and client-facing API paths instead of hand-written route strings.

#### Scenario: create-session response uses contract builders
- **WHEN** a session is created
- **THEN** the returned `websocket_url`, `events_url`, and `submit_url` values are
  built from the canonical endpoint contract

#### Scenario: fork-session response uses contract builders
- **WHEN** a session is forked
- **THEN** the returned child session URLs are built from the canonical endpoint
  contract

### Requirement: Remote Access Scope Metadata
The daemon SHALL derive remote-control authorization requirements from endpoint
contract metadata rather than independent path prefix or suffix rules.

#### Scenario: endpoint scope is resolved from metadata
- **WHEN** remote-control middleware evaluates a daemon request
- **THEN** it resolves the required `SessionScope` from the matched endpoint
  metadata

#### Scenario: unknown API path is not silently treated as a known endpoint
- **WHEN** remote-control middleware evaluates an unknown `/api/v1/...` path
- **THEN** it applies the configured unknown-path behavior explicitly and records
  that no endpoint metadata matched

### Requirement: Relay Policy Metadata
The daemon SHALL derive relay forwarding, streaming exclusion, WebSocket
exclusion, session binding extraction, and response URL rewriting from endpoint
contract metadata.

#### Scenario: relay forwarding allows only contract-approved endpoints
- **WHEN** a relay request attempts to forward a daemon API path
- **THEN** the relay layer allows forwarding only when the matched endpoint
  metadata permits relay forwarding

#### Scenario: relay URL rewriting uses response URL metadata
- **WHEN** a relayed session lifecycle response contains daemon-relative URL
  fields
- **THEN** the relay layer rewrites only the response fields identified by the
  endpoint contract

### Requirement: Generated Client Endpoint Helpers
The repository SHALL generate or verify TypeScript client endpoint helpers from
the daemon endpoint contract, and the TUI daemon client SHALL use those helpers
for API path construction.

#### Scenario: generated endpoint helper is current
- **WHEN** the daemon endpoint contract changes
- **THEN** the generated TypeScript endpoint helper changes deterministically or
  a drift check fails

#### Scenario: TUI client avoids raw daemon route construction
- **WHEN** the TUI daemon client calls a supported API endpoint
- **THEN** it constructs the path through the generated endpoint helper rather
  than embedding a raw `/api/v1/...` string

### Requirement: Protocol And Payload Drift Checks
The repository SHALL replace static hand-written generated TypeScript protocol
files with a real generated or schema-checked surface for protocol event types
and selected daemon API payloads.

#### Scenario: protocol event list drifts
- **WHEN** the Rust protocol event enum adds, removes, or renames a serialized
  event type
- **THEN** the TypeScript generated or checked protocol surface detects the
  difference

#### Scenario: selected daemon payload drifts
- **WHEN** a selected daemon API response shape changes in Rust
- **THEN** the TypeScript generated or checked payload surface detects the
  difference

### Requirement: Public Route Compatibility
The first implementation SHALL preserve existing public daemon route paths unless
a task explicitly marks a route change as breaking and provides migration notes.

#### Scenario: contract migration keeps route paths stable
- **WHEN** the endpoint registry is introduced
- **THEN** existing public session, connection, skill, relay, WebSocket, and
  health route paths continue to resolve as before

### Requirement: Raw Route String Guardrail
The repository SHALL include focused guardrails that prevent new raw canonical
daemon route strings in production client, relay, remote-control, and daemon URL
construction code outside the approved contract or generated files.

#### Scenario: raw route string is added outside allowed files
- **WHEN** production code outside the approved contract or generated files adds
  a new raw `/api/v1/...` route string
- **THEN** the route string guardrail fails and points to the endpoint contract
  helper surface
