# Alan Agent Root and Runtime Model Plan (2026-03-12)

> Status: active architecture migration plan.

## Context

Alan's current conceptual model is close to the right shape, but not yet elegant enough for a
Unix-like agent abstraction:

- `README.md` describes `Agent` as the stateless program, `Workspace` as persistent context, and
  `Session` as bounded execution.
- the runtime today exposes `AgentConfig`, `WorkspaceRuntimeConfig`, and `Session`, but `Agent`
  itself is not yet a first-class runtime object.
- global agent-facing configuration still lives in `~/.config/alan/config.toml`, which splits
  "Alan home" across two roots: `~/.alan` and `~/.config/alan`.
- "parent-child" relationships are currently overloaded: global defaults, workspace defaults, and
  runtime-spawned subagents can all feel like ancestry, but they are not the same kind of thing.

If Alan wants a truly Unix-like model, it should not treat all of these relationships as one tree.

The core design shift is:

- define agents as explicit on-disk roots
- define running agents as explicit instances
- make spawning behave like `exec`, not implicit inheritance
- move agent-facing global config into `~/.alan`

## Problem Statement

Today Alan has three different concepts mixed together:

1. configuration overlay
2. workspace context
3. runtime parent-child supervision

This creates two forms of ambiguity:

1. it is unclear whether a "child agent" inherits from a parent definition or only from a parent
   runtime invocation.
2. it is unclear which files represent the global agent definition, because `~/.alan` and
   `~/.config/alan` both participate.

The result is workable, but not yet clean:

- a subagent model risks becoming prompt inheritance instead of process spawning.
- global and workspace agent definitions are not described as one unified filesystem model.
- future agent orchestration, evaluation, and parallel execution would inherit conceptual debt.

## Goals

1. Make `Agent` a first-class object in Alan's model.
2. Make agent definition and agent execution clearly separate.
3. Make child agent startup behave like Unix `exec`:
   - fresh start from a resolved definition
   - no implicit inheritance from the parent instance
4. Make all agent-facing global configuration live under `~/.alan`.
5. Define a clear filesystem model for:
   - global base agent
   - workspace base agent
   - named agents
6. Define explicit spawn-time bindings for any shared context or capabilities.
7. Preserve room for future subagents, parallel execution, and agent evaluation without forcing
   prompt-based hacks.

## Non-Goals

1. Do not design skill packs in this phase.
2. Do not design multi-agent mesh or peer-to-peer agent communication.
3. Do not make child agents inherit parent tape, prompt, or memory by default.
4. Do not keep `~/.config/alan` as the main source of agent definition.
5. Do not collapse definition overlay and runtime supervision into one "parent" abstraction.

## Design Principles

1. Default to non-inheritance.
2. Use explicit binding instead of implicit sharing.
3. Treat agent startup as `exec`, not `fork`.
4. Keep definition layering separate from runtime supervision.
5. Keep one Alan home root: `~/.alan`.
6. Keep host-machine config separate from agent-definition config.

## Core Model

Alan should standardize on four first-class objects:

### 1. Agent Root

An `AgentRoot` is an on-disk definition directory.

It answers:

- who this agent is
- how it thinks
- which defaults it uses

It is analogous to:

- an executable image
- a program definition

### 2. Agent Instance

An `AgentInstance` is a running execution of an `AgentRoot`.

It answers:

- what this agent is doing now
- what task it is executing
- what runtime state it currently has

It is analogous to:

- a Unix process

### 3. Spawn Spec

A `SpawnSpec` is the explicit launch contract for a child instance.

It answers:

- what to execute
- which task to run
- which handles to bind
- which runtime overrides to apply

It is analogous to:

- `argv`
- `cwd`
- environment overrides
- file descriptor and mount bindings

### 4. Host Config

`HostConfig` is configuration for the local machine and transport host.

It answers:

- how Alan runs on this machine
- how clients and daemons connect

It is not part of agent definition.

## Relationship to the Turing Machine Metaphor

This design does not replace Alan's Turing machine metaphor. It clarifies where that metaphor
applies and where Unix-like process semantics apply instead.

The right mental model is two aligned layers:

### 1. Computation Layer: AI Turing Machine

At the computation layer, each running agent is still an AI Turing machine:

- `Tape` remains the tape
- LLM generation remains the transition function
- messages and tool calls remain the alphabet
- tool execution remains side effects
- halting remains the point at which the machine no longer emits new actions

At this layer, the important runtime object is still the bounded execution state represented by
session/tape state.

### 2. Hosting Layer: Unix-Like Runtime

At the hosting layer, Alan behaves like a Unix-like runtime:

- `AgentRoot` is the program image or executable definition
- `AgentInstance` is the running process
- `SpawnSpec` is the explicit launch contract, similar to `exec` inputs and bound handles
- runtime supervision forms a process tree

This hosting layer explains how multiple AI Turing machines are defined, launched, supervised, and
composed.

### Separation of Concerns

The two layers must stay separate:

- definition overlay is not runtime parent-child supervision
- runtime spawning is not tape inheritance
- child-agent startup should be modeled as `exec`, not as prompt-level cloning

