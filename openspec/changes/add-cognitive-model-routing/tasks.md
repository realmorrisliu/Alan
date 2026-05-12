## 1. Configuration And Resolution

- [ ] 1.1 Add cognition config types for routing mode, default system, System 1 model binding, System 2 model binding, and per-system reasoning-effort intent.
- [ ] 1.2 Resolve cognitive model bindings through provider/model availability without duplicating provider credentials.
- [ ] 1.3 Preserve existing `connection_profile` behavior when cognition config is absent.
- [ ] 1.4 Add startup diagnostics for missing or invalid cognitive model binding references.

## 2. Runtime Routing Core

- [ ] 2.1 Add routing intent and metadata types for cognitive system, routing source, provider/model binding id, model, effort, and bounded reason.
- [ ] 2.2 Implement `CognitiveRouter` precedence for explicit System 2 override, deterministic safety gates, eligible System 1 override, config default, and System 1 fallback route.
- [ ] 2.3 Compose selected cognitive model binding with existing request-control resolution before provider dispatch.
- [ ] 2.4 Keep provider adapters unaware of cognitive routing decisions beyond the normalized generation request.
- [ ] 2.5 Partition, clear, or replay provider-native continuation when cognitive routing switches provider/model bindings.

## 3. System 1 Escalation

- [ ] 3.1 Add an internal-only `escalate_to_system2` virtual action with bounded reason and needed-context fields.
- [ ] 3.2 Inject the escalation contract only for System 1 attempts where auto routing allows escalation.
- [ ] 3.3 Suppress System 1 visible output when escalation is captured.
- [ ] 3.4 Gate side-effecting tools during unaccepted System 1 attempts until runtime accepts the fast route or routes the turn to System 2.
- [ ] 3.5 Rerun the original logical turn on System 2 with bounded System 1 triage notes when only read-only tools have executed.
- [ ] 3.6 Continue from observed post-side-effect state when escalation happens after an accepted execution phase or external state change has already completed side effects.

## 4. Metadata And API Surfaces

- [ ] 4.1 Persist routing metadata in rollout turn context and session state.
- [ ] 4.2 Expose routing metadata in daemon create/list/read/reconnect/fork surfaces where request-control metadata is reported.
- [ ] 4.3 Accept and validate session, fork, and turn-scoped cognitive-system override intent.
- [ ] 4.4 Update generated or checked client DTO surfaces and endpoint drift checks.

## 5. Verification

- [ ] 5.1 Add unit tests for cognition config parsing, fallback behavior, invalid model binding diagnostics, override precedence, and gated System 1 override rejection/supersession.
- [ ] 5.2 Add runtime tests for deterministic System 2 gates, configured default routing, System 1 fallback route, System 1 escalation, fast-draft suppression, unaccepted System 1 side-effect gating, accepted side-effect continuation, and continuation partitioning.
- [ ] 5.3 Add request-control tests proving selected cognitive model binding effort composes with existing turn/session/model precedence.
- [ ] 5.4 Add daemon/API tests for routing metadata and override validation.
- [ ] 5.5 Run `cargo test --workspace` or the narrower documented Rust test suites covering runtime and daemon routing behavior.
- [ ] 5.6 Run `openspec validate add-cognitive-model-routing --strict`.

## 6. PR Review And Archive Readiness

- [ ] 6.1 Review the implementation diff for provider-boundary violations, hidden draft leakage, and metadata overexposure.
- [ ] 6.2 After merge, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.3 Archive the completed OpenSpec change after the synced specs validate.
