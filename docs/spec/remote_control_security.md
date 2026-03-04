# Remote Control Security Model

> Status: VNext security model for remote client control over Alan nodes.

## Trust Boundaries

1. **Agent Node Boundary**
   - Trusted execution + policy/sandbox enforcement boundary.
2. **Relay Boundary**
   - Transport/router boundary; untrusted for execution semantics.
3. **Client Boundary**
   - User device/app boundary with scoped credentials.

## AuthN Model

### Node identity

1. Agent node authenticates to relay with node credential (`node_token` or mTLS cert).
2. Node identity is stable (`node_id`) and revocable.

### Client identity

1. Client authenticates with device/user token.
2. Token must carry explicit allowed node/session scopes.

## AuthZ Scopes

Minimum scope classes:

1. `session.read`
2. `session.write` (submit/input/interrupt)
3. `session.resume` (yield resolution)
4. `session.admin` (fork/rollback/delete)
5. `node.manage` (node-level settings/registration)

Rules:

1. `session.resume` is required for remote approval actions.
2. Relay enforces coarse routing scopes; node re-validates all authorization.
3. Node-side authorization is final source of truth.
4. `/submit` and `/ws` perform a route-level precheck that accepts any mutating scope (`session.write` or `session.resume` or `session.admin`), then enforce exact operation scope on each submitted `Op`.

## Phase A Daemon Configuration (Implemented)

Direct-remote scope enforcement in `alan-agentd` is controlled by:

1. `ALAN_REMOTE_AUTH_ENABLED`
   - truthy values (`1`, `true`, `yes`, `on`) enable bearer-scope checks.
2. `ALAN_REMOTE_AUTH_TOKENS`
   - semicolon-delimited `token=scopes` bindings.
   - scope list is comma-delimited and supports:
     - `session.read`
     - `session.write`
     - `session.resume`
     - `session.admin`
   - `*` grants all scopes.

Examples:

1. `ALAN_REMOTE_AUTH_ENABLED=true`
2. `ALAN_REMOTE_AUTH_TOKENS=reader=session.read;writer=session.read,session.write;operator=*`

Additive remote metadata headers accepted by the API surface:

1. `x-alan-node-id`
2. `x-alan-client-id`
3. `x-alan-trace-id`
4. `x-alan-transport-mode` (`direct` or `relay`)

## Governance Preservation

1. Remote transport cannot bypass policy engine decisions.
2. Yield escalation payloads are signed/traceable to originating node event.
3. Resume decisions are tied to `request_id` + scoped principal.
4. Replay/recovery paths use same authorization checks as live paths.

## Token Lifecycle

1. Short-lived access tokens + refresh/rotation support.
2. Server-side revocation list for compromised tokens.
3. Node credential rotation with overlap window.
4. Denied/revoked tokens produce explicit auth failure codes (never silent).

## Replay and Message Integrity

1. Requests include nonce/timestamp window to reduce replay risk.
2. Event stream cursors are monotonic and session-bound.
3. Connection-level trace IDs are propagated for audit correlation.

## Relay Security Constraints

1. Relay forwards opaque protocol payloads; no semantic mutation.
2. Relay cannot manufacture terminal runtime state transitions.
3. Relay stores minimal metadata needed for routing and diagnostics.

## Audit Requirements

Each remote control decision should log:

1. `node_id/client_id/session_id`
2. `request_id/submission_id`
3. `scope_check_result`
4. `policy_action` (`allow/deny/escalate`)
5. `transport_mode` (`direct|relay`)
6. `resolved_by` (`human|policy|runtime`)

## Revocation Flow (Recommended)

1. Mark token/cert as revoked.
2. Propagate revocation cache to relay and node.
3. Terminate active connections bound to revoked credential.
4. Require re-auth for resumed control sessions.

## Threat Notes (MVP)

1. **Stolen client token**
   - Mitigation: short TTL, scoped claims, revocation, device binding.
2. **Relay compromise**
   - Mitigation: node-side authz finality, payload integrity checks, audit trails.
3. **Replay on flaky links**
   - Mitigation: nonce/timestamp checks and request-id idempotency.
4. **Approval bypass attempt**
   - Mitigation: resume only via valid pending `request_id` + scope.
