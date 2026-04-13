# Remote Control Architecture (Agent Node + App-Server + Relay)

> Status: VNext architecture contract (mobile/cloud remote control).

## Goals

1. Allow mobile/web/desktop clients to control Alan running on a remote node.
2. Preserve execution continuity on the agent node (laptop/cloud host).
3. Keep multi-client state consistent with replayable event semantics.
4. Preserve governance/execution-backend/yield invariants end-to-end.

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

### Responsibility Matrix

| Capability | Agent Node | Relay | Remote Client |
| --- | --- | --- | --- |
| Runtime execution | Authoritative | Not allowed | Not allowed |
| Governance and execution-backend enforcement | Authoritative | Not allowed | Not allowed |
| Event ordering (`event_id`) | Authoritative | Transport only | Consumer |
| Cursor replay decisions | Authoritative | Pass-through | Initiates requests |
| Session control ops | Validates + applies | Routes | Initiates |
| Auth scope re-validation | Required | Coarse pre-check | Provides credentials |

### Planes

1. **Control Plane**
   - Session/thread lifecycle, submit/resume/interrupt, approvals.
2. **Data Plane**
   - Event stream + cursor replay + snapshots.
3. **Management Plane**
   - Node registration, tunnel health, token rotation/revocation.

## Core Invariants

1. Execution always stays on Agent Node.
2. Relay never bypasses governance or execution-backend checks.
3. Event replay uses stable cursors and deterministic gap handling.
4. Remote resume/approval must use same Yield/Resume contract as local clients.

## Connection Models

## Direct Remote

1. Client connects directly to `alan-agentd` over TLS.
2. Client uses `/sessions/*` compatibility surface (future thread/turn aliases).
3. Reconnect uses `events/read?after_event_id=...` to fill gaps.

## Relay Transport

1. Agent Node opens outbound persistent tunnel to relay.
2. Client connects to relay endpoint with scoped token.
3. Relay routes opaque protocol frames between client and node.
4. Event ordering remains node-authored (`event_id/sequence`).

### MVP Transport Surface (Implemented)

1. Node tunnel endpoint: `GET /api/v1/relay/tunnel` (WebSocket).
2. Relay node status endpoint: `GET /api/v1/relay/nodes`.
3. Relay proxy endpoint: `ANY /api/v1/relay/nodes/{node_id}/{*path}`.
4. Allowed proxied target paths are constrained to `/api/v1/*` and explicitly exclude `/api/v1/relay/*`.

Current MVP limitation:

1. Long-lived `/events` streaming is not proxied through relay in this phase.
2. Session WebSocket (`/ws`) upgrade paths are not proxied through relay in this phase.
3. Clients should use `/events/read` cursor polling for reconnect-safe remote consumption.

Operational model:

1. Relay accepts node tunnel auth via `x-alan-node-id` + bearer token (optional strict mode).
2. Node maintains heartbeat and reconnect loop over the tunnel.
3. Relay forwards control/data HTTP requests through the node tunnel without becoming execution authority.
4. For proxied `create_session`/`fork_session`, relay rewrites returned session URLs with
   `/api/v1/relay/nodes/{node_id}` prefix so follow-up calls stay on relay surface.

## Multi-Node Management

### Discovery and routing metadata (Implemented)

`GET /api/v1/relay/nodes` returns node-level routing signals:

1. `node_id`, `connection_id`, `connected_at_ms`, `last_heartbeat_ms`
2. `heartbeat_age_ms`, `health` (`healthy|stale`), `selectable`
3. `pending_requests`
4. `bound_sessions` and `last_binding_update_ms`

Relay proxied responses include `x-alan-routed-node-id` so clients can surface the resolved node
for each operation.

### Sticky session-to-node contract (Implemented)

1. First successful session operation establishes sticky binding `session_id -> node_id`.
2. Subsequent requests for the same session to another node are rejected with:
   - HTTP `409`
   - code `relay_session_node_conflict`
