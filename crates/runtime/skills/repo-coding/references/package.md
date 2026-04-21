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
