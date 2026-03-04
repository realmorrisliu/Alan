# Remote Control Phased Plan

> Owner issue: `#9`  
> Goal: mobile/cloud remote control with governance-safe continuity.

## Implementation Tracking Issues

1. Phase A (Direct Remote): `#32`
2. Phase B (Relay MVP): `#33`
3. Phase C (Multi-Node Management): `#35`
4. Phase D (Mobile Reliability + Notifications): `#34`

## Dependency Baseline

Required foundations (already scoped in VNext issues):

1. `#1` protocol input modes
2. `#2` runtime interaction inbox
3. `#3` task store
4. `#4` scheduler
5. `#5` durable checkpoint restore
6. `#6` side-effect dedupe
7. `#7` autonomy harness
8. `#8` self-eval harness

## Phase A: Direct Remote (Node-Exposed)

### Target

1. Remote client can create/submit/resume sessions directly against `alan-agentd`.
2. Event stream and `events/read` reconnect path is stable for mobile links.
3. Tracking issue: `#32`.

### Outputs

1. Remote metadata acceptance (`node_id/client_id/trace_id`) in protocol edge.
2. Auth scopes for remote session control.
3. Direct-mode validation scenarios in harness.

## Phase B: Relay MVP (Outbound Tunnel)

### Target

1. Node maintains outbound tunnel to relay for NAT traversal.
2. Client controls node via relay without execution-state authority shifting.
3. Tracking issue: `#33` (depends on `#32`).

### Outputs

1. Relay handshake contract and heartbeat semantics.
2. Relay routing + reconnect behavior with cursor replay.
3. Security hardening for relay trust boundaries.

## Phase C: Multi-Node Management

### Target

1. One client can discover/switch/control multiple nodes.
2. Node-scoped auth and audit stay explicit.
3. Tracking issue: `#35` (depends on `#33`).

### Outputs

1. Node registry/discovery contract.
2. Node-level status and routing metadata.
3. Cross-node session switching safety constraints.
4. Sticky session-node conflict/switch behavior (`relay_session_node_conflict`, explicit `x-alan-node-switch: force`).

## Phase D: Mobile Reliability + Notifications

### Target

1. Robust offline reconnect UX for approvals/resume.
2. Push-style signaling for pending escalations/yields.
3. Tracking issue: `#34` (depends on `#35`).

### Outputs

1. Durable reconnect state snapshots for mobile clients.
2. Notification trigger contracts (non-bypass, informational only).
3. Nightly reliability regressions in harness.

## Tracking Matrix

| Track | Primary Artifact | Validation |
| --- | --- | --- |
| Architecture | `docs/spec/remote_control_architecture.md` | `#32`/`#33` design review + harness scenarios |
| Security | `docs/spec/remote_control_security.md` | `#32`/`#33` scope/revocation tests |
| Protocol | `docs/spec/app_server_protocol.md` extension notes | `#32` compatibility tests |
| Reliability | harness autonomy + reconnect suites | `#34` CI + nightly |

## Exit Criteria (for #9)

1. Architecture and security docs are approved.
2. Protocol extension notes are explicit and non-breaking.
3. Phase-by-phase implementation plan is linked to concrete tracking issues (`#32/#33/#35/#34`).
4. Direct vs relay trade-offs are documented for execution planning.
