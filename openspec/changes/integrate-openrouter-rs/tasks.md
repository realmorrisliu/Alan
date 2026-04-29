## 1. SDK Dependency And Provider Skeleton

- [ ] 1.1 Add `openrouter-rs = "0.8.1"` to `alan-llm` dependencies and update the lockfile.
- [ ] 1.2 Add `crates/llm/src/openrouter.rs` and export it from `crates/llm/src/lib.rs`.
- [ ] 1.3 Add `ProviderType::OpenRouter`, `ProviderConfig::openrouter(...)`, and OpenRouter-specific config fields for base URL, model, `http_referer`, `x_title`, and `app_categories`.
- [ ] 1.4 Route `ProviderType::OpenRouter` through the new SDK-backed adapter in the provider factory.
- [ ] 1.5 Remove the retired `OpenRouterOpenAiChatCompletionsCompatible` provider factory type/path and make `openrouter_openai_chat_completions_compatible` fail fast instead of acting as an alias.

## 2. OpenRouter Request Mapping

- [ ] 2.1 Implement Alan message-role conversion to OpenRouter SDK chat messages, including system prompts, context messages, assistant tool calls, and tool-result messages.
- [ ] 2.2 Implement Alan tool-definition conversion to OpenRouter SDK chat tool definitions with automatic tool choice.
- [ ] 2.3 Map temperature, max tokens, and `thinking_budget_tokens` to SDK request fields.
- [ ] 2.4 Add an explicit allowlist for OpenRouter `extra_params` and reject or warn on unsupported keys before dispatch.
- [ ] 2.5 Add unit tests for message, tool, reasoning-budget, and unsupported-extra-parameter projection.

## 3. Non-Streaming Response Mapping

- [ ] 3.1 Implement `generate(...)` using `openrouter-rs` `client.chat().create(...)`.
- [ ] 3.2 Map content, reasoning text, tool calls, usage, finish reason, and provider response id into `GenerationResponse`.
- [ ] 3.3 Handle malformed tool-call JSON by dropping the malformed call and surfacing a provider warning.
- [ ] 3.4 Add unit tests for content-only, reasoning, tool-call, malformed-tool-call, usage, finish-reason, and response-id responses.

## 4. Streaming Response Mapping

- [ ] 4.1 Implement `generate_stream(...)` using an `openrouter-rs` streaming API, preferring `stream_unified` if it exposes the required fields.
- [ ] 4.2 Convert SDK content events to `StreamChunk.text` deltas in order.
- [ ] 4.3 Convert SDK reasoning events to `StreamChunk.thinking` deltas in order.
- [ ] 4.4 Convert streamed tool-call events to `StreamChunk.tool_call_delta` values that preserve id, name, index, and argument deltas.
- [ ] 4.5 Emit a final chunk with `is_finished = true`, finish reason, usage when available, and provider response id when available.
- [ ] 4.6 Propagate SDK stream errors through the provider stream channel so runtime partial-stream recovery can run.
- [ ] 4.7 Add streaming unit tests for text, reasoning, fragmented tool calls, completion metadata, and stream errors after partial output.

## 5. Runtime And Connection Profiles

- [ ] 5.1 Add `LlmProvider::OpenRouter` serialized as `openrouter` and update `as_str()`, config defaults, reset/merge behavior, effective model lookup, and `to_provider_config()`.
- [ ] 5.2 Add OpenRouter resolved config fields to `Config` without documenting new inline `agent.toml` provider examples.
- [ ] 5.3 Add an OpenRouter `ProviderDescriptor` with secret-string credentials, required `base_url` and `model`, optional `http_referer`, `x_title`, and `app_categories`, and the OpenRouter base URL default.
- [ ] 5.4 Update `apply_resolved_profile_to_config(...)` to load the OpenRouter secret and resolved settings into runtime config.
- [ ] 5.5 Add a migration or repair routine for saved connection metadata that rewrites retired `profiles.<id>.provider` and `credentials.<id>.provider_family` values to `openrouter` while preserving profile ids, credential ids, secrets, `base_url`, and `model`.
- [ ] 5.6 Keep the retired OpenRouter-compatible id recognizable only at the connection-file diagnostic/migration boundary; exclude it from provider descriptors, resolved runtime state, session metadata, and provider factory dispatch.
- [ ] 5.7 Update persisted provider enums and runtime provider detection so only `openrouter` is accepted for OpenRouter state after the migration boundary.
- [ ] 5.8 Add runtime tests for profile validation, profile application, effective model handling, provider detection, old-id migration, old-id rejection after migration, and old-id appearances in both profile providers and credential provider families.

## 6. CLI, Daemon, And Catalog Surfaces

- [ ] 6.1 Update `alan connection` provider parsing so `openrouter` can be added, listed, tested, pinned, and set as default.
- [ ] 6.2 Update daemon connection control/routes so `/api/v1/connections/catalog` exposes OpenRouter metadata and connection profile operations accept the provider.
- [ ] 6.3 Update provider test behavior to exercise the SDK-backed OpenRouter provider when an OpenRouter profile is tested.
- [ ] 6.4 Add focused CLI and daemon tests for OpenRouter provider parsing, catalog output, profile creation, and profile resolution.

## 7. Provider Capabilities And Documentation

- [ ] 7.1 Declare explicit `ProviderCapabilities` for OpenRouter rather than sharing the generic compatible-provider branch.
- [ ] 7.2 Update `docs/spec/provider_capability_contract.md` so OpenRouter is documented as `openrouter` with a first-class SDK-backed adapter and compatibility-tier semantics.
- [ ] 7.3 Update provider setup documentation and examples to use `alan connection add openrouter ...` with OpenRouter-specific optional settings.
- [ ] 7.4 Search docs and code comments for stale "OpenRouter via adapter" or `openrouter_openai_chat_completions_compatible` public wording and update it where appropriate.

## 8. Live Harness And Verification

- [ ] 8.1 Extend the live provider harness with an OpenRouter case gated by `ALAN_LIVE_OPENROUTER_API_KEY`, `ALAN_LIVE_OPENROUTER_MODEL`, and optional OpenRouter metadata env vars.
- [ ] 8.2 Add harness coverage for OpenRouter non-streaming generation, streaming generation, reasoning when the configured model supports it, and tool calls when the configured model supports them.
- [ ] 8.3 Run `cargo fmt --all`.
- [ ] 8.4 Run focused `cargo test -p alan-llm` tests for OpenRouter adapter mapping and factory behavior.
- [ ] 8.5 Run focused `cargo test -p alan-runtime` and `cargo test -p alan` tests for provider config, connection profiles, CLI parsing, and daemon catalog behavior.
- [ ] 8.6 Run the OpenRouter live provider harness when credentials are available; otherwise document that live verification was skipped.
- [ ] 8.7 Run `openspec status --change integrate-openrouter-rs` and ensure the change is apply-ready.
