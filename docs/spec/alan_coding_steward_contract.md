# Alan Coding Steward Contract

> Status: target product-boundary contract for Alan-as-steward and
> repo-scoped child coding workers.

## Goal

Alan's coding product must not be framed as a single-repo coding shell.

The intended model is:

1. Alan itself acts as the home-root coding steward.
2. Focused coding execution inside a repo or directory runs through fresh child
   runtimes.
3. Repo-scoped coding workers are product-layer compositions on top of the
   generic kernel, not a separate top-level app.

This document defines that boundary so later repo-worker, governance, and
evaluation work can build on one stable model.

## Non-Goals

This contract does not:

1. graduate the full repo-worker capability baseline,
2. define the complete coding governance policy,
3. define the external benchmark ladder or steward eval harness in detail,
4. turn `AgentRoot` overlay semantics into runtime parent-child inheritance.

## Design Principles

The coding line should stay aligned to these product principles:

1. **Benchmark results are measurement, not behavior definition.**
   External benchmark ladders may reveal real gaps, but they must not become
   the source of repo-specific heuristics, task-specific prompts, or
   benchmark-corpus special cases.
2. **The repo worker is a bounded Alan child, not a disposable patch bot.**
   A repo-scoped child runtime should preserve Alan's product strengths through
   explicit handles such as `plan`, `conversation_snapshot`, `tool_results`,
   and, when appropriate, `memory`.
3. **Improvements should target general coding consensus.**
   The right optimization surface is reusable coding behavior such as code
   understanding, change-boundary control, verification discipline, and honest
   delivery, not familiarity with one repository or evaluation set.
4. **Continuity must remain explicit and auditable.**
   Child inheritance is an explicit launch-contract choice rather than ambient
   parent-session inheritance.

## Stable Vocabulary

- **Coding steward**: the parent Alan runtime that owns goal intake, workspace
  discovery, routing, approvals, and result integration.
- **Repo worker**: a child runtime launched into a specific repo, directory, or
  project to perform bounded coding work.
- **Coding launch**: a `SpawnSpec` used specifically for repo-scoped coding
  execution.
- **Bound handles**: the explicit parent-side state the child receives, such as
  `workspace`, `plan`, or `conversation_snapshot`.
- **Bounded result integration**: the parent consumes the child's outcome
  through bounded summaries and runtime metadata rather than inheriting the full
  child transcript into the parent tape.

## Product Model

Alan's coding line should be read in this order:

1. the parent steward accepts the user's coding-oriented goal,
2. the parent discovers or selects the right workspace,
3. the parent launches one or more repo workers through explicit `SpawnSpec`
   contracts,
4. each repo worker performs bounded repo-scoped coding work,
5. the parent integrates results, handles approvals, and decides whether more
   routing or additional child launches are needed.

Important boundary:

1. runtime parent-child relations apply to `AgentInstance` launches,
2. `AgentRoot` overlays remain definition layering only,
3. the coding product must not blur those two models together.

## Parent Steward Responsibilities

The parent coding steward owns:

1. broad goal intake and clarification,
2. workspace discovery, comparison, and selection,
3. task routing across repos, directories, or projects,
4. launch-shape decisions for child runtimes,
5. approval ownership for risky cross-workspace or external actions,
6. result integration, dedupe, and follow-up planning,
7. deciding whether the task remains repo-local or has expanded into a broader
   orchestration problem.

The parent steward should not be treated as the place where every repo-local
edit loop runs by default.

## Repo-Scoped Child Worker Responsibilities

The child repo worker owns:

1. inspect -> plan -> edit -> verify -> deliver inside the delegated repo or
   directory,
2. repo-local side effects within the bound workspace scope,
3. maintaining a bounded coding transcript for the delegated task,
4. producing a clear delivery summary with verification and residual-risk
   status,
5. returning control when the task is complete, blocked, or attempting to
   expand beyond delegated scope.

