# Provider Capability Contract

> Status: proposed V1 target contract.
>
> Scope: LLM-adapter capability boundaries, wire-semantics targets, and
> provider-specific degradation rules above the kernel and below product UX.

This document defines how Alan's provider adapters relate to the core
Turing-machine abstraction and to each provider family's official API
semantics.

It does not define login, saved profiles, or operator-facing connection UX.
Those are specified separately in:

- [`provider_auth_contract.md`](./provider_auth_contract.md)
- [`connection_profile_contract.md`](./connection_profile_contract.md)

## Problem Statement

Alan needs one stable machine model and several non-identical provider
adapters.

The failure mode to avoid is pretending all providers are the same at the wire
level. That creates silent capability loss, ambiguous behavior, and adapter
bugs that are difficult to audit.

Alan therefore needs an explicit contract for:

1. which semantics are part of the shared machine model
2. which semantics are provider-native and must remain explicit
3. which providers are expected to reach near-full fidelity
4. which providers can only support a best-effort compatibility subset

## Goals

This contract must satisfy all of the following:

1. Keep Alan's kernel semantics provider-agnostic.
2. Preserve provider-native semantics when they matter to correctness.
3. Prevent feature loss caused by prematurely flattening rich provider inputs
   into plain text.
4. Make unsupported or degraded behavior explicit.
5. Define what "aligned" means for each provider family.

## Non-Goals

This contract does not require:

1. making all providers expose identical wire formats
2. exposing every provider-native feature as a kernel primitive
3. promising full fidelity for generic compatible endpoints
4. collapsing provider-native server tools into Alan's host-side tool loop

## Core Model

Alan's machine model remains:

| Machine concept | Alan surface |
| --- | --- |
| tape | [`tape::Message`](../../crates/runtime/src/tape.rs) and `ContentPart` |
| transition input | `GenerationRequest` |
| transition output | `GenerationResponse` and `StreamChunk` |
| actions / side effects | model-issued tool calls plus runtime tool execution |
| continuation state | `Session` plus optional provider-native continuation state |

The stable adapter boundary is the unified LLM contract in
[`crates/llm/src/lib.rs`](../../crates/llm/src/lib.rs):

1. `Message`
2. `ToolDefinition`
3. `GenerationRequest`
4. `GenerationResponse`
5. `StreamChunk`

## Layer Boundary

### Kernel MUST own

1. Tape semantics.
2. Turn boundaries and halt conditions.
3. Host-side tool execution semantics.
4. Session persistence and replay semantics.

### Provider adapter layer MUST own

1. Request projection into provider-native wire format.
2. Streaming event parsing.
3. Mapping provider-native usage, status, and identifiers back into the
   unified response surface.
4. Provider-native continuation, compaction, or retrieval endpoints where
   applicable.
5. Capability detection and explicit degradation behavior.

### Product / host layer MUST NOT assume

1. that all providers support server-managed conversation state
2. that all providers support the same role model
3. that all providers support the same tool protocol
4. that all providers support native multimodal or file inputs

## Stable Vocabulary

- **Common core**: semantics Alan expects across all first-class providers when
  the underlying API supports them.
- **Provider-native extension**: semantics that belong to one provider family
  and must remain explicit.
- **First-class provider**: a provider family Alan intends to support with
  deliberate, documented fidelity.
- **Compatibility provider**: an endpoint family that follows another
  provider's API shape only partially and is therefore supported on a
  best-effort basis.
- **Lossless projection**: projecting tape/runtime state into provider input
  without discarding machine-relevant structure.
- **Explicit degradation**: preserving behavior by emulation, warning, or hard
  rejection rather than silent lossy conversion.

## Provider Tiers

Alan recognizes three support tiers.

### Tier A: full-fidelity stateful provider

Providers in this tier should approach official API fidelity for all
machine-relevant capabilities.

V1 targets:

1. `openai_responses`
2. `chatgpt`

### Tier B: full-fidelity stateless provider

Providers in this tier should approach official API fidelity for the
stateless request/response loop, while remaining explicit that server-managed
conversation state does not exist.

