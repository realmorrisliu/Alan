# runtime-harness-contract Specification

## Purpose
Defines normative harness contracts for scenario semantics, runner pass/fail
criteria, KPI expectations, self-eval boundaries, and external bridge delivery.

## Requirements
### Requirement: Harness contracts live in OpenSpec
alan SHALL specify normative harness behavior, KPI expectations, self-eval
boundaries, and bridge semantics in OpenSpec, while keeping JSON scenarios and
runner docs as executable fixtures and operator guidance.

#### Scenario: Harness behavior changes
- **WHEN** a change modifies scenario semantics, runner pass/fail criteria,
  KPI meanings, self-eval governance, or bridge message delivery semantics
- **THEN** the requirement is captured in this capability or a more specific
  active OpenSpec capability
- **AND** fixture JSON under `docs/harness/scenarios/` remains data rather than
  the contract source

#### Scenario: Harness docs describe current commands
- **WHEN** `docs/harness/README.md`, self-eval docs, KPI docs, or live
  validation guides document runner usage
- **THEN** they may remain under `docs/` as current validation instructions
- **AND** they point to OpenSpec when they state reusable normative behavior

### Requirement: Harness bridge delivery is explicit
alan SHALL treat external harness bridges as bounded control/data-plane
surfaces with explicit envelope, recovery, consistency, security, and
observability expectations.

#### Scenario: External runner integrates with alan
- **WHEN** a harness runner sends operations, receives events, reconnects, or
  reports assertions
- **THEN** the bridge contract identifies delivery semantics, recovery behavior,
  and failure reporting in OpenSpec before the integration becomes a required
  validation path

### Requirement: Harness bridge roles and planes are stable
alan SHALL model harness bridge integrations as extensions of execution and
governance transport, not as a replacement for runtime state-machine or app
server semantics.

Bridge roles:

- **Bridge Controller**: daemon-side manager for connection, authentication,
  routing, and reconnect recovery.
- **Bridge Node Agent**: target-machine agent that bridges local runtime or
  extension host and executes requests.
- **Relay**: optional routing layer for NAT or mobile-network paths.
- **Client**: TUI, native, web, mobile, or external runner sending
  control/subscription requests through the app-server surface.

Control-plane messages:

1. `bridge.register`
2. `bridge.authenticate`
3. `bridge.heartbeat`
4. `bridge.attach_session`
5. `bridge.detach_session`
6. `bridge.drain`

Data-plane messages:

1. `bridge.call`
2. `bridge.result`
3. `bridge.event`
4. `bridge.cancel`
5. `bridge.resume`

#### Scenario: Bridge integration is introduced
- **WHEN** a harness or remote runner integrates through bridge transport
- **THEN** it identifies controller, node agent, relay, and client roles
- **AND** it keeps business semantics in app-server Op/Event contracts rather
  than creating a separate business protocol

### Requirement: Harness bridge envelope supports replay and tracing
alan SHALL require bridge messages to carry stable identity, sequence, ack, and
trace metadata needed for recovery and audit.

Minimum message envelope fields:

1. `bridge_id`
2. `node_id`
3. `message_id`
4. `seq`
5. `ack`
6. `timestamp`
7. `type`
8. `payload`
9. `trace_context`

Rules:

- `seq` is monotonic for replay compensation.
- `ack` is explicit and delivery is not inferred from socket presence.
- Trace context follows control, data, and result messages.

#### Scenario: Bridge message is sent
- **WHEN** a bridge controller, node, relay, or client sends a protocol message
- **THEN** the envelope includes sequence, acknowledgement, target node, and
  trace context sufficient for replay and diagnostics

### Requirement: Harness bridge reconnects without duplicating side effects
alan SHALL treat bridge delivery as at-least-once and require idempotency for
irreversible or side-effecting work.

Connection establishment:

1. Node sends register plus authenticate.
2. Controller grants session and capability authorization scopes.
3. Both sides enter heartbeat loop.

Disconnect recovery:

1. Reconnecting side sends `last_acked_seq`.
2. Peer replays unacked messages from cursor.
3. In-flight `bridge.call` is deduped by `call_id` plus
   `idempotency_key`.

Node restart:

1. Node reregisters and resyncs health.
2. Controller reconciles non-terminal tasks.
3. Recoverable tasks continue dispatch.
4. Undecidable tasks move to human or policy path.

Consistency rules:

- Bridge delivery is at-least-once.
- Exactly-once irreversible effects depend on idempotency keys and
  `EffectRecord`.
