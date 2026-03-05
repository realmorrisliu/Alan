# Mobile Reliability + Notification Contract

> Status: Phase D contract for remote mobile reconnect reliability.

## Goals

1. Reconnect should restore actionable session state without replaying side effects.
2. Pending approval/yield states should be discoverable through explicit signal fields.
3. Notification signals must be informational and never bypass governance boundaries.

## Reconnect Snapshot API

### Endpoints

1. Direct mode:
   - `GET /api/v1/sessions/{id}/reconnect_snapshot`
2. Relay mode:
   - `GET /api/v1/relay/nodes/{node_id}/api/v1/sessions/{id}/reconnect_snapshot`

### Response Contract (normative)

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

1. `latest_event_id` + `latest_submission_id` are dedupe hints for reconnecting clients.
2. `resume_required=true` means only explicit resume operation can advance execution.
3. `notifications.signals` may be empty; clients must tolerate sparse signal streams.

## Notification Signal Contract

Signals can be emitted via reconnect snapshot and future push channels. Initial signal types:

1. `pending_yield`
2. `pending_structured_input`
3. `resume_failed`
4. `gap_detected`

Signal constraints:

1. Stable `signal_id` for dedupe.
2. `informational=true` is required in transport payload.
3. Signal delivery loss is recoverable by reconnect snapshot read.

## Non-Bypass Governance Rules

1. Signals never authorize execution changes by themselves.
2. Approval/resume still requires `Op::Resume` on node authority path.
3. Token scopes and policy checks remain unchanged (`session.resume` is still required).
4. Relay/client cannot convert notification delivery into implicit resume.

## Harness Mapping

Phase D reliability scenarios should validate this contract:

1. `autonomy/mobile_reconnect_snapshot`
   - reconnect snapshot reflects latest event/submission and pending yield state.
2. `autonomy/mobile_notification_signal`
   - pending yield appears as informational signal; no execution-state mutation.
3. `autonomy/mobile_flaky_network_recovery`
   - reconnect + gap compensation remain deterministic under packet loss.