V1 targets:

1. `openai_chat_completions`
2. `anthropic_messages`

### Tier C: best-effort compatibility provider

Providers in this tier expose a borrowed API shape but do not offer one
reliable cross-vendor contract beyond a conservative subset.

V1 targets:

1. `openai_chat_completions_compatible`
2. `openrouter_openai_chat_completions_compatible`

## Common Core Contract

The following capabilities are part of Alan's common core for first-class
providers whenever the provider API supports them:

1. text input and text output
2. non-streaming generation
3. streaming text deltas
4. function or tool definitions
5. model-issued tool call requests
6. tool results fed back into the next turn
7. finish reason / stop reason propagation
8. token usage propagation
9. provider response identifier propagation
10. provider response status propagation when the API exposes it
11. thinking / reasoning content propagation when the API exposes it
12. thinking-signature or encrypted-thinking propagation when the API exposes
    it
13. structured multimodal or file inputs when the API exposes them

Normative rules:

1. If an official first-class provider exposes a capability in its public API,
   Alan should preserve it whenever it affects turn semantics, context,
   streaming, or tool orchestration.
2. If a capability is unsupported by the provider, Alan must either emulate it
   intentionally or reject it. Silent no-op behavior is not acceptable.
3. If a capability is unsupported by a compatibility provider, Alan may drop
   it only when the degradation is explicit and observable.

## Capability Classes

### Class 1: must be unified

These semantics belong to Alan's machine model and must have one unified
surface:

1. final text
2. streaming text
3. tool definitions
4. tool call identity
5. tool result identity
6. finish reason
7. token usage
8. provider response id
9. provider response status

### Class 2: must be preserved when present

These semantics are optional in the kernel, but once a provider supports them
Alan should preserve them:

1. reasoning or thinking text
2. encrypted or signed reasoning state
3. redacted thinking blocks
4. cached token usage
5. native multimodal input parts
6. native file or document input parts

### Class 3: must remain provider-native

These semantics must stay explicit and must not be normalized away into fake
kernel invariants:

1. Responses `previous_response_id`, background mode, retrieval, cancellation,
   and compaction endpoints
2. Chat Completions `choices` and legacy message-centric shape
3. Anthropic `tool_use` / `tool_result` block ordering rules
4. Anthropic extended-thinking request constraints
5. provider-native server tools and server-managed side effects
6. provider-specific request invariants on Responses-compatible surfaces

## Rich Content Contract

Alan must not reduce all provider input to plain strings before the adapter
boundary.

Normative rules:

1. Tape-level rich content must remain structured through projection until the
   provider adapter has made a provider-specific decision.
2. Official providers with native multimodal or document inputs must receive
   structured content, not stringified placeholders.
3. Compatibility providers may fall back to text-only projection, but that
   fallback must be explicit in the capability matrix.

Implication:

1. The current `llm::Message { content: String }` surface is insufficient as
   the long-term canonical projection surface for full provider fidelity.
2. Alan should evolve toward a richer provider-input abstraction that can
   preserve text, image, document, tool, and reasoning items until the final
   adapter step.

## Degradation Rules

Every capability mismatch must use one of four strategies:

1. **Preserve**
   Use the provider's native representation.
2. **Emulate**
   Recreate the semantics in Alan, for example replaying full history for a
   stateless API.
3. **Reject**
   Return a first-class error when the capability is incompatible with the
   provider.
4. **Drop with warning**
   Allowed only for Tier C compatibility providers or clearly non-critical
   metadata.

Silent degradation is forbidden for:

1. tool semantics
2. continuation semantics
3. multimodal or document inputs on official providers
4. reasoning-signature continuity

## Provider-Specific Contracts

### OpenAI Responses

This provider is the closest fit for Alan's item-oriented machine model.

Required fidelity target:

1. preserve `instructions` separately from input items
2. preserve itemized tool calls and tool results
3. preserve provider-native `response.id`
4. preserve provider-native `status`
5. preserve `previous_response_id` continuation
6. preserve retrieval, cancellation, background polling, and compaction where
   the API exposes them
