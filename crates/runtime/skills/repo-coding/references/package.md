# Repo-Coding Package

This first-party package productizes Alan's repo-scoped coding worker under
`crates/runtime/skills/repo-coding/`.

It is not the full coding product boundary by itself. The steward/worker model
is defined in `docs/spec/alan_coding_steward_contract.md`.

## What this package contains

1. `SKILL.md` as the parent-facing entry for repo-scoped coding delegation.
2. `skill.yaml` declaring delegated execution through the package-local
   `repo-worker` child target.
3. `agents/repo-worker/` with the child agent root, coding micro-skills, and
   extension manifest examples.
4. External smoke and harness entrypoints under `scripts/repo-worker/` and
   `scripts/harness/`.

## Quick validation

1. `bash scripts/repo-worker/run_smoke.sh`
2. `bash scripts/harness/run_repo_worker_suite.sh --ci-blocking`
