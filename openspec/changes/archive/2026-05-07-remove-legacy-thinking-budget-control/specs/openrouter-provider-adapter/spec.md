## MODIFIED Requirements

### Requirement: OpenRouter request projection
Alan SHALL map Alan generation requests to OpenRouter SDK chat requests without
requiring runtime code to depend on OpenRouter SDK types. Reasoning control
precedence and validation are owned by `provider-request-controls`; this
capability covers the OpenRouter-specific wire projection. Normalized request
controls, including effective reasoning effort, are supplied on
`GenerationRequest` by `provider-request-controls`; the OpenRouter adapter only
translates those controls to supported OpenRouter SDK fields and does not accept
legacy `thinking_budget_tokens` fallback input.

#### Scenario: Basic message projection
- **WHEN** a request contains a system prompt and user, assistant, context, and tool messages
- **THEN** the OpenRouter adapter maps them to the SDK chat message roles and content fields expected by OpenRouter

#### Scenario: Tool definition projection
- **WHEN** a request contains Alan tool definitions
- **THEN** the OpenRouter adapter maps them to OpenRouter chat tool definitions and enables automatic tool choice behavior

#### Scenario: Tool result projection
- **WHEN** a request contains a tool-result message with a tool call id
- **THEN** the OpenRouter adapter preserves the tool call id in the projected OpenRouter message

#### Scenario: Reasoning effort projection
- **WHEN** a request contains normalized effective reasoning effort
- **THEN** the OpenRouter adapter maps the effort to OpenRouter reasoning request fields supported by the SDK without recomputing Alan-level precedence or defaults

#### Scenario: Unsupported provider extra parameter
- **WHEN** a request contains an OpenRouter `extra_params` key that the adapter does not support
- **THEN** the adapter fails before dispatching the request or returns an explicit provider warning rather than silently dropping the parameter