- Duplicate messages for the same `call_id` must not cause duplicate
  irreversible execution.

#### Scenario: Bridge reconnects after dropped transport
- **WHEN** either side reconnects with a last acknowledged sequence
- **THEN** alan replays unacknowledged messages and dedupes in-flight
  side-effect calls by call id and idempotency key

#### Scenario: Side-effect status is undecidable after node restart
- **WHEN** a bridge node restarts and cannot prove whether a non-terminal side
  effect completed
- **THEN** alan moves the task to human or policy recovery rather than
  re-executing blindly

### Requirement: Harness bridge preserves app-server and capability-router semantics
alan SHALL keep bridge routing below the runtime/app-server semantics and
capability governance layer.

Rules:

- Clients still interact through thread, turn, input, resume, interrupt, or
  compatibility session semantics.
- Bridge changes the transport path, not Op/Event semantics.
- `steer`, `follow_up`, and `next_turn` queue semantics remain consistent on
  the target node.
- Capability router may classify provider source as `extension_bridge`.
- Routing may factor node health, latency, policy, and capability version.
- Bridge failure may trigger safe fallback only for non-side-effect calls.
- Router and bridge do not auto-reenter while yielded; they wait for explicit
  resume.

#### Scenario: Bridge forwards a runtime event
- **WHEN** a runtime event is forwarded through bridge transport
- **THEN** event ordering, cursor semantics, turn identity, and input-mode
  semantics remain authored by the target runtime

#### Scenario: Bridge provider fails during side-effect call
- **WHEN** a bridge-backed capability fails after a side-effect may have
  happened
- **THEN** alan does not silently switch providers or retry without idempotency
  and governance recovery

### Requirement: Harness bridge security does not bypass target governance
alan SHALL keep target-node governance and execution backend as the final
authority for bridge, relay, and external harness integrations.

Security rules:

1. Node authentication uses short-lived tokens plus long-lived, rotatable node
   identity.
2. Authorization uses least-privilege capability or session scopes.
3. Target-node policy remains final authority.
4. Relay or bridge credentials cannot grant governance bypass.
5. Unauthorized nodes cannot attach to existing sessions.
6. The audit chain records who, where, what, why, and result.

Required audit/log fields:

1. `bridge_id`
2. `node_id`
3. `session_id`
4. `run_id`
5. `turn_id`
6. `call_id`
7. `seq`
8. `ack`
9. `route`
10. `policy_action`
11. `status`

#### Scenario: External runner requests a privileged action
- **WHEN** a bridge-connected runner requests a side-effecting or privileged
  action
- **THEN** target-node policy and execution backend authorize the action before
  execution
- **AND** bridge or relay credentials alone are not sufficient

### Requirement: Harness bridge exposes recovery and SLO signals
alan SHALL expose enough bridge metrics and failure modes for operators and
harnesses to diagnose transport reliability separately from runtime behavior.

Minimum metrics:

1. `bridge_connected_nodes`
2. `bridge_heartbeat_lag_ms`
3. `bridge_reconnect_count`
4. `bridge_call_latency_ms`
5. `bridge_call_timeout_rate`
6. `bridge_replay_gap_count`

Failure strategy:

- Relay unavailable: local node keeps running and remote control degrades.
- Controller restart: nodes reconnect automatically and resume through cursors.
- Long offline period: tasks move to `degraded` while preserving recoverable
  context.
- Unrecoverable replay gap: mark `gap_detected` and rebuild from snapshot.

#### Scenario: Replay gap is detected
- **WHEN** bridge recovery cannot fill a sequence gap from retained messages
- **THEN** alan emits an observable gap signal and rebuilds from snapshot rather
  than pretending the stream is continuous

### Requirement: Harness scenarios remain executable fixtures, not hidden contracts
alan SHALL keep scenario JSON, runner scripts, and harness docs aligned with
OpenSpec-owned behavior while treating them as executable fixtures and operator
guidance.

Rules:

- Normative semantics live in OpenSpec specs.
- Fixture JSON under `docs/harness/scenarios/` remains scenario data.
- Runner docs may describe current commands and environment setup.
- When docs state reusable behavior, they point back to the OpenSpec owner.
- Self-eval and external bridge outputs distinguish pass/fail criteria from
  mocked, skipped, or environment-blocked checks.

#### Scenario: Harness fixture captures new behavior
- **WHEN** a fixture begins asserting reusable product behavior
- **THEN** the behavior is also captured in an active OpenSpec owner before the
  fixture becomes a required validation path
