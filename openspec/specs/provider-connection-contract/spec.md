# provider-connection-contract Specification

## Purpose
Defines provider and connection-profile contracts, including provider
capabilities, authentication boundaries, host credential storage, request
controls, model metadata, and explicit degradation.

## Requirements
### Requirement: Provider and connection contracts live in OpenSpec
alan SHALL specify provider capabilities, provider authentication, connection
profiles, request controls, provider-specific degradation, and host credential
boundaries in OpenSpec.

#### Scenario: Provider setup changes
- **WHEN** a change modifies provider descriptors, connection profiles,
  credential storage, managed auth, profile selection, provider request
  shaping, or model metadata
- **THEN** the OpenSpec delta updates this capability,
  `provider-request-controls`, a provider-specific capability such as
  `openrouter-provider-adapter`, or an active provider change
- **AND** no duplicate provider contract is maintained under `docs/spec/`

#### Scenario: Legacy provider doc is referenced
- **WHEN** `docs/spec/provider_capability_contract.md`,
  `docs/spec/provider_auth_contract.md`,
  `docs/spec/connection_profile_contract.md`, or a provider migration doc is
  opened
- **THEN** the page is only a bridge to the OpenSpec owner

### Requirement: Host auth and runtime provider state remain separated
alan SHALL keep host credential control, managed login state, connection profile
metadata, runtime request shaping, and provider-native features in their
respective layers.

#### Scenario: Credential material is configured
- **WHEN** a user configures API keys, managed ChatGPT login, or provider
  credential references
- **THEN** secrets are stored through host credential mechanisms rather than in
  agent-facing `agent.toml`

#### Scenario: Provider feature differs by adapter
- **WHEN** a provider supports or rejects a feature such as reasoning effort,
  continuation, rich content, or provider-native metadata
- **THEN** alan projects that capability through the provider contract and
  degrades explicitly when the selected provider cannot support it
