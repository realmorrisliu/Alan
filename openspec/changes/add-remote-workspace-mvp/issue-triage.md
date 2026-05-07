## OpenSpec

- Tracking issue: `#349`
- Change: `add-remote-workspace-mvp`
- Proposal: `openspec/changes/add-remote-workspace-mvp/proposal.md`
- Design: `openspec/changes/add-remote-workspace-mvp/design.md`
- Requirements:
  - `openspec/changes/add-remote-workspace-mvp/specs/remote-workspace-access/spec.md`
  - `openspec/changes/add-remote-workspace-mvp/specs/daemon-api-contract/spec.md`
- Tasks: `openspec/changes/add-remote-workspace-mvp/tasks.md`

## Summary

Define and implement Alan Remote Workspace MVP: a signed-in Mac automatically
becomes remotely connectable, and a signed-in iPhone using the same account can
discover that Mac, choose a workspace/session, stream events, send messages,
interrupt execution, resume pending yields, and recover after reconnect without
requiring public IPs, router setup, VPNs, tunnels, SSH, daemon URLs, or port
forwarding.

The user-facing product framing is:

> Your Alan is continuing work on another device.

Not:

> Remote desktop, LAN tunnel, or network configuration.

## Scope

- Account-bound Mac/iPhone device enrollment.
- Automatic Desktop remote availability over outbound encrypted relay.
- iPhone device/workspace/session discovery.
- Realtime remote event streaming plus `events/read` and reconnect snapshot
  recovery.
- Remote message submit, interrupt, and yield resume.
- Device-bound, scoped, short-lived credentials and revocation.
- Mac-authoritative execution, workspace access, governance, and event
  ordering.

## Non-goals

- Remote desktop or screen sharing.
- P2P hole punching.
- LAN discovery.
- Multi-user collaboration.
- Enterprise networking/MDM policy.
- Cloud-side agent/tool execution.

## Issue Cleanup

- Close `#9` as superseded by `#349`, this OpenSpec-backed Remote Workspace MVP issue.
  The lower-level Agent Node / Relay / Client architecture remains the
  transport foundation, but this issue becomes the product contract.
- Keep `#75` open as the iOS task-manager/product IA follow-up. It should
  depend on this MVP rather than replace it.
- Leave `#305` unchanged; it is unrelated to remote access.
- Leave closed phase issues `#32`, `#33`, `#34`, and `#35` closed; their
  completed direct/relay/multi-node/reliability work becomes prior foundation.

## Verification

- `openspec validate add-remote-workspace-mvp --type change --strict --json`
- `openspec validate --all --strict --json`
- `git diff --check`
