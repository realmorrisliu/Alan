## Context

Issue 77 asks Alan to replace the current OpenRouter adapter with
`openrouter-rs`. The current `alan-llm` implementation has an internal
`ProviderType::OpenRouterOpenAiChatCompletionsCompatible`, but factory creation
still routes it through `OpenAiChatCompletionsClient::openrouter_compatible_with_params(...)`.
The runtime-facing `LlmProvider` enum, connection catalog, and CLI parser do not
yet expose a first-class `openrouter` provider id.

`openrouter-rs` 0.8.x is now a better fit for this boundary: it exposes
`OpenRouterClient::builder()`, domain clients such as `chat()`, streaming and
unified streaming methods, reasoning support, typed/manual tools, multimodal
content, and OpenRouter-specific client metadata (`http_referer`, `x_title`,
`app_categories`). The SDK intentionally leaves profile/config resolution to
the application, which matches Alan's existing connection-profile model.

The implementation must preserve Alan's provider-agnostic kernel contract:
runtime code should continue to speak `GenerationRequest`, `GenerationResponse`,
and `StreamChunk`, while the provider adapter owns OpenRouter-specific request
projection and response normalization.

## Goals / Non-Goals

**Goals:**

- Make `openrouter` a user-facing provider id in connection profiles and runtime
  resolved state.
- Hard-remove the retired OpenRouter-compatible provider id and route because it
  was not a supported user-facing configuration and has no known users.
- Back OpenRouter generation and streaming with `openrouter-rs`, not Alan's
  generic OpenAI-compatible HTTP client.
- Preserve Alan's common core semantics for text, reasoning, tool calls, tool
  results, finish reasons, usage, provider response ids, and stream completion.
- Keep OpenRouter-specific settings separate from generic
  `openai_chat_completions_compatible` settings.
- Add tests that prove factory routing, request mapping, response mapping,
  streaming aggregation, profile resolution, and CLI/daemon catalog behavior.
- Update provider docs so OpenRouter is documented as a dedicated provider path
  with an SDK-backed implementation.

**Non-Goals:**

- Do not adopt the `openrouter-rs` CLI/profile configuration system. Alan
  remains responsible for connection profiles and secrets.
- Do not expose OpenRouter management, model discovery, TTS, video, rerank, or
  embeddings through Alan's LLM provider trait in this change.
- Do not migrate Alan's core provider abstraction to the OpenRouter Responses or
  Messages endpoints.
- Do not remove the generic `openai_chat_completions_compatible` provider.
- Do not migrate, repair, or alias
  `openrouter_openai_chat_completions_compatible`.
- Do not guarantee that every OpenRouter model supports every provider-level
  capability; model-specific limitations remain upstream behavior.

## Decisions

### Decision: Add a dedicated `OpenRouterClient` adapter module

Create a new `crates/llm/src/openrouter.rs` module that implements
`LlmProvider` for an Alan-owned wrapper around `openrouter_rs::OpenRouterClient`.
The wrapper should own the resolved model and client settings, expose
`provider_name() == "openrouter"`, and be constructed by
`ProviderType::OpenRouter`.

Rationale: A separate module gives Alan a clear OpenRouter boundary and avoids
spreading SDK-specific types through the existing OpenAI-compatible adapter.

Alternative considered: keep the existing OpenAI-compatible adapter and add
headers/options. That would still make OpenRouter behavior depend on generic
request/stream parsing and would not use the SDK surfaces issue 77 calls out.

### Decision: Use `openrouter-rs` chat domain APIs for V1

Map Alan requests to `openrouter-rs` chat-completion requests and call
`client.chat().create(...)` for non-streaming generation. Streaming should use
the SDK streaming surface, preferring `stream_unified` if it carries the needed
reasoning/tool/completion events cleanly; otherwise use the chat stream and
adapt its chunks directly.

Rationale: Alan's current provider trait is closest to chat-completion semantics
and already models message history plus tool calls. The OpenRouter Responses and
Messages endpoints are useful SDK capabilities, but adopting them would require
new runtime semantics that are outside this issue.

Alternative considered: use OpenRouter Responses for all models. That would
increase the semantic gap against Alan's current message/tool projection and is
not needed to replace the thin adapter.

### Decision: Make `openrouter` the only OpenRouter provider id

Add `LlmProvider::OpenRouter` serialized as `openrouter`, add
`ProviderType::OpenRouter`, and add an OpenRouter descriptor to the connection
catalog. Remove the old
`openrouter_openai_chat_completions_compatible` provider type/path instead of
keeping it as a compatibility alias or adding a migration. New runtime state,
connection profiles, docs, tests, and provider-name detection should use
`openrouter` only. The retired id should be absent from provider descriptors,
`alan connection add`, daemon catalogs, runtime resolved provider state,
persisted provider state, and LLM factory dispatch.

If an unsupported local `connections.toml` still contains the retired id, Alan
may fail to load that file as unsupported configuration. That is acceptable for
this change because the old id was an implementation-detail path rather than a
supported user-facing provider, and there are no known users to migrate.
Implementors may add a targeted diagnostic that names `openrouter` as the
replacement when doing so is simple, but they should not add automatic rewrite or
alias behavior.

Rationale: The old id describes an implementation detail instead of the product
provider. Keeping it as an alias would preserve the compatible-path concept this
change is intentionally retiring.

Alternative considered: preserve the long provider id. That avoids small enum
changes but keeps the public contract tied to the adapter Alan is removing.

### Decision: Keep OpenRouter settings in connection profiles

The OpenRouter profile descriptor should use secret-string credentials and
settings owned by OpenRouter:

- required: `model`
- optional: `base_url`, `http_referer`, `x_title`, `app_categories`
- default `base_url`: `https://openrouter.ai/api/v1`

