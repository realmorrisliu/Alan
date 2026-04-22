# Alan Coding Eval Contract

> Status: executable V1 contract for steward orchestration harness,
> repo-worker harness, and package-local benchmark scaffolding.

## Goal

Define the minimum validation ladder for Alan's coding line so evaluation is
not reduced to a single repo-worker smoke loop or an ad hoc external benchmark.

The coding eval surface should distinguish:

1. parent-steward orchestration behavior,
2. repo-worker bounded execution behavior,
3. package-local benchmark scaffolding for operator-run comparisons,
4. future external benchmark adapters.

## Non-Goals

This contract does not:

1. redefine the steward / repo-worker product boundary,
2. replace the coding governance contract,
3. promise that external benchmark adapters are CI-blocking today,
4. require full transcript-level grading inside parent runtime prompts.

## Validation Ladder

### 1) Coding Steward Harness

The `coding_steward` suite validates parent-side orchestration behavior:

1. delegated launch contracts,
2. workspace-root versus nested-cwd binding,
3. default non-inheritance and explicit handle handoff,
4. bounded result integration into rollout and tape surfaces,
5. fail-safe behavior when delegated execution or artifact routing is
   unavailable.

This suite exists because repo-worker-only validation does not prove that Alan
itself is behaving like the home-root steward.

### 2) Repo-Worker Harness

The `repo_worker` suite validates the bounded repo-scoped child path:

1. minimum inspect -> plan -> edit -> verify -> deliver loop,
2. `steer` / `follow_up` / `next_turn` stability,
3. restart recovery and irreversible-effect dedupe continuity,
4. repo-worker governance boundary coverage.

### 3) Package-Local Benchmark Scaffold

The first-party `repo-coding` package should ship a manifest-first eval
scaffold under `evals/evals.json`.

That scaffold should cover at least:

1. selection checks for when `$repo-coding` should or should not activate,
2. bounded single-repo routing cases,
3. multi-repo orchestration cases that remain owned by the steward,
4. owner-boundary escalation cases.

This is a package-local authoring and evaluation surface. It is not part of
default runtime activation.

### 4) External Benchmark Adapters

External benchmark ladders such as SWE-bench-style task sets should be treated
as adapters on top of the package-local scaffold rather than as a replacement
for harness coverage.

The intended mapping is:

1. deterministic steward / repo-worker invariants live in harness,
2. package-local eval manifests hold comparison-oriented benchmark fixtures,
3. external benchmark adapters transform outside task corpora into those
   operator-side eval surfaces.

The recommended implementation order is Lite-first:

1. single-case `SWE-bench Lite` bring-up through the steward entrypoint,
2. curated Lite subset runs,
3. full Lite runs,
4. curated `SWE-bench Pro` expansion after the Lite path is stable.

## Current Executable Surfaces

Today the minimum executable coding eval surface should include:

1. `bash scripts/harness/run_coding_steward_suite.sh`
2. `bash scripts/harness/run_repo_worker_suite.sh`
3. `cargo run -p alan -- skills eval crates/runtime/skills/repo-coding`

The first external benchmark bring-up path is operator-run rather than
CI-blocking:

4. `bash crates/runtime/skills/repo-coding/scripts/run_swebench_full_steward_case.sh <case-json>`
5. `bash crates/runtime/skills/repo-coding/scripts/run_swebench_full_steward_subset.sh <suite-json>`
6. `bash crates/runtime/skills/repo-coding/scripts/score_swebench_predictions.sh <predictions-jsonl>`

For the Lite-first path, first-party scripts may also prepare clean benchmark
git workspaces and materialize Alan suite manifests from official dataset rows.
That workspace preparation step is separate from official Docker-backed
SWE-bench scoring and does not by itself promise identical host-native runtime
dependencies.

## Shared KPI Contract

Harness KPI output should keep these shared fields:

1. `suite`
2. `mode`
3. `total`
4. `passed`
5. `failed`
6. `skipped`
7. `pass_rate_percent`
8. `duration_secs`
9. `executed_scenarios`
10. `kpi_tag_counts`

Suite-specific fields may extend this, such as `profile` for autonomy.

## Relationship To Adjacent Contracts

1. `alan_coding_steward_contract.md` defines the steward / repo-worker product
   split.
2. `alan_coding_governance_contract.md` defines coding-specific owner
   boundaries.
3. `docs/harness/README.md` documents the executable harness surface.
4. `docs/harness/metrics/kpi.md` documents the current KPI artifact contract.

## Acceptance Criteria

This contract is satisfied when:

1. a steward-specific harness suite exists separately from repo-worker harness,
2. the first-party `repo-coding` package ships a manifest-first eval scaffold,
3. local docs describe the validation ladder in steward-first rather than
   repo-worker-only terms,
4. shared harness KPI output includes scenario lists and tag counts suitable
   for later aggregation,
5. external benchmark work is framed as an adapter layer on top of local eval
   surfaces rather than the only measure of coding quality,
6. the Lite-first adapter includes both single-case bring-up and curated-subset
   aggregation surfaces before any full-dataset expansion.
