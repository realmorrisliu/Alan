# Remote Control Architecture (Agent Node + App-Server + Relay)

> Status: VNext architecture contract (mobile/cloud remote control).

## Goals

1. Allow mobile/web/desktop clients to control Alan running on a remote node.
2. Preserve execution continuity on the agent node (laptop/cloud host).
3. Keep multi-client state consistent with replayable event semantics.
4. Preserve policy/sandbox/yield invariants end-to-end.

## Topology

### Components

1. **Agent Node**
   - Runs `alan-agentd` + runtime + scheduler + durable stores.
   - Source of truth for session/run state and tool execution.
2. **Remote Client**
   - Mobile/web/desktop UI.
   - Sends control-plane ops and consumes event/data streams.
3. **Relay (optional)**
   - Routing layer for NAT traversal and intermittent links.
   - Never becomes execution source of truth.

### Planes

1. **Control Plane**
   - Session/thread lifecycle, submit/resume/interrupt, approvals.
2. **Data Plane**
   - Event stream + cursor replay + snapshots.
3. **Management Plane**
   - Node registration, tunnel health, token rotation/revocation.

## Core Invariants

1. Execution always stays on Agent Node.
2. Relay never bypasses policy/sandbox checks.
3. Event replay uses stable cursors and deterministic gap handling.
4. Remote resume/approval must use same Yield/Resume contract as local clients.

## Connection Models

## Phase A: Direct Remote

1. Client connects directly to `alan-agentd` over TLS.
2. Client uses `/sessions/*` compatibility surface (future thread/turn aliases).
3. Reconnect uses `events/read?after_event_id=...` to fill gaps.

## Phase B: Relay MVP

1. Agent Node opens outbound persistent tunnel to relay.
2. Client connects to relay endpoint with scoped token.
3. Relay routes opaque protocol frames between client and node.
4. Event ordering remains node-authored (`event_id/sequence`).

## Session Binding and Reconnect

### Handshake (recommended)

1. Client authenticates and selects `node_id`.
2. Client binds to `session_id` (or creates one).
3. Client provides latest known `event_id` cursor.
4. Node/relay replies with:
   - accepted cursor status (`gap=false|true`)
   - current turn/run status
   - replay window metadata

### Reconnect rules

1. If cursor is valid: replay from `after_event_id`.
2. If cursor evicted (`gap=true`): fetch session snapshot then continue streaming.
3. Reconnect never triggers implicit turn re-execution.

## Multi-Client Consistency

1. All clients observe same event stream for a session.
2. Last-writer semantics for control ops are explicit via submission/turn IDs.
3. Conflict responses are machine-readable (`state_conflict`, `stale_turn_id`).
4. Client UIs remain eventually consistent through snapshot + replay loop.

## Protocol Extension Notes

Recommended additive fields for remote routing:

1. `node_id` (target execution node)
2. `client_id` (logical device/app instance)
3. `connection_id` (transport session id)
4. `trace_id` (cross-node diagnostics)
5. `transport_mode` (`direct` | `relay`)

Notes:

1. These fields are metadata only; runtime semantics remain unchanged.
2. Existing `/sessions/*` endpoints can accept optional metadata headers first.
3. Future canonical APIs can map to thread/turn surface without breaking flow.

## Yield/Resume and Governance in Remote Mode

1. Yield events must include full checkpoint payload remotely.
2. Resume payload is validated on Agent Node, not relay.
3. High-risk recoveries still require escalation/confirmation.
4. Relay cannot downgrade or auto-resolve governance actions.

## Reliability Model

1. Heartbeats on client<->relay and relay<->node links.
2. Dead-link timeout transitions connection state, not run state.
3. Durable run/session stores remain node-local authority.
4. Event gap detection is mandatory on every reconnect.

## Direct vs Relay Trade-offs

### Direct mode

1. Pros: simpler path, fewer moving parts, lower latency.
2. Cons: requires node exposure/public reachability, harder on NAT/mobile networks.

### Relay mode

1. Pros: NAT traversal, stable mobile reachability, centralized node switching.
2. Cons: extra infra, more auth/session complexity, additional hop latency.

## Acceptance Mapping

This architecture supports issue acceptance targets by design:

1. Remote start/resume/stream: control/data planes are explicit.
2. Disconnect/reconnect safety: cursor replay + snapshot fallback.
3. Gap handling determinism: event-id contract remains authoritative.
4. Governance invariants: yield/resume remains node-validated.
5. Direct vs relay trade-offs: documented above for phased rollout.
