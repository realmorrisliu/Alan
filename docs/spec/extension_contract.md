# Extension Contract (Plugin / Extension Host)

> Status: VNext contract (defines Alan's extension mechanism and lifecycle contract).

## Goals

Keep `alan-runtime` stable while enabling pluggable capability growth:

1. Add/replace capabilities via extensions without changing runtime main loop.
2. Let skills orchestrate extension capabilities instead of encoding system behavior in prompt text.
3. Keep call/governance semantics consistent across local and remote (bridge) modes.

## Non-Goals

1. Extensions are not a business workflow engine.
2. Extensions cannot bypass policy/sandbox for high-risk actions.
3. VNext does not mandate a single packaging format or language.

## Terminology

1. **Extension**: loadable capability unit (tool/memory/channel/domain module).
2. **Extension Host**: manages extension lifecycle, isolation, and health checks.
3. **Capability**: routable interface (`tool.read_file`, `memory.search`, etc.).
4. **Capability Router**: runtime routing layer selecting providers (`capability_router.md`).
5. **Bridge**: cross-process/machine extension hosting channel (`harness_bridge.md`).

## Layer Positioning

1. `kernel/runtime`: state machine, idempotency, governance boundaries, invariants.
2. `extension`: capability implementation and external integration.
3. `skills`: workflow orchestration and strategy.
4. `harness`: regression validation of extension/system behavior.

## Extension Types (VNext)

1. `tool_provider`
   - executable capabilities (file/command/external API)
2. `memory_provider`
   - long-term memory read/write and retrieval backend
3. `channel_adapter`
   - external interaction channels (notifications/mobile control/webhooks)
4. `domain_module`
   - domain bundles (coding/research/ops)

Type classification is for governance/observability and does not constrain implementation language.

## Manifest Contract (Draft v0)

Each extension must provide a manifest with at least:

1. `id`: stable unique identifier (reverse-domain recommended)
2. `version`: extension semver
3. `contract_version`: contract version (`0.x`)
4. `kind`: `tool_provider | memory_provider | channel_adapter | domain_module`
5. `entrypoint`: startup target (local executable or bridge endpoint)
6. `capabilities[]`: declarations (name/version/risk/schema)
7. `permissions`: minimum requested permissions (`fs/network/process/scheduler`)
8. `config_schema`: optional JSON schema
9. `state_namespace`: private state namespace (required)
10. `healthcheck`: endpoint/probe capability (required)

Example:

```yaml
id: io.alan.coding.base
version: 0.1.0
contract_version: 0.1
kind: domain_module
entrypoint:
  mode: local_process
  command: ["alan-ext-coding", "serve"]
state_namespace: ext/io.alan.coding.base
capabilities:
  - name: tool.code_edit
    version: 1
    effects: [write, process]
    risk_level: B
    idempotency: required
permissions:
  fs: workspace_only
  network: deny
  process: allowlist
healthcheck:
  probe: ext.health
```

## Capability Declaration Contract

Each capability must declare:

1. `name` and `version`
2. `effects`: `read | write | network | process | memory | channel | scheduler`
3. `risk_level`: `A | B | C`
4. `input_schema` / `output_schema`
5. `timeout_ms`
6. `idempotency`: `required | optional | unsupported`

Rules:

1. `required` idempotency capabilities must accept and honor `idempotency_key`.
2. `risk_level C` defaults to `escalate` unless policy explicitly allows.

## Lifecycle Contract

Extension Host must support:

1. `load`: parse manifest/config and perform compatibility checks
2. `init`: inject runtime context (workspace/policy/trace) and initialize resources
3. `start`: enter serving state and register capabilities
4. `stop(reason)`: graceful shutdown with in-flight handling
5. `recover(checkpoint_ref)`: optional restart-state recovery
6. `health`: health + version report

Constraints:

1. `start` failure must not crash runtime; isolate and degrade safely.
2. `stop` must be interruptible and not block daemon shutdown.
3. Lifecycle transitions must be auditable.

## Call Contract

### Request

`CapabilityRequest` must include at least:

1. `request_id`
2. `task_id/run_id/session_id/turn_id`
3. `capability`
4. `input` (JSON)
5. `idempotency_key` (if required)
6. `deadline_ms`
7. `trace_context`
8. `governance_context` (policy-decision summary)

### Response

`CapabilityResponse` must include at least:

1. `request_id`
2. `status`: `ok | dedup_hit | retryable_error | fatal_error | denied | escalated`
3. `output`
4. `effect_refs` (required if side effects occurred)
5. `error` (required on failure)
6. `retry_after_ms` (optional for retryable errors)

Rules:

1. `request_id` must remain idempotent and traceable.
2. Extension must return promptly on cancel/timeout signals.
3. Irreversible side effects require auditable `effect_refs`.

## Governance and Sandbox

1. Router applies `policy -> allow/deny/escalate` before invocation.
2. Extension must not bypass governance.
3. Declared permissions are upper bounds; sandbox remains the authoritative execution-boundary contract. In the current runtime that contract is implemented by the best-effort `workspace_path_guard` backend, while future strict backends are a target-state upgrade.
4. Recovery paths use same governance as normal paths (no "recovery bypass").

## State and Persistence Boundaries

1. Extension private state path:
  - `{workspace}/.alan/extensions/{extension_id}/state/`
2. Temporary file path:
  - `{workspace}/.alan/extensions/{extension_id}/tmp/`
3. Extensions must not mutate rollout/checkpoint source-of-truth files.
4. Restart-recoverable extension state must be versioned and migratable.

## Isolation Tiers (Recommended)

1. `tier_local_process` (default): local subprocess, low latency, host-managed.
2. `tier_remote_bridge`: remote process via bridge, ideal for mobile/cloud.
3. `tier_in_process` (dev only): experimental, not production default.

## Observability and Audit

Each capability call should record at least:

1. `extension_id/capability/request_id`
2. `run_id/session_id/turn_id`
3. `route` (`local/bridge`)
4. `latency_ms/status`
5. `dedupe_hit` (bool)
6. `policy_action/risk_level`

## Error and Degradation Strategy

1. `retryable_error`: bounded exponential retry allowed.
2. `fatal_error`: mark extension unhealthy and isolate.
3. `denied/escalated`: governance outcomes, not extension availability failures.
4. Repeated failures beyond threshold trigger circuit breaker and provider fallback (if available).

## Versioning and Compatibility

1. Host must reject unsupported major `contract_version`.
2. Breaking capability schema changes require major bump.
3. Deprecated capabilities should provide migration window and compatibility aliases.

## Phased Rollout

1. Phase 1: model builtin tools as `tool_provider` semantics.
2. Phase 2: add `memory_provider` and `channel_adapter`.
3. Phase 3: host remote extensions through `harness_bridge`.

## Acceptance Criteria

1. New/replaced capability providers work without runtime main-loop changes.
2. Extension failures do not corrupt session state machine.
3. High-risk capability calls remain under governance boundaries.
4. Local/remote calls preserve consistent audit and idempotency semantics.