The child worker should not silently broaden its workspace, approval, or
external-action scope beyond what the launch contract granted.
It should also be optimized for general repo-local coding quality rather than
for any one repository, benchmark corpus, or issue family.

## Minimum Repo-Worker Loop

The minimum repo-worker loop remains:

1. receive the delegated coding task,
2. plan and decompose the work into actionable steps,
3. apply code changes through tools,
4. run verification commands,
5. produce a delivery summary with what changed, what was verified, and
   residual risk.

This loop belongs to the repo worker, not to the parent steward.

## Coding Workflow Control Semantics

Inside coding workflows:

1. `steer` should re-plan the active coding loop quickly and may skip remaining
   safe steps when needed,
2. `follow_up` should queue additional coding intent for the immediate next
   cycle,
3. `next_turn` should queue future coding context without breaking the current
   turn's causality.

## Bounded Result Integration

The parent should consume child outcomes through:

1. the child terminal status,
2. output text and summary,
3. bounded runtime metadata such as child session id and rollout reference,
4. any explicit structured outputs added by higher-level product layers.

The parent should not depend on inheriting the child's full transcript into the
parent tape as the default integration mechanism.

## Coding Child Launch Contract

### Launch Target

Coding launches may target either:

1. a resolved on-disk agent root, or
2. a package-exported child-agent target.

The important constraint is freshness: a coding launch starts a fresh child
runtime rather than implicitly continuing the parent transcript.

### Recommended Launch Inputs

For coding workloads:

1. `launch.task` should describe the delegated coding objective and any hard
   constraints.
2. `launch.cwd` should point at the repo root or the narrower task-local
   directory where commands should execute.
3. `launch.workspace_root` should point at the repo or project root that defines
   the worker's writable boundary.
4. `launch.timeout_secs` should be set when the child should not hold the
   parent hostage indefinitely.
5. `launch.budget_tokens` is optional and should be used when the parent needs
   a bounded reasoning budget for the worker.

### Recommended Handle Profiles

#### Minimal Repo-Local Worker

Recommended handles:

1. `workspace`
2. `approval_scope`

Use this when the task is straightforward and the child can rediscover local
context cheaply.

#### Standard Coding Handoff

Recommended handles:

1. `workspace`
2. `approval_scope`
3. `plan`
4. `conversation_snapshot`

Use this when the parent has already clarified the task shape and the child
benefits from inheriting the current plan and a bounded conversation summary.

#### Verification Or Debugging Follow-Up

Recommended addition:

1. `tool_results`

Use this only when the child needs specific recent tool evidence that would be
expensive or lossy to recompute.

#### Durable Project Continuity

Recommended addition:

1. `memory`

Use this only when the repo-scoped child should share durable workspace memory
for that project. `memory` is not the default coding handoff.
This is how the coding child inherits Alan's durable project continuity when it
materially improves the task; it is not a benchmark-only escape hatch.

### Intentionally Not Inherited By Default

Coding launches should not implicitly inherit:

1. the full parent tape,
2. the parent's full active skill set,
3. the parent's dynamic-tool registry by default,
4. the parent's session identity,
5. artifact-routing behavior before artifact handles are implemented,
6. ambient trust outside the bound workspace.

### Relationship To The Delegated-Skill Default

The V1 delegated-skill path documents a narrow runtime-wide default launch
shape.

That default is intentionally conservative and should not be mistaken for the
full coding handoff recommendation in this document. Coding-oriented
orchestrators may opt into richer explicit handles such as `plan`,
`conversation_snapshot`, or `tool_results` when the parent has meaningful
context worth transferring.

## First-Party Package Integration Target

The repo worker should not remain in a long-lived top-level `reference/`
staging path.

The target repository shape is a first-party built-in skill package under
`crates/runtime/skills/`:

