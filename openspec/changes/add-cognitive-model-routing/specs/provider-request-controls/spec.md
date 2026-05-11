## ADDED Requirements

### Requirement: Request Controls Compose With Cognitive Routing
Alan SHALL resolve request controls after cognitive routing selects the
effective cognitive profile, and the existing request-control resolver SHALL
remain the sole authority for effective reasoning effort.

#### Scenario: System 1 profile has effort intent
- **WHEN** cognitive routing selects System 1 with configured reasoning effort
  `low`
- **THEN** Alan passes that intent through request-control resolution and
  validates it against the selected System 1 provider and model

#### Scenario: System 2 profile has effort intent
- **WHEN** cognitive routing selects System 2 with configured reasoning effort
  `high`
- **THEN** Alan passes that intent through request-control resolution and
  validates it against the selected System 2 provider and model

#### Scenario: Turn effort override still wins
- **WHEN** a turn has an explicit reasoning-effort override and cognitive
  routing selects a profile with a different configured effort
- **THEN** the request-control resolver applies the existing turn-override
  precedence for the selected profile

#### Scenario: Provider adapters do not route
- **WHEN** a provider adapter receives a `GenerationRequest`
- **THEN** it projects the normalized request controls and does not select,
  override, or reinterpret the cognitive system
