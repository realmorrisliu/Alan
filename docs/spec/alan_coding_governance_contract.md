# Alan Coding Governance Contract

> Status: partially implemented target contract for workspace-aware coding
> governance across the steward and repo-worker line.

## Goal

Define how Alan's coding governance should distinguish:

1. the home-root steward's discovery and routing actions,
2. the repo worker's bounded repo-local coding loop,
3. owner-boundary actions that must escalate or deny.

This document is coding-specific. It complements the general HITE governance
docs rather than replacing them.

## Non-Goals

This contract does not:

1. redefine the general HITE model,
2. promise a strict containment backend,
3. define the external benchmark ladder,
4. replace the repo-worker package contract.

## Governance Split

### Parent Steward Fast Path

The parent steward fast path should cover:

1. workspace discovery and comparison,
2. safe read-heavy repo selection,
3. planning and routing decisions,
4. spawn preparation and bounded result integration.

The parent steward should not silently mutate multiple repos or publish
externally under a generic "coding task" interpretation.

### Repo-Worker Fast Path

The child repo-worker fast path should cover:

1. repo-local reads and searches,
2. repo-local edits inside the bound workspace,
3. targeted deterministic verification,
4. bounded delivery summaries and residual-risk reporting.

The child worker fast path ends when the task attempts to cross trust,
workspace, credential, or publish boundaries.

## Verification Honesty

Coding governance should treat verification claims as owner-visible evidence,
not as persuasive copy.

Rules:

1. delivery summaries must derive from actual verification command outcomes,
2. `passed` verification may only be claimed when the cited commands actually
   exited successfully,
3. `mixed`, `environment_blocked`, `blocked`, `failed`, and `not_attempted`
   states must be reported explicitly rather than paraphrased as generic
   success,
4. residual risk must mention blocked environments, missing broader coverage,
   or failed checks when those conditions exist.

## Behavior-Preserving Change Policy

Repo-scoped coding work should prefer preserving surrounding product behavior
unless there is strong evidence that behavior itself must change.

Rules:

1. existing tests and nearby behavior guards should be treated as presumptive
   constraints,
2. the default repair shape is minimal product-code change plus focused
   regression coverage,
3. the worker should not weaken or rewrite existing tests merely to make a
   guessed patch pass,
4. modifying an existing test requires an explicit behavior-level reason, not
   only local convenience,
5. when the issue statement and existing tests appear to conflict, the worker
   should surface that discrepancy instead of silently normalizing one side
   away.

## Owner-Boundary Classes For Coding

Coding governance should escalate or deny at least these classes:

1. cross-workspace mutation beyond the delegated repo scope,
2. network or external publishing actions,
3. credential exploration or modification,
4. shared deploy or infrastructure changes,
5. destructive or ambiguous high-blast-radius actions,
6. unknown-capability tools whose real blast radius is unclear.

## Current Implementation Hooks

Today Alan can express part of this boundary through `policy.yaml` plus the
runtime policy engine.

Current matcher surface:

1. `tool`
2. `capability`
3. `match_command`
4. `match_path_prefix`

`match_path_prefix` is currently evaluated against common file-oriented
arguments such as `path`, `paths`, `directory`, `cwd`, and `workspace_root`.
Before matching, Alan lexically normalizes `.` / `..` segments and lets
relative policy prefixes still match absolute tool paths on component
boundaries.
When the runtime has a current tool `cwd`, relative path arguments are also
evaluated against that base so parent-traversal paths do not bypass policy.
Alan also case-folds path-prefix comparisons conservatively so case variants do
not bypass policy on case-insensitive hosts.

This does not make bash fully path-aware. For shell commands, `match_command`
remains the current mechanism.

## Repo-Worker Child Policy Guidance

The first-party repo worker should keep these defaults explicit in its child
policy:

1. unknown capability -> escalate,
2. network capability -> escalate,
3. publish commands -> escalate,
4. deploy and infrastructure commands -> escalate,
5. credential reads or writes -> escalate,
6. dangerous destructive commands -> deny.

Path-sensitive escalation is appropriate for files such as:

1. `.github/workflows/`
2. `deploy/`
3. `infra/`
4. `.env*`

## Known Gaps

The current implementation still has important limits:

1. bash payloads are not fully path-classified,
2. cross-workspace intent is still inferred mainly from launch shape and path
   guard rather than a dedicated policy dimension,
3. trust-boundary metadata such as `owner_boundary` or `blast_radius` is not
   yet modeled as first-class policy fields,
4. the current backend remains `workspace_path_guard`, which is best-effort
   rather than strict containment.

## Relationship To Other Contracts

1. `alan_coding_steward_contract.md` defines the product split between steward
   and repo worker.
2. `hite_governance.md` defines the broader target governance model.
3. `governance_boundaries.md` defines generic HITE boundary classes.
4. `governance_current_contract.md` remains the source of truth for current
   shipped behavior.
5. `alan_coding_eval_contract.md` defines the current validation ladder for
   steward orchestration and repo-worker coverage.

## Acceptance Criteria

This contract is satisfied when:

1. steward fast-path actions are described separately from repo-worker fast-path actions,
2. coding owner-boundary classes are explicit,
3. current policy matcher support and its limits are documented honestly,
4. the first-party repo-worker policy uses explicit rules for publish,
   credential, deploy, and unknown-capability boundaries,
5. tests and/or harness coverage exist for at least one path-sensitive coding
   governance rule,
6. local docs require verification claims to match real command evidence,
7. local docs state that existing tests are behavior constraints, not default
   rewrite targets.