7. preserve reasoning items and encrypted reasoning state
8. preserve native multimodal and file input items
9. preserve cached input-token usage when returned

Normative rules:

1. `openai_responses` is the canonical full-fidelity stateful Responses
   provider in Alan.
2. Server-managed continuation must be modeled as provider-native state, not
   as a fake kernel invariant.
3. Alan must not assume that every Responses-compatible provider supports the
   same `store`, `background`, `retrieve`, `cancel`, or provider-compaction
   semantics.
4. `previous_response_id` support and `store=true` support are related but not
   identical capabilities.
5. Provider-managed continuation, background execution, retrieve/cancel, and
   provider compaction must be modeled as separate capabilities even when two
   providers share a similar wire shape.

### Managed ChatGPT Responses

This provider is Responses-compatible at the wire-shape level but must be
treated as Tier C until live validation proves otherwise.

Required fidelity target:

1. preserve `instructions` separately from input items
2. preserve itemized tool calls and tool results when surfaced by the stream
3. preserve provider-native `response.id`
4. preserve provider-native `status`
5. preserve reasoning items and encrypted reasoning state when surfaced by the
   provider
6. preserve cached input-token usage when returned

Live-verified constraints as of April 13, 2026:

1. request transport must use `stream=true`
2. request transport must force `store=false`
3. `temperature` must be omitted
4. `max_output_tokens` must be omitted
5. `previous_response_id` continuation must be treated as unsupported
6. background execution, retrieve/cancel, and provider compaction must be
   treated as unsupported unless revalidated

Normative rules:

1. `chatgpt` must remain a separate provider from `openai_responses` for auth,
   account, and capability semantics as defined in
   [`provider_auth_contract.md`](./provider_auth_contract.md).
2. Alan must reject unsupported continuation semantics for `chatgpt` rather
   than silently dropping them.
3. Product/runtime code must branch on explicit `chatgpt` capabilities instead
   of assuming official Responses parity.

### OpenAI Chat Completions

This provider is message-centric and stateless, but it is still a first-class
official API.

Required fidelity target:

1. preserve the official role model, including `developer`
2. preserve official multimodal message content arrays when supported
3. preserve tool calls and `tool` messages
4. preserve response `id`
5. preserve streaming deltas
6. preserve cached prompt-token usage when returned
7. preserve reasoning-token usage when returned

Normative rules:

1. Alan must not pretend Chat Completions has Responses-style
   `previous_response_id`.
2. Multi-turn state must be emulated by replay.
3. Differences from Responses such as `choices`, `response_format`, and
   function-calling shape must remain explicit at the adapter layer.

### Anthropic Messages

Anthropic is a first-class alternate backend with a block-based protocol.

Required fidelity target:

1. preserve assistant `tool_use` blocks
2. preserve user `tool_result` blocks
3. preserve Anthropic's ordering requirements for tool-result replies
4. preserve extended thinking output
5. preserve thinking signatures
6. preserve redacted thinking blocks
7. preserve native image inputs
8. preserve native document or PDF inputs
9. preserve prompt-caching semantics and cached-token accounting when returned
10. preserve provider-native response `id`
11. preserve `stop_reason`, including `tool_use` and `pause_turn`

Normative rules:

1. Alan must not map Anthropic tool results through a fake `tool` role at the
   wire layer.
2. Alan must preserve the fact that tool results are carried inside a `user`
   message and must immediately follow the corresponding assistant tool-use
   message.
3. Alan must preserve block structure until the Anthropic adapter step.
4. Extended thinking constraints that are incompatible with certain
   `tool_choice` modes must be enforced explicitly.

### Chat Completions-Compatible Providers

These providers are intentionally limited to a conservative compatibility
subset.

Required fidelity target:

1. text input and output
2. streaming text
3. OpenAI-style tool calls when the endpoint actually supports them
4. basic usage mapping when returned
5. best-effort reasoning-field interop using commonly observed extension
   fields such as `reasoning` or `reasoning_content`

Normative rules:

1. Alan must not define its own universal semantics from this family.
2. No capability should be marked "supported" unless Alan has explicit schema
   knowledge or verified behavior for that endpoint family.
