# Harness Bridge Contract

> Status: VNext contract (defines control and capability bridging across local/remote Alan instances).

## Goals

Harness Bridge extends Alan's execution and governance plane for:

1. Remote control of any Alan instance (local machine, home machine, cloud host).
2. Cross-process/cross-machine hosting of capability providers.
3. Recoverable execution under disconnects, restarts, and network jitter.

Bridge does not replace runtime state machine; it extends run continuity and remote connectivity.

## Non-Goals

1. Does not become a new business protocol layer (business semantics remain in App Server Op/Event).
2. Does not bypass target-node governance or execution backend.
3. Does not immediately include all multi-tenant cloud-console features.

## Architecture Roles

1. `Bridge Controller` (in daemon)
   - manages connection, auth, routing, reconnect recovery
2. `Bridge Node Agent` (target machine)
   - bridges local runtime/extension host and executes requests
3. `Relay` (optional)
   - supports NAT/mobile-network relay paths
4. `Client` (TUI/Native/Web/Mobile)
   - sends control/subscription requests through app server

## Control Plane and Data Plane

### Control Plane

1. `bridge.register`
2. `bridge.authenticate`
3. `bridge.heartbeat`
4. `bridge.attach_session`
5. `bridge.detach_session`
6. `bridge.drain`

### Data Plane

1. `bridge.call` (capability invocation)
2. `bridge.result` (result return)
3. `bridge.event` (event forwarding)
4. `bridge.cancel` (cancel in-flight call)
5. `bridge.resume` (cursor-based replay resume)

## Message Envelope Contract (Draft)

Each bridge message should include:

1. `bridge_id`
2. `node_id`
3. `message_id`
4. `seq` (monotonic sequence)
5. `ack` (highest peer sequence acknowledged)
6. `timestamp`
7. `type`
8. `payload`
9. `trace_context`

Requirements:

1. `seq` must be monotonic for replay compensation.
2. `ack` must be explicit; delivery cannot be inferred from socket presence.

## Connection and Recovery Semantics

### Establishing Link

1. Node sends `register + authenticate`.
2. Controller grants session and capability authorization scopes.
3. Both sides enter heartbeat loop.

### Disconnect Recovery

1. Reconnecting side sends `last_acked_seq`.
2. Peer replays unacked messages from cursor.
3. In-flight `bridge.call` is deduped by `call_id + idempotency_key`.

### Node Restart

1. Node must reregister and resync health after restart.
2. Controller reconciles non-terminal tasks:
   - recoverable tasks continue dispatch;
   - undecidable tasks move to human/policy path.

## Consistency and Delivery Semantics

1. Bridge delivery is at-least-once.
2. Exactly-once for irreversible side effects depends on idempotency keys + `EffectRecord` (see durable run contract).
3. Duplicate messages for same `call_id` must not cause duplicate irreversible execution.

## Alignment with App Server Protocol

1. Clients still interact via `thread/turn/input/resume/interrupt` semantics.
2. Bridge only changes transport path, not Op/Event semantics.
3. `steer/follow_up/next_turn` queue semantics remain consistent on target node.

## Alignment with Capability Router

1. Router may classify provider source as `extension_bridge`.
2. Routing should factor node health, latency, policy, and capability version.
3. Bridge failure may trigger safe fallback only for non-side-effect calls.

## Security Model

1. Authentication:
   - short-lived tokens + long-lived node identity (rotatable)
2. Authorization:
   - capability-level scopes (least privilege)
3. Policy:
   - target-node policy remains final authority
4. Audit:
   - full chain `who -> where -> what -> why -> result`

Prohibited:

1. Unauthorized node attaching to existing sessions.
2. Bridge token granting governance bypass.

## Observability and SLO Metrics

Recommended minimum metrics:

1. `bridge_connected_nodes`
2. `bridge_heartbeat_lag_ms`
3. `bridge_reconnect_count`
4. `bridge_call_latency_ms`
5. `bridge_call_timeout_rate`
6. `bridge_replay_gap_count`

Recommended log fields:

1. `bridge_id/node_id/session_id/run_id/turn_id/call_id`
2. `seq/ack`
3. `route/policy_action/status`

## Failure and Degradation Strategy

1. Relay unavailable:
   - local node keeps running; remote control degrades.
2. Controller restart:
   - nodes reconnect automatically and resume via cursors.
3. Long offline period:
   - tasks move to `degraded` while keeping recoverable context.
4. Unrecoverable replay gap:
   - mark `gap_detected` and rebuild from snapshot.

## Alignment with Alan Philosophy

1. Runtime keeps Turing-machine semantics; Bridge changes execution location, not state-machine rules.
2. UNIX-style composability: Bridge is a replaceable channel, not business-logic core.
3. Human-in-the-End: remote control increases owner intervention capability, not blanket approval burden.

## Acceptance Criteria

1. Mobile/remote clients can reliably control target Alan instances.
2. Reconnect recovers calls/events via cursors without losing critical info.
3. Redelivery does not duplicate irreversible side effects.
4. Remote paths do not bypass target-node governance boundaries.
