## 1. Tracking And Product Boundary

- [x] 1.1 Create the GitHub tracking issue for `add-alan-anywhere-mvp` and link
  it to this OpenSpec change.
- [x] 1.2 Close or mark issue `#9` as superseded by the Alan Anywhere MVP issue.
- [x] 1.3 Keep issue `#75` open as the iOS task-manager IA follow-up and link
  it to the Alan Anywhere MVP issue.
- [ ] 1.4 Decide the MVP account provider and document any remaining
  auth-provider assumptions before implementation starts.

## 2. Account And Device Model

- [ ] 2.1 Define account-owned device records for Mac and iPhone, including
  `device_id`, display name, platform, owner account, enrollment state, last
  seen, and revocation state.
- [ ] 2.2 Add Mac device enrollment after Desktop account login with
  Keychain-backed device credentials.
- [ ] 2.3 Add iPhone device enrollment after mobile account login with platform
  secure credential storage.
- [ ] 2.4 Add device revocation handling that prevents future remote access and
  terminates or rejects active state-changing requests.

## 3. Cloud Presence And Relay Broker

- [ ] 3.1 Add Cloud service endpoints for listing account-owned devices and
  their online/offline/connectable status.
- [ ] 3.2 Add short-lived relay ticket issuance scoped to account, client
  device, target Mac device, workspace/session, and operation class.
- [ ] 3.3 Add Mac presence heartbeats that publish online/stale/offline status
  without moving runtime authority from the Mac.
- [ ] 3.4 Add audit records for enrollment, connection, revocation, and
  state-changing remote control attempts.

## 4. Mac Desktop Remote Availability

- [ ] 4.1 Start product-managed outbound relay connection automatically when
  Desktop is signed in and Alan Anywhere is enabled.
- [ ] 4.2 Keep environment-configured relay mode as development/operator
  compatibility while making account/device relay the Desktop default path.
- [ ] 4.3 Publish Mac-authored session/work-context status, including
  connectable local context state and active agent/session state.
- [ ] 4.4 Ensure Mac remains the final authority for local context identity,
  session liveness, governance, tool execution, and event ordering.

## 5. Realtime Relay And Daemon Contract

- [ ] 5.1 Extend daemon endpoint metadata for relay-approved realtime session event subscriptions.
- [ ] 5.2 Implement relay realtime event transport without making relay the
  author of event IDs, sequence, or runtime state.
- [ ] 5.3 Preserve `events/read` and `reconnect_snapshot` as the required
  recovery path after reconnect or event gaps.
- [ ] 5.4 Add tests that reject realtime relay subscription attempts for
  endpoints not approved by daemon endpoint metadata.

## 6. iPhone Alan Anywhere Experience

- [ ] 6.1 Replace manual daemon/relay connection as the primary iPhone path
  with account device discovery.
- [ ] 6.2 Show online Macs and connectable sessions/work contexts using
  product-facing labels, not relay node IDs or tunnel URLs.
- [ ] 6.3 Allow iPhone to send messages, interrupt runs, and resume pending
  yields against the selected Mac session/work context.
- [ ] 6.4 Ensure iPhone reconnects with its latest event cursor and rebuilds
  state from reconnect snapshots when gaps are reported.
- [ ] 6.5 Keep relay, node, routing, and daemon diagnostics behind explicit debug surfaces.

## 7. Security Verification

- [ ] 7.1 Add tests for account/device scope checks on read, write, resume, and
  admin remote operations.
- [ ] 7.2 Add tests showing Cloud cannot advance runtime state without routing
  to and receiving authorization from the Mac.
- [ ] 7.3 Add tests for revoked Mac and iPhone devices denying new state-changing operations.
- [ ] 7.4 Add tests or harness scenarios for dropped mobile connections, cursor
  replay, gap recovery, and no duplicate execution.

## 8. Documentation And OpenSpec Closure

- [ ] 8.1 Update product and maintainer docs to describe Alan Anywhere as
  device-to-device Alan continuation.
- [ ] 8.2 Update remote-control architecture/security docs to reference Alan
  Anywhere as the product layer above direct/relay transport.
- [ ] 8.3 Run focused Rust/Swift tests for changed daemon, relay, Desktop, and iPhone surfaces.
- [ ] 8.4 Run `openspec validate add-alan-anywhere-mvp --type change --strict --json`.
- [ ] 8.5 Run `openspec validate --all --strict --json`.
- [ ] 8.6 Run `git diff --check`.
- [ ] 8.7 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 8.8 Archive the OpenSpec change after implementation is merged.
