# Workspace Routing Contract

> Status: target product/runtime contract for routing local workspace-targeted
> tasks through fresh child runtimes instead of stretching one session across
> multiple workspaces.

## Goal

Define how Alan should behave when the current runtime is asked to inspect,
search, edit, or verify content in a different local workspace.

The intent is to preserve Alan's hosting model:

1. one running `AgentInstance` is bound to one workspace at a time,
2. parent stewards discover and route,
3. target-local execution happens in fresh child runtimes with explicit launch
   contracts.

## Non-Goals

This contract does not:

1. redefine general HITE governance semantics,
2. promise a strict OS sandbox,
3. replace the coding steward / repo-worker contract,
4. require every cross-workspace task to use the same child skill package.

## Stable Vocabulary

- **Current workspace**: the workspace bound to the active runtime.
- **Target workspace**: the local repo, project, or directory the user actually
  wants Alan to inspect or modify.
- **Workspace-targeted task**: a task whose local evidence or side effects are
  expected to happen inside the target workspace rather than the current
  workspace.
- **Routing failure**: the runtime tries to execute a target-workspace local
  action directly in the current workspace instead of delegating first.
- **Workspace-targeted child launch**: a fresh child runtime launched with an
  explicit `workspace_root` and optional nested `cwd`.

## Core Rules

### 1. One Runtime, One Workspace

An `AgentInstance` remains bound to one workspace for its lifetime.

Natural-language acknowledgements such as "already switched" or "now in
/path/to/repo" do not mutate the runtime binding. Workspace changes must happen
through explicit host/runtime control-plane actions such as new session
creation, fork, or child launch.

### 2. Stewards Route, Workers Execute

The parent steward fast path may handle:

1. workspace discovery,
2. workspace comparison,
3. safe read-heavy selection of the best target workspace,
4. preparation of an explicit child launch,
5. bounded result integration after the child finishes.

Once the task becomes target-local, the parent should not keep executing it
inline.

Target-local work includes:

1. repo-local reads and searches,
2. reading or explaining local docs in that target workspace,
3. repo-local edits,
4. target-local verification commands.

Workspace-routing preflight should only apply to workspace-local capabilities
such as repo-bound file/search tools and local shell execution. Host-provided
tools are not implicitly workspace-local merely because they accept fields
named `path`, `cwd`, `workspace_root`, or similar.

### 3. Workspace-Targeted Execution Requires A Fresh Child

When the current runtime determines that the user wants local work inside a
different workspace, it should launch a fresh child runtime with:

1. explicit `workspace_root`,
2. optional nested `cwd`,
3. bounded task text,
4. explicit handles such as `workspace` and `approval_scope`.

The child launch must not rely on implicit inheritance from the parent's
current workspace.
Relative launch paths must be resolved against the chosen target workspace
before the child launch is recorded or executed. If they cannot be resolved
unambiguously, the launch must fail instead of inheriting process-local working
directory semantics. When both `workspace_root` and `cwd` are present, `cwd`
must stay inside that `workspace_root`.

### 4. Structured Input Is Not Workspace Rebinding

`request_user_input` may collect additional facts such as:

1. which workspace the user intends,
2. which file or directory inside that workspace matters,
3. whether the task is read-only or may edit files.

It must not be treated as if the runtime binding itself changed.

### 5. Cross-Workspace Local Shell Is A Routing Failure

If a local shell command in the current runtime explicitly targets a path
outside the current workspace, that is a routing failure before it is a policy
question.

Expected behavior:

1. do not treat this as ordinary `tool_escalation`,
2. do not imply that user approval alone would make the current runtime the
   right execution site,
3. return a recoverable result that explains the task should be delegated to a
   workspace-targeted child runtime.

### 6. Policy Escalation And Routing Failure Must Stay Distinct

`tool_escalation` remains reserved for policy boundaries.

Workspace-routing failures should surface as their own recoverable execution
result with guidance such as:

1. the command targets another workspace,
2. the current runtime is still bound to the current workspace,
3. the next correct step is a delegated child launch.

### 7. Search Provenance Must Stay Honest

When the target workspace differs from the current workspace, target-local
search should run inside the delegated child rather than by searching parent
state directories.

Parent runtime state such as `.alan/memory/`, handoffs, and working-memory
artifacts must not be treated as target-workspace source material merely
because they contain echoed mentions of the target path or task text.

## Relationship To Other Contracts

1. `kernel_contract.md` defines the one-workspace-per-instance invariant.
2. `tool_catalog_binding_contract.md` defines tool locality, exposure, and
   runtime execution binding semantics.
3. `alan_coding_steward_contract.md` defines the steward / repo-worker product
   split for coding execution.
4. `alan_coding_governance_contract.md` defines coding-specific fast paths and
   owner-boundary classes.
5. `governance_boundaries.md` defines the generic HITE boundary model.
6. `governance_current_contract.md` remains the source of truth for shipped
   semantics until implementation catches up.

## Acceptance Criteria

This contract is satisfied when:

1. natural-language replies cannot silently rebind the active runtime to a
   different workspace,
2. target-local reads/searches/edits/verification in another workspace route
   through explicit child launches,
3. cross-workspace local shell attempts no longer fall into ordinary policy
   escalation by default,
4. user-visible errors distinguish routing failures from policy escalation,
5. target-local search no longer falls back to parent `.alan` state as if it
   were target-workspace source material.