3. Features such as native multimodal inputs, cached-token accounting,
   response retrieval, or official reasoning-state continuity must default to
   unsupported unless explicitly verified.

## Capability Matrix

The host or runtime must expose a provider capability matrix.

At minimum, it should answer:

```text
supports_streaming_text
supports_streaming_tool_calls
supports_provider_response_id
supports_provider_response_status
supports_reasoning_text
supports_reasoning_signature
supports_redacted_thinking
supports_multimodal_input
supports_document_input
supports_cached_token_usage
supports_server_managed_continuation
supports_background_execution
supports_retrieve_cancel
supports_provider_compaction
supports_provider_managed_tools
compatibility_tier
instruction_role
```

Normative rules:

1. Product code must branch on the capability matrix, not on ad hoc provider
   string checks spread throughout the codebase.
2. Tier C compatibility providers must be pessimistic by default.
3. Capabilities that often travel together in one provider family must still be
   declared independently when the underlying APIs can diverge.
4. The capability matrix is part of the implementation contract, not a
   documentation-only convenience.

## Minimum V1 Gap Closures

Before Alan can claim this contract is materially implemented, the following
gaps must be closed:

1. propagate provider response identifiers for official Chat Completions and
   Anthropic Messages
2. propagate cached token usage for official Chat Completions and Anthropic
   Messages when available
3. preserve official Chat Completions `developer` role rather than collapsing
   everything to `system`
4. preserve official Chat Completions native multimodal message content
5. preserve Anthropic native image and document inputs without flattening to
   text
6. preserve Anthropic `pause_turn` and related stop semantics as provider
   status and finish-reason data
7. stop treating text-only `llm::Message` projection as sufficient for
   first-class provider fidelity
8. propagate finish reason through the unified non-streaming response surface

## Near-Term Rich Input Bridge

Alan still needs a richer long-term provider-input abstraction than the current
`llm::Message { content: String }` surface.

That deeper refactor is not a prerequisite for every fidelity improvement in
the current tree.

Near-term rule for current implementation batches:

1. first-class providers may use explicit raw-message or raw-item override
   paths from runtime to adapter when that is necessary to preserve official
   multimodal, document, or role semantics,
2. those override paths must remain provider-specific and explicit,
3. this bridge must not be mistaken for the final canonical abstraction.

## Acceptance Criteria

This contract is satisfied when all of the following are true:

1. Alan has one documented common-core adapter surface.
2. Responses-class providers preserve official item semantics and
   provider-managed continuation.
3. Official Chat Completions preserves official message semantics without
   pretending to be Responses.
4. Anthropic Messages preserves block semantics without pretending to be OpenAI
   tool or role semantics.
5. Compatibility providers are explicitly documented as best-effort and
   capability-limited.
6. Unsupported capabilities fail explicitly or degrade explicitly.
7. Product code can determine effective behavior from a capability matrix.

## External References

The target boundaries in this document are informed by the following official
API documentation:

- OpenAI Responses migration guide:
  <https://developers.openai.com/api/docs/guides/migrate-to-responses>
- OpenAI Chat Completions API reference:
  <https://api.openai.com/v1/chat/completions>
- OpenAI gpt-oss compatibility guidance for Chat Completions-compatible APIs:
  <https://developers.openai.com/cookbook/articles/gpt-oss/verifying-implementations#chat-completions>
- Anthropic Messages API reference:
  <https://platform.claude.com/docs/en/api/messages/create>
- Anthropic tool-use docs:
  <https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools>
- Anthropic tool-result handling docs:
  <https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls>
- Anthropic extended thinking docs:
  <https://platform.claude.com/docs/en/build-with-claude/extended-thinking>
- Anthropic vision docs:
  <https://platform.claude.com/docs/en/build-with-claude/vision>
- Anthropic prompt caching docs:
  <https://platform.claude.com/docs/en/build-with-claude/prompt-caching>
- Anthropic PDF support docs:
  <https://platform.claude.com/docs/en/build-with-claude/pdf-support>
