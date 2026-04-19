# Repo-Worker Package Migration Plan (2026-04-19)

> Status: active implementation plan for `#283`; partially landed for package target, issue alignment, and path migration execution.

## Why This Plan Exists

This plan started from a top-level `reference/` staging path:

1. `reference/coding-agent/`
2. `scripts/reference/run_coding_reference_smoke.sh`
3. `scripts/harness/run_coding_reference_suite.sh`
4. `docs/harness/scenarios/coding/*`

That shape no longer matches the product and package model now defined by:

1. `docs/spec/alan_coding_steward_contract.md`
2. `docs/spec/skill_system_contract.md`
3. the steward vs child-worker boundary tracked in `#10` and `#282`

`#283` should therefore do more than harden the worker loop. It should also
formalize the worker as a first-party built-in package and remove the
transitional `reference/` staging path entirely.

## Scope

This batch covers:

1. migrating the repo worker into `crates/runtime/skills/repo-coding/`,
2. creating a package-local child launch target under `agents/repo-worker/`,
3. replacing the old `profile.toml` staging shape with package metadata plus an
   `agent.toml` child root,
4. renaming smoke, harness, and scenario paths away from `reference` naming,
5. updating docs and issue-facing references so the new structure is the only
   productized path.

This batch does not cover:

1. cross-workspace governance policy completion,
2. steward-level orchestration eval,
3. the full external benchmark ladder.

## Target Repository Shape

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

Target validation and harness naming:

1. `scripts/repo-worker/run_smoke.sh`
2. `scripts/harness/run_repo_worker_suite.sh`
3. `docs/harness/scenarios/repo_worker/*`
4. `just` task names aligned to `repo-worker`, not `coding-reference`

## Delivery Order

1. lock the package target and migration plan,
2. create the new first-party package shape,
3. migrate skills and launch-target assets,
4. migrate smoke and harness entrypoints,
5. update docs and issue references,
6. delete the top-level `reference/coding-agent/` path.

## Implementation Slices

### Slice 0: Plan Lock-In

Deliverables:

1. this plan,
2. `plans/README.md` update if needed.

Exit criteria:

1. the migration has one explicit target layout and naming plan.

### Slice 1: First-Party Package Skeleton

Files:

1. `crates/runtime/skills/repo-coding/SKILL.md`
2. `crates/runtime/skills/repo-coding/skill.yaml`
3. package-local support directories as needed

Required changes:

1. create the parent-facing package root,
2. define delegated execution metadata for the repo worker,
3. move any package-level references and eval assets out of `reference/`.

Exit criteria:

1. there is one first-party package root for repo coding,
2. `reference/coding-agent/profile.toml` is no longer the authoritative worker
   entry shape.

### Slice 2: Repo-Worker Child Root Migration

Files:

1. `crates/runtime/skills/repo-coding/agents/repo-worker/agent.toml`
2. child-root `skills/`
3. child-root `extensions/`
4. optional child persona / policy files

Required changes:

1. move `decompose`, `edit-verify`, and `deliver` into the child agent root,
2. move extension manifests into the child root,
3. make the worker launch target reflect the actual repo-scoped coding loop.

Exit criteria:

1. the repo worker is launchable from a package-local child root,
2. there is no need for a duplicate top-level staging directory.

### Slice 3: Smoke And Harness Migration

Files:

1. `scripts/repo-worker/run_smoke.sh`
2. `scripts/harness/run_repo_worker_suite.sh`
3. `docs/harness/scenarios/repo_worker/*`
4. `justfile`

Required changes:

1. rename smoke and harness entrypoints away from `reference` terminology,
2. update artifact roots and summary names,
3. keep the blocking subset stable during the rename.

Exit criteria:

1. the repo worker validation path no longer uses `reference` naming,
2. blocking harness coverage remains runnable.

### Slice 4: Docs, Issue, And Path Cleanup

Files:

1. docs that still mention `reference/coding-agent/`
2. issue-alignment comments or bodies when needed
3. the top-level `reference/coding-agent/` path

Required changes:

1. update remaining docs to the package-native path,
2. update issue language so `#283` explicitly owns reference removal,
3. delete the old staging directory after the new package path is live.

Exit criteria:

1. no repo-worker product path remains under top-level `reference/`,
2. docs and issue descriptions align to the new package-native structure.

## Review Heuristics

Review this batch against these questions:

1. Does the resulting layout match the skill-system contract rather than a
   custom one-off staging scheme?
2. Is there exactly one authoritative repo-worker package path after the
   migration?
3. Do smoke and harness entrypoints describe the productized worker rather than
   a reference implementation?
4. Does the migration preserve the clear steward vs child-worker split?
