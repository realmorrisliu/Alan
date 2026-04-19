# Repo-Worker Evals

Package-local repo-coding eval fixtures and review assets live here.

Current surfaces:

1. `evals.json` as the manifest-first entrypoint for `alan skills eval`.
2. `files/benchmark_cases.json` as deterministic steward-vs-legacy routing fixtures.
3. `../scripts/run_benchmark_fixture.sh` and `../scripts/grade_benchmark_case.sh`
   as package-local benchmark helpers.

Run the scaffold locally with:

```bash
cargo run -p alan -- skills eval crates/runtime/skills/repo-coding --output-dir target/skills-eval/repo-coding/latest
```

Cross-package harness scenarios remain under:

- `docs/harness/scenarios/repo_worker/`
- `docs/harness/scenarios/coding_steward/`
