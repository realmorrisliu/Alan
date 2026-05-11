## 1. Configuration And Resolution

- [ ] 1.1 Add cognition config types for routing mode, default system, System 1 profile, System 2 profile, and per-system reasoning-effort intent.
- [ ] 1.2 Resolve cognitive profiles through existing connection profile loading without duplicating provider credentials.
- [ ] 1.3 Preserve existing `connection_profile` behavior when cognition config is absent.
- [ ] 1.4 Add startup diagnostics for missing or invalid cognitive profile references.

## 2. Runtime Routing Core

- [ ] 2.1 Add routing intent and metadata types for cognitive system, routing source, profile id, model, effort, and bounded reason.
- [ ] 2.2 Implement `CognitiveRouter` precedence for turn override, session override, config default, deterministic gates, and System 1 default route.
- [ ] 2.3 Compose selected cognitive profile with existing request-control resolution before provider dispatch.
- [ ] 2.4 Keep provider adapters unaware of cognitive routing decisions beyond the normalized generation request.

## 3. System 1 Escalation

- [ ] 3.1 Add an internal-only `escalate_to_system2` virtual action with bounded reason and needed-context fields.
- [ ] 3.2 Inject the escalation contract only for System 1 attempts where auto routing allows escalation.
- [ ] 3.3 Suppress System 1 visible output when escalation is captured.
- [ ] 3.4 Rerun the original logical turn on System 2 with bounded System 1 triage notes.

## 4. Metadata And API Surfaces

- [ ] 4.1 Persist routing metadata in rollout turn context and session state.
- [ ] 4.2 Expose routing metadata in daemon create/list/read/reconnect/fork surfaces where request-control metadata is reported.
- [ ] 4.3 Accept and validate session, fork, and turn-scoped cognitive-system override intent.
- [ ] 4.4 Update generated or checked client DTO surfaces and endpoint drift checks.

## 5. Verification

- [ ] 5.1 Add unit tests for cognition config parsing, fallback behavior, invalid profile diagnostics, and override precedence.
- [ ] 5.2 Add runtime tests for deterministic System 2 gates, System 1 default route, System 1 escalation, and fast-draft suppression.
- [ ] 5.3 Add request-control tests proving selected cognitive profile effort composes with existing turn/session/model precedence.
- [ ] 5.4 Add daemon/API tests for routing metadata and override validation.
- [ ] 5.5 Run `cargo test --workspace` or the narrower documented Rust test suites covering runtime and daemon routing behavior.
- [ ] 5.6 Run `openspec validate add-cognitive-model-routing --strict`.

## 6. PR Review And Archive Readiness

- [ ] 6.1 Review the implementation diff for provider-boundary violations, hidden draft leakage, and metadata overexposure.
- [ ] 6.2 After merge, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.3 Archive the completed OpenSpec change after the synced specs validate.
