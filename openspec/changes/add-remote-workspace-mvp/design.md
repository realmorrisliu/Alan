## Context

The current remote-control architecture already separates Agent Node, Relay,
and Remote Client responsibilities. It also has a relay MVP, node routing
metadata, sticky session-to-node binding, reconnect snapshots, and scoped
remote-control checks. That foundation is intentionally technical: operators
enable relay mode with environment variables, pass node IDs and tokens, and
clients address relay nodes directly.

Alan Remote Workspace changes the product boundary. The user should experience
their Mac as an available Alan device that can continue work from iPhone. The
system can still use an outbound relay under the hood, but the product must not
ask the user to understand tunnels, daemon URLs, VPNs, public IPs, SSH, router
configuration, or port forwarding.

## Goals / Non-Goals

**Goals:**

- Make a signed-in Alan Desktop automatically become remotely reachable from
  the user's own iPhone without inbound network exposure.
- Let the iPhone app discover the user's online Macs and connect to a selected
  workspace/session using the same Alan account.
- Preserve Mac-authoritative runtime execution, tool execution, governance,
  session state, and event ordering.
- Support realtime message submission, streamed output, interrupt, yield
  resume, and reconnect recovery over the remote path.
- Bind remote access to account identity, device identity, scoped short-lived
  credentials, revocation, and encrypted transport.
- Keep existing direct/relay architecture as the lower-level transport
  foundation while replacing operator-facing setup with product-managed
  enrollment and presence.

**Non-Goals:**

- Remote desktop, screen sharing, terminal screen mirroring, or arbitrary
  desktop control.
- P2P hole punching, LAN discovery, router configuration, user-managed VPNs,
  SSH setup, or Cloudflare-style user-managed tunnels.
- Multi-user collaboration or shared workspaces.
- Complex enterprise networking, MDM, organization policy, or delegated
  account administration.
- Moving agent execution, workspace reads, tool execution, or governance
  authority to Alan Cloud.
- Building a full push-notification system as a blocker for foreground iPhone
  realtime use.

## Decisions

1. Use Alan Cloud as an account/device directory plus relay broker, not as a
   runtime authority.

   Alan Cloud owns user authentication, device enrollment, presence, relay
   routing, short-lived token issuance, revocation, and audit metadata. It does
   not execute tools, read local workspace files, decide governance outcomes,
   author session events, or advance runtime state.

   Alternative considered: expose `alan-agentd` directly from the Mac. That
   conflicts with zero configuration and would require public IP, router, or
   tunnel knowledge for many users.

2. Treat the Mac as the authoritative execution device.

   The Mac owns the local daemon/runtime, session store, workspace identity,
   event ordering, reconnect snapshot, tool execution, and policy decisions.
   Relay and iPhone requests are remote inputs to the Mac, not remote execution
   contexts.

   Alternative considered: proxy workspace state through Alan Cloud and allow
   cloud-side execution for continuity. That would break the security and
   product requirement that user tasks execute on the user's own device.

3. Add product-managed device enrollment above the existing relay tunnel.

   A signed-in Desktop creates or refreshes a stable local device identity,
   stores device credentials in Keychain, requests short-lived relay tickets,
   and starts the outbound relay connection automatically. The user sees
   device availability, not relay configuration.

   Alternative considered: continue using `ALAN_RELAY_URL`,
   `ALAN_RELAY_NODE_ID`, and `ALAN_RELAY_NODE_TOKEN` as the primary path.
   Those remain useful for development/operator modes but cannot be the MVP
   product path.

4. Model the user-facing surface as devices, workspaces, and sessions.

   The iPhone app should list the user's online Macs and connectable
   workspaces/sessions. It should not display relay node IDs, daemon base URLs,
   tunnel status, or raw routing headers unless a debug surface is explicitly
   opened.

   Alternative considered: reuse the existing relay node list directly in the
   mobile UI. That leaks implementation details and makes the product feel like
   remote infrastructure instead of workspace continuation.

5. Provide realtime relay subscriptions with cursor recovery.

   Remote Workspace needs realtime streamed output while preserving
   reconnect-safe recovery. The transport should support a realtime event
   subscription through the relay path, and clients must still use node-authored
   `events/read` and `reconnect_snapshot` after reconnect or gap detection.

   Alternative considered: use high-frequency polling only. Polling is a useful
   fallback, but it does not satisfy the product expectation of streamed output
   and responsive interrupt/approval flows.

6. Use device-bound, scoped, short-lived credentials.

   Account login proves user identity. Device enrollment binds a Mac/iPhone app
   installation to that account. Each remote connection uses short-lived access
   tokens scoped to the account, device, target Mac, workspace/session, and
   permitted operations. The Mac re-validates state-changing requests before
   applying them.

   Alternative considered: one long-lived bearer token per node. That matches
   the current technical MVP but is too hard to revoke safely and too weak for
   an account-driven consumer product.

## Risks / Trade-offs

- Relay compromise exposes routing metadata -> Keep relay non-authoritative,
  issue short-lived scoped credentials, minimize stored metadata, and require
  Mac-side revalidation for state-changing operations.
- Mac goes offline during iPhone use -> Keep session state on the Mac, mark the
  device offline/stale in presence, and allow the iPhone to show the last known
  state without pretending execution can continue.
- Realtime relay stream drops under mobile network churn -> Require cursor
  recovery through `events/read` and `reconnect_snapshot`; never re-drive a turn
  due to reconnect.
- Device list becomes noisy or confusing -> Show only user-owned devices with
  product labels, last activity, current workspace status, and clear offline
  state; keep relay diagnostics debug-only.
- Workspace path disclosure on mobile -> Show friendly workspace identity and
  status by default; avoid exposing full local paths unless the Mac authorizes
  that metadata for the signed-in user.
- Existing environment-configured relay paths diverge from product-managed
  remote workspace -> Keep environment configuration as development/operator
  compatibility but make the account/device path the default in Desktop and
  iPhone builds.

## Migration Plan

1. Add the OpenSpec requirements and GitHub tracking issue for Remote Workspace
   MVP; mark the old architecture issue as superseded by this product contract.
2. Introduce account/device data models and local device identity storage
   without changing existing relay behavior.
3. Implement Desktop device enrollment and automatic outbound relay connection
   behind a feature flag or development cloud endpoint.
4. Add Cloud device/presence/workspace directory endpoints and short-lived
   relay ticket issuance.
5. Add realtime relay event subscription while preserving polling fallback and
   reconnect snapshot recovery.
6. Update iPhone to use account device discovery and remote workspace/session
   selection instead of manual daemon connection.
7. Harden revocation, audit, and offline/reconnect behavior before making the
   feature default.
8. Keep rollback simple: Desktop can stop advertising remote availability and
   iPhone can fall back to existing manual/local daemon connection paths during
   development.

## Open Questions

- Which Alan account provider is authoritative for MVP login: Alan-hosted auth,
  Sign in with Apple, GitHub, or an existing managed account surface?
- Should remote event payloads be end-to-end encrypted between iPhone and Mac
  in MVP, or is TLS plus node-authoritative execution acceptable for the first
  product slice?
- What is the minimum workspace metadata that iPhone may display without
  exposing sensitive local paths?
- Should APNs pending-approval notifications be included in this MVP or tracked
  as a follow-up after foreground realtime remote workspace works?
