## 1. SDK Dependency And Provider Skeleton

- [x] 1.1 Add `openrouter-rs = "0.8.1"` to `alan-llm` dependencies and update the lockfile.
- [x] 1.2 Add `crates/llm/src/openrouter.rs` and export it from `crates/llm/src/lib.rs`.
- [x] 1.3 Add `ProviderType::OpenRouter`, `ProviderConfig::openrouter(...)`, and OpenRouter-specific config fields for base URL, model, `http_referer`, `x_title`, and `app_categories`.
- [x] 1.4 Route `ProviderType::OpenRouter` through the new SDK-backed adapter in the provider factory.
- [x] 1.5 Remove the retired `OpenRouterOpenAiChatCompletionsCompatible` provider factory type/path and make `openrouter_openai_chat_completions_compatible` fail fast instead of acting as an alias.

## 2. OpenRouter Request Mapping

- [x] 2.1 Implement Alan message-role conversion to OpenRouter SDK chat messages, including system prompts, context messages, assistant tool calls, and tool-result messages.
- [x] 2.2 Implement Alan tool-definition conversion to OpenRouter SDK chat tool definitions with automatic tool choice.
- [x] 2.3 Map temperature, max tokens, and `thinking_budget_tokens` to SDK request fields.
- [x] 2.4 Add an explicit allowlist for OpenRouter `extra_params` and reject or warn on unsupported keys before dispatch.
- [x] 2.5 Add unit tests for message, tool, reasoning-budget, and unsupported-extra-parameter projection.

## 3. Non-Streaming Response Mapping

- [x] 3.1 Implement `generate(...)` using `openrouter-rs` `client.chat().create(...)`.
- [x] 3.2 Map content, reasoning text, tool calls, usage, finish reason, and provider response id into `GenerationResponse`.
- [x] 3.3 Handle malformed tool-call JSON by dropping the malformed call and surfacing a provider warning.
- [x] 3.4 Add unit tests for content-only, reasoning, tool-call, malformed-tool-call, usage, finish-reason, and response-id responses.

## 4. Streaming Response Mapping

- [x] 4.1 Implement `generate_stream(...)` using an `openrouter-rs` streaming API, preferring `stream_unified` if it exposes the required fields.
- [x] 4.2 Convert SDK content events to `StreamChunk.text` deltas in order.
- [x] 4.3 Convert SDK reasoning events to `StreamChunk.thinking` deltas in order.
- [x] 4.4 Convert streamed tool-call events to `StreamChunk.tool_call_delta` values that preserve id, name, index, and argument deltas.
- [x] 4.5 Emit a final chunk with `is_finished = true`, finish reason, usage when available, and provider response id when available.
- [x] 4.6 Propagate SDK stream errors through the provider stream channel so runtime partial-stream recovery can run.
- [x] 4.7 Add streaming unit tests for text, reasoning, fragmented tool calls, completion metadata, and stream errors after partial output.

## 5. Runtime And Connection Profiles

- [x] 5.1 Add `LlmProvider::OpenRouter` serialized as `openrouter` and update `as_str()`, config defaults, reset/merge behavior, effective model lookup, and `to_provider_config()`.
- [x] 5.2 Add OpenRouter resolved config fields to `Config` without documenting new inline `agent.toml` provider examples.
- [x] 5.3 Add an OpenRouter `ProviderDescriptor` with secret-string credentials, required `model`, optional `base_url`, `http_referer`, `x_title`, and `app_categories`, and the OpenRouter base URL default.
- [x] 5.4 Update `apply_resolved_profile_to_config(...)` to load the OpenRouter secret and resolved settings into runtime config.
- [x] 5.5 Remove the retired OpenRouter-compatible provider id from supported connection metadata and persisted provider state; do not add automatic rewrite, repair, or alias behavior for `profiles.<id>.provider` or `credentials.<id>.provider_family`.
- [x] 5.6 Exclude the retired id from provider descriptors, resolved runtime state, session metadata, CLI/daemon provider parsing, and provider factory dispatch.
- [x] 5.7 Add runtime tests for profile validation, profile application, effective model handling, provider detection, old-id rejection, and old-id appearances in both profile providers and credential provider families.

## 6. CLI, Daemon, And Catalog Surfaces

- [x] 6.1 Update `alan connection` provider parsing so `openrouter` can be added, listed, tested, pinned, and set as default.
- [x] 6.2 Update daemon connection control/routes so `/api/v1/connections/catalog` exposes OpenRouter metadata and connection profile operations accept the provider.
- [x] 6.3 Update provider test behavior to exercise the SDK-backed OpenRouter provider when an OpenRouter profile is tested.
- [x] 6.4 Add focused CLI and daemon tests for OpenRouter provider parsing, catalog output, profile creation, and profile resolution.

## 7. Provider Capabilities And Documentation

- [x] 7.1 Declare explicit `ProviderCapabilities` for OpenRouter rather than sharing the generic compatible-provider branch.
- [x] 7.2 Update `docs/spec/provider_capability_contract.md` so OpenRouter is documented as `openrouter` with a first-class SDK-backed adapter and compatibility-tier semantics.
- [x] 7.3 Update provider setup documentation and examples to use `alan connection add openrouter ...` with OpenRouter-specific optional settings.
- [x] 7.4 Search docs and code comments for stale "OpenRouter via adapter" or `openrouter_openai_chat_completions_compatible` public wording and update it where appropriate.

## 8. Live Harness And Verification

- [x] 8.1 Extend the live provider harness with an OpenRouter case gated by `ALAN_LIVE_OPENROUTER_API_KEY`, `ALAN_LIVE_OPENROUTER_MODEL`, and optional OpenRouter metadata env vars.
- [x] 8.2 Add harness coverage for OpenRouter non-streaming generation, streaming generation, reasoning when the configured model supports it, and tool calls when the configured model supports them.
- [x] 8.3 Run `cargo fmt --all`.
- [x] 8.4 Run focused `cargo test -p alan-llm` tests for OpenRouter adapter mapping and factory behavior.
- [x] 8.5 Run focused `cargo test -p alan-runtime` and `cargo test -p alan` tests for provider config, connection profiles, CLI parsing, and daemon catalog behavior.
- [x] 8.6 Run the OpenRouter live provider harness when credentials are available; otherwise document that live verification was skipped.
- [x] 8.7 Run `openspec status --change integrate-openrouter-rs` and ensure the change is apply-ready.
