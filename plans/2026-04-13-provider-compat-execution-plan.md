# Provider Compatibility Closure Plan (2026-04-13)

> Status: implemented and audited for provider capability closure across spec, runtime, adapters, and tests.

## Why This Plan Exists

The current provider line is past the point where ad hoc fixes are sufficient.

Recent investigation exposed a concrete incompatibility on the `chatgpt`
provider path:

1. runtime treated `chatgpt` and `openai_responses` as one continuation class,
2. continuation logic injected `store=true`,
3. the managed ChatGPT Responses surface rejected that request.

That bug is not isolated. It reflects a broader mismatch between:

1. the provider capability contract,
2. the runtime/provider branching strategy,
3. the current automated-test coverage.

This plan closes that gap in one execution batch.

## Scope

This batch covers:

1. spec corrections and clarifications for provider capability boundaries,
2. a concrete runtime-visible capability matrix implementation,
3. provider/runtime fixes for known incompatibilities and missing fidelity,
4. automated tests that lock the new behavior,
5. a final audit pass against this document.

This batch does not attempt a full long-term replacement of the unified
`llm::Message { content: String }` surface. It does, however, establish:

1. the capability-driven branching model,
2. raw rich-input override paths where needed for first-class providers,
3. the spec and execution hooks needed for the later deeper refactor.

## Baseline Problems

### Spec Gaps

1. `chatgpt` and `openai_responses` are described as near neighbors but their
   request invariants are not explicit enough.
2. The capability matrix is required by spec but not implemented in code.
3. The common-core contract requires finish-reason propagation, while the
   unified non-streaming response surface does not currently carry it.
4. Official Chat Completions and Anthropic fidelity targets are stricter than
   the current implementation.

### Implementation Gaps

1. Product/runtime code still branches on provider strings and `is_*()` checks.
2. `chatgpt` inherited official Responses defaults that are not valid on the
   managed surface.
3. Official Chat Completions currently collapses `Context` into `system`
   rather than preserving `developer`.
4. Official Chat Completions and Anthropic do not currently propagate provider
   response identifiers on non-streaming responses.
5. Official Chat Completions and Anthropic do not preserve rich attachment
   inputs with first-class fidelity.
6. Anthropic stop semantics are narrower than the provider contract target.

### Test Gaps

1. There is no code-level capability-matrix test surface.
2. There is no adapter contract suite that systematically exercises the same
   feature classes across providers.
3. There is no regression test for the `chatgpt` continuation/store mismatch at
   the runtime layer.
4. There is no end-to-end coverage for official Chat Completions `developer`
   projection or rich attachment projection.
5. There is no end-to-end coverage for Anthropic rich attachment projection and
   stop-semantics mapping.

## Execution Order

The batch will land in this order:

1. lock the execution plan in-repo,
2. update provider specs to match the intended implementation boundary,
3. implement the runtime-visible capability matrix,
4. switch runtime branching to capabilities,
5. close provider fidelity gaps in adapters,
6. add and expand tests,
7. run an explicit review and audit against this plan.

## Implementation Slices

### Slice 0: Plan Lock-In

Deliverables:

1. this execution plan,
2. plan index update if needed.

Exit criteria:

1. there is one repo-local source of truth for the execution sequence,
2. later review can audit against explicit checklist items rather than memory.

### Slice 1: Spec Corrections

Files:

1. `docs/spec/provider_capability_contract.md`
2. `docs/spec/provider_auth_contract.md`
3. optional cross-reference updates in `docs/spec/README.md`

Required changes:

1. make `chatgpt` request invariants explicit:
   `previous_response_id` may be supported without inheriting every official
   Responses server-state option;
2. require capability-matrix-driven product/runtime branching rather than ad hoc
   provider checks;
3. make unified finish-reason propagation explicit in both streaming and
   non-streaming paths;
4. document the current batch’s rich-input override strategy as the near-term
   bridge to the longer-term richer provider-input abstraction;
5. clarify that provider-managed continuation, compaction, retrieve/cancel, and
   background execution are independent capabilities, not one boolean bucket.

Exit criteria:

1. the target contract matches the concrete code changes in this batch,
2. the remaining long-term rich-input refactor is clearly separated from the
   current closure batch.

### Slice 2: Capability Matrix Implementation

Files:

1. `crates/llm/src/lib.rs`
2. `crates/runtime/src/llm.rs`
3. tests near those modules

Required changes:

1. add a public provider capability type,
2. add a compatibility-tier enum,
3. define provider capabilities per provider family,
4. expose capabilities through `LlmClient`,
5. add unit tests for capability declarations.

Minimum capability fields for this batch:

1. `supports_streaming_text`
2. `supports_streaming_tool_calls`
3. `supports_provider_response_id`
4. `supports_provider_response_status`
5. `supports_reasoning_text`
6. `supports_reasoning_signature`
7. `supports_redacted_thinking`
8. `supports_multimodal_input`
9. `supports_document_input`
10. `supports_cached_token_usage`
11. `supports_server_managed_continuation`
12. `supports_background_execution`
13. `supports_retrieve_cancel`
14. `supports_provider_compaction`
15. `instruction_role`
16. `compatibility_tier`