```text
crates/runtime/skills/repo-coding/
├── SKILL.md
├── skill.yaml
├── references/
├── evals/
├── scripts/
└── agents/
    └── repo-worker/
        ├── agent.toml
        ├── persona/
        ├── policy.yaml
        ├── skills/
        │   ├── decompose/SKILL.md
        │   ├── edit-verify/SKILL.md
        │   └── deliver/SKILL.md
        └── extensions/
            ├── code-index.yaml
            ├── test-analyzer.yaml
            └── pr-helper.yaml
```

Roles inside that package:

1. `repo-coding/SKILL.md` is the parent-facing capability package entry for
   repo-scoped coding work.
2. `repo-coding/skill.yaml` expresses the Alan-native delegated execution
   defaults for that package.
3. `repo-coding/agents/repo-worker/` is the package-local child launch target
   for bounded repo execution.
4. `references/`, `evals/`, and `scripts/` stay package-local authoring,
   validation, and harness surfaces rather than becoming a second product
   boundary.

Current productization rule:

1. the repo worker belongs only under the first-party package path
   `crates/runtime/skills/repo-coding/`,
2. no duplicate top-level `reference/` staging copy should remain.

## Minimal Execution Constraints

For repo-scoped coding execution:

1. unfinished coding runs should resume after restart through the normal
   checkpoint and session-recovery path,
2. irreversible side effects should remain dedupable through the runtime's
   effect-record and audit surfaces,
3. unknown side-effect status must fail safe and escalate rather than silently
   proceeding.

## Canonical Workflow Shapes

### Single-Repo Bug Fix

1. the parent steward receives a bug-fix request,
2. the parent selects the repo,
3. the parent launches one repo worker with `workspace`, `approval_scope`,
   `plan`, and `conversation_snapshot`,
4. the child performs inspect -> edit -> verify -> deliver,
5. the parent integrates the result and decides whether the task is done.

### Discovery-First Then Code

1. the parent steward first performs repo discovery or comparison,
2. the parent chooses the best target workspace,
3. only after that does the parent launch a repo worker into the chosen repo,
4. the child stays bounded to coding execution inside that workspace.

### Cross-Repo Task With Multiple Children

1. the parent steward breaks the broader objective into repo-scoped slices,
2. the parent launches separate repo workers for each repo or project,
3. each child returns a bounded outcome,
4. the parent reconciles sequencing, approvals, and final delivery across the
   multi-repo flow.

## Relationship To The Productized Repo Worker

The current repo worker belongs to the first-party package line under
`crates/runtime/skills/repo-coding/`.

Productization rule:

1. keep the coding boundary in this contract,
2. keep the repo worker and its child launch target under the package-native
   `crates/runtime/skills/repo-coding/` path,
3. keep smoke, harness, and related validation surfaces aligned to
   `repo-worker` naming,
4. do not keep a duplicate top-level `reference/` staging copy.

## Relationship To Governance And Evaluation

This contract intentionally stops at the product boundary.

Adjacent tracks:

1. coding governance belongs primarily to `alan_coding_governance_contract.md`,
   `hite_governance.md`, `governance_boundaries.md`, and the follow-on
   coding-governance issue,
2. steward and repo-worker evaluation belong to
   `alan_coding_eval_contract.md`, harness, and benchmark work tracked
   separately.

## Acceptance Criteria

This contract is satisfied when:

1. local docs clearly describe Alan coding as steward plus repo-scoped workers,
2. the coding child launch contract defines recommended launch inputs and handle
   profiles,
3. the minimum repo-worker loop and recovery constraints are defined without a
   second top-level coding spec,
4. the target first-party package layout for the repo worker is explicit,
5. local docs point at the productized repo-worker package path rather than a
   top-level `reference/` staging copy,
6. adjacent docs no longer imply that Alan coding is primarily a default
   single-repo shell,
7. local docs explicitly state that external benchmarks measure coding quality
   but do not define coding behavior,
8. local docs describe the repo worker as a bounded Alan child that may inherit
   explicit continuity handles, including optional `memory`.