The model setting should remain explicit in the profile or command line. If a
default model is added later, it should be a curated Alan model-catalog decision,
not a hidden SDK default.

Rationale: Alan's current direction puts provider setup in
`~/.alan/connections.toml` plus the secret store. OpenRouter's app-identifying
headers are provider-specific and should not pollute the generic compatible
provider.

Alternative considered: add more `agent.toml` inline provider fields. That
would expand a compatibility surface the project is already moving away from.

### Decision: Use explicit mapping helpers and an allowlist for extra params

The adapter should convert Alan messages, tool definitions, tool results,
temperature, max tokens, and thinking budget into SDK request fields through
OpenRouter-owned mapping helpers. OpenRouter-specific `extra_params` should be
handled through an allowlist of SDK-supported request fields. Unsupported
`extra_params` must not be silently dropped; the adapter should either return a
clear error before dispatch or surface an explicit warning where the runtime
already supports provider warnings.

Rationale: The generic OpenAI-compatible path can forward arbitrary JSON, but a
typed SDK adapter should make the supported contract deliberate. Silent loss of
request knobs would be harder to debug than an explicit unsupported-parameter
failure.

Alternative considered: serialize through raw JSON and bypass typed SDK fields
for maximum compatibility. That would undercut the reason to adopt the SDK and
make tests less valuable.

### Decision: Declare OpenRouter capabilities explicitly, but keep it in Tier C

OpenRouter should no longer inherit the generic compatible provider capability
matrix. It should declare supported API-surface behavior explicitly, including
streaming text, streaming tool calls, reasoning text when exposed by the chosen
model, multimodal input when the request projection supports it, token usage,
and provider response ids. It should not claim server-managed continuation,
background execution, retrieve/cancel, provider compaction, or provider status
unless Alan maps those surfaces through the provider trait.

OpenRouter should remain a Tier C compatibility provider in the provider
capability contract because it routes to multiple upstream model families and
does not provide one lossless cross-vendor semantic contract.

Rationale: "First-class adapter" and "full-fidelity provider semantics" are
different things. The SDK-backed path is first-class in Alan's code, while the
provider capability tier remains conservative.

Alternative considered: promote OpenRouter to Tier B. That would overstate the
uniformity of model behavior behind the aggregator.

## Risks / Trade-offs

- `openrouter-rs` 0.8.1 currently depends on a different `reqwest` line than
  Alan's workspace dependency -> Keep SDK transport types behind the adapter
  boundary and accept duplicate transitive dependencies unless implementation
  shows measurable build/runtime cost.
- SDK typed request structs may not expose every OpenRouter request knob Alan
  users expect -> Start with a tested allowlist and add new mappings as concrete
  use cases appear.
- OpenRouter model behavior varies by upstream provider -> Document that the
  capability matrix describes the API/adapter surface, not a guarantee for every
  model.
- Streaming tool-call deltas can arrive fragmented or out of order -> Reuse the
  existing Alan stream aggregation rules where possible and add adapter-specific
  tests for argument assembly, malformed JSON, and final chunks.
- Provider-id hard cutover can break local unsupported profile or persisted-state
  fixtures that still name `openrouter_openai_chat_completions_compatible` ->
  Update or remove those fixtures as part of the change and fail fast on the old
  id rather than silently treating it as OpenRouter.

## Migration Plan

1. Add the `openrouter-rs` dependency to `alan-llm` and wire the new module into
   `lib.rs`.
2. Add `ProviderType::OpenRouter`, the `ProviderConfig::openrouter(...)`
   constructor, factory creation through the SDK adapter, provider-name
   detection, and explicit capabilities.
3. Add `LlmProvider::OpenRouter` to runtime config, connection profiles,
   persisted provider state, daemon session metadata, and CLI/daemon connection
   parsing, and remove the old OpenRouter-compatible provider id/path without a
   migration alias.
4. Remove the old OpenRouter-compatible provider factory type/path, and make any
   remaining attempts to construct it fail fast instead of acting as an alias.
5. Add OpenRouter resolved config fields for API key, base URL, model,
   `http_referer`, `x_title`, and `app_categories`; keep them internal resolved
   state rather than new user-facing `agent.toml` examples.
6. Implement non-streaming request/response mapping and focused unit tests.
7. Implement streaming mapping and tests for content deltas, reasoning deltas,
   tool-call deltas, usage, finish reason, response id, and stream errors.
8. Add or extend live provider harness support gated by
   `ALAN_LIVE_OPENROUTER_API_KEY`, `ALAN_LIVE_OPENROUTER_MODEL`, and optional
   OpenRouter metadata env vars.
9. Update docs and provider capability text from "OpenRouter via adapter" to the
   new SDK-backed provider path.
10. Run `cargo fmt --all`, focused `cargo test -p alan-llm` filters, focused
   `cargo test -p alan-runtime` / `cargo test -p alan` filters for connection
   profiles, and the live harness only when credentials are available.

Rollback is straightforward before release: remove the public `openrouter`
descriptor and restore factory routing to the existing compatible client. After
release, keep the `openrouter` provider id and revert only the SDK-backed
adapter internals if an SDK regression appears.

## Open Questions

- Should Alan ship a curated default OpenRouter model, or require every
  OpenRouter profile to set `model` explicitly?
- Does `stream_unified` expose all content, reasoning, tool-call, usage, finish,
  and response-id fields Alan needs, or should V1 use `chat().stream(...)`
  directly?
- Which OpenRouter-specific request knobs should be in the first
  `extra_params` allowlist beyond reasoning, provider preferences, transforms,
  route, and response format?
- Should the model catalog gain an `openrouter` provider section now, or should
  OpenRouter model validation initially accept any non-empty model id because
  OpenRouter's catalog changes frequently?
