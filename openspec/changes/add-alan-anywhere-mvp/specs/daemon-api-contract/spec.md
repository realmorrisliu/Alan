## MODIFIED Requirements

### Requirement: Relay Policy Metadata
The daemon SHALL derive relay forwarding, realtime streaming support,
WebSocket/subscription support, streaming exclusion, WebSocket exclusion,
session binding extraction, and response URL rewriting from endpoint contract
metadata.

#### Scenario: relay forwarding allows only contract-approved endpoints
- **WHEN** a relay request attempts to forward a daemon API path
- **THEN** the relay layer allows forwarding only when the matched endpoint
  metadata permits relay forwarding

#### Scenario: relay realtime event subscriptions allow only contract-approved endpoints
- **WHEN** an Alan Anywhere client attempts to subscribe to realtime session
  events through the relay path
- **THEN** the relay layer allows the subscription only when the matched
  endpoint metadata permits relay realtime event delivery
- **AND** relay event delivery preserves node-authored event IDs and sequence
  metadata

#### Scenario: relay URL rewriting uses response URL metadata
- **WHEN** a relayed session lifecycle response contains daemon-relative URL
  fields
- **THEN** the relay layer rewrites only the response fields identified by the
  endpoint contract
