## MODIFIED Requirements

### Requirement: Request control intent is separate from resolved controls
Alan SHALL represent user, session, and turn request-control intent separately
from resolved request controls. `Config` and transport DTOs MAY contain
canonical reasoning-effort intent, but they MUST NOT expose legacy
thinking-budget intent and MUST NOT be the authority for final effective
request-control values.

#### Scenario: Agent config sets effort
- **WHEN** `agent.toml` sets `model_reasoning_effort = "high"`
- **THEN** the resolved runtime config carries reasoning-effort intent for `high`

#### Scenario: Legacy budget config is rejected
- **WHEN** `agent.toml` sets `thinking_budget_tokens`
- **THEN** Alan rejects the configuration with a breaking-change error that names `model_reasoning_effort` as the replacement
- **AND** Alan does not preserve the budget as request-control intent

#### Scenario: RuntimeConfig does not duplicate independent effective truth
- **WHEN** a workspace overlay changes model metadata or a runtime launch applies a session override
- **THEN** Alan resolves request controls through the resolver
- **AND** `RuntimeConfig` does not require a separate effective reasoning field that can drift from `Config`

### Requirement: Runtime-owned request control resolution
Alan SHALL resolve effective provider request controls through a runtime-owned
resolver before dispatching a model request. The resolver SHALL combine turn
override, session/runtime override, agent config intent, model catalog default,
provider capabilities, and provider default in a single typed result.

#### Scenario: Turn override has highest precedence
- **WHEN** a session has `model_reasoning_effort = "high"` and a turn requests `reasoning_effort = "low"`
- **THEN** the resolved request controls use `low` for that turn
- **AND** the session-level resolved controls remain unchanged for later turns

#### Scenario: Session override wins over agent config
- **WHEN** a session is created with a reasoning effort override and the resolved agent config has a different effort
- **THEN** Alan uses the session override for that runtime

#### Scenario: Child agent override
- **WHEN** a child-agent spawn spec includes a reasoning effort runtime override
- **THEN** Alan applies that effort to the child runtime after validating it against the child model

#### Scenario: Model default is applied once by the resolver
- **WHEN** no explicit reasoning effort is configured and the resolved model catalog entry declares default effort `medium`
- **THEN** the resolver returns reasoning effort `medium` with source `model_default`
- **AND** no other runtime, daemon, or provider-adapter layer recomputes that default independently

#### Scenario: Unknown model metadata uses provider default
- **WHEN** the selected provider/model has no model catalog metadata and no explicit request control is configured
- **THEN** the resolver returns no explicit reasoning effort
- **AND** the source records that provider defaults will apply

### Requirement: Explicit request controls are validated before dispatch
Alan SHALL validate explicit request controls against provider capability and
resolved model metadata before making a provider request. Explicit unsupported
controls SHALL fail before dispatch instead of being silently dropped.

#### Scenario: Provider does not support effort control
- **WHEN** a turn explicitly requests reasoning effort and the selected provider declares no effort-control support
- **THEN** Alan rejects the turn before provider dispatch
- **AND** the error identifies the unsupported request control

#### Scenario: Model rejects unsupported effort
- **WHEN** the resolved model catalog entry supports only `low` and `high`
- **AND** a session or turn explicitly requests `xhigh`
- **THEN** Alan rejects the request before provider dispatch
- **AND** the error lists the supported efforts from the model metadata

#### Scenario: Legacy budget request is rejected
- **WHEN** config, protocol, API, or client payloads contain `thinking_budget_tokens`
- **THEN** Alan rejects the request before provider dispatch
- **AND** the error identifies `model_reasoning_effort` as the supported reasoning control

### Requirement: Generation requests carry normalized controls
Alan SHALL carry canonical reasoning controls on `GenerationRequest` rather than
requiring provider adapters to infer them from ad hoc `extra_params` or legacy
budget fields.

#### Scenario: Turn request includes resolved effort
- **WHEN** runtime constructs a generation request for a reasoning-capable model
- **THEN** the request includes the validated effective reasoning effort

#### Scenario: No reasoning control
- **WHEN** neither explicit effort nor model default applies
- **THEN** the request omits reasoning controls and lets the provider use its default behavior

#### Scenario: Legacy public budget field is unavailable
- **WHEN** a caller tries to construct or mutate a generation request with `thinking_budget_tokens`
- **THEN** Alan provides no supported public request-control path for that field
- **AND** provider projection cannot use a legacy budget fallback

### Requirement: Provider adapters only project normalized controls
Provider adapters SHALL consume normalized request controls from
`GenerationRequest` and project them to provider-specific payload fields.
Provider adapters SHALL NOT own Alan-level override precedence, model default
selection, config conflict resolution, or legacy budget compatibility.

#### Scenario: Canonical effort overrides provider extra params
- **WHEN** a generation request contains normalized reasoning effort `low` and provider-specific extra params include `reasoning_effort = "high"`
- **THEN** the provider adapter sends `low`
- **AND** the provider-specific extra param does not create a competing effective value

