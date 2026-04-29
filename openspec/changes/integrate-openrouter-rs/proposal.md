## Why

Alan currently treats OpenRouter as an OpenAI Chat Completions-compatible
endpoint, even though the product and provider capability docs describe a
dedicated OpenRouter path. `openrouter-rs` has reached a stable enough 0.8.x
surface to replace the thin alias with a first-class adapter that can preserve
OpenRouter-specific streaming, reasoning, tool, and client-identification
semantics.

## What Changes

- Add `openrouter-rs` as the implementation dependency behind Alan's OpenRouter
  provider path.
- Introduce a dedicated OpenRouter LLM adapter in `alan-llm` that maps Alan's
  `GenerationRequest`, `GenerationResponse`, and `StreamChunk` contracts to the
  SDK's domain-oriented chat API.
- Stop constructing OpenRouter providers through the generic
  `OpenAiChatCompletionsClient::openrouter_compatible_with_params(...)` path.
- Add a user-facing `openrouter` provider/profile surface with OpenRouter-owned
  defaults and optional settings such as `http_referer`, `x_title`, and
  `app_categories`.
- Add a one-time migration or repair path for saved connection metadata that
  still names the retired OpenRouter-compatible provider id, without keeping
  that id as a runtime alias.
- Update provider capability declarations so OpenRouter's supported reasoning,
  streaming, tool-call, usage, and response-id behavior is explicit rather than
  inherited from the pessimistic compatible-provider defaults.
- Update configuration, connection-profile, testing, and documentation examples
  so the documented OpenRouter path matches the real implementation.
- Keep the generic `openai_chat_completions_compatible` provider available for
  non-OpenRouter endpoints.

## Capabilities

### New Capabilities

- `openrouter-provider-adapter`: Defines Alan's first-class OpenRouter provider
  behavior, configuration/profile surface, SDK-backed generation and streaming
  mapping, capability matrix, and verification requirements.

### Modified Capabilities

- None. No archived OpenSpec capability currently owns provider adapters or
  connection-profile behavior.

## Impact

- `crates/llm`: new OpenRouter adapter module, `ProviderType` wiring, SDK
  dependency, request/response/stream mapping tests, and live provider harness
  coverage.
- `crates/runtime`: `LlmProvider` enum/profile resolution, config defaults,
  provider capability detection, model validation/catalog behavior, and
  persisted provider-state handling.
- `crates/alan`: `alan connection` parsing, daemon connection catalog/control
  plane, and provider test behavior.
- `docs/` and `AGENTS.md` examples that mention provider setup, provider
  capability tiers, or OpenRouter via adapter.
- Cargo dependency graph and lockfile.
