# provider-request-controls Specification

## Purpose
Define Alan's canonical provider request-control contract. This capability owns
reasoning effort, legacy thinking-budget compatibility, request-control
resolution, validation, provider projection, metadata mirroring, and guardrails
that prevent request-control truth from spreading across runtime, daemon,
clients, and provider adapters.

## Requirements
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

### Requirement: Request control intent is separate from resolved controls
Alan SHALL represent user, session, and turn request-control intent separately
from resolved request controls. `Config` and transport DTOs MAY contain user
intent, but they MUST NOT be the authority for final effective request-control
values.

#### Scenario: Agent config sets effort
- **WHEN** `agent.toml` sets `model_reasoning_effort = "high"`
- **THEN** the resolved runtime config carries reasoning-effort intent for `high`

#### Scenario: Existing budget config remains valid
- **WHEN** `agent.toml` sets `thinking_budget_tokens` and omits `model_reasoning_effort`
- **THEN** Alan preserves the budget as provider-specific compatibility intent

#### Scenario: Explicit effort and budget conflict
- **WHEN** user configuration explicitly sets both `model_reasoning_effort` and `thinking_budget_tokens`
- **THEN** Alan rejects the configuration as ambiguous

#### Scenario: RuntimeConfig does not duplicate independent effective truth
- **WHEN** a workspace overlay changes model metadata or a runtime launch applies a session override
- **THEN** Alan resolves request controls through the resolver
- **AND** `RuntimeConfig` does not require a separate effective reasoning field that can drift from `Config`

### Requirement: Runtime-owned request control resolution
Alan SHALL resolve effective provider request controls through a runtime-owned
resolver before dispatching a model request. The resolver SHALL combine turn
override, session/runtime override, agent config intent, model catalog default,
legacy provider budget intent, provider capabilities, and provider default in a
single typed result.

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
- **WHEN** no explicit reasoning effort or thinking budget is configured and the resolved model catalog entry declares default effort `medium`
- **THEN** the resolver returns reasoning effort `medium` with source `model_default`
- **AND** no other runtime, daemon, or provider-adapter layer recomputes that default independently

#### Scenario: Unknown model metadata uses provider default
- **WHEN** the selected provider/model has no model catalog metadata and no explicit request control is configured
- **THEN** the resolver returns no explicit reasoning effort or budget
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

### Requirement: Generation requests carry normalized controls
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

#### Scenario: Legacy public budget field remains compatible
- **WHEN** a caller constructs or mutates `GenerationRequest.thinking_budget_tokens` directly and canonical budget is unset
- **THEN** provider projection uses the legacy budget as a fallback instead of silently dropping it
- **AND** canonical `reasoning.budget_tokens` still takes precedence when both are set

### Requirement: Provider adapters only project normalized controls
Provider adapters SHALL consume normalized request controls from
`GenerationRequest` and project them to provider-specific payload fields.
Provider adapters SHALL NOT own Alan-level override precedence, model default
selection, or config conflict resolution.

#### Scenario: Canonical effort overrides provider extra params
- **WHEN** a generation request contains normalized reasoning effort `low` and provider-specific extra params include `reasoning_effort = "high"`
- **THEN** the provider adapter sends `low`
- **AND** the provider-specific extra param does not create a competing effective value

#### Scenario: Legacy budget is projected without default inference
- **WHEN** `thinking_budget_tokens` is the only configured compatibility input
- **THEN** provider adapters project that budget according to provider rules without inferring Alan-level defaults

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
- **THEN** Alan maps the budget to an effort using documented compatibility thresholds

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

#### Scenario: OpenRouter budget fallback
- **WHEN** the SDK-backed `openrouter` provider receives only `thinking_budget_tokens`
- **THEN** Alan maps the budget to OpenRouter reasoning request fields supported by the SDK

### Requirement: Daemon and clients mirror resolver metadata
Daemon session metadata, fork metadata, read responses, and client DTOs SHALL
report request-control metadata produced by the runtime resolver. They SHALL
NOT reconstruct reasoning-effort precedence independently.

#### Scenario: Create session response uses resolver output
- **WHEN** a session is created without explicit reasoning effort and the selected model default resolves to `medium`
- **THEN** the create-session response reports `reasoning_effort = "medium"`
- **AND** the value comes from runtime startup metadata

#### Scenario: Fork override uses resolver output
- **WHEN** a fork request explicitly overrides source-session reasoning effort
- **THEN** the forked session metadata reports the newly resolved effort
- **AND** daemon code does not recompute model defaults independently

#### Scenario: Fork without override preserves source intent
- **WHEN** a fork request omits `reasoning_effort` and the source session has an effective reasoning effort
- **THEN** Alan preserves that source setting as the fork session intent before runtime resolution

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

### Requirement: Request control tests guard layer boundaries
Alan SHALL include tests that cover resolver precedence, validation, provider
projection, and daemon metadata mirroring. Tests SHALL make it hard to re-add
effective request-control logic in clients, daemon routes, provider adapters, or
runtime execution call sites.

#### Scenario: Resolver precedence matrix is tested
- **WHEN** resolver tests run
- **THEN** they cover turn override, session override, agent config, model default, legacy budget, and provider default cases

#### Scenario: Layering contract rejects duplicate resolution
- **WHEN** contract tests inspect request-control plumbing
- **THEN** they fail if `turn_executor` or daemon routes directly recompute effective reasoning effort instead of consuming resolver output
