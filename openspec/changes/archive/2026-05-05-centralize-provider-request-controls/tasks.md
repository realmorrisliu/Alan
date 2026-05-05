## 1. Resolver Foundation

- [x] 1.1 Add `crates/runtime/src/request_controls.rs` with intent, source, diagnostics, and resolved-control types.
- [x] 1.2 Implement resolver precedence for turn override, session override, agent config, model default, legacy budget, and provider default.
- [x] 1.3 Move explicit effort support validation into the resolver using provider capabilities and resolved model metadata.
- [x] 1.4 Add resolver unit tests covering the full precedence matrix and unsupported provider/model cases.

## 2. Runtime Integration

- [x] 2.1 Replace `Config::effective_model_reasoning_effort` call sites with resolver calls.
- [x] 2.2 Replace `Config::validate_reasoning_effort_for_resolved_model` call sites with resolver validation.
- [x] 2.3 Refactor `RuntimeConfig` and `AgentConfig` so request controls are stored as intent or resolver snapshot, not duplicated effective truth.
- [x] 2.4 Update workspace overlay and explicit runtime override merge tests for the new request-control intent model.
- [x] 2.5 Update `turn_executor` to consume `ResolvedRequestControls` when building `GenerationRequest`.
- [x] 2.6 Update turn-scoped reasoning override handling to feed resolver input without recomputing effective controls in `turn_executor`.

## 3. Provider Projection

- [x] 3.1 Normalize `GenerationRequest` so `ReasoningControls` is the canonical request-control carrier.
- [x] 3.2 Keep compatibility builder methods while ensuring legacy budget setters populate normalized controls.
- [x] 3.3 Update OpenAI Responses and Chat Completions adapters to project normalized controls only.
- [x] 3.4 Update Gemini, Anthropic, OpenRouter, and compatible adapters to avoid Alan-level default or precedence decisions.
- [x] 3.5 Add provider projection tests for canonical effort, legacy budget, and extra-param precedence.

## 4. Daemon, Session, And Client Metadata

- [x] 4.1 Route runtime startup metadata through resolver output.
- [x] 4.2 Update daemon create/read/fork/session-store code to mirror resolver metadata rather than recomputing effective effort.
- [x] 4.3 Reduce repeated `reasoning_effort: None` fixture churn with helper constructors where practical.
- [x] 4.4 Confirm TUI and Apple clients remain DTO/presentation-only and do not encode request-control semantics.
- [x] 4.5 Add daemon payload contract tests for create session and fork override metadata.

## 5. Documentation And Contract Guardrails

- [x] 5.1 Update `docs/spec/provider_capability_contract.md` with the runtime resolver ownership boundary.
- [x] 5.2 Update `docs/spec/provider_reasoning_effort_migration.md` if compatibility behavior changes during refactor.
- [x] 5.3 Add lightweight contract checks or focused tests that fail when daemon routes or `turn_executor` recompute effective reasoning controls directly.
- [x] 5.4 Run `cargo fmt --all`.
- [x] 5.5 Run focused runtime, LLM, daemon payload, and protocol tests.
- [x] 5.6 Run `openspec validate centralize-provider-request-controls`.