This separation is what keeps the combined model elegant:

- the Turing machine metaphor explains single-agent computation
- the Unix-like process metaphor explains multi-agent hosting and orchestration

In short:

- Alan is a Unix-like runtime for AI Turing machines
- each `AgentInstance` is one running machine
- each child agent is a fresh machine execution unless the parent explicitly binds shared handles

## Two Separate Trees

Alan should explicitly model two different trees.

### 1. Definition Tree

The definition tree resolves an agent's effective definition by overlaying directories.

This is static and filesystem-based.

It answers:

- which base definitions apply
- which named agent overlay applies
- which defaults win

### 2. Runtime Tree

The runtime tree is the supervision tree of running agent instances.

This is dynamic and runtime-based.

It answers:

- which instance spawned which child
- who owns cancellation and join
- which results return to which parent

These two trees must not be conflated.

## Filesystem Layout

Alan should use one home root:

```text
~/.alan/
  agent/                  # global base agent root
    agent.toml
    persona/
    skills/
    policy.yaml
  agents/                 # global named agent roots
    reviewer/
      agent.toml
    benchmarker/
      agent.toml
  host.toml               # host/daemon/client machine config
  models.toml             # home-level model catalog overlay
  sessions/               # persisted session/run data
  state/                  # local state
  cache/                  # caches
```

Workspace layout should mirror the same shape:

```text
<workspace>/.alan/
  agent/                  # workspace base agent root
    agent.toml
    persona/
    skills/
    policy.yaml
  agents/                 # workspace named agent roots
    grader/
      agent.toml
    analyzer/
      agent.toml
  models.toml             # workspace model catalog overlay
  sessions/               # workspace session/run data
  memory/                 # workspace memory data
```

## Agent Root Contract

An `AgentRoot` should support at least:

- `agent.toml`
- `persona/`
- `skills/`
- `policy.yaml`

Optional future extensions can add more directories, but these are enough for the first formal
contract.

Suggested responsibility boundaries:

- `agent.toml`
  - provider and model
  - reasoning budget
  - tool defaults
  - memory behavior
  - compaction behavior
  - skill enable/disable defaults
  - references to local policy defaults
- `persona/`
  - persona and prompt fragments owned by this agent root
- `skills/`
  - agent-local skills that should be available when this root is active
- `policy.yaml`
  - default governance or execution boundary for this root

## Configuration Boundary

Alan should separate configuration into two classes.

### Agent-Facing Config

This belongs in `agent.toml`, not `~/.config/alan/config.toml`:

- LLM provider and model
- reasoning budget
- tool profile defaults
- policy defaults
- memory behavior
- compaction defaults
- persona and prompt defaults
- agent-level skill defaults

### Host-Facing Config

This belongs in `~/.alan/host.toml`:

- daemon bind address
- remote control or relay config
- CLI default endpoint
- local transport configuration
- client bundle path
- other machine-local runtime settings

This separation is important because the first class defines the agent, while the second defines
the host machine running Alan.

## Agent Resolution Contract

Agent resolution should be filesystem overlay, not runtime inheritance.

### Default Workspace Agent

When Alan starts in a workspace without an explicitly named agent:

```text
~/.alan/agent
-> <workspace>/.alan/agent
```

### Named Agent

When Alan resolves a named agent such as `reviewer`:

```text
~/.alan/agent
-> <workspace>/.alan/agent
-> ~/.alan/agents/reviewer           # if present
-> <workspace>/.alan/agents/reviewer # if present
-> spawn-time overrides
```

This is not parent-child ancestry. It is ordered overlay resolution.

The resulting object is a resolved `AgentRoot` definition that can be executed.

## Runtime Supervision Contract

Runtime supervision should apply only to `AgentInstance`s.

When a running instance spawns a child:

- the parent instance becomes the runtime supervisor
- the child instance is created from a resolved agent definition
- the child does not inherit the parent's runtime state unless explicitly bound

This means:

- global base agent is not the runtime parent
- workspace base agent is not the runtime parent
- named agent root is not the runtime parent
- only the spawning instance is the runtime parent

## Spawn Semantics

Alan should treat child-agent startup as `exec`, not `fork`.

That means:

- child instances start fresh from the resolved `AgentRoot`
- parent runtime state is not copied by default
- any shared state must be explicitly passed as a bound handle or explicit input

### Default Non-Inheritance

The following should not be inherited by default:

- parent tape
- parent active skills
- parent prompt state
- parent memory view
- parent plan state
- parent approval cache
- parent tool cache
- parent transient dynamic tools

### Explicitly Bound State

If the parent wants the child to access something, it should say so in the spawn request.

This is the core Unix-like simplification:

- no hidden inheritance
- no ambiguous ancestry
- explicit binding only

## Spawn Spec Contract

The initial `SpawnSpec` should support three classes of data.

### 1. Launch Inputs

- `task`
- `cwd`
- `workspace_root`
- `timeout`
- `budget`
- `output_dir`

### 2. Bound Handles

These are explicit capabilities or views the child may access:

