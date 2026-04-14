# Alan Kernel Contract

> Status: V1 contract for long-term stability.
> Scope: `alan-runtime` core behavior and invariants.

## Goals

This document defines immutable kernel contracts (invariants) that constrain future changes:

- **Small, stable kernel**: only state progression and execution control, not business strategy.
- **Auditable behavior**: all key decisions and side effects are traceable.
- **Replaceable extensions**: capability growth happens via skills/tools/host layers.

This document has higher priority than philosophy docs. Protocol details remain source-of-truth in `alan-protocol` and `alan-runtime` code.

## Boundary Definition

### Kernel MUST

1. Manage Tape and Rollout lifecycle.
2. Run the turn execution loop (LLM generation, tool orchestration, yield/resume).
3. Connect policy decisions with sandbox execution boundaries.
4. Maintain state consistency between emitted events and received Ops.

### Kernel MUST NOT

1. Own domain-specific workflow logic.
2. Own product-layer UI protocol details (daemon/client concern).
3. Embed platform-specific channel semantics (CRM/ticketing/etc.).

## Core Entity Contracts

### HostConfig

- Defines machine-local daemon/client settings outside the kernel.
- Must not be treated as part of agent identity or prompt/runtime state.

### AgentRoot

- Defines the on-disk agent root: `agent.toml`, `persona/`, `skills/`, `policy.yaml`.
- Standard public skill install directories such as `~/.agents/skills/` and
  `<workspace>/.agents/skills/` may also feed the resolved capability view as
  single-skill packages associated with the base layers.
- Multiple `AgentRoot`s may overlay into one effective agent definition.
- Definition overlay is separate from runtime supervision or process ancestry.

### Resolved Agent Definition

- Kernel consumes one resolved agent definition assembled from the `AgentRoot`
  overlay chain.
- Current code may materialize this as runtime config structs such as
  `AgentConfig`; that representation is an implementation detail, not the
  primary hosting abstraction.

### Workspace

- Defines "where this agent lives": identity, memory, session archive, and
  workspace-local state bound to a resolved agent definition.
- Persistent context container, not equivalent to a single execution.

### AgentInstance

- Defines the running agent process bound to one resolved agent definition and
  one workspace at a time.
- Runtime parent/child relations apply here, not in the `AgentRoot` overlay
  chain.

### SpawnSpec

- Defines explicit child-agent launch inputs, bindings, and runtime overrides.
- Child startup should be fresh by default; no implicit tape or prompt
  inheritance.
- Current V1 transport shape includes:
  - launch inputs such as `task`, `cwd`, `workspace_root`, `timeout_secs`,
    `budget_tokens`, and `output_dir`
  - explicit bound handles such as `workspace`, `artifacts`, `memory`, `plan`,
    `conversation_snapshot`, `tool_results`, and `approval_scope`
  - small runtime overrides such as model, policy path, and tool allowlist
- `artifacts` / `output_dir` remain reserved transport fields until runtime
  artifact routing is implemented; current child-runtime launches reject them
  instead of treating them as prompt-only hints.
- A child launch is an `exec`-like runtime action: it starts a fresh
  `AgentInstance`, binds only the requested handles, and returns result/cancel
  state through explicit runtime lifecycle APIs rather than implicit parent
  session mutation.

### Session

- Defines "what is happening now": bounded execution window inside one
  `AgentInstance`.
- Holds Tape, runtime state, and current turn context.

### Tape

- Execution source of truth for messages and context segments.
- Supports explicit compaction/rollback; forbids implicit loss.

### Rollout

- Event audit chain for key state transitions and tool decisions.
- Should contain minimally sufficient info for replay/fork.
- Durable rollout persistence may redact or truncate tool payloads as long as
  auditability and replay/dedupe semantics remain intact.

## Invariants

### 1) Monotonic State Progression

- Within one Session, turn lifecycle must be decidable:
  `started -> (yield/resume)* -> completed|error|interrupted`.
- Illegal state: resuming the same turn after it has already terminated.

### 2) Session Exclusivity

- Only one active `AgentInstance` is allowed per Workspace at a time.
- Conflicts must be explicitly rejected by hosting layer, never silently overwritten.

### 3) Explicit Side Effects

- All external side effects must happen through tool-call paths.
- LLM generation path must not produce unaudited side effects.

### 4) Traceable Decisions

- Every tool decision must be traceable to policy source, matched rule, action, and reason.
- `escalate` must enter a recoverable `Yield -> Resume` symmetric flow.

### 5) Accepted Output Only

- User-visible assistant text must come from an accepted draft.
- Kernel-level response guardrails may retry once before emission when the draft
  contradicts runtime-known capability facts.
- Rejected drafts must not leak into emitted assistant text deltas.

### 6) Context Projection Isolation

- Tape is internal source of truth; provider input is a projected view.
- Provider-specific adaptation must not contaminate Tape abstractions.

### 7) Boundedness First

- Context window is a hard constraint; kernel must support compaction, segmentation, and session rotation.
- Infinite-history injection is not an acceptable workaround.

## Error and Recovery Contract

1. **Recoverable errors**: preserve session and continue subsequent turns when possible.
2. **Non-recoverable errors**: emit diagnosable error events and stop current execution.
3. **Recovery entrypoint**: explicit Ops (for example `resume`), never implicit reentry.

## Extension Interface Constraints

1. Skills affect behavior via prompt injection and tool orchestration only.
2. Tool implementations are replaceable but must follow shared schema/timeout/capability semantics.
3. Host layers may extend protocols but must preserve kernel turn semantics.

## Compatibility Strategy

- New capabilities should default to extension points, avoiding kernel-loop changes.
- If kernel invariants must change, update all of:
  1. this document,
  2. related contract-test guidance in `docs/testing_strategy.md`,
  3. migration notes for breaking changes.

## Minimal Acceptance Checklist

For each kernel-related change:

1. New behavior maps to existing turn state machine, with no hidden states.
2. Tool side-effect path remains unique and auditable.
3. Protocol event sequence is verifiable via contract tests.
4. Rollback/compaction does not break subsequent session recovery.
