# Repo-Coding Package

This first-party package productizes Alan's repo-scoped coding worker under
`crates/runtime/skills/repo-coding/`.

It is not the full coding product boundary by itself. The steward/worker model
is defined in `docs/spec/alan_coding_steward_contract.md`.

## Package Principles

This package should make Alan better at general repo-local coding work. It
should not evolve into a SWE-bench adapter with product-specific behavior.

Rules:

1. The parent Alan runtime remains the coding steward; this package provides
   the bounded repo-worker path inside that broader stewardship model.
2. Any improvements motivated by benchmark findings should be reusable coding
   improvements that help normal repository work as well.
3. Explicit continuity handles handed down by the steward, including optional
   memory, are for project continuity and task execution quality, not for
   benchmark-only escape hatches.
4. Verification claims and delivery summaries produced through this package
   must remain evidence-backed and behavior-preserving.

## What this package contains

1. `SKILL.md` as the parent-facing entry for repo-scoped coding delegation.
2. `skill.yaml` declaring delegated execution through the package-local
   `repo-worker` child target.
3. `agents/openai.yaml` with package-level compatibility metadata for catalog
   display and activation hints.
4. `agents/repo-worker/` with the child agent root, coding micro-skills, and
   extension manifest examples.
5. `references/delivery_contract.md` and `references/evaluator_boundary.md`
   describing the bounded output contract and conditional evaluator boundary.
6. `evals/evals.json`, `evals/files/benchmark_cases.json`, and
   `evals/files/delivery_contract_*.json` for manifest-first benchmark and
   delivery-contract scaffolding.
7. `scripts/` with deterministic validators and benchmark helpers.
8. External smoke and harness entrypoints under `scripts/repo-worker/` and
   `scripts/harness/`.

## Quick validation

1. `bash scripts/repo-worker/run_smoke.sh`
2. `bash scripts/harness/run_repo_worker_suite.sh --ci-blocking`
3. `bash scripts/harness/run_coding_steward_suite.sh --ci-blocking`
4. `cargo run -p alan -- skills eval crates/runtime/skills/repo-coding --output-dir target/skills-eval/repo-coding/latest`

## External Benchmark Bring-Up

The Lite-first full-steward external benchmark path starts with a single
operator-run case:

1. prepare one clean `SWE-bench Lite` workspace checkout,
2. prepare the benchmark problem statement text,
3. fill in `evals/files/swebench_lite_case.template.json`,
4. run:

```bash
bash crates/runtime/skills/repo-coding/scripts/run_swebench_full_steward_case.sh \
  crates/runtime/skills/repo-coding/evals/files/swebench_lite_case.template.json
```

That runner should:

1. use the steward runtime as the benchmark entrypoint,
2. require repo-local work to happen through delegated child launch,
3. export `model.patch` plus single-case prediction artifacts,
4. emit Alan-native orchestration metadata alongside the benchmark patch.

Those artifacts measure transfer quality for Alan's coding line. They should
never become the source of benchmark-corpus-specific runtime rules.

The next bring-up step is a curated subset aggregator:

```bash
python3 crates/runtime/skills/repo-coding/scripts/prepare_swebench_lite_workspaces.py \
  --instance-ids-file crates/runtime/skills/repo-coding/evals/files/swebench_lite_pilot_v1.ids.txt \
  --dataset-name princeton-nlp/SWE-bench_Lite \
  --workspace-root target/benchmarks/swebench_lite/workspaces/pilot_v1

python3 crates/runtime/skills/repo-coding/scripts/prepare_swebench_lite_subset.py \
  --instance-ids-file crates/runtime/skills/repo-coding/evals/files/swebench_lite_pilot_v1.ids.txt \
  --dataset-name princeton-nlp/SWE-bench_Lite \
  --workspace-root target/benchmarks/swebench_lite/workspaces/pilot_v1 \
  --output-dir target/benchmarks/swebench_lite/manifests/pilot_v1

bash crates/runtime/skills/repo-coding/scripts/run_swebench_full_steward_subset.sh \
  target/benchmarks/swebench_lite/manifests/pilot_v1/suite.json
```

That suite runner should:

1. execute each Lite case through the same steward entrypoint,
2. aggregate suite-level `predictions.jsonl`,
3. emit `run.json`, `benchmark.json`, `kpi.json`, and `case_results.jsonl`,
4. surface orchestration counters such as `spawn_count` and `escalation_count`
   inside the run artifacts,
5. generate `score_with_official_harness.sh` by delegating to the package-local
   `score_swebench_predictions.sh` wrapper.

The workspace preparation script intentionally materializes only clean git
checkouts at each case's `base_commit`. Official resolved/unresolved scoring
still happens through the Docker-backed SWE-bench harness, while host-native
dependency provisioning remains operator-owned.

At the owned-output level, reruns are recoverable: stale or partial workspace
directories are recreated automatically, while `--reuse-existing-workspaces`
keeps already-clean matching checkouts in place.
