# Governance Current Contract

> Status: authoritative current implementation contract
>
> Scope: current Alan runtime / daemon / built-in tools. This document describes
> what the repository guarantees today. VNext target docs may describe broader
> HITE governance semantics and optional stronger containment, but they must not
> be read as statements about current behavior unless they explicitly say so.

## Purpose

This document pins the current governance semantics so design, docs, and code
can stay aligned while Alan evolves toward fuller HITE governance.

## Current Decisions

### 1. Policy File Resolution Is Override, Not Merge

Policy resolution order is:

1. `governance.policy_path`, if set
2. the highest-precedence existing `policy.yaml` in the resolved `AgentRoot` chain
3. builtin profile defaults

Default workspace agents resolve:

- `~/.alan/agent/policy.yaml -> {workspace}/.alan/agent/policy.yaml`

Named agents extend that chain with:

- `~/.alan/agents/<name>/policy.yaml -> {workspace}/.alan/agents/<name>/policy.yaml`

When a policy file is found, its `rules` and `default_action` replace the
builtin profile rule set for that session. Alan does not implicitly merge a
policy file with builtin profile rules.

Rationale:

- override semantics are predictable and testable
- implicit merge semantics would require extra precedence rules
- explicit inheritance can be added later if needed as a separate feature

### 2. `tool_escalation` Is Reserved For Policy Escalation

`tool_escalation` means one thing only: `PolicyEngine` returned `escalate` for a
tool call.

Other confirmation checkpoints must use their own types. In particular:

- replaying a side effect after an `unknown` prior result uses
  `effect_replay_confirmation`

Rationale:

- a checkpoint type should map to one semantic source
- audit logs and UI surfaces should distinguish policy boundaries from runtime
  safety checks

### 3. No Session-Scoped Approval Cache

Alan does not keep a session-wide approval cache for governance escalations.
Each `escalate` outcome yields an explicit confirmation request and each approval
applies only to the pending checkpoint being resumed.

Turn-local replay bookkeeping is still allowed for resuming the exact pending
tool call or tool batch. That bookkeeping is execution control, not an approval
policy.

Rationale:

- the V2 governance model is explicit `Yield` / `Resume`
- cached approvals blur auditability and make behavior harder to predict

### 4. The Current Sandbox Backend Is Best-Effort

The current built-in sandbox backend is `workspace_path_guard`.

It provides:

- workspace path containment checks
- protected subpath blocking for `.git`, `.alan`, and `.agents`
- conservative shell-shape validation for direct commands with statically
  addressable paths

It does **not** provide a strict OS sandbox, and it does **not** guarantee full
network or process isolation. It is a best-effort execution guard, not a hard
containment boundary.

Optional stronger containment backends may be added later for deployments that
need them, but they must be documented as separate backend levels and not
conflated with `workspace_path_guard`.

Current daemon session APIs report this host-side guard as
`execution_backend: "workspace_path_guard"` so clients can present the active
execution backend honestly without implying strict containment.

### 5. Current Policy Matchers Include Path Prefix Rules

Alan's current `policy.yaml` matcher surface includes:

1. `tool`
2. `capability`
3. `match_command`
4. `match_path_prefix`

`match_path_prefix` currently applies to common file-oriented arguments such as
`path`, `paths`, `directory`, `cwd`, and `workspace_root`.
Before matching, Alan lexically normalizes `.` / `..` segments and lets
relative policy prefixes still match absolute tool paths on component
boundaries.
When the runtime has a current tool `cwd`, relative path arguments are also
evaluated against that base so parent-traversal paths do not bypass policy.
Alan also case-folds path-prefix comparisons conservatively so case variants do
not bypass policy on case-insensitive hosts.

This is useful for coding-governance boundaries on sensitive paths such as
workflow, deploy, infrastructure, or credential files. It does not make bash
commands fully path-aware; shell payloads still rely on `match_command` plus
the execution backend's own path/shaping logic.

## Alignment Rules

- README and current-user docs must describe the semantics in this document.
- Code comments about current behavior must not claim stricter guarantees than
  this document.
- VNext / target docs must link here when their target state differs from the
  current implementation.
- Governance changes are incomplete until docs, behavior, and tests all match
  this contract.
