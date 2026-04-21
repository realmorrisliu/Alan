# Repo-Coding Package

This first-party package productizes Alan's repo-scoped coding worker under
`crates/runtime/skills/repo-coding/`.

It is not the full coding product boundary by itself. The steward/worker model
is defined in `docs/spec/alan_coding_steward_contract.md`.

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
6. `evals/evals.json` plus `evals/files/benchmark_cases.json` for manifest-first
   benchmark scaffolding.
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