Exit criteria:

1. runtime can branch on capabilities without duplicating provider-family logic,
2. capability declarations are tested and reviewable in one place.

### Slice 3: Runtime Capability-Driven Branching

Files:

1. `crates/runtime/src/runtime/turn_executor.rs`
2. `crates/runtime/src/runtime/turn_support.rs`

Required changes:

1. replace the current `chatgpt || openai_responses` response-provider bucket
   with capability-driven logic;
2. gate `context_management` on provider compaction capability rather than
   provider family;
3. gate server-managed continuation on capability rather than provider family;
4. keep provider-name detection only for logging/audit, not behavior control;
5. add runtime tests proving the new branching behavior.

Exit criteria:

1. no core runtime continuation logic depends on ad hoc provider pairing,
2. `chatgpt` and `openai_responses` can diverge safely where required.

### Slice 4: Adapter Fidelity Fixes

Files:

1. `crates/llm/src/lib.rs`
2. `crates/llm/src/openai_chat_completions.rs`
3. `crates/llm/src/openai_responses.rs`
4. `crates/llm/src/chatgpt_responses.rs`
5. `crates/llm/src/anthropic_messages.rs`

Required changes:

1. add non-streaming finish-reason support to the unified response surface;
2. propagate official Chat Completions response ids;
3. propagate Anthropic response ids;
4. preserve official Chat Completions `developer` role instead of collapsing
   context to `system`;
5. add rich-input override paths for official Chat Completions and Anthropic so
   image/document-capable tape inputs survive to the adapter layer;
6. preserve Anthropic stop semantics more explicitly;
7. keep `chatgpt` request construction provider-specific and pessimistic about
   unsupported server-state knobs.

Exit criteria:

1. official first-class providers preserve more of their native semantics,
2. the unified surface stops hiding finish reasons and provider ids where the
   API exposes them,
3. rich attachment inputs no longer flatten unnecessarily on first-class
   providers in the scenarios covered by this batch.

### Slice 5: Automated Tests

Files:

1. provider unit tests in `crates/llm/src/*`
2. runtime tests in `crates/runtime/src/runtime/turn_executor.rs`
3. optional higher-level tests in `crates/alan/tests/*` if needed

Required coverage:

1. capability matrix snapshot tests,
2. `chatgpt` continuation/store regression tests,
3. runtime continuation gating tests for `chatgpt` vs `openai_responses`,
4. official Chat Completions `developer` role projection tests,
5. official Chat Completions attachment projection tests,
6. Anthropic attachment projection tests,
7. Anthropic stop-reason / provider-id mapping tests,
8. non-streaming finish-reason propagation tests.

Exit criteria:

1. the new behavior is covered by automated tests at both adapter and runtime
   layers,
2. the original bug class is prevented from regressing silently.

### Slice 6: Review And Audit

Deliverables:

1. a post-change audit against this checklist,
2. a concise completion summary with any residual gaps explicitly called out.

Audit checklist:

- [x] Spec updates landed and match code behavior.
- [x] Capability matrix is implemented and tested.
- [x] Runtime branches on capabilities for continuation/compaction behavior.
- [x] `chatgpt` no longer inherits invalid official Responses defaults.
- [x] Unified non-streaming responses carry finish reasons.
- [x] Official Chat Completions propagates response ids.
- [x] Anthropic propagates response ids.
- [x] Official Chat Completions preserves `developer` role.
- [x] First-class rich attachment projection exists for official Chat
      Completions in the supported cases of this batch.
- [x] First-class rich attachment projection exists for Anthropic in the
      supported cases of this batch.
- [x] Adapter/runtime tests cover the new behavior.
- [x] Final verification commands ran successfully.

Audit notes:

1. Spec updates landed in `provider_capability_contract.md` and
   `provider_auth_contract.md`, including capability-driven branching,
   provider-specific request invariants, and rich-input bridge guidance.
2. Runtime/provider compatibility now branches on `ProviderCapabilities`
   instead of hard-coded `chatgpt || openai_responses` buckets.
3. Adapter fidelity closures landed for ChatGPT Responses, official OpenAI Chat
   Completions, and Anthropic Messages, including richer request projection and
   provider id / finish reason propagation.
4. Verification completed with:
   `cargo test -p alan-llm`
   `cargo test -p alan-runtime`
   `cargo test -p alan --no-run`
   `cargo test -p alan --test smoke_test --test compaction_integration_test --test agent_root_overlay_integration_test`

## Verification Commands

Expected commands by the end of the batch:

1. `cargo test -p alan-llm`
2. `cargo test -p alan-runtime`
3. targeted regression commands for provider and runtime slices as needed

## Success Criteria

This plan is complete when all of the following are true:

1. the repo contains an explicit implementation plan and matching code/spec
   changes,
2. provider behavior differences are modeled as capabilities rather than
   scattered string checks,
3. first-class providers preserve the batch-targeted semantics required by the
   updated contract,
4. the original `chatgpt` compatibility failure is covered by tests,
5. the final audit can mark every checklist item complete or explicitly justify
   any remaining exception.
