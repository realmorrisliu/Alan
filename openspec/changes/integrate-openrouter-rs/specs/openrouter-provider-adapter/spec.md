## ADDED Requirements

### Requirement: OpenRouter provider identity
Alan SHALL expose OpenRouter as a first-class provider id named `openrouter` in
runtime configuration, connection profiles, daemon connection catalogs, and CLI
connection commands.

#### Scenario: Creating an OpenRouter profile
- **WHEN** an operator runs `alan connection add openrouter --profile openrouter-main --setting model=<model-id>`
- **THEN** Alan creates a secret-backed connection profile whose provider is `openrouter`

#### Scenario: Resolving an OpenRouter profile
- **WHEN** a session starts with `connection_profile = "openrouter-main"`
- **THEN** Alan resolves the runtime provider to OpenRouter rather than `openai_chat_completions_compatible`

#### Scenario: Rejecting the retired OpenRouter-compatible id
- **WHEN** Alan encounters a provider id named `openrouter_openai_chat_completions_compatible`
- **THEN** Alan rejects it instead of treating it as an OpenRouter alias

### Requirement: OpenRouter connection settings
Alan SHALL keep OpenRouter-specific settings separate from generic
OpenAI-compatible settings.

#### Scenario: Profile descriptor settings
- **WHEN** the daemon or CLI lists provider descriptors
- **THEN** the OpenRouter descriptor includes `base_url` and `model` as required settings and `http_referer`, `x_title`, and `app_categories` as optional settings

#### Scenario: Default base URL
- **WHEN** an OpenRouter profile omits `base_url`
- **THEN** Alan uses `https://openrouter.ai/api/v1` as the resolved OpenRouter base URL

#### Scenario: Unknown OpenRouter setting
- **WHEN** an OpenRouter profile includes a setting that is not declared by the OpenRouter descriptor
- **THEN** Alan rejects the profile with a provider-setting validation error

#### Scenario: Generic compatible settings remain isolated
- **WHEN** an operator configures `openai_chat_completions_compatible`
- **THEN** OpenRouter-only settings such as `http_referer`, `x_title`, and `app_categories` are not accepted by the generic compatible provider

### Requirement: SDK-backed provider construction
Alan SHALL construct OpenRouter providers through `openrouter-rs` and SHALL NOT
route OpenRouter generation through Alan's generic OpenAI Chat Completions
compatible client.

#### Scenario: Factory creates OpenRouter SDK adapter
- **WHEN** `ProviderConfig::openrouter(...)` is passed to the LLM provider factory
- **THEN** the factory returns an OpenRouter SDK-backed provider whose `provider_name()` is `openrouter`

#### Scenario: Retired OpenRouter-compatible factory path
- **WHEN** code or configuration tries to construct `openrouter_openai_chat_completions_compatible`
- **THEN** Alan fails fast because the retired compatible path is no longer supported

#### Scenario: Non-streaming dispatch uses SDK
- **WHEN** the OpenRouter provider executes a non-streaming generation request
- **THEN** the provider dispatches through the `openrouter-rs` OpenRouter client chat API

#### Scenario: Streaming dispatch uses SDK
- **WHEN** the OpenRouter provider executes a streaming generation request
- **THEN** the provider dispatches through an `openrouter-rs` streaming API and converts SDK stream events into Alan `StreamChunk` values

### Requirement: OpenRouter request projection
Alan SHALL map Alan generation requests to OpenRouter SDK chat requests without
requiring runtime code to depend on OpenRouter SDK types.

#### Scenario: Basic message projection
- **WHEN** a request contains a system prompt and user, assistant, context, and tool messages
- **THEN** the OpenRouter adapter maps them to the SDK chat message roles and content fields expected by OpenRouter

#### Scenario: Tool definition projection
- **WHEN** a request contains Alan tool definitions
- **THEN** the OpenRouter adapter maps them to OpenRouter chat tool definitions and enables automatic tool choice behavior

#### Scenario: Tool result projection
- **WHEN** a request contains a tool-result message with a tool call id
- **THEN** the OpenRouter adapter preserves the tool call id in the projected OpenRouter message

#### Scenario: Reasoning budget projection
- **WHEN** a request contains `thinking_budget_tokens`
- **THEN** the OpenRouter adapter maps the budget to OpenRouter reasoning request fields supported by the SDK

#### Scenario: Unsupported provider extra parameter
- **WHEN** a request contains an OpenRouter `extra_params` key that the adapter does not support
- **THEN** the adapter fails before dispatching the request or returns an explicit provider warning rather than silently dropping the parameter

