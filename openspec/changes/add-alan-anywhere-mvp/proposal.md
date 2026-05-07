## Why

Alan already has a technical remote-control foundation, but it still feels like
an operator-configured relay/tunnel system. The MVP should turn that foundation
into a product experience where a user opens Alan on their Mac, opens Alan on
iPhone with the same account, and continues Alan without learning
VPN, tunnels, public IPs, router configuration, SSH, or port forwarding.

## What Changes

- Add Alan Anywhere as an account-bound, zero-configuration way to continue
  Alan from a user's own Mac on iPhone.
- Have Alan Desktop automatically register and advertise the Mac as an online,
  trusted execution device after account login.
- Have Alan iPhone automatically discover the user's online Macs, connect to a
  selected Mac, continue a session or work context, stream events, send messages,
  interrupt execution, resume pending yields, and recover after reconnect.
- Introduce product-level device/session/current-work status instead of exposing
  relay nodes, tunnel URLs, daemon URLs, public IPs, or router concepts.
- Preserve the existing invariant that agent execution, tool execution,
  governance checks, workspace access, and event ordering remain authoritative
  on the user's Mac.
- Add device binding, scoped authorization, revocation, and encrypted transport
  requirements for Alan Anywhere access.
- Fold the current open remote-control architecture issue into this product
  contract while keeping the iOS task-manager issue as a follow-up UI framing
  track.

## Capabilities

### New Capabilities

- `alan-anywhere`: Defines Alan account-bound device discovery, automatic Mac
  availability, iPhone continuation, realtime control/event flow, reconnect
  recovery, and security boundaries for the MVP.

### Modified Capabilities

- `daemon-api-contract`: Extends relay/API endpoint metadata so Alan Anywhere
  clients can subscribe to realtime session events through the relay path while
  preserving cursor replay and node-authoritative execution.

## Impact

- Alan Desktop/macOS account login, device enrollment, Keychain-backed device
  credentials, and automatic outbound relay connection.
- Alan iPhone account login, device/session discovery, connection selection,
  realtime session view, message submission, interrupt, and yield resume.
- Alan Cloud/App Server account, device registry, presence, relay broker, token
  issuance, revocation, and audit surfaces.
- Daemon/relay session event routing, reconnect snapshot usage, and endpoint
  contract metadata for realtime Alan Anywhere flows.
- Existing GitHub issue tracking for remote access: close or supersede `#9`
  with this OpenSpec-backed product issue; keep `#75` open as iOS IA follow-up
  unless it is rewritten to depend on this change.
