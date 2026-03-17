# App Server Protocol Contract

> Status: VNext target contract (current HTTP/WS API remains a transition layer).

## Goals

Provide a unified protocol layer for multi-client Alan frontends (TUI/Native/Web/IDE) with:

1. Streaming interaction over long-lived connections.
2. Explicit thread/turn lifecycle semantics.
3. Stable event subscription and recovery behavior.
4. Extensible input routing (`steer/follow_up/next_turn`) and autonomy execution (scheduler/durable run).

## Design Principles

1. **Protocol stability first**: clients should not depend on runtime internals.
2. **Explicit state**: thread/turn/item are first-class objects.
3. **Recoverable streams**: clients can fill gaps using event cursors.
4. **Backward compatibility**: keep `/sessions/*` APIs while evolving.

## Core Objects

### Thread

- Long-lived conversation container (maps to current session).
- Includes metadata, status, and history index.
- Can carry `task_id/run_id` metadata as additive fields.

### Turn

- One execution cycle inside a Thread.
- Explicit states: `running/yielded/completed/interrupted/failed`.

### Item

- Atomic record inside a Turn:
  - `user_input`
  - `queued_input` (`follow_up` / `next_turn`)
  - `assistant_delta/final`
  - `tool_call/tool_result`
  - `reasoning_delta`
  - `yield_request/resume`
  - `compaction_marker`

## Protocol Layers

### Control Plane

1. `thread/start|resume|fork|archive|rollback|compact`
2. `turn/start|input|interrupt|resume`
3. Tool-governance responses (approval/rejection/structured input)
4. Scheduler controls (optional extension): `run/sleep|run/wake|run/schedule`

### Data Plane

1. `events/stream` (real-time)
2. `events/read` (compensating pull)
3. `thread/read` (snapshot read)
4. `reconnect_snapshot` (mobile reconnect handoff snapshot)

## Current API Mapping (Compatibility Layer)

Current endpoints map to target semantics:

1. `POST /sessions` -> `thread/start`
2. `POST /sessions/{id}/submit` -> `turn/start` / `turn/input`
3. `GET /sessions/{id}/events` -> `events/stream`
4. `GET /sessions/{id}/events/read` -> `events/read`
5. `POST /sessions/{id}/resume` -> `turn/resume`
6. `POST /sessions/{id}/rollback` -> `thread/rollback`
7. `POST /sessions/{id}/compact` -> `thread/compact`

Compatibility notes:

1. Legacy `turn/steer` can be treated as `turn/input{mode=steer}` alias.
2. Legacy mode-less `Op::Input` defaults to `mode=steer`.
3. `thread/rollback` is explicitly non-durable; compatibility responses surface `durability.durable=false` and an in-memory warning.
4. `POST /sessions/{id}/compact` always maps to `Op::CompactWithOptions`; clients may omit the body or send `{ "focus": "..." }`.

## Input Modes (First-Class Semantics)

Suggested `turn/input` structure:

1. `thread_id`
2. `input` (content parts)
3. `mode`: `steer | follow_up | next_turn`
4. `expected_turn_id` (optional concurrency guard)

Semantics:

1. `steer`: inject guidance into active turn.
2. `follow_up`: execute right after current turn completes.
3. `next_turn`: context for the next turn only.

## Event Model (Normative Recommendation)

Each event should contain:

1. `event_id` (monotonic or sortable)
2. `thread_id`
3. `turn_id` (if applicable)
4. `type`
5. `timestamp`
6. `payload`

Client recovery logic:

1. Track `latest_event_id`.
2. Pull gaps using `after_event_id` after reconnect.
3. If `gap=true`, rebuild state from thread snapshot.

## Lifecycle Constraints

1. `turn/start` must produce start and terminal turn-boundary events.
2. `turn/input{mode=steer}` applies only to active turn.
3. `turn/input{mode=follow_up|next_turn}` may queue outside active execution.
4. `turn/resume` is valid only in yielded state.
5. `turn/interrupt` must end with interrupted or failed terminal state.

## Error Semantics

Two classes:

1. **Request-level errors**: invalid parameters, state conflicts, missing resources.
2. **Execution-level errors**: runtime/provider/tool failures.

Requirements:

1. Request-level errors return synchronously with machine-readable codes.
2. Execution-level errors flow through events with `turn_id` and context.
3. Queue-capacity overflow should return recoverable request-level errors.

## Subscription and Backpressure

1. Server should provide bounded queues and overload protection.
2. Overload rejection should return explicit retryable error codes.
3. Clients should implement exponential backoff and reconnect recovery.

## Security and Governance

1. Approval and user-input requests should use unified Yield/Resume flow.
2. Sensitive operations must be traceable to policy decisions.
3. Protocol layer must not bypass sandbox/policy constraints.
4. High-risk actions in recovery/replay must not bypass governance boundaries.

## Remote Routing Extension Notes

For remote node control (direct + relay modes), protocol metadata can be extended additively:

1. `node_id` (target execution node)
2. `client_id` (logical device/session identity)
3. `connection_id` (transport-level connection id)
4. `transport_mode` (`direct|relay`)
5. `trace_id` (cross-hop diagnostics)
6. `node_switch_mode` (`force` for explicit relay session rebind)

Cross-node routing safeguards (relay multi-node mode):

1. Sticky `session_id -> node_id` routing should reject accidental cross-node requests with
   machine-readable conflict code `relay_session_node_conflict` (HTTP 409).
2. Explicit session switch should be opt-in (`x-alan-node-switch: force`) and deterministic.
3. Relay should expose resolved routing decision (`x-alan-routed-node-id`) in proxied responses.
4. Core turn/run semantics remain node-authoritative.

Mobile reliability extension notes (Phase D):

1. `reconnect_snapshot` response should expose `latest_submission_id` for reconnect dedupe hints.
2. Pending-yield notifications are informational transport signals and do not change turn state.
3. Any signal-driven follow-up still uses explicit `turn/resume` or `turn/input` operations.

Related specs:

1. `remote_control_architecture.md`
2. `remote_control_security.md`
3. `mobile_reliability_contract.md`

## Versioning Strategy

1. Add new fields in backward-compatible way whenever possible.
2. Version breaking changes (`v2`/`v3`) with migration windows.
3. Include schema/type generation checks in CI.
4. Prefer extending `mode` over introducing frequent new methods.

## Acceptance Criteria

1. Multi-client state stays consistent on the same thread.
2. Reconnect recovery avoids duplicate execution and detectable event loss.
3. `steer/follow_up/next_turn` behavior is reproducible in protocol tests.
4. `turn/input/resume/interrupt` behavior is reproducible in protocol tests.