### Requirement: OpenRouter response normalization
Alan SHALL normalize OpenRouter SDK responses into Alan's provider-agnostic
response types.

#### Scenario: Non-streaming content and reasoning
- **WHEN** OpenRouter returns final assistant content and reasoning text
- **THEN** Alan returns a `GenerationResponse` with `content` and `thinking` populated

#### Scenario: Non-streaming tool call
- **WHEN** OpenRouter returns model-issued tool calls with JSON arguments
- **THEN** Alan returns `GenerationResponse.tool_calls` with the tool id, name, and parsed arguments

#### Scenario: Malformed non-streaming tool arguments
- **WHEN** OpenRouter returns a tool call whose arguments are not valid JSON
- **THEN** Alan does not execute the malformed tool call and surfaces a provider warning

#### Scenario: Usage and finish reason
- **WHEN** OpenRouter returns token usage and a finish reason
- **THEN** Alan preserves both values in `GenerationResponse`

#### Scenario: Provider response id
- **WHEN** OpenRouter returns a provider-native response id
- **THEN** Alan stores that id in `provider_response_id`

### Requirement: OpenRouter stream normalization
Alan SHALL normalize OpenRouter SDK streaming events into Alan `StreamChunk`
values and emit a terminal chunk.

#### Scenario: Streaming text delta
- **WHEN** OpenRouter streams assistant content
- **THEN** Alan emits `StreamChunk.text` deltas in provider order

#### Scenario: Streaming reasoning delta
- **WHEN** OpenRouter streams reasoning content
- **THEN** Alan emits `StreamChunk.thinking` deltas in provider order

#### Scenario: Streaming tool-call delta
- **WHEN** OpenRouter streams a model-issued tool call over multiple events
- **THEN** Alan emits `StreamChunk.tool_call_delta` values that allow the runtime to assemble the final tool call

#### Scenario: Streaming completion metadata
- **WHEN** the OpenRouter stream reaches completion
- **THEN** Alan emits a final chunk with `is_finished = true`, finish reason, usage when available, and provider response id when available

#### Scenario: Streaming error after partial output
- **WHEN** the OpenRouter SDK stream returns an error after partial output
- **THEN** Alan propagates the stream failure through the provider stream channel so runtime partial-stream recovery policy can handle it

### Requirement: OpenRouter capability matrix
Alan SHALL declare OpenRouter capabilities explicitly instead of inheriting the
generic OpenAI-compatible capability matrix.

#### Scenario: Capability query
- **WHEN** runtime code queries the OpenRouter provider capabilities
- **THEN** the returned matrix reflects OpenRouter adapter support for streaming text, streaming tool calls, reasoning text, token usage, and provider response ids

#### Scenario: Unsupported stateful capabilities
- **WHEN** runtime code queries OpenRouter provider capabilities
- **THEN** the returned matrix does not claim server-managed continuation, background execution, retrieve/cancel, provider compaction, or provider status unless those behaviors are implemented for OpenRouter

#### Scenario: Capability tier
- **WHEN** provider capability documentation describes OpenRouter
- **THEN** OpenRouter remains documented as a compatibility-tier provider with a first-class SDK-backed adapter

### Requirement: Generic compatible provider preservation
Alan SHALL preserve the existing generic OpenAI Chat Completions-compatible
provider for non-OpenRouter endpoints.

#### Scenario: Generic compatible factory path
- **WHEN** `ProviderConfig::openai_chat_completions_compatible(...)` is passed to the provider factory
- **THEN** Alan still constructs the generic OpenAI-compatible provider path

#### Scenario: OpenRouter does not alter compatible defaults
- **WHEN** a generic compatible profile omits optional OpenRouter metadata settings
- **THEN** Alan resolves the profile exactly as it did before this change

### Requirement: Verification and documentation
Alan SHALL include focused automated coverage and documentation for the
OpenRouter SDK-backed provider path.

#### Scenario: Unit and integration coverage
- **WHEN** the change is implemented
- **THEN** tests cover provider factory routing, profile resolution, CLI parser behavior, request mapping, response mapping, stream mapping, and capability declarations

#### Scenario: Live provider harness
- **WHEN** OpenRouter live-test credentials are available
- **THEN** the live provider harness can validate OpenRouter non-streaming and streaming behavior through the SDK-backed adapter

#### Scenario: Documentation examples
- **WHEN** docs show OpenRouter setup
- **THEN** they use the `openrouter` provider id and OpenRouter-specific connection settings rather than the generic compatible provider