3. Client can explicitly request switch by setting `x-alan-node-switch: force`.
4. Switch takes effect only after a successful response from the target node.
5. Successful `DELETE /api/v1/sessions/{id}` removes sticky binding.

This prevents silent cross-node misrouting and makes switch behavior deterministic/user-visible.

## Remote Reliability + Notifications

### Reconnect Snapshot Contract

1. Add explicit reconnect snapshot endpoint:
   - `GET /api/v1/sessions/{id}/reconnect_snapshot`
   - relay form: `GET /api/v1/relay/nodes/{node_id}/api/v1/sessions/{id}/reconnect_snapshot`
2. Snapshot must include dedupe hints (`latest_event_id`, `latest_submission_id`) and actionable
   execution state (`run_status`, `next_action`, pending yield checkpoint details).
3. Snapshot reads are side-effect free and must not re-drive runtime execution.

Normative response contract:

```json
{
  "session_id": "sess-1",
  "workspace_id": "ws-abc",
  "captured_at_ms": 1731000000000,
  "replay": {
    "oldest_event_id": "evt_0000000000001001",
    "latest_event_id": "evt_0000000000001050",
    "latest_submission_id": "sub-123",
    "buffered_event_count": 50
  },
  "execution": {
    "run_status": "yielded",
    "next_action": "await_user_resume",
    "resume_required": true,
    "latest_checkpoint": {
      "checkpoint_id": "cp-1",
      "checkpoint_type": "yield",
      "summary": "runtime yielded awaiting external input",
      "created_at": "2026-03-05T10:00:00Z",
      "payload": {
        "request_id": "req-1",
        "kind": "confirmation"
      }
    }
  },
  "notifications": {
    "latest_signal_cursor": "cp-1",
    "signals": [
      {
        "signal_id": "cp-1",
        "signal_type": "pending_yield",
        "request_id": "req-1",
        "yield_kind": "confirmation",
        "summary": "runtime yielded awaiting external input",
        "created_at": "2026-03-05T10:00:00Z",
        "informational": true
      }
    ]
  }
}
```

Field requirements:

1. `latest_event_id` and `latest_submission_id` are reconnect dedupe hints.
2. `resume_required=true` means only explicit resume can advance execution.
3. `notifications.signals` may be empty and clients must tolerate sparse
   signal streams.

### Notification Signal Contract

1. Pending-yield/approval states must surface as informational notification signals.
2. Signals are recovery-friendly (can be rebuilt from reconnect snapshot).
3. Notification delivery cannot imply authorization or automatic resume.
4. Initial signal types are:
   - `pending_yield`
   - `pending_structured_input`
   - `resume_failed`
   - `gap_detected`

Signal constraints:

1. Stable `signal_id` for dedupe.
2. `informational=true` is required in transport payload.
3. Signal delivery loss is recoverable via reconnect snapshot reads.

### Non-Bypass Governance Rules

1. Signals never authorize execution changes by themselves.
2. Approval or resume still requires explicit node-authority operations.
3. Token scopes and policy checks remain unchanged.
4. Relay and clients cannot convert notification delivery into implicit resume.

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
6. `node_switch_mode` (`force` when explicit rebind is requested)

Notes:

1. These fields are metadata only; runtime semantics remain unchanged.
2. Existing `/sessions/*` endpoints can accept optional metadata headers first.
3. Relay conflict/switch signals are machine-readable (`relay_session_node_conflict`, `x-alan-routed-node-id`).
4. Future canonical APIs can map to thread/turn surface without breaking flow.

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

## Acceptance Criteria

1. Remote start, resume, and stream paths keep execution authority on the
   target node.
2. Disconnect and reconnect recovery uses cursor replay plus reconnect snapshot
   fallback without implicit re-execution.
3. Gap handling remains deterministic and machine-readable.
4. Governance invariants remain node-validated in both direct and relay modes.
5. Notification and reconnect reliability surfaces remain informational until a
   scoped explicit control operation advances execution.
