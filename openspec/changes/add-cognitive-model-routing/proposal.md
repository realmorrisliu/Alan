## Why

Alan can already control reasoning effort for a selected model, but it cannot
act like a two-speed agent that normally thinks quickly and deliberately
escalates when a task needs deeper reasoning. Alan should support configurable
System 1/System 2 model bindings with automatic, visible, and overridable
routing.

## What Changes

- Add cognitive routing configuration for System 1 and System 2 model bindings,
  layered above provider/credential configuration and optional reasoning-effort
  intent.
- Add a runtime-owned `CognitiveRouter` that applies explicit overrides,
  deterministic safety gates, and System 1 self-escalation before provider
  dispatch, with safety gates able to supersede forced System 1 intent.
- Add an internal-only escalation action that lets the System 1 model request a
  System 2 rerun without exposing the fast draft to the user.
- Gate side-effecting tools during unaccepted System 1 attempts so workspace
  mutation waits for accepted fast execution or System 2 routing.
- Record routing decisions in turn/session metadata, rollout entries, logs, and
  daemon/client DTOs.
- Preserve existing provider adapter boundaries: adapters only project the
  normalized request they receive and do not decide routing.
- Partition provider-native continuation by compatible prompt and tool state so
  System 1-only escalation tools or speculative prompt context cannot leak into
  System 2.
- Keep first implementation single-runtime and single-active-turn; this is not
  parallel multi-agent execution.

## Capabilities

### New Capabilities

- `cognitive-model-routing`: Owns System 1/System 2 model binding configuration,
  automatic routing, internal escalation, override precedence, and routing
  observability.

### Modified Capabilities

- `provider-request-controls`: Request-control resolution must compose with the
  selected cognitive model binding and remain the only authority for effective
  reasoning effort.
- `daemon-api-contract`: Daemon session and turn surfaces must expose routing
  metadata and accept explicit cognitive-system overrides.

## Impact

- Affected runtime modules: request-control resolution, turn execution,
  LLM-client construction, virtual/internal actions, rollout persistence, and
  session startup metadata.
- Affected configuration: `agent.toml` gains a cognition block that binds
  System 1 and System 2 to available provider/model entries without duplicating
  provider credentials.
- Affected daemon/clients: session create/fork/submit DTOs and read/list
  responses expose selected cognitive system and routing reason.
- Affected tests: routing precedence, deterministic gates, System 1 escalation,
  side-effect gating, hidden fast-draft suppression, metadata persistence,
  prompt/tool continuation partitioning, and provider-boundary contract tests.
