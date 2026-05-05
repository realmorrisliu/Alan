## ADDED Requirements

### Requirement: Canonical reasoning effort type
Alan SHALL define a shared typed reasoning effort model with lowercase
serialization values `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`.

#### Scenario: Parsing valid effort values
- **WHEN** config, protocol, or API payloads contain `none`, `minimal`, `low`, `medium`, `high`, or `xhigh`
- **THEN** Alan parses the value into the canonical reasoning effort enum

#### Scenario: Rejecting invalid effort values
- **WHEN** config, protocol, or API payloads contain an unknown reasoning effort string
- **THEN** Alan rejects the value with an error that names the supported values

#### Scenario: Distinguishing unset from none
- **WHEN** reasoning effort is omitted
- **THEN** Alan treats the effort as unset rather than as `none`
- **AND** `none` remains an explicit request to disable reasoning where the model supports it

### Requirement: Reasoning-capable model metadata
Alan SHALL declare model-level supported and default reasoning efforts in the
model catalog.

#### Scenario: Model catalog entry declares efforts
- **WHEN** a bundled or overlay model entry supports reasoning
- **THEN** the entry can declare `supported_reasoning_efforts` and `default_reasoning_effort`

#### Scenario: Default effort must be supported
- **WHEN** a model entry declares `default_reasoning_effort`
- **THEN** Alan validates that the default appears in `supported_reasoning_efforts`

#### Scenario: Existing supports_reasoning compatibility
- **WHEN** an existing catalog entry only declares `supports_reasoning = true`
- **THEN** Alan derives a conservative supported/default effort set or requires the entry to be migrated before validation passes

#### Scenario: Client-visible model metadata
- **WHEN** daemon or client-facing model metadata is exposed
- **THEN** it includes supported reasoning efforts and the default reasoning effort for each listed model

### Requirement: Reasoning effort configuration
Alan SHALL support explicit reasoning effort in resolved runtime configuration
and SHALL keep legacy token budgets as provider-specific compatibility controls.

#### Scenario: Agent config sets effort
- **WHEN** `agent.toml` sets `model_reasoning_effort = "high"`
- **THEN** the resolved runtime config carries `ReasoningEffort::High`

#### Scenario: Existing budget config remains valid
- **WHEN** `agent.toml` sets `thinking_budget_tokens` and omits `model_reasoning_effort`
- **THEN** Alan preserves the budget for provider-specific mapping

#### Scenario: Explicit effort and budget conflict
- **WHEN** user configuration explicitly sets both `model_reasoning_effort` and `thinking_budget_tokens`
- **THEN** Alan rejects the configuration as ambiguous

#### Scenario: Effective effort defaults from model catalog
- **WHEN** no explicit effort or budget is configured
- **THEN** Alan uses the resolved model's catalog default reasoning effort when present

### Requirement: Runtime override precedence
Alan SHALL resolve effective reasoning effort using explicit runtime override
precedence before provider dispatch.

#### Scenario: Session override wins over agent config
- **WHEN** a session is created with a reasoning effort override and the resolved agent config has a different effort
- **THEN** Alan uses the session override for that runtime

#### Scenario: Turn override wins over session override
- **WHEN** a turn includes a reasoning effort override and the session has a different effective effort
- **THEN** Alan uses the turn override for that turn

#### Scenario: Child agent override
- **WHEN** a child-agent spawn spec includes a reasoning effort runtime override
- **THEN** Alan applies that effort to the child runtime after validating it against the child model

#### Scenario: Unsupported override
- **WHEN** a runtime override selects an effort unsupported by the resolved model
- **THEN** Alan rejects the override before making a provider request

### Requirement: Generation request reasoning controls
Alan SHALL carry canonical reasoning controls on `GenerationRequest` rather than
requiring provider adapters to infer them from ad hoc `extra_params`.

#### Scenario: Turn request includes resolved effort
- **WHEN** runtime constructs a generation request for a reasoning-capable model
- **THEN** the request includes the validated effective reasoning effort

#### Scenario: Legacy budget request
- **WHEN** runtime constructs a generation request with no effective effort but with `thinking_budget_tokens`
- **THEN** the request includes the budget as provider-specific reasoning control

#### Scenario: No reasoning control
- **WHEN** neither effort, budget, nor model default applies
- **THEN** the request omits reasoning controls and lets the provider use its default behavior

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

#### Scenario: Legacy OpenAI budget mapping
- **WHEN** an OpenAI provider receives only `thinking_budget_tokens`
- **THEN** Alan maps the budget to an effort using documented compatibility thresholds and emits or records a migration warning

### Requirement: Anthropic provider mapping
Alan SHALL map canonical reasoning effort to Anthropic extended-thinking budget
configuration when the selected Anthropic model supports extended thinking.

#### Scenario: Anthropic effort maps to budget
- **WHEN** `anthropic_messages` receives a generation request with effective effort
- **THEN** Alan maps the effort to the configured Anthropic `thinking.budget_tokens` preset for the selected model

#### Scenario: Anthropic explicit budget
- **WHEN** `anthropic_messages` receives only `thinking_budget_tokens`
- **THEN** Alan sends the explicit budget through Anthropic `thinking.budget_tokens`

#### Scenario: Anthropic minimum budget
- **WHEN** the mapped or explicit Anthropic budget is below the provider minimum
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

#### Scenario: Gemini explicit budget
- **WHEN** a Gemini 2.5 model receives only `thinking_budget_tokens`
- **THEN** Alan sends the budget through `thinkingConfig.thinkingBudget`

### Requirement: Compatible-provider mapping
Alan SHALL only send reasoning-effort extension fields to compatibility
providers when the provider/model explicitly declares support.

#### Scenario: Compatible provider supports effort extension
- **WHEN** `openai_chat_completions_compatible` receives effective effort for a model that declares `reasoning_effort` support
- **THEN** Alan sends the compatible extension field

#### Scenario: Compatible provider does not support effort extension
- **WHEN** a compatibility provider/model lacks declared effort support
- **THEN** Alan rejects explicit reasoning effort rather than silently dropping it

#### Scenario: OpenRouter SDK-backed provider
- **WHEN** the SDK-backed `openrouter` provider is available and receives effective effort
- **THEN** Alan maps the effort to the OpenRouter SDK/provider-native reasoning field supported by the selected endpoint and model

### Requirement: Reasoning effort observability
Alan SHALL expose effective reasoning effort in runtime/session metadata and
testable request traces.

#### Scenario: Session metadata includes effort
- **WHEN** a session is created or listed through the daemon API
- **THEN** the response includes the effective reasoning effort when one is resolved

#### Scenario: Request log includes effort
- **WHEN** runtime logs or records provider request metadata
- **THEN** it includes the effective reasoning effort without exposing hidden reasoning content

#### Scenario: Rollout persistence
- **WHEN** a turn is persisted to rollout metadata
- **THEN** Alan records the effective reasoning effort used for that turn when available

### Requirement: Documentation and migration
Alan SHALL document effort-first reasoning controls and the remaining
provider-specific budget escape hatch.

#### Scenario: Agent config example
- **WHEN** documentation shows reasoning configuration
- **THEN** it uses `model_reasoning_effort = "medium"` as the primary example

#### Scenario: Budget documentation
- **WHEN** documentation mentions `thinking_budget_tokens`
- **THEN** it describes the field as provider-specific compatibility control rather than Alan's canonical reasoning control

#### Scenario: Migration from budget to effort
- **WHEN** users have existing `thinking_budget_tokens` config
- **THEN** documentation explains how to replace it with a named reasoning effort when the selected provider/model supports effort
