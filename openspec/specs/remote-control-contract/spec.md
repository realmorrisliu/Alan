# remote-control-contract Specification

## Purpose
Defines remote-control contracts for direct and relay modes, node/client
identity, scopes, reconnect behavior, notification signals, and non-bypass
governance.

## Requirements
### Requirement: Remote control contracts live in OpenSpec
alan SHALL specify remote control topology, direct and relay modes, remote
security, node/client identity, scopes, reconnect snapshots, notification
signals, and non-bypass governance rules in OpenSpec.

#### Scenario: Remote control behavior changes
- **WHEN** a change modifies direct-mode daemon configuration, relay routing,
  app-server protocol extensions, node discovery, reconnect behavior,
  notification signaling, token lifecycle, or revocation
- **THEN** the OpenSpec delta updates this capability, `alan-anywhere`,
  `daemon-api-contract`, or another named remote-control owner
- **AND** remote control docs under `docs/maintainer/` remain planning/runbook
  surfaces rather than the contract source

#### Scenario: Legacy remote doc is referenced
- **WHEN** `docs/spec/remote_control_architecture.md` or
  `docs/spec/remote_control_security.md` is opened
- **THEN** the file is a bridge to the relevant OpenSpec owner

### Requirement: Remote governance cannot bypass local policy
alan SHALL preserve local governance and workspace authorization boundaries
when sessions are controlled through direct remote clients, relay transports, or
mobile-style reconnect flows.

#### Scenario: Remote client resumes a yielded session
- **WHEN** a remote client submits approval, resume, interrupt, or follow-up
  input
- **THEN** the daemon applies the same session governance and authorization
  rules as a local client
- **AND** remote notification signals remain informational rather than policy
  bypasses