#### Scenario: Provider-native budget is derived internally
- **WHEN** a provider requires a budget-shaped wire field for the effective reasoning effort
- **THEN** the provider adapter derives that budget from normalized effort, model metadata, and provider rules
- **AND** the adapter does not accept public `thinking_budget_tokens` as an alternate effective value

### Requirement: OpenAI provider mapping
Alan SHALL map canonical reasoning effort to OpenAI-native request fields for
OpenAI Responses and OpenAI Chat Completions providers.

#### Scenario: OpenAI Responses effort
- **WHEN** `openai_responses` receives a generation request with effective effort
- **THEN** Alan sends it as `reasoning.effort`

#### Scenario: OpenAI Chat Completions effort
- **WHEN** `openai_chat_completions` receives a generation request with effective effort
- **THEN** Alan sends it as `reasoning_effort`

#### Scenario: OpenAI unsupported effort
- **WHEN** a selected OpenAI model does not support the effective effort
- **THEN** Alan rejects the request before provider dispatch

### Requirement: Anthropic provider mapping
Alan SHALL map canonical reasoning effort to Anthropic extended-thinking budget
configuration when the selected Anthropic model supports extended thinking.

#### Scenario: Anthropic effort maps to budget
- **WHEN** `anthropic_messages` receives a generation request with effective effort
- **THEN** Alan maps the effort to the configured Anthropic `thinking.budget_tokens` preset for the selected model

#### Scenario: Anthropic minimum budget
- **WHEN** the mapped Anthropic budget is below the provider minimum
- **THEN** Alan rejects the request before dispatch

#### Scenario: Anthropic max tokens relationship
- **WHEN** Anthropic thinking is enabled
- **THEN** Alan ensures `max_tokens` is greater than `budget_tokens` or rejects/adjusts according to the provider adapter contract

### Requirement: Gemini provider mapping
Alan SHALL map canonical reasoning effort to Gemini thinking controls according
to model family.

#### Scenario: Gemini 3 thinking level
- **WHEN** `google_gemini_generate_content` uses a Gemini 3 model and receives effective effort
- **THEN** Alan maps supported efforts to `thinkingConfig.thinkingLevel`

#### Scenario: Gemini 2.5 thinking budget
- **WHEN** `google_gemini_generate_content` uses a Gemini 2.5 model and receives effective effort
- **THEN** Alan maps the effort to a catalog-defined `thinkingBudget`

#### Scenario: Gemini disable thinking
- **WHEN** a Gemini model does not support disabling thinking and the effective effort is `none`
- **THEN** Alan rejects the request before dispatch

### Requirement: Compatible-provider and OpenRouter mapping
Alan SHALL only send reasoning-effort extension fields to compatibility
providers when the provider/model explicitly declares support.

#### Scenario: Compatible provider supports effort extension
- **WHEN** `openai_chat_completions_compatible` receives effective effort for a model that declares `reasoning_effort` support
- **THEN** Alan sends the compatible extension field

#### Scenario: Compatible provider does not support effort extension
- **WHEN** a compatibility provider/model lacks declared effort support
- **THEN** Alan rejects explicit reasoning effort rather than silently dropping it

#### Scenario: OpenRouter SDK-backed provider
- **WHEN** the SDK-backed `openrouter` provider receives effective effort
- **THEN** Alan maps the effort to the OpenRouter SDK/provider-native reasoning field supported by the selected endpoint and model

### Requirement: Documentation and migration
Alan SHALL document effort-first reasoning controls and the breaking removal of
legacy `thinking_budget_tokens` public configuration.

#### Scenario: Agent config example
- **WHEN** documentation shows reasoning configuration
- **THEN** it uses `model_reasoning_effort = "medium"` as the primary example

#### Scenario: Budget documentation
- **WHEN** documentation mentions `thinking_budget_tokens`
- **THEN** it describes the field as removed legacy configuration and directs users to `model_reasoning_effort`

#### Scenario: Migration from budget to effort
- **WHEN** users have existing `thinking_budget_tokens` config
- **THEN** documentation explains that the field is rejected and must be replaced with a named reasoning effort when the selected provider/model supports effort

### Requirement: Request control tests guard layer boundaries
Alan SHALL include tests that cover resolver precedence, validation, provider
projection, and daemon metadata mirroring. Tests SHALL make it hard to re-add
effective request-control logic in clients, daemon routes, provider adapters, or
runtime execution call sites.

#### Scenario: Resolver precedence matrix is tested
- **WHEN** resolver tests run
- **THEN** they cover turn override, session override, agent config, model default, and provider default cases

#### Scenario: Legacy budget rejection is tested
- **WHEN** config, protocol, API, client DTO, or generation-request construction paths are tested
- **THEN** they reject or do not expose `thinking_budget_tokens` as a supported public request-control input

#### Scenario: Layering contract rejects duplicate resolution
- **WHEN** contract tests inspect request-control plumbing
- **THEN** they fail if `turn_executor` or daemon routes directly recompute effective reasoning effort instead of consuming resolver output
