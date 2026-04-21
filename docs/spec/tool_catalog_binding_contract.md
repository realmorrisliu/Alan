# Tool Catalog And Binding Contract

> Status: target runtime contract for tool identity, locality, execution
> binding, and exposure.

## Goals

This document defines how Alan should model tools so workspace routing,
child-runtime launches, and host/tool composition all follow one contract:

1. tool identity is stable across runtimes,
2. workspace binding is explicit runtime state rather than hidden tool state,
3. tool visibility is chosen separately from execution binding,
4. child runtimes derive their tool surface from the same catalog contract as
   launch roots.

## Non-Goals

This contract does not:

1. replace the capability-router/provider-routing design,
2. redefine dynamic tool registration payloads,
3. require all tools to support every locality,
4. promise an OS-level sandbox beyond the current execution backend.

## Stable Vocabulary

- **Tool catalog**: the stable set of tool definitions available to a runtime or
  host. A catalog entry defines `name`, `description`, parameter schema,
  capability classification, timeout hint, and locality.
- **Materialized tool instance**: the executable implementation object for one
  catalog entry. Materialization may allocate runtime helpers, but must not
  silently bind the instance to one workspace.
- **Tool locality**: whether the tool's semantics are global to the runtime or
  tied to the runtime's bound local workspace.
- **Tool execution binding**: the explicit runtime-owned binding supplied at
  execution time, including the current `cwd`, scratch area, and optional
  `workspace_root`.
- **Tool context**: the per-call execution object passed to tool
  implementations, containing the execution binding plus shared config.
- **Exposure profile**: the allowlisted subset of catalog entries visible to a
  given runtime.

## Core Rules

### 1. Catalog Entries Are Workspace-Agnostic

A tool catalog entry defines what the tool is, not where it executes.

Catalog metadata such as name, schema, description, timeout, capability, and
locality must stay stable across workspaces. Two runtimes bound to different
workspaces should still describe the same built-in tool catalog entry with the
same identity.

### 2. Locality Is Explicit

Every tool belongs to one of these coarse locality classes:

1. `global`: the tool is not implicitly tied to the runtime's bound workspace,
2. `workspace_local`: the tool acts on the runtime's currently bound local
   workspace.

Workspace-routing preflight only applies to `workspace_local` tools. A tool is
not treated as workspace-local merely because its JSON arguments contain fields
named `path`, `cwd`, `workspace_root`, or similar.

### 3. Execution Binding Is Runtime State, Not Tool State

Workspace roots, working directories, scratch directories, and similar
execution-site facts belong to runtime binding/context, not to catalog
identity.

Built-in tool constructors must therefore be workspace-agnostic. A runtime may
materialize the same built-in tool implementation once and execute it under
many different bindings over time, as long as the execution binding is
provided explicitly.

### 4. Workspace-Local Tools Require Explicit Workspace Binding

When executing a `workspace_local` tool, the runtime must provide an execution
binding whose `workspace_root` is explicit.

If both `workspace_root` and `cwd` are present:

1. `cwd` must be nested inside `workspace_root`,
2. path resolution must stay relative to the bound `cwd`,
3. execution backends must enforce the bound `workspace_root` rather than any
   hidden process-global default.

Running a workspace-local tool without an explicit workspace binding is a
runtime binding error, not a reason for the tool implementation to guess.

### 5. Exposure Profile Is Separate From Binding

Tool visibility answers "which tools may this runtime call", not "what
workspace are those tools bound to".

Changing a tool allowlist:

1. may add or remove visible catalog entries,
2. must not mutate execution binding,
3. must not require distinct per-workspace catalog identities for built-ins.

### 6. Child Runtimes Materialize From Catalog Plus Profile

Child-runtime tool surfaces must be derived from:

1. the shared tool catalog,
2. the child runtime's exposure profile,
3. the child runtime's own execution binding.

They must not depend on inheriting parent tool instances that already carry
workspace-specific state. Parent/child runtime differences are expressed
through exposure profiles and execution bindings, not by redefining the built-in
catalog per workspace.

### 7. Persistence And Audit Must Match Resolved Binding

When Alan persists delegated-launch requests, routing decisions, or audit
records, the stored workspace-related fields must match the resolved execution
binding that the child runtime will actually use.

Alan must not persist unresolved relative workspace fields and then depend on
process-local defaults at execution time.

## Relationship To Other Contracts

1. `kernel_contract.md` defines the one-workspace-per-instance invariant.
2. `workspace_routing_contract.md` defines when target-local work must route to
   a child runtime.
3. `capability_router.md` defines provider routing above the tool layer.
4. `governance_boundaries.md` defines policy and execution-backend boundaries.

## Acceptance Criteria

This contract is satisfied when:

1. built-in tool constructors no longer require a workspace path just to define
   the tool,
2. workspace-routing preflight consults explicit tool locality instead of
   argument-shape heuristics,
3. workspace-local tool execution receives runtime-owned binding/context with
   explicit `workspace_root`,
4. child runtimes can expose built-in tools from catalog/profile selection
   without inheriting parent-bound workspace state,
5. persisted launch metadata reflects the same workspace binding the runtime
   actually executes with.