- `workspace`
- `artifacts`
- `memory`
- `plan`
- `conversation_snapshot`
- `tool_results`
- `approval_scope`

### 3. Runtime Overrides

- model override
- policy override
- tool profile override
- other small execution-scoped settings

Example shape:

```text
spawn_agent(
  target = "grader",
  task = "Grade these outputs against the rubric",
  cwd = "<workspace>",
  handles = ["workspace", "artifacts"],
  overrides = {
    timeout = "5m",
    tool_profile = "read_only"
  }
)
```

This should be read as:

- execute the `grader` agent root
- on a fresh instance
- with this task
- in this working directory
- with only these handles bound

## Recommended Default Handle Policy

To keep the model strict and intuitive, the child should receive the smallest useful default set.

Recommended defaults:

- `workspace`
- `result_channel`

Everything else should require explicit binding.

For example:

- if the child needs a summary of parent conversation, pass `conversation_snapshot`
- if the child should update a shared plan, pass `plan`
- if the child should consume prior tool outputs, pass `tool_results`

This avoids "half-shared" hidden state and keeps child behavior testable.

## Desired End State

By the end of this work:

- `~/.alan` is the single Alan home root.
- `~/.alan/agent` is the global base agent root.
- workspace `.alan/agent` is the workspace base agent root.
- named agents live under `agents/<name>/`.
- agent definition uses filesystem overlay, not implicit parent-child inheritance.
- runtime parent-child exists only between running agent instances.
- child-agent startup is `exec`-like and defaults to non-inheritance.
- any shared context or capability is explicitly bound in the spawn request.

## Phase Plan

### PR1: Formalize the Model in Docs and Terminology

Goal: align Alan's language before changing implementation.

Changes:

- document `AgentRoot`, `AgentInstance`, `SpawnSpec`, and `HostConfig`
- document the difference between definition tree and runtime tree
- define `~/.alan` as Alan home
- deprecate the idea that `~/.config/alan` is the main agent-definition root

Files likely touched:

- `README.md`
- `AGENTS.md`
- `docs/architecture.md`
- new focused design doc under `docs/`

### PR2: Introduce Agent Root Filesystem Layout

Goal: establish the on-disk contracts without yet implementing full subagent runtime.

Changes:

- add `agent/` and `agents/` layout conventions
- introduce `agent.toml` as the canonical agent-definition file
- introduce `host.toml` as the canonical host-machine config file
- add migration path from `~/.config/alan/config.toml` to `~/.alan/agent/agent.toml`

Files likely touched:

- `crates/runtime/src/config.rs`
- `crates/alan/src/cli/init.rs`
- migration command paths in `crates/alan/src/cli/`

### PR3: Add Agent Resolution Layer

Goal: resolve an effective agent definition from filesystem overlays.

Changes:

- implement base-agent and named-agent resolution
- define overlay order for:
  - global base
  - workspace base
  - global named
  - workspace named
  - launch overrides
- keep this as a pure definition-layer mechanism

Files likely touched:

- new runtime agent-resolution module
- `crates/runtime/src/runtime/engine.rs`
- `crates/alan/src/daemon/runtime_manager.rs`

### PR4: Introduce Spawn Spec and Child Agent Runtime Primitive

Goal: create the first Alan-native subagent primitive.

Changes:

- add a runtime-native child-agent launch API
- start child instances from resolved agent roots
- define explicit launch inputs and bound handles
- ensure default non-inheritance
- add join/cancel/result semantics

Files likely touched:

- `crates/protocol/`
- `crates/runtime/src/runtime/`
- `crates/alan/src/daemon/`

### PR5: Add Explicit Shared Handle Types

Goal: make shared state formal instead of ad hoc.

Changes:

- define the first stable handle types
- formalize `conversation_snapshot`, `plan`, `artifacts`, and `workspace`
- ensure each handle has clear lifetime and ownership semantics

### PR6: Migrate Existing Agent-Facing Config Into Agent Roots

Goal: complete the conceptual move from standalone config to agent-root config.

Changes:

- move provider/model/tool/policy/memory settings into `agent.toml`
- leave only host-local settings in `host.toml`
- keep a migration path and explicit error messages for legacy config locations

## Open Questions

1. Should global named agents be able to override workspace base, or should workspace base always
   come last for local safety?
2. Which handles, if any, should be bound by default besides `workspace` and `result_channel`?
3. Should child agents be allowed to mount a read-only conversation snapshot by default, or should
   even that require an explicit binding?
4. How should `policy.yaml` compose across global base, workspace base, and named agent overlays?
5. Should the first child-agent primitive be local-only, or designed from day one to survive
   daemon restart and reconnect?

## Summary

Alan should move toward a process-oriented agent model:

- one Alan home
- explicit agent roots
- explicit agent instances
- explicit spawn specs
- explicit bound handles

The decisive simplification is:

- definition layering is overlay resolution
- runtime parent-child is supervision
- child startup is `exec`
- inheritance is opt-in, never implicit

That is the cleanest path to making Alan's agent model feel Unix-like instead of magical.
